use super::condition::Condition;
use super::processor::Processor;
use super::sink::Sink;
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Clone)]
pub enum Stage<DataType, ErrorType, OutputType> {
    Process(
        Arc<dyn Processor<Input = DataType, Output = DataType, Error = ErrorType> + Send + Sync>,
    ),
    Branch(Arc<BranchStage<DataType, ErrorType, OutputType>>),
    Sink(
        Arc<dyn Sink<Input = DataType, Output = DataType, Error = ErrorType> + Send + Sync>,
    ),
}

#[derive(Clone)]
pub struct BranchStage<DataType, ErrorType, OutputType> {
    pub condition:
        Arc<dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync>,
    pub then_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
    pub else_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
    pub(crate) _marker: PhantomData<(DataType, ErrorType, OutputType)>,
}
