use super::component::FlowComponent;
use super::types::FlowFuture;

pub trait Processor: FlowComponent {
    fn process(&self, input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error>;
}
