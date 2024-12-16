use super::{Flow, OtherwiseBuilder};
use crate::flow::{
    condition::Condition,
    processor::Processor,
    stage::{BranchStage, Stage},
};
use either::Either;
use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;
use tracing::debug;

/// A builder for constructing conditional branches in a flow.
///
/// # Examples
///
/// ```
/// use cortex_ai::composer::Flow;
/// use cortex_ai::flow::condition::Condition;
/// use cortex_ai::flow::processor::Processor;
/// use cortex_ai::flow::source::Source;
/// use cortex_ai::flow::sink::Sink;
/// use cortex_ai::flow::types::SourceOutput;
/// use cortex_ai::FlowComponent;
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
/// struct MySource;
/// impl FlowComponent for MySource {
///     type Input = ();
///     type Output = MyData;
///     type Error = MyError;
/// }
///
/// impl Source for MySource {
///     fn stream(&self) -> Pin<Box<dyn Future<Output = Result<SourceOutput<Self::Output, Self::Error>, Self::Error>> + Send>> {
///         Box::pin(async move {
///             let (tx, rx) = flume::bounded(1);
///             let (feedback_tx, _) = flume::bounded(1);
///             Ok(SourceOutput { receiver: rx, feedback: feedback_tx })
///         })
///     }
///
///     fn on_feedback(&self, _result: Result<Self::Output, Self::Error>) {}
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
/// #[derive(Clone)]
/// struct MyCondition;
/// impl FlowComponent for MyCondition {
///     type Input = MyData;
///     type Output = bool;
///     type Error = MyError;
/// }
///
/// impl Condition for MyCondition {
///     fn evaluate(&self, input: Self::Input) -> Pin<Box<dyn Future<Output = Result<(bool, Option<Self::Output>), Self::Error>> + Send>> {
///         Box::pin(async move { Ok((true, Some(false))) })
///     }
/// }
///
/// struct MySink;
/// impl FlowComponent for MySink {
///     type Input = MyData;
///     type Output = MyData;
///     type Error = MyError;
/// }
///
/// impl Sink for MySink {
///     fn sink(&self, input: Self::Input) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>> {
///         Box::pin(async move { Ok(input) })
///     }
/// }
///
/// let flow = Flow::<MyData, MyError, bool>::new()
///     .source(MySource)
///     .when(MyCondition)
///     .process(MyProcessor)
///     .otherwise()
///     .process(MyProcessor)
///     .end()
///     .sink(MySink);
/// ```
///
#[derive(Clone)]
pub struct BranchBuilder<DataType, OutputType, ErrorType>
where
    DataType: Clone + Send + Sync + Clone + 'static,
    ErrorType: Error + Send + Sync + Clone + 'static,
    OutputType: Send + Sync + Clone + 'static,
{
    condition:
        Arc<dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync>,
    then_branch: Vec<Stage<DataType, ErrorType, OutputType>>,
    parent: Flow<DataType, ErrorType, OutputType>,
    parent_branch: Option<Arc<BranchBuilder<DataType, OutputType, ErrorType>>>,
}

impl<DataType, OutputType, ErrorType> BranchBuilder<DataType, OutputType, ErrorType>
where
    DataType:  Send + Sync + Clone + 'static,
    ErrorType: Error + Send + Sync + Clone + 'static,
    OutputType: Send + Sync + Clone + 'static,
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
    #[must_use]
    pub fn new(
        condition: Box<
            dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync,
        >,
        parent: Flow<DataType, ErrorType, OutputType>,
    ) -> Self {
        debug!("Creating new branch builder");
        Self {
            condition: Arc::from(condition),
            then_branch: Vec::new(),
            parent,
            parent_branch: None,
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
    #[must_use]
    pub fn process<ProcessorType>(mut self, processor: ProcessorType) -> Self
    where
        ProcessorType: Processor<Input = DataType, Output = DataType, Error = ErrorType>
            + Send
            + Sync
            + 'static,
    {
        debug!("Adding processor to then branch");
        self.then_branch.push(Stage::Process(Arc::new(processor)));
        self
    }

    /// Transitions to building the alternative branch that will be executed when the condition is false.
    ///
    /// # Returns
    ///
    /// An `OtherwiseBuilder` for constructing the alternative branch
    #[must_use]
    pub fn otherwise(self) -> OtherwiseBuilder<DataType, OutputType, ErrorType> {
        debug!("Creating otherwise builder");
        OtherwiseBuilder::new(self.condition, self.then_branch, self.parent)
    }

    /// Starts a nested branch within the current branch
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition for the nested branch
    ///
    /// # Returns
    ///
    /// A new `BranchBuilder` for the nested branch
    #[must_use]
    pub fn when<ConditionType>(
        self,
        condition: ConditionType,
    ) -> BranchBuilder<DataType, OutputType, ErrorType>
    where
        ConditionType: Condition<Input = DataType, Output = OutputType, Error = ErrorType>
            + Send
            + Sync
            + 'static,
    {
        debug!("Creating nested branch");
        let mut new_branch = BranchBuilder::new(Box::new(condition), self.clone().parent);
        new_branch.parent_branch = Some(Arc::new(self));
        new_branch
    }

    /// Ends the branch without an otherwise clause
    ///
    /// # Returns
    ///
    /// The parent flow with the branch added
    #[must_use]
    pub fn end(
        self,
    ) -> Either<BranchBuilder<DataType, OutputType, ErrorType>, Flow<DataType, ErrorType, OutputType>>
    {
        debug!("Ending branch");
        let branch_stage = Stage::Branch(Arc::new(BranchStage {
            condition: self.condition,
            then_branch: self.then_branch,
            else_branch: Vec::new(),
            _marker: PhantomData,
        }));

        match self.parent_branch {
            Some(parent_branch) => {
                let mut parent = Arc::try_unwrap(parent_branch)
                    .unwrap_or_else(|_| panic!("Failed to unwrap Arc"));
                parent.then_branch.push(branch_stage);
                Either::Left(parent)
            }
            None => {
                let mut flow = self.parent;
                flow.stages.push(branch_stage);
                Either::Right(flow)
            }
        }
    }
}
