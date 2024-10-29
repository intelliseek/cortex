use std::marker::PhantomData;
use super::processor::Processor;
use super::condition::Condition;

pub enum Stage<T, E> {
    Process(Box<dyn Processor<Input = T, Output = T, Error = E>>),
    Branch(Box<BranchStage<T, E>>),
}

pub struct BranchStage<T, E> {
    pub condition: Box<dyn Condition<Input = T, Output = bool, Error = E>>,
    pub then_branch: Vec<Stage<T, E>>,
    pub else_branch: Vec<Stage<T, E>>,
    pub(crate) _marker: PhantomData<(T, E)>,
} 