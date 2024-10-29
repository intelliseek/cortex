use super::types::ConditionFuture;
use super::component::PipelineComponent;

pub trait Condition: PipelineComponent {
    fn evaluate<'a>(
        &'a self,
        input: Self::Input,
    ) -> ConditionFuture<'a, (bool, Option<Self::Output>), Self::Error>;
} 