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
    clients::{Client, Error},
    config::ServiceConfig,
    health::HealthCheckResult,
    models::{DetectionResult, DetectorParams},
};

const TEXT_GENERATION_DETECTOR_ENDPOINT: &str = "/api/v1/text/generation";

#[derive(Clone)]
pub struct TextGenerationDetectorClient(DetectorClient);

impl TextGenerationDetectorClient {
    pub async fn new(
        config: &ServiceConfig,
        health_config: Option<&ServiceConfig>,
    ) -> Result<Self, Error> {
        let client = DetectorClient::new(config, health_config).await?;
        Ok(Self(client))
    }

    pub async fn text_generation(
        &self,
        model_id: &str,
        request: GenerationDetectionRequest,
        headers: HeaderMap,
    ) -> Result<Vec<DetectionResult>, Error> {
        let url = self.0.client.endpoint(TEXT_GENERATION_DETECTOR_ENDPOINT);
        info!("sending text generation detector request to {}", url);
        self.0.handle(model_id, url, request, headers).await
    }
}

#[async_trait]
impl Client for TextGenerationDetectorClient {
    fn name(&self) -> &str {
        "text_context_doc_detector"
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
/// /api/v1/text/generation endpoint.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone, Serialize)]
pub struct GenerationDetectionRequest {
    /// User prompt sent to LLM
    pub prompt: String,

    /// Text generated from an LLM
    pub generated_text: String,

    /// Detector parameters (available parameters depend on the detector)
    pub detector_params: DetectorParams,
}

impl GenerationDetectionRequest {
    pub fn new(prompt: String, generated_text: String, detector_params: DetectorParams) -> Self {
        Self {
            prompt,
            generated_text,
            detector_params,
        }
    }
}
