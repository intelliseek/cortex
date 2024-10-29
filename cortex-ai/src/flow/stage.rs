use super::condition::Condition;
use super::processor::Processor;
use std::marker::PhantomData;

pub enum Stage<DataType, ErrorType, OutputType> {
    Process(
        Box<dyn Processor<Input = DataType, Output = DataType, Error = ErrorType> + Send + Sync>,
    ),
    Branch(Box<BranchStage<DataType, ErrorType, OutputType>>),
}

pub struct BranchStage<DataType, ErrorType, OutputType> {
    pub condition:
        Box<dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync>,
    pub then_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
    pub else_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
    pub(crate) _marker: PhantomData<(DataType, ErrorType, OutputType)>,
}
