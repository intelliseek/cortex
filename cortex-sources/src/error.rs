use cortex_ai::FlowError;
use thiserror::Error;

/// Errors that can occur when working with sources
#[derive(Error, Debug, Clone)]
pub enum SourceError {
    /// An IO error occurred
    #[error("IO error: {0}")]
    Io(String),

    /// A custom error occurred
    #[error("{0}")]
    Custom(String),
}

impl From<std::io::Error> for SourceError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<FlowError> for SourceError {
    fn from(err: FlowError) -> Self {
        Self::Custom(err.to_string())
    }
}

/// Result type for source operations
pub type Result<T> = std::result::Result<T, SourceError>; 