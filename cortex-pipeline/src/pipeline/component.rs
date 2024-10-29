use std::error::Error;

pub trait PipelineComponent {
    type Input;
    type Output;
    type Error: Error + Send + Sync + 'static;
} 