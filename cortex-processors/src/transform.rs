//! Transform processors for data streams

use cortex_ai::{FlowComponent, FlowFuture, Processor};
use std::marker::PhantomData;

/// A generic transform processor that can transform data from one type to another
pub struct TransformProcessor<I, O, F, E>
where
    F: Fn(I) -> O,
    E: std::error::Error + Send + Sync + 'static,
{
    transform: F,
    _phantom: PhantomData<(I, O, E)>,
}

impl<I, O, F, E> TransformProcessor<I, O, F, E>
where
    F: Fn(I) -> O,
    E: std::error::Error + Send + Sync + 'static,
{
    /// Creates a new transform processor with the given transformation function
    #[must_use]
    pub const fn new(transform: F) -> Self {
        Self {
            transform,
            _phantom: PhantomData,
        }
    }
}

impl<I, O, F, E> FlowComponent for TransformProcessor<I, O, F, E>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    F: Fn(I) -> O + Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    type Input = I;
    type Output = O;
    type Error = E;
}

impl<I, O, F, E> Processor for TransformProcessor<I, O, F, E>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    F: Fn(I) -> O + Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    fn process(&self, input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error> {
        let output = (self.transform)(input);
        Box::pin(async move { Ok(output) })
    }
}
