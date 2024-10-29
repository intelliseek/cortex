use std::future::Future;
use std::pin::Pin;

// For general pipeline operations
pub type PipelineFuture<'a, T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>;

// For condition evaluations specifically
pub type ConditionFuture<'a, T, E> = PipelineFuture<'a, (bool, Option<T>), E>; 