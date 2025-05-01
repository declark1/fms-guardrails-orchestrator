use crate::clients;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Client(#[from] clients::Error),
    #[error("detector `{0}` not found")]
    DetectorNotFound(String),
    #[error("chunker `{0}` not found")]
    ChunkerNotFound(String),
    #[error("detector request failed for `{id}`: {error}")]
    DetectorRequestFailed { id: String, error: clients::Error },
    #[error("chunker request failed for `{id}`: {error}")]
    ChunkerRequestFailed { id: String, error: clients::Error },
    #[error("generate request failed for `{id}`: {error}")]
    GenerateRequestFailed { id: String, error: clients::Error },
    #[error("chat completion request failed for `{id}`: {error}")]
    ChatCompletionRequestFailed { id: String, error: clients::Error },
    #[error("tokenize request failed for `{id}`: {error}")]
    TokenizeRequestFailed { id: String, error: clients::Error },
    #[error("validation error: {0}")]
    Validation(String),
    #[error("{0}")]
    Other(String),
    #[error("cancelled")]
    Cancelled,
    #[error("json deserialization error: {0}")]
    JsonError(String),
}

impl From<tokio::task::JoinError> for Error {
    fn from(error: tokio::task::JoinError) -> Self {
        if error.is_cancelled() {
            Self::Cancelled
        } else {
            Self::Other(format!("task panicked: {error}"))
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::JsonError(value.to_string())
    }
}

impl From<ValidationError> for Error {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("`{0}` is required")]
    Required(String),
    #[error("{0}")]
    Invalid(String),
}
