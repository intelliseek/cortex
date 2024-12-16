//! Error types for cortex-sinks

use cortex_ai::FlowError;
use thiserror::Error;

/// Errors that can occur when working with sinks
#[derive(Error, Debug, Clone)]
pub enum SinkError {
    /// An IO error occurred
    #[error("IO error: {0}")]
    Io(String),

    /// A database error occurred
    #[error("Database error: {0}")]
    Database(String),

    /// A queue error occurred
    #[error("Queue error: {0}")]
    Queue(String),

    /// A metrics error occurred
    #[error("Metrics error: {0}")]
    Metrics(String),

    /// A vector store error occurred
    #[error("Vector store error: {0}")]
    VectorStore(String),

    /// A custom error occurred
    #[error("{0}")]
    Custom(String),
}

impl From<std::io::Error> for SinkError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<FlowError> for SinkError {
    fn from(err: FlowError) -> Self {
        Self::Custom(err.to_string())
    }
}

/// Result type for sink operations
pub type Result<T> = std::result::Result<T, SinkError>; 