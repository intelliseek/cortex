use super::component::FlowComponent;
use super::types::{FlowFuture, SourceOutput};

pub trait Source: FlowComponent<Input = ()> {
    fn stream(&self) -> FlowFuture<'_, SourceOutput<Self::Output, Self::Error>, Self::Error>;

    /// Handle feedback from downstream components
    fn on_feedback(&self, result: Result<Self::Output, Self::Error>);
}
