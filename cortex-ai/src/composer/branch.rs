use super::{Flow, OtherwiseBuilder};
use crate::flow::{condition::Condition, processor::Processor, stage::Stage};
use std::error::Error;
use tracing::{debug, instrument};

/// A builder for constructing conditional branches in a flow.
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
/// let flow = Flow::<MyData, MyError, bool>::new();
/// let branch = flow
///     .when(MyCondition)
///     .process(MyProcessor)
///     .otherwise();
/// ```
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
    /// Creates a new `BranchBuilder` with the specified condition and parent flow.
    ///
    /// # Arguments
    ///
    /// * `condition` - A boxed condition that determines when this branch should execute
    /// * `parent` - The parent flow this branch belongs to
    ///
    /// # Returns
    ///
    /// A new instance of `BranchBuilder`
    #[instrument(skip(condition, parent))]
    #[must_use]
    pub fn new(
        condition: Box<
            dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync,
        >,
        parent: Flow<DataType, ErrorType, OutputType>,
    ) -> Self {
        debug!("Creating new branch builder");
        Self {
            condition,
            then_branch: Vec::new(),
            parent,
        }
    }

    /// Adds a processor to the branch that will be executed when the condition is true.
    ///
    /// # Arguments
    ///
    /// * `processor` - The processor to add to the branch
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
        debug!("Adding processor to then branch");
        self.then_branch.push(Stage::Process(Box::new(processor)));
        self
    }

    /// Transitions to building the alternative branch that will be executed when the condition is false.
    ///
    /// # Returns
    ///
    /// An `OtherwiseBuilder` for constructing the alternative branch
    #[instrument(skip(self))]
    #[must_use]
    pub fn otherwise(self) -> OtherwiseBuilder<DataType, OutputType, ErrorType> {
        debug!("Creating otherwise builder");
        OtherwiseBuilder::new(self.condition, self.then_branch, self.parent)
    }
}
