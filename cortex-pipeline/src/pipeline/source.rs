use super::types::PipelineFuture;
use super::component::PipelineComponent;
use flume::Receiver;

pub trait Source: PipelineComponent<Input = ()> {
    fn stream<'a>(&'a self) -> PipelineFuture<'a, Receiver<Result<Self::Output, Self::Error>>, Self::Error>;
} 