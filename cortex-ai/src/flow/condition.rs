use super::component::FlowComponent;
use super::types::ConditionFuture;

pub trait Condition: FlowComponent {
    fn evaluate(&self, input: Self::Input) -> ConditionFuture<'_, Self::Output, Self::Error>;
}
