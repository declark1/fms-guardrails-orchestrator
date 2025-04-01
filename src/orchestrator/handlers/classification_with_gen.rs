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

use std::{collections::HashMap, sync::Arc};

use http::HeaderMap;
use opentelemetry::trace::TraceId;
use tracing::{error, info};

use crate::{
    clients::GenerationClient,
    models::{
        ClassifiedGeneratedTextResult, DetectionWarning, DetectorParams, GuardrailsConfig,
        GuardrailsHttpRequest, GuardrailsTextGenerationParameters,
        TextGenTokenClassificationResults,
    },
    orchestrator::{Context, Error, Orchestrator, common},
};

use super::Handle;

impl Handle<ClassificationWithGenTask> for Orchestrator {
    type Response = ClassifiedGeneratedTextResult;

    async fn handle(&self, task: ClassificationWithGenTask) -> Result<Self::Response, Error> {
        let ctx = self.ctx.clone();
        let trace_id = task.trace_id;
        let input_detectors = task.guardrails_config.input_detectors().cloned();
        let output_detectors = task.guardrails_config.output_detectors().cloned();
        info!(%trace_id, "task started");

        // TODO: validate requested guardrails

        if let Some(detectors) = input_detectors {
            // Handle input detection
            match handle_input_detection(ctx.clone(), &task, detectors).await {
                Ok(Some(response)) => {
                    info!(%trace_id, "task completed: returning response with input detections");
                    // Return response with input detections and terminate
                    return Ok(response);
                }
                Ok(None) => (), // No input detections
                Err(error) => {
                    // Input detections failed
                    return Err(error);
                }
            }
        }

        // Handle generation
        let client = ctx
            .clients
            .get_as::<GenerationClient>("generation")
            .unwrap();
        let generation = common::generate(
            client,
            task.headers.clone(),
            task.model_id.clone(),
            task.inputs.clone(),
            task.text_gen_parameters.clone(),
        )
        .await?;

        if let Some(detectors) = output_detectors {
            // Handle output detection
            handle_output_detection(ctx.clone(), task, detectors, generation).await
        } else {
            // No output detectors, return generation
            info!(%trace_id, "task completed: returning generation response");
            Ok(generation)
        }
    }
}

async fn handle_input_detection(
    ctx: Arc<Context>,
    task: &ClassificationWithGenTask,
    detectors: HashMap<String, DetectorParams>,
) -> Result<Option<ClassifiedGeneratedTextResult>, Error> {
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
        Ok((_, detections)) => detections,
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
        let response = ClassifiedGeneratedTextResult {
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
    task: ClassificationWithGenTask,
    detectors: HashMap<String, DetectorParams>,
    generation: ClassifiedGeneratedTextResult,
) -> Result<ClassifiedGeneratedTextResult, Error> {
    let trace_id = task.trace_id;
    let generated_text = generation.generated_text.clone().unwrap_or_default();
    let detections = match common::text_contents_detections(
        ctx,
        task.headers,
        detectors,
        0,
        vec![(0, generated_text)],
    )
    .await
    {
        Ok((_, detections)) => detections,
        Err(error) => {
            error!(%trace_id, %error, "task failed: error processing input detections");
            return Err(error);
        }
    };
    let mut response = generation;
    if !detections.is_empty() {
        response.token_classification_results.output = Some(detections.into());
        response.warnings = Some(vec![DetectionWarning::unsuitable_output()]);
    }
    info!(%trace_id, "task completed: returning response with output detections");
    Ok(response)
}

#[derive(Debug)]
pub struct ClassificationWithGenTask {
    pub trace_id: TraceId,
    pub model_id: String,
    pub inputs: String,
    pub guardrails_config: GuardrailsConfig,
    pub text_gen_parameters: Option<GuardrailsTextGenerationParameters>,
    pub headers: HeaderMap,
}

impl ClassificationWithGenTask {
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
