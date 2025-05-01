use serde::{Deserialize, Serialize};

pub const UNSUITABLE_INPUT_MESSAGE: &str = "Unsuitable input detected. \
    Please check the detected entities on your input and try again \
    with the unsuitable input removed.";

pub const UNSUITABLE_OUTPUT_MESSAGE: &str = "Unsuitable output detected.";

/// Detection warning reason and message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectionWarning {
    /// Warning reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<DetectionWarningReason>,
    /// Warning message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl DetectionWarning {
    pub fn unsuitable_input() -> Self {
        DetectionWarning {
            id: Some(DetectionWarningReason::UnsuitableInput),
            message: Some(UNSUITABLE_INPUT_MESSAGE.to_string()),
        }
    }

    pub fn unsuitable_output() -> Self {
        DetectionWarning {
            id: Some(DetectionWarningReason::UnsuitableOutput),
            message: Some(UNSUITABLE_OUTPUT_MESSAGE.to_string()),
        }
    }
}

/// Detection warning reason.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DetectionWarningReason {
    /// Unsuitable text detected on input
    #[serde(rename = "UNSUITABLE_INPUT")]
    UnsuitableInput,
    /// Unsuitable text detected on output
    #[serde(rename = "UNSUITABLE_OUTPUT")]
    UnsuitableOutput,
}
