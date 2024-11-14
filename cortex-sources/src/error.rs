//! Error types for cortex-sources

use thiserror::Error;

/// Errors that can occur when working with sources
#[derive(Error, Debug)]
pub enum SourceError {
    /// An IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A custom error occurred
    #[error("{0}")]
    Custom(String),
}

/// Result type for source operations
pub type Result<T> = std::result::Result<T, SourceError>;