/*
 Copyright FMS Guardrails Orchestrator Authors

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

     http://www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.

*/

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use futures::StreamExt;
use http::HeaderMap;
use opentelemetry::trace::TraceId;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

use super::Handle;
use crate::{
    clients::GenerationClient,
    models::{
        ClassifiedGeneratedTextStreamResult, DetectionWarning, DetectorParams, GuardrailsConfig,
        GuardrailsHttpRequest, GuardrailsTextGenerationParameters,
        TextGenTokenClassificationResults,
    },
    orchestrator::{
        Context, Error, Orchestrator, common,
        types::{
            Chunk, DetectionBatchStream, DetectionStream, Detections, GenerationStream,
            MaxProcessedIndexBatcher,
        },
    },
};

impl Handle<StreamingClassificationWithGenTask> for Orchestrator {
    type Response = ReceiverStream<Result<ClassifiedGeneratedTextStreamResult, Error>>;

    async fn handle(
        &self,
        task: StreamingClassificationWithGenTask,
    ) -> Result<Self::Response, Error> {
        let ctx = self.ctx.clone();

        // Create response channel
        let (response_tx, response_rx) =
            mpsc::channel::<Result<ClassifiedGeneratedTextStreamResult, Error>>(128);

        tokio::spawn(async move {
            let trace_id = task.trace_id;
            let input_detectors = task.guardrails_config.input_detectors().cloned();
            let output_detectors = task.guardrails_config.output_detectors().cloned();
            info!(%trace_id, "task started");

            // TODO: validate requested guardrails

            if let Some(detectors) = input_detectors {
                // Handle input detection
                match handle_input_detection(ctx.clone(), &task, detectors.clone()).await {
                    Ok(Some(response)) => {
                        info!(%trace_id, "task completed: returning response with input detections");
                        // Send message with input detections to response channel and terminate
                        let _ = response_tx.send(Ok(response)).await;
                        return;
                    }
                    Ok(None) => (), // No input detections
                    Err(error) => {
                        // Input detections failed
                        // Send error to response channel and terminate
                        let _ = response_tx.send(Err(error)).await;
                        return;
                    }
                }
            }

            // Create generation stream
            let client = ctx
                .clients
                .get_as::<GenerationClient>("generation")
                .unwrap();
            let generation_stream = match common::generate_stream(
                client,
                task.headers.clone(),
                task.model_id.clone(),
                task.inputs.clone(),
                task.text_gen_parameters.clone(),
            )
            .await
            {
                Ok(stream) => stream,
                Err(error) => {
                    error!(%trace_id, %error, "task failed: error creating generation stream");
                    // Send error to response channel and terminate
                    let _ = response_tx.send(Err(error)).await;
                    return;
                }
            };

            if let Some(detectors) = output_detectors {
                // Handle output detection
                handle_output_detection(
                    ctx.clone(),
                    task,
                    detectors,
                    generation_stream,
                    response_tx,
                )
                .await;
            } else {
                // No output detectors, forward generation stream to response stream
                forward_generation_stream(trace_id, generation_stream, response_tx).await;
            }
        });

        Ok(ReceiverStream::new(response_rx))
    }
}

async fn handle_input_detection(
    ctx: Arc<Context>,
    task: &StreamingClassificationWithGenTask,
    detectors: HashMap<String, DetectorParams>,
) -> Result<Option<ClassifiedGeneratedTextStreamResult>, Error> {
    let trace_id = task.trace_id;
    let inputs = common::apply_masks(task.inputs.clone(), task.guardrails_config.input_masks());
    let detections = match common::text_contents_detections(
        ctx.clone(),
        task.headers.clone(),
        detectors.clone(),
        0,
        inputs,
    )
    .await
    {
        Ok((_input_id, detections)) => detections,
        Err(error) => {
            error!(%trace_id, %error, "task failed: error processing input detections");
            return Err(error);
        }
    };
    if !detections.is_empty() {
        // Get token count
        let client = ctx
            .clients
            .get_as::<GenerationClient>("generation")
            .unwrap();
        let input_token_count = match common::tokenize(
            client,
            task.headers.clone(),
            task.model_id.clone(),
            task.inputs.clone(),
        )
        .await
        {
            Ok((token_count, _tokens)) => token_count,
            Err(error) => {
                error!(%trace_id, %error, "task failed: error tokenizing input text");
                return Err(error);
            }
        };
        // Build response with input detections
        let response = ClassifiedGeneratedTextStreamResult {
            input_token_count,
            token_classification_results: TextGenTokenClassificationResults {
                input: Some(detections.into()),
                output: None,
            },
            warnings: Some(vec![DetectionWarning::unsuitable_input()]),
            ..Default::default()
        };
        Ok(Some(response))
    } else {
        // No input detections
        Ok(None)
    }
}

async fn handle_output_detection(
    ctx: Arc<Context>,
    task: StreamingClassificationWithGenTask,
    detectors: HashMap<String, DetectorParams>,
    mut generation_stream: GenerationStream,
    response_tx: mpsc::Sender<Result<ClassifiedGeneratedTextStreamResult, Error>>,
) {
    let trace_id = task.trace_id;
    // Create input channel for detection pipeline
    let (input_tx, input_rx) = mpsc::channel(128);
    // Create shared generations
    let generations: Arc<RwLock<Vec<ClassifiedGeneratedTextStreamResult>>> =
        Arc::new(RwLock::new(Vec::new()));
    // Create detection streams
    let detection_streams = common::text_contents_detection_streams(
        ctx,
        task.headers.clone(),
        detectors.clone(),
        0,
        input_rx,
    )
    .await;

    // Spawn task to process detection streams
    tokio::spawn({
        let generations = generations.clone();
        async move {
            match detection_streams {
                Ok(mut detection_streams) if detection_streams.len() == 1 => {
                    // Process single detection stream, batching not applicable
                    let detection_stream = detection_streams.swap_remove(0);
                    process_detection_stream(trace_id, generations, detection_stream, response_tx)
                        .await;
                }
                Ok(detection_streams) => {
                    // Create detection batch stream
                    let detection_batch_stream = DetectionBatchStream::new(
                        MaxProcessedIndexBatcher::new(detectors.len()),
                        detection_streams,
                    );
                    process_detection_batch_stream(
                        trace_id,
                        generations,
                        detection_batch_stream,
                        response_tx,
                    )
                    .await;
                }
                Err(error) => {
                    error!(%trace_id, %error, "task failed: error creating detection streams");
                    // Send error to response channel and terminate
                    let _ = response_tx.send(Err(error)).await;
                }
            }
        }
    });

    // Spawn task to consume generations
    tokio::spawn(async move {
        while let Some((index, result)) = generation_stream.next().await {
            match result {
                Ok(generation) => {
                    // Send generated text to input channel
                    let input = (index, generation.generated_text.clone().unwrap_or_default());
                    let _ = input_tx.send(Ok(input)).await;
                    // Update shared generations
                    generations.write().unwrap().push(generation);
                }
                Err(error) => {
                    // Send error to input channel
                    let _ = input_tx.send(Err(error)).await;
                    // TODO: catch generation errors here to terminate all tasks?
                }
            }
        }
    });
}

/// Consumes a generation stream, forwarding messages to a response channel.
async fn forward_generation_stream(
    trace_id: TraceId,
    mut generation_stream: GenerationStream,
    response_tx: mpsc::Sender<Result<ClassifiedGeneratedTextStreamResult, Error>>,
) {
    while let Some((_index, result)) = generation_stream.next().await {
        match result {
            Ok(generation) => {
                // Send message to response channel
                if response_tx.send(Ok(generation)).await.is_err() {
                    info!(%trace_id, "task completed: client disconnected");
                    return;
                }
            }
            Err(error) => {
                error!(%trace_id, %error, "task failed: error received from generation stream");
                // Send error to response channel and terminate
                let _ = response_tx.send(Err(error)).await;
                return;
            }
        }
    }
    info!(%trace_id, "task completed: generation stream closed");
}

