use super::{Flow, OtherwiseBuilder};
use crate::flow::{condition::Condition, processor::Processor, stage::Stage};
use std::error::Error;

pub struct BranchBuilder<DataType, OutputType, ErrorType> {
    condition:
        Box<dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync>,
    then_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
    parent: Flow<DataType, ErrorType, OutputType>,
}

impl<DataType, OutputType, ErrorType> BranchBuilder<DataType, OutputType, ErrorType>
where
    DataType: Clone + Send + Sync + 'static,
    OutputType: Send + Sync + 'static,
    ErrorType: Error + Send + Sync + 'static,
{
    pub fn new(
        condition: Box<
            dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync,
        >,
        parent: Flow<DataType, ErrorType, OutputType>,
    ) -> Self {
        Self {
            condition,
            then_branch: Vec::new(),
            parent,
        }
    }

    pub fn process<ProcessorType>(mut self, processor: ProcessorType) -> Self
    where
        ProcessorType: Processor<Input = DataType, Output = DataType, Error = ErrorType>
            + Send
            + Sync
            + 'static,
    {
        self.then_branch.push(Stage::Process(Box::new(processor)));
        self
    }

    pub fn otherwise(self) -> OtherwiseBuilder<DataType, OutputType, ErrorType> {
        OtherwiseBuilder::new(self.condition, self.then_branch, self.parent)
    }
}
