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

use async_trait::async_trait;
use hyper::HeaderMap;
use serde::Serialize;
use tracing::info;

use super::DetectorClient;
use crate::{
    clients::{
        Client, Error,
        openai::{Message, Tool},
    },
    config::ServiceConfig,
    health::HealthCheckResult,
    models::{DetectionResult, DetectorParams},
};

const TEXT_CHAT_DETECTOR_ENDPOINT: &str = "/api/v1/text/chat";

#[derive(Clone)]
pub struct TextChatDetectorClient(DetectorClient);

impl TextChatDetectorClient {
    pub async fn new(
        config: &ServiceConfig,
        health_config: Option<&ServiceConfig>,
    ) -> Result<Self, Error> {
        let client = DetectorClient::new(config, health_config).await?;
        Ok(Self(client))
    }

    pub async fn text_chat(
        &self,
        model_id: &str,
        request: ChatDetectionRequest,
        headers: HeaderMap,
    ) -> Result<Vec<DetectionResult>, Error> {
        let url = self.0.client.endpoint(TEXT_CHAT_DETECTOR_ENDPOINT);
        info!("sending text chat detector request to {}", url);
        self.0.handle(model_id, url, request, headers).await
    }
}

#[async_trait]
impl Client for TextChatDetectorClient {
    fn name(&self) -> &str {
        "text_chat_detector"
    }

    async fn health(&self) -> HealthCheckResult {
        if let Some(health_client) = &self.0.health_client {
            health_client.health().await
        } else {
            self.0.client.health().await
        }
    }
}

/// A struct representing a request to a detector compatible with the
/// /api/v1/text/chat endpoint.
// #[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone, Serialize)]
pub struct ChatDetectionRequest {
    /// Chat messages to run detection on
    pub messages: Vec<Message>,

    /// Optional list of tool definitions
    pub tools: Vec<Tool>,

    /// Detector parameters (available parameters depend on the detector)
    pub detector_params: DetectorParams,
}

impl ChatDetectionRequest {
    pub fn new(messages: Vec<Message>, tools: Vec<Tool>, detector_params: DetectorParams) -> Self {
        Self {
            messages,
            tools,
            detector_params,
        }
    }
}
