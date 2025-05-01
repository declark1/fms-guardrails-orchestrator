use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Detector parameters.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DetectorParams(BTreeMap<String, serde_json::Value>);

impl DetectorParams {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Threshold to filter detector results by score.
    pub fn pop_threshold(&mut self) -> Option<f64> {
        self.0.remove("threshold").and_then(|v| v.as_f64())
    }
}

impl std::ops::Deref for DetectorParams {
    type Target = BTreeMap<String, serde_json::Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DetectorParams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