/// Consumes a detection stream, builds responses, and sends them to a response channel.
async fn process_detection_stream(
    trace_id: TraceId,
    generations: Arc<RwLock<Vec<ClassifiedGeneratedTextStreamResult>>>,
    mut detection_stream: DetectionStream,
    response_tx: mpsc::Sender<Result<ClassifiedGeneratedTextStreamResult, Error>>,
) {
    while let Some(result) = detection_stream.next().await {
        match result {
            Ok((_, _detector_id, chunk, detections)) => {
                // Create response for this batch with output detections
                let response = output_detection_response(&generations, chunk, detections).unwrap();
                // Send message to response channel
                if response_tx.send(Ok(response)).await.is_err() {
                    info!(%trace_id, "task completed: client disconnected");
                    return;
                }
            }
            Err(error) => {
                error!(%trace_id, %error, "task failed: error received from detection stream");
                // Send error to response channel and terminate
                let _ = response_tx.send(Err(error)).await;
                return;
            }
        }
    }
    info!(%trace_id, "task completed: detection stream closed");
}

/// Consumes a detection batch stream, builds responses, and sends them to a response channel.
async fn process_detection_batch_stream(
    trace_id: TraceId,
    generations: Arc<RwLock<Vec<ClassifiedGeneratedTextStreamResult>>>,
    mut detection_batch_stream: DetectionBatchStream<MaxProcessedIndexBatcher>,
    response_tx: mpsc::Sender<Result<ClassifiedGeneratedTextStreamResult, Error>>,
) {
    while let Some(result) = detection_batch_stream.next().await {
        match result {
            Ok((chunk, detections)) => {
                // Create response for this batch with output detections
                let response = output_detection_response(&generations, chunk, detections).unwrap();
                // Send message to response channel
                if response_tx.send(Ok(response)).await.is_err() {
                    info!(%trace_id, "task completed: client disconnected");
                    return;
                }
            }
            Err(error) => {
                error!(%trace_id, %error, "task failed: error received from detection batch stream");
                // Send error to response channel and terminate
                let _ = response_tx.send(Err(error)).await;
                return;
            }
        }
    }
    info!(%trace_id, "task completed: detection batch stream closed");
}

/// Builds a response with output detections.
fn output_detection_response(
    generations: &Arc<RwLock<Vec<ClassifiedGeneratedTextStreamResult>>>,
    chunk: Chunk,
    detections: Detections,
) -> Result<ClassifiedGeneratedTextStreamResult, Error> {
    // Get subset of generations relevant for this chunk
    let generations_slice = generations
        .read()
        .unwrap()
        .get(chunk.input_start_index..=chunk.input_end_index)
        .unwrap_or_default()
        .to_vec();
    let last = generations_slice.last().cloned().unwrap_or_default();
    let tokens = generations_slice
        .iter()
        .flat_map(|generation| generation.tokens.clone().unwrap_or_default())
        .collect::<Vec<_>>();
    let mut response = ClassifiedGeneratedTextStreamResult {
        generated_text: Some(chunk.text),
        start_index: Some(chunk.start as u32),
        processed_index: Some(chunk.end as u32),
        tokens: Some(tokens),
        ..last
    };
    response.token_classification_results.output = Some(detections.into());
    if chunk.input_start_index == 0 {
        // Get input_token_count and seed from first generation message
        let first = generations_slice.first().unwrap();
        response.input_token_count = first.input_token_count;
        response.seed = first.seed;
        // Get input_tokens from second generation message (if specified)
        response.input_tokens = if let Some(second) = generations_slice.get(1) {
            second.input_tokens.clone()
        } else {
            Some(Vec::default())
        };
    }
    Ok(response)
}

#[derive(Debug)]
pub struct StreamingClassificationWithGenTask {
    pub trace_id: TraceId,
    pub model_id: String,
    pub inputs: String,
    pub guardrails_config: GuardrailsConfig,
    pub text_gen_parameters: Option<GuardrailsTextGenerationParameters>,
    pub headers: HeaderMap,
}

impl StreamingClassificationWithGenTask {
    pub fn new(trace_id: TraceId, request: GuardrailsHttpRequest, headers: HeaderMap) -> Self {
        Self {
            trace_id,
            model_id: request.model_id,
            inputs: request.inputs,
            guardrails_config: request.guardrail_config.unwrap_or_default(),
            text_gen_parameters: request.text_gen_parameters,
            headers,
        }
    }
}
