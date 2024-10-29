use super::Flow;
use crate::flow::{
    condition::Condition,
    processor::Processor,
    stage::{BranchStage, Stage},
};
use std::error::Error;
use std::marker::PhantomData;

pub struct OtherwiseBuilder<DataType, OutputType, ErrorType> {
    condition:
        Box<dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync>,
    then_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
    else_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
    parent: Flow<DataType, ErrorType, OutputType>,
}

impl<DataType, OutputType, ErrorType> OtherwiseBuilder<DataType, OutputType, ErrorType>
where
    DataType: Clone + Send + Sync + 'static,
    OutputType: Send + Sync + 'static,
    ErrorType: Error + Send + Sync + 'static,
{
    pub(crate) fn new(
        condition: Box<
            dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync,
        >,
        then_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
        parent: Flow<DataType, ErrorType, OutputType>,
    ) -> Self {
        Self {
            condition,
            then_branch,
            else_branch: Vec::new(),
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
        self.else_branch.push(Stage::Process(Box::new(processor)));
        self
    }

    pub fn end(self) -> Flow<DataType, ErrorType, OutputType> {
        let branch_stage = Stage::Branch(Box::new(BranchStage {
            condition: self.condition,
            then_branch: self.then_branch,
            else_branch: self.else_branch,
            _marker: PhantomData,
        }));

        let mut flow = self.parent;
        flow.stages.push(branch_stage);
        flow
    }
}
