use std::future::Future;
use std::pin::Pin;

use flume::Receiver;

// For general flow operations
pub type FlowFuture<'a, T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>;

// For condition evaluations specifically - simplified to just return (bool, Option<O>)
pub type ConditionFuture<'a, O, E> = FlowFuture<'a, (bool, Option<O>), E>;

pub type SourceReceiver<Output, Error> = Receiver<Result<Output, Error>>;

// For source feedback
pub type ProcessingResult<T, E> = Result<T, E>;
pub type SourceChannel<T, E> = Receiver<ProcessingResult<T, E>>;
pub type FeedbackChannel<T, E> = flume::Sender<ProcessingResult<T, E>>;

// Combined source output type
pub struct SourceOutput<T, E> {
    pub receiver: SourceChannel<T, E>,
    pub feedback: FeedbackChannel<T, E>,
}
