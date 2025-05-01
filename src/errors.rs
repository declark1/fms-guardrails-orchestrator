#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("`{0}` is required")]
    Required(String),
    #[error("{0}")]
    Invalid(String),
}
