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
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::HeaderMap;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};
use uuid::Uuid;

use super::{
    ChatCompletionsDetectionTask, Context, Error, Orchestrator, UNSUITABLE_OUTPUT_MESSAGE,
};
use crate::{
    clients::{
        chunker::MODEL_ID_HEADER_NAME,
        detector::{ChatDetectionRequest, ContentAnalysisRequest},
        openai::{
            ChatCompletion, ChatCompletionChoice, ChatCompletionsRequest, ChatCompletionsResponse,
            ChatDetections, Content, DetectionResult, OpenAiClient, OrchestratorWarning, Role,
        },
    },
    config::DetectorType,
    models::{DetectionWarningReason, DetectorParams, GuardrailDetection},
    orchestrator::{
        detector_processing::content, get_chunker_ids, unary, Chunk, UNSUITABLE_INPUT_MESSAGE,
    },
};

/// Internal structure to capture chat messages (both request and response)
/// and prepare it for processing
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessageInternal {
    /// Index of the message
    pub message_index: usize,
    /// The role of the messages author.
    pub role: Role,
    /// The contents of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    /// The refusal message by the assistant. (assistant message only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

pub enum DetectorRequest {
    ContentAnalysisRequest(ContentAnalysisRequest),
    ChatDetectionRequest(ChatDetectionRequest),
}

impl From<&ChatCompletionsRequest> for Vec<ChatMessageInternal> {
    fn from(value: &ChatCompletionsRequest) -> Self {
        value
            .messages
            .iter()
            .enumerate()
            .map(|(index, message)| ChatMessageInternal {
                message_index: index,
                role: message.role.clone(),
                content: message.content.clone(),
                refusal: message.refusal.clone(),
            })
            .collect()
    }
}

impl From<&Box<ChatCompletion>> for Vec<ChatMessageInternal> {
    fn from(value: &Box<ChatCompletion>) -> Self {
        value
            .choices
            .iter()
            .map(|choice| ChatMessageInternal {
                message_index: choice.index,
                role: choice.message.role.clone(),
                content: Some(Content::Text(
                    choice.message.content.clone().unwrap_or_default(),
                )),
                refusal: choice.message.refusal.clone(),
            })
            .collect()
    }
}

impl From<ChatCompletionChoice> for Vec<ChatMessageInternal> {
    fn from(value: ChatCompletionChoice) -> Self {
        vec![ChatMessageInternal {
            message_index: value.index,
            role: value.message.role,
            content: Some(Content::Text(value.message.content.unwrap_or_default())),
            refusal: value.message.refusal,
        }]
    }
}

impl Orchestrator {
    #[instrument(skip_all, fields(trace_id = ?task.trace_id, headers = ?task.headers))]
    pub async fn handle_chat_completions_detection(
        &self,
        task: ChatCompletionsDetectionTask,
    ) -> Result<ChatCompletionsResponse, Error> {
        info!("handling chat completions detection task");
        let ctx = self.ctx.clone();
        let handle = if task.request.stream {
            tokio::spawn(handle_streaming(ctx, task))
        } else {
            tokio::spawn(handle_unary(ctx, task))
        };
        match handle.await {
            // Task completed successfully
            Ok(Ok(response)) => Ok(response),
            // Task failed, return error propagated from child task that failed
            // TODO: log error here?
            Ok(Err(error)) => Err(error),
            // Task cancelled or panicked
            Err(error) => Err(error.into()),
        }
    }
}

async fn handle_unary(
    ctx: Arc<Context>,
    task: ChatCompletionsDetectionTask,
) -> Result<ChatCompletionsResponse, Error> {
    let detectors = task.request.detectors.clone().unwrap_or_default();
    let headers = task.headers;
    // Handle input detection
    let input_detections = match detectors.input {
        Some(detectors) if !detectors.is_empty() => {
            let chunker_ids = get_chunker_ids(&ctx, &detectors)?;
            let messages = Vec::<ChatMessageInternal>::from(&task.request);
            let messages = content::filter_chat_messages(&messages)?;
            let chunks = chunk(ctx.clone(), chunker_ids, messages).await?;
            let detections = detect(ctx.clone(), headers.clone(), detectors, chunks).await?;
            (!detections.is_empty()).then_some(detections)
        }
        _ => None,
    };
    debug!(?input_detections);
    if let Some(detections) = input_detections {
        // Return response with input detections
        Ok(ChatCompletion {
            id: Uuid::new_v4().simple().to_string(),
            model: task.request.model.clone(),
            choices: vec![],
            created: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            detections: Some(ChatDetections {
                input: detections.into_iter().map(Into::into).collect(),
                output: vec![],
            }),
            warnings: vec![OrchestratorWarning::new(
                DetectionWarningReason::UnsuitableInput,
                UNSUITABLE_INPUT_MESSAGE,
            )],
            ..Default::default()
        }
        .into())
    } else {
        // Handle chat generation
        let chat_completions = chat_completions(&ctx, headers.clone(), task.request).await?;
        use ChatCompletionsResponse::*;
        match chat_completions {
            Unary(mut chat_completion) => {
                // Handle output detection
                let output_detections = match detectors.output {
                    Some(detectors) if !detectors.is_empty() => {
                        let chunker_ids = get_chunker_ids(&ctx, &detectors)?;
                        let messages = Vec::<ChatMessageInternal>::from(&chat_completion);
                        let chunks = chunk(ctx.clone(), chunker_ids, messages).await?;
                        let detections =
                            detect(ctx.clone(), headers.clone(), detectors, chunks).await?;
                        (!detections.is_empty()).then_some(detections)
                    }
                    _ => None,
                };
                debug!(?output_detections);
                if let Some(detections) = output_detections {
                    // Return response with output detections
                    chat_completion.choices = vec![];
                    chat_completion.detections = Some(ChatDetections {
                        input: vec![],
                        output: detections.into_iter().map(Into::into).collect(),
                    });
                    chat_completion.warnings = vec![OrchestratorWarning::new(
                        DetectionWarningReason::UnsuitableOutput,
                        UNSUITABLE_OUTPUT_MESSAGE,
                    )];
                    Ok(chat_completion.into())
                } else {
                    // Return response with choices
                    Ok(chat_completion.into())
                }
            }
            Streaming(_chunk_rx) => {
                unimplemented!()
            }
        }
    }
}

async fn handle_streaming(
    _ctx: Arc<Context>,
    _task: ChatCompletionsDetectionTask,
) -> Result<ChatCompletionsResponse, Error> {
    Err(Error::NotImplemented(
        "chat completions streaming is not yet implemented".into(),
    ))
}

#[instrument(skip_all)]
#[allow(clippy::type_complexity)]
async fn chunk(
    ctx: Arc<Context>,
    chunker_ids: Vec<String>,
    messages: Vec<ChatMessageInternal>,
) -> Result<HashMap<String, Vec<(usize, Vec<Chunk>)>>, Error> {
    let tasks = chunker_ids
        .iter()
        .flat_map(|chunker_id| {
            messages
                .iter()
                .map(|message| {
                    // Spawn task to chunk message with chunker_id
                    tokio::spawn({
                        let ctx = ctx.clone();
                        let chunker_id = chunker_id.clone();
                        let message = message.clone();
                        let Some(Content::Text(text)) = message.content else {
                            panic!("Only text content accepted") // TODO: return Error
                        };
                        let offset: usize = 0;
                        async move {
                            let chunks =
                                unary::chunk(&ctx, chunker_id.clone(), offset, text).await?;
                            Ok::<_, Error>((chunker_id, (message.message_index, chunks)))
                        }
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let results = try_join_all(tasks)
        .await?
        .into_iter()
        .collect::<Result<Vec<_>, Error>>()?;
    let mut chunks: HashMap<String, Vec<(usize, Vec<Chunk>)>> =
        HashMap::with_capacity(chunker_ids.len());
    for (chunker_id, value) in results {
        match chunks.entry(chunker_id) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(value);
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![value]);
            }
        }
    }
    Ok(chunks)
}

#[instrument(skip_all)]
async fn detect(
    ctx: Arc<Context>,
    headers: HeaderMap,
    detectors: HashMap<String, DetectorParams>,
    chunks: HashMap<String, Vec<(usize, Vec<Chunk>)>>,
) -> Result<Vec<DetectionResult>, Error> {
    debug!(?detectors, "starting detection on chat completions");
    let tasks = detectors
        .iter()
        .flat_map(|(detector_id, detector_params)| {
            let detector_config = ctx
                .config
                .detectors
                .get(detector_id)
                .unwrap_or_else(|| panic!("detector config not found for {}", detector_id));
            let default_threshold = detector_config.default_threshold;
            let message_chunks = chunks.get(&detector_config.chunker_id).unwrap().clone();
            match detector_config.r#type {
                DetectorType::TextContents => {
                    // spawn concurrent tasks to run detections on message chunks
                    message_chunks
                        .into_iter()
                        .map(|(index, chunks)| {
                            let ctx = ctx.clone();
                            let detector_id = detector_id.clone();
                            let detector_params = detector_params.clone();
                            let headers = headers.clone();
                            tokio::spawn({
                                async move {
                                    let results = unary::detect_content(
                                        ctx.clone(),
                                        detector_id.clone(),
                                        default_threshold,
                                        detector_params.clone(),
                                        chunks,
                                        headers.clone(),
                                    )
                                    .await?
                                    .into_iter()
                                    .map(GuardrailDetection::ContentAnalysisResponse)
                                    .collect::<Vec<_>>();
                                    Ok(DetectionResult { index, results })
                                }
                            })
                        })
                        .collect::<Vec<_>>()
                }
                _ => unimplemented!(), // TODO: return Error
            }
        })
        .collect::<Vec<_>>();
    let results = try_join_all(tasks)
        .await?
        .into_iter()
        .collect::<Result<Vec<_>, Error>>()?;
    let detections = results
        .into_iter()
        .filter(|detection| !detection.results.is_empty())
        .collect::<Vec<_>>();
    Ok(sort_detections(detections))
}

fn sort_detections(mut detections: Vec<DetectionResult>) -> Vec<DetectionResult> {
    // Sort input detections by message_index
    detections.sort_by_key(|value| value.index);

    detections
        .into_iter()
        .map(|mut detection| {
            let last_idx = detection.results.len();
            // sort detection by starting span, if span is not present then move to the end of the message
            detection.results.sort_by_key(|r| match r {
                GuardrailDetection::ContentAnalysisResponse(value) => value.start,
                _ => last_idx,
            });
            detection
        })
        .collect::<Vec<_>>()
}

#[instrument(skip_all)]
async fn chat_completions(
    ctx: &Arc<Context>,
    headers: HeaderMap,
    mut request: ChatCompletionsRequest,
) -> Result<ChatCompletionsResponse, Error> {
    let client = ctx
        .clients
        .get_as::<OpenAiClient>("chat_generation")
        .expect("chat_generation client not found");
    let model_id = request.model.clone();
    // Remove detectors as chat completion server would reject extra parameter
    request.detectors = None;
    let result = client.chat_completions(request, headers).await;
    result.map_err(|error| Error::ChatGenerateRequestFailed {
        id: model_id,
        error,
    })
}
