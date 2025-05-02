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
use serde::{Deserialize, Serialize};
use tracing::info;

use super::DetectorClient;
use crate::{
    clients::{Client, Error},
    config::ServiceConfig,
    health::HealthCheckResult,
    models::{DetectionResult, DetectorParams},
};

const TEXT_CONTEXT_DOC_DETECTOR_ENDPOINT: &str = "/api/v1/text/context/doc";

#[derive(Clone)]
pub struct TextContextDocDetectorClient(DetectorClient);

impl TextContextDocDetectorClient {
    pub async fn new(
        config: &ServiceConfig,
        health_config: Option<&ServiceConfig>,
    ) -> Result<Self, Error> {
        let client = DetectorClient::new(config, health_config).await?;
        Ok(Self(client))
    }

    pub async fn text_context_doc(
        &self,
        model_id: &str,
        request: ContextDocsDetectionRequest,
        headers: HeaderMap,
    ) -> Result<Vec<DetectionResult>, Error> {
        let url = self.0.client.endpoint(TEXT_CONTEXT_DOC_DETECTOR_ENDPOINT);
        info!("sending text context doc detector request to {}", url);
        self.0.handle(model_id, url, request, headers).await
    }
}

#[async_trait]
impl Client for TextContextDocDetectorClient {
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
/// /api/v1/text/context/doc endpoint.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone, Serialize)]
pub struct ContextDocsDetectionRequest {
    /// Content to run detection on
    pub content: String,

    /// Type of context being sent
    pub context_type: ContextType,

    /// Context to run detection on
    pub context: Vec<String>,

    /// Detector parameters (available parameters depend on the detector)
    pub detector_params: DetectorParams,
}

/// Enum representing the context type of a detection
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ContextType {
    #[serde(rename = "docs")]
    Document,
    #[serde(rename = "url")]
    Url,
}

impl ContextDocsDetectionRequest {
    pub fn new(
        content: String,
        context_type: ContextType,
        context: Vec<String>,
        detector_params: DetectorParams,
    ) -> Self {
        Self {
            content,
            context_type,
            context,
            detector_params,
        }
    }
}
