use std::error::Error;
use crate::pipeline::{
    stage::Stage,
    processor::Processor,
    condition::Condition,
};
use super::{Pipeline, OtherwiseBuilder};

pub struct BranchBuilder<T, E> {
    condition: Box<dyn Condition<Input = T, Output = bool, Error = E>>,
    then_branch: Vec<Stage<T, E>>,
    parent: Pipeline<T, E>,
}

impl<T, E> BranchBuilder<T, E>
where
    T: Clone + Send + 'static,
    E: Error + Send + Sync + 'static,
{
    pub fn new(condition: Box<dyn Condition<Input = T, Output = bool, Error = E>>, parent: Pipeline<T, E>) -> Self {
        Self {
            condition,
            then_branch: Vec::new(),
            parent,
        }
    }

    pub fn process<P>(mut self, processor: P) -> Self
    where
        P: Processor<Input = T, Output = T, Error = E> + 'static
    {
        self.then_branch.push(Stage::Process(Box::new(processor)));
        self
    }

    pub fn otherwise(self) -> OtherwiseBuilder<T, E> {
        OtherwiseBuilder::new(self.condition, self.then_branch, self.parent)
    }
} 