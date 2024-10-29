use super::component::FlowComponent;
use super::types::FlowFuture;
use flume::Receiver;

pub type SourceReceiver<Output, Error> = Receiver<Result<Output, Error>>;

pub trait Source: FlowComponent<Input = ()> {
    fn stream(&self) -> FlowFuture<'_, SourceReceiver<Self::Output, Self::Error>, Self::Error>;
}
