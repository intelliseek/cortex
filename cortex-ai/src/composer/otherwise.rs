use super::Flow;
use crate::flow::{
    condition::Condition,
    processor::Processor,
    stage::{BranchStage, Stage},
};
use std::error::Error;
use std::marker::PhantomData;
use tracing::{debug, instrument};

/// A builder for constructing the alternative branch of a conditional flow.
///
/// # Examples
///
/// ```
/// use cortex_ai::composer::Flow;
/// use cortex_ai::flow::{FlowComponent, Processor, Condition};
/// use cortex_ai::FlowError;
/// use std::error::Error;
/// use std::fmt;
/// use std::pin::Pin;
/// use std::future::Future;
///
/// #[derive(Clone, Debug)]
/// struct MyData;
///
/// #[derive(Clone, Debug)]
/// struct MyError;
///
/// impl fmt::Display for MyError {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "MyError")
///     }
/// }
///
/// impl Error for MyError {}
///
/// impl From<FlowError> for MyError {
///     fn from(e: FlowError) -> Self { MyError }
/// }
///
/// struct MyProcessor;
/// impl FlowComponent for MyProcessor {
///     type Input = MyData;
///     type Output = MyData;
///     type Error = MyError;
/// }
///
/// impl Processor for MyProcessor {
///     fn process(&self, input: Self::Input) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>> {
///         Box::pin(async move { Ok(input) })
///     }
/// }
///
/// struct MyCondition;
/// impl FlowComponent for MyCondition {
///     type Input = MyData;
///     type Output = bool;
///     type Error = MyError;
/// }
///
/// impl Condition for MyCondition {
///     fn evaluate(&self, input: Self::Input) -> Pin<Box<dyn Future<Output = Result<(bool, Self::Output), Self::Error>> + Send>> {
///         Box::pin(async move { Ok((true, false)) })
///     }
/// }
///
/// let flow = Flow::<MyData, MyError, bool>::new()
///     .when(MyCondition)
///     .process(MyProcessor)
///     .otherwise()
///     .process(MyProcessor)
///     .end();
/// ```
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
    /// Creates a new `OtherwiseBuilder` with the specified condition, then branch, and parent flow.
    ///
    /// This is an internal constructor used by the `BranchBuilder`.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition from the parent branch
    /// * `then_branch` - The stages to execute when the condition is true
    /// * `parent` - The parent flow this branch belongs to
    #[instrument(skip(condition, then_branch, parent))]
    pub(crate) fn new(
        condition: Box<
            dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync,
        >,
        then_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
        parent: Flow<DataType, ErrorType, OutputType>,
    ) -> Self {
        debug!("Creating new otherwise builder");
        Self {
            condition,
            then_branch,
            else_branch: Vec::new(),
            parent,
        }
    }

    /// Adds a processor to the alternative branch that will be executed when the condition is false.
    ///
    /// # Arguments
    ///
    /// * `processor` - The processor to add to the alternative branch
    ///
    /// # Returns
    ///
    /// The builder instance for method chaining
    ///
    /// # Type Parameters
    ///
    /// * `ProcessorType` - The type of the processor being added
    #[instrument(skip(self, processor))]
    #[must_use]
    pub fn process<ProcessorType>(mut self, processor: ProcessorType) -> Self
    where
        ProcessorType: Processor<Input = DataType, Output = DataType, Error = ErrorType>
            + Send
            + Sync
            + 'static,
    {
        debug!("Adding processor to else branch");
        self.else_branch.push(Stage::Process(Box::new(processor)));
        self
    }

    /// Finalizes the branch construction and returns the parent flow.
    ///
    /// This method combines the condition, then branch, and else branch into a single
    /// branch stage and adds it to the parent flow.
    ///
    /// # Returns
    ///
    /// The parent flow with the completed branch stage added
    #[instrument(skip(self))]
    #[must_use]
    pub fn end(self) -> Flow<DataType, ErrorType, OutputType> {
        debug!("Finalizing branch construction");
        let branch_stage = Stage::Branch(Box::new(BranchStage {
            condition: self.condition,
            then_branch: self.then_branch,
            else_branch: self.else_branch,
            _marker: PhantomData,
        }));

        let mut flow = self.parent;
        flow.stages.push(branch_stage);
        debug!("Branch added to flow");
        flow
    }
}
