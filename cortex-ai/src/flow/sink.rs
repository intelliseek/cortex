use super::component::FlowComponent;
use super::types::FlowFuture;

pub trait Sink: FlowComponent {
    fn sink(&self, input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error>;
} 