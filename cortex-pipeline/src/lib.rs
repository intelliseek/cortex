pub mod pipeline;
pub mod builder;

// Re-export main types for easier access
pub use pipeline::component::PipelineComponent;
pub use pipeline::condition::Condition;
pub use pipeline::processor::Processor;
pub use pipeline::source::Source;
pub use pipeline::stage::Stage;
pub use pipeline::types::{ConditionFuture, PipelineFuture};
pub use builder::Pipeline;
