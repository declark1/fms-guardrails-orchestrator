use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A detection.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    /// Start index of the detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,
    /// End index of the detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<usize>,
    /// Text corresponding to the detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// ID of the detector
    pub detector_id: Option<String>,
    /// Type of detection
    pub detection_type: String,
    /// Detection class
    pub detection: String,
    /// Confidence level of the detection class
    pub score: f64,
    /// Detection evidence
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<DetectionEvidence>,
    /// Detection metadata
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// Detection evidence.
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DetectionEvidence {
    /// Evidence name
    pub name: String,
    /// Evidence value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Evidence score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Additional evidence
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

/// Additional detection evidence.
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    /// Evidence name
    pub name: String,
    /// Evidence value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Evidence score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

/// An array of detections.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Detections(Vec<Detection>);

impl Detections {
    pub fn new() -> Self {
        Self::default()
    }
}

impl std::ops::Deref for Detections {
    type Target = Vec<Detection>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Detections {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for Detections {
    type Item = Detection;
    type IntoIter = <Vec<Detection> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<Detection> for Detections {
    fn from_iter<T: IntoIterator<Item = Detection>>(iter: T) -> Self {
        let mut detections = Detections::new();
        for value in iter {
            detections.push(value);
        }
        detections
    }
}

impl From<Vec<Detection>> for Detections {
    fn from(value: Vec<Detection>) -> Self {
        Self(value)
    }
}
