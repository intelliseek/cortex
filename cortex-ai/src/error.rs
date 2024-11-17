use std::{error::Error, fmt};

/// Error type specific to Flow operations.
#[derive(Debug, Clone)]
pub enum FlowError {
    Source(String),
    Process(String),
    Condition(String),
    NoSource,
    NoSink,
    Sink(String),
    Custom(String),
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Source(msg) => write!(f, "Source error: {msg}"),
            Self::Process(msg) => write!(f, "Process error: {msg}"),
            Self::Condition(msg) => write!(f, "Condition error: {msg}"),
            Self::NoSource => write!(f, "Flow error: No source configured"),
            Self::NoSink => write!(f, "Flow error: No sink configured"),
            Self::Sink(msg) => write!(f, "Sink error: {msg}"),
            Self::Custom(msg) => write!(f, "Flow error: {msg}"),
        }
    }
}
impl Error for FlowError {}
