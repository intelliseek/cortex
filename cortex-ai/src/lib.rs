pub mod composer;
pub mod flow;

// Re-export main types for easier access
pub use composer::flow::FlowError;
pub use composer::Flow;
pub use flow::component::FlowComponent;
pub use flow::condition::Condition;
pub use flow::processor::Processor;
pub use flow::source::Source;
pub use flow::stage::Stage;
pub use flow::types::{ConditionFuture, FlowFuture};
