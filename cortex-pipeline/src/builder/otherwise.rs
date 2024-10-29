use std::error::Error;
use std::marker::PhantomData;
use crate::pipeline::{
    stage::{Stage, BranchStage},
    processor::Processor,
    condition::Condition,
};
use super::Pipeline;

pub struct OtherwiseBuilder<T, E> {
    condition: Box<dyn Condition<Input = T, Output = bool, Error = E>>,
    then_branch: Vec<Stage<T, E>>,
    else_branch: Vec<Stage<T, E>>,
    parent: Pipeline<T, E>,
}

impl<T, E> OtherwiseBuilder<T, E>
where
    T: Clone + Send + 'static,
    E: Error + Send + Sync + 'static,
{
    pub(crate) fn new(
        condition: Box<dyn Condition<Input = T, Output = bool, Error = E>>,
        then_branch: Vec<Stage<T, E>>,
        parent: Pipeline<T, E>,
    ) -> Self {
        Self {
            condition,
            then_branch,
            else_branch: Vec::new(),
            parent,
        }
    }

    pub fn process<P>(mut self, processor: P) -> Self
    where
        P: Processor<Input = T, Output = T, Error = E> + 'static
    {
        self.else_branch.push(Stage::Process(Box::new(processor)));
        self
    }

    pub fn end(self) -> Pipeline<T, E> {
        let branch_stage = Stage::Branch(Box::new(BranchStage {
            condition: self.condition,
            then_branch: self.then_branch,
            else_branch: self.else_branch,
            _marker: PhantomData::default(),
        }));

        let mut pipeline = self.parent;
        pipeline.stages.push(branch_stage);
        pipeline
    }
} 