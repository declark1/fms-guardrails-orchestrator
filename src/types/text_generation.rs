use serde::{Deserialize, Serialize};

use super::detection_warning::DetectionWarning;
use crate::pb;

/// Text generation and detection results.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifiedGeneratedTextResult {
    /// Generated text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_text: Option<String>,
    /// Classification results for input to text generation model and/or
    /// output from the text generation model
    pub token_classification_results: TokenClassificationResults,
    /// Why text generation stopped
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    /// Length of sequence of generated tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_token_count: Option<u32>,
    /// Random seed used for text generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// Length of input
    pub input_token_count: u32,
    /// Vector of warnings on input detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<DetectionWarning>>,
    /// Individual generated tokens and associated details, if requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Vec<GeneratedToken>>,
    /// Input tokens and associated details, if requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<Vec<GeneratedToken>>,
}

impl From<pb::fmaas::BatchedGenerationResponse> for ClassifiedGeneratedTextResult {
    fn from(mut value: pb::fmaas::BatchedGenerationResponse) -> Self {
        let value = value.responses.swap_remove(0);
        Self {
            generated_text: Some(value.text.clone()),
            finish_reason: Some(value.stop_reason().into()),
            generated_token_count: Some(value.generated_token_count),
            seed: Some(value.seed as u32),
            input_token_count: value.input_token_count,
            warnings: None,
            tokens: if value.tokens.is_empty() {
                None
            } else {
                Some(value.tokens.into_iter().map(Into::into).collect())
            },
            input_tokens: if value.input_tokens.is_empty() {
                None
            } else {
                Some(value.input_tokens.into_iter().map(Into::into).collect())
            },
            token_classification_results: TokenClassificationResults {
                input: None,
                output: None,
            },
        }
    }
}

impl From<pb::caikit_data_model::nlp::GeneratedTextResult> for ClassifiedGeneratedTextResult {
    fn from(value: pb::caikit_data_model::nlp::GeneratedTextResult) -> Self {
        Self {
            generated_text: Some(value.generated_text.clone()),
            finish_reason: Some(value.finish_reason().into()),
            generated_token_count: Some(value.generated_tokens as u32),
            seed: Some(value.seed as u32),
            input_token_count: value.input_token_count as u32,
            warnings: None,
            tokens: if value.tokens.is_empty() {
                None
            } else {
                Some(value.tokens.into_iter().map(Into::into).collect())
            },
            input_tokens: if value.input_tokens.is_empty() {
                None
            } else {
                Some(value.input_tokens.into_iter().map(Into::into).collect())
            },
            token_classification_results: TokenClassificationResults {
                input: None,
                output: None,
            },
        }
    }
}

/// Streaming text generation and detection results.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifiedGeneratedTextStreamResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_text: Option<String>,
    /// Classification results for input to text generation model and/or
    /// output from the text generation model
    pub token_classification_results: TokenClassificationResults,
    /// Why text generation stopped
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    /// Length of sequence of generated tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_token_count: Option<u32>,
    /// Random seed used for text generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// Length of input
    pub input_token_count: u32,
    /// Vector of warnings on input detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<DetectionWarning>>,
    /// Individual generated tokens and associated details, if requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Vec<GeneratedToken>>,
    /// Input tokens and associated details, if requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<Vec<GeneratedToken>>,
    /// Result index up to which text is processed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_index: Option<u32>,
    /// Result start index for processed text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<u32>,
}

impl From<pb::fmaas::GenerationResponse> for ClassifiedGeneratedTextStreamResult {
    fn from(value: pb::fmaas::GenerationResponse) -> Self {
        Self {
            generated_text: Some(value.text.clone()),
            finish_reason: Some(value.stop_reason().into()),
            generated_token_count: Some(value.generated_token_count),
            seed: Some(value.seed as u32),
            input_token_count: value.input_token_count,
            warnings: None,
            tokens: if value.tokens.is_empty() {
                None
            } else {
                Some(value.tokens.into_iter().map(Into::into).collect())
            },
            input_tokens: if value.input_tokens.is_empty() {
                None
            } else {
                Some(value.input_tokens.into_iter().map(Into::into).collect())
            },
            token_classification_results: TokenClassificationResults {
                input: None,
                output: None,
            },
            processed_index: None,
            start_index: Some(0),
        }
    }
}

impl From<pb::caikit_data_model::nlp::GeneratedTextStreamResult>
    for ClassifiedGeneratedTextStreamResult
{
    fn from(value: pb::caikit_data_model::nlp::GeneratedTextStreamResult) -> Self {
        let details = value.details.as_ref();
        Self {
            generated_text: Some(value.generated_text.clone()),
            finish_reason: details.map(|v| v.finish_reason().into()),
            generated_token_count: details.map(|v| v.generated_tokens),
            seed: details.map(|v| v.seed as u32),
            input_token_count: details
                .map(|v| v.input_token_count as u32)
                .unwrap_or_default(), // TODO
            warnings: None,
            tokens: if value.tokens.is_empty() {
                None
            } else {
                Some(value.tokens.into_iter().map(Into::into).collect())
            },
            input_tokens: if value.input_tokens.is_empty() {
                None
            } else {
                Some(value.input_tokens.into_iter().map(Into::into).collect())
            },
            token_classification_results: TokenClassificationResults {
                input: None,
                output: None,
            },
            processed_index: None,
            start_index: None,
        }
    }
}

/// Token classification results.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenClassificationResults {
    /// Classification results on input to a text generation model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<TokenClassificationResult>>,
    /// Classification results on output from a text generation model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<TokenClassificationResult>>,
}

/// Token classification result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenClassificationResult {
    /// Beginning/start offset of token
    pub start: u32,
    /// End offset of token
    pub end: u32,
    /// Text referenced by token
    pub word: String,
    /// Predicted relevant class name for the token
    pub entity: String,
    /// Aggregate label, if applicable
    pub entity_group: String,
    /// Optional id of detector (model) responsible for result(s)
    pub detector_id: Option<String>,
    /// Confidence-like score of this classification prediction in [0, 1]
    pub score: f64,
    /// Length of tokens in the text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u32>,
}

/// Generated token details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedToken {
    /// Token text
    pub text: String,
    /// Logprob (log of normalized probability)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprob: Option<f64>,
    /// One-based rank relative to other tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
}

impl From<pb::fmaas::TokenInfo> for GeneratedToken {
    fn from(value: pb::fmaas::TokenInfo) -> Self {
        Self {
            text: value.text,
            logprob: Some(value.logprob as f64),
            rank: Some(value.rank),
        }
    }
}

impl From<pb::caikit_data_model::nlp::GeneratedToken> for GeneratedToken {
    fn from(value: pb::caikit_data_model::nlp::GeneratedToken) -> Self {
        Self {
            text: value.text,
            logprob: Some(value.logprob),
            rank: Some(value.rank as u32),
        }
    }
}

/// Text generation stop reason.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinishReason {
    NotFinished,
    MaxTokens,
    EosToken,
    Cancelled,
    TimeLimit,
    StopSequence,
    TokenLimit,
    Error,
}

impl From<pb::fmaas::StopReason> for FinishReason {
    fn from(value: pb::fmaas::StopReason) -> Self {
        use pb::fmaas::StopReason::*;
        match value {
            NotFinished => FinishReason::NotFinished,
            MaxTokens => FinishReason::MaxTokens,
            EosToken => FinishReason::EosToken,
            Cancelled => FinishReason::Cancelled,
            TimeLimit => FinishReason::TimeLimit,
            StopSequence => FinishReason::StopSequence,
            TokenLimit => FinishReason::TokenLimit,
            Error => FinishReason::Error,
        }
    }
}

impl From<pb::caikit_data_model::nlp::FinishReason> for FinishReason {
    fn from(value: pb::caikit_data_model::nlp::FinishReason) -> Self {
        use pb::caikit_data_model::nlp::FinishReason::*;
        match value {
            NotFinished => FinishReason::NotFinished,
            MaxTokens => FinishReason::MaxTokens,
            EosToken => FinishReason::EosToken,
            Cancelled => FinishReason::Cancelled,
            TimeLimit => FinishReason::TimeLimit,
            StopSequence => FinishReason::StopSequence,
            TokenLimit => FinishReason::TokenLimit,
            Error => FinishReason::Error,
        }
    }
}
