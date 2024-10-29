use super::types::PipelineFuture;
use super::component::PipelineComponent;

pub trait Processor: PipelineComponent {
    fn process<'a>(
        &'a self,
        input: Self::Input,
    ) -> PipelineFuture<'a, Self::Output, Self::Error>;
} 