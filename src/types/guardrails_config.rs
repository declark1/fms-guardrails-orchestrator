use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::detector_params::DetectorParams;

/// Configuration of guardrails models for either or both input to a text generation model
/// (e.g. user prompt) and output of a text generation model
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailsConfig {
    /// Configuration for detection on input to a text generation model (e.g. user prompt)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<GuardrailsConfigInput>,
    /// Configuration for detection on output of a text generation model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<GuardrailsConfigOutput>,
}

impl GuardrailsConfig {
    pub fn input_masks(&self) -> Option<&[(usize, usize)]> {
        self.input.as_ref().and_then(|input| input.masks.as_deref())
    }

    pub fn input_detectors(&self) -> HashMap<String, DetectorParams> {
        self.input
            .as_ref()
            .map(|input| input.models.clone())
            .unwrap_or_default()
    }

    pub fn output_detectors(&self) -> HashMap<String, DetectorParams> {
        self.output
            .as_ref()
            .map(|output| output.models.clone())
            .unwrap_or_default()
    }
}

/// Configuration for detection on input to a text generation model (e.g. user prompt)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailsConfigInput {
    /// Map of model name to model specific parameters
    pub models: HashMap<String, DetectorParams>,
    /// Vector of spans are in the form of (span_start, span_end) corresponding
    /// to spans of input text on which to run input detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masks: Option<Vec<(usize, usize)>>,
}

/// Configuration for detection on output of a text generation model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailsConfigOutput {
    /// Map of model name to model specific parameters
    pub models: HashMap<String, DetectorParams>,
}
