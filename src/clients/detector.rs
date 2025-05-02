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

use std::fmt::Debug;

use axum::http::HeaderMap;
use http::header::CONTENT_TYPE;
use hyper::StatusCode;
use serde::{Deserialize, de::DeserializeOwned};
use url::Url;

use crate::config::ServiceConfig;

use super::{
    Error, HttpClient, create_http_client,
    http::{JSON_CONTENT_TYPE, RequestBody},
};

pub mod text_contents;
pub use text_contents::*;
pub mod text_chat;
pub use text_chat::*;
pub mod text_context_doc;
pub use text_context_doc::*;
pub mod text_generation;
pub use text_generation::*;

const DEFAULT_PORT: u16 = 8080;

#[derive(Clone)]
pub struct DetectorClient {
    client: HttpClient,
    health_client: Option<HttpClient>,
}

impl DetectorClient {
    pub async fn new(
        config: &ServiceConfig,
        health_config: Option<&ServiceConfig>,
    ) -> Result<Self, Error> {
        let client = create_http_client(DEFAULT_PORT, config).await?;
        let health_client = if let Some(health_config) = health_config {
            Some(create_http_client(DEFAULT_PORT, health_config).await?)
        } else {
            None
        };
        Ok(Self {
            client,
            health_client,
        })
    }

    async fn handle<R, S>(
        &self,
        model_id: &str,
        url: Url,
        request: R,
        mut headers: HeaderMap,
    ) -> Result<S, Error>
    where
        R: RequestBody,
        S: DeserializeOwned,
    {
        // Add required headers
        headers.append("detector-id", model_id.parse().unwrap());
        headers.append(CONTENT_TYPE, JSON_CONTENT_TYPE);
        // Used by router, if available
        headers.append("x-model-name", model_id.parse().unwrap());
        let response = self.client.post(url, headers, request).await?;
        match response.status() {
            StatusCode::OK => response.json::<S>().await,
            _ => {
                let code = response.status();
                let message = if let Ok(response) = response.json::<DetectorError>().await {
                    response.message
                } else {
                    "unknown error occurred".into()
                };
                Err(Error::Http { code, message })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DetectorError {
    pub code: u16,
    pub message: String,
}

impl From<DetectorError> for Error {
    fn from(error: DetectorError) -> Self {
        Error::Http {
            code: StatusCode::from_u16(error.code).unwrap(),
            message: error.message,
        }
    }
}
