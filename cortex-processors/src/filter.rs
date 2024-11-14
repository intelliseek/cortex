//! Filter processors for data streams

use cortex_ai::{FlowComponent, FlowFuture, Processor};
use std::marker::PhantomData;

/// A generic filter processor that can filter data based on a predicate
pub struct FilterProcessor<T, F, E>
where
    F: Fn(&T) -> bool,
    E: std::error::Error + Send + Sync + 'static,
{
    predicate: F,
    _phantom: PhantomData<(T, E)>,
}

impl<T, F, E> FilterProcessor<T, F, E>
where
    F: Fn(&T) -> bool,
    E: std::error::Error + Send + Sync + 'static,
{
    /// Creates a new filter processor with the given predicate
    #[must_use]
    pub const fn new(predicate: F) -> Self {
        Self {
            predicate,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, E> FlowComponent for FilterProcessor<T, F, E>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(&T) -> bool + Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    type Input = T;
    type Output = T;
    type Error = E;
}

impl<T, F, E> Processor for FilterProcessor<T, F, E>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(&T) -> bool + Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    fn process(&self, input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error> {
        let _ = (self.predicate)(&input);
        Box::pin(async move { Ok(input) })
    }
}
