use super::BranchBuilder;
use crate::flow::{condition::Condition, processor::Processor, source::Source, stage::Stage};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument, warn};

/// Error type specific to Flow operations.
#[derive(Debug, Clone)]
pub struct FlowError(String);

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Flow error: {}", self.0)
    }
}

impl Error for FlowError {}

/// A builder for constructing and executing data processing flows.
///
/// `Flow` represents a sequence of processing stages that data flows through. It supports
/// various operations including data transformation, conditional branching, and async execution.
///
/// # Type Parameters
///
/// * `DataType` - The type of data flowing through the pipeline
/// * `ErrorType` - The error type that can be produced during processing
/// * `OutputType` - The output type produced by conditions in branches
///
/// # Examples
///
/// ```
/// use cortex_ai::composer::Flow;
/// use cortex_ai::flow::source::Source;
/// use cortex_ai::flow::types::SourceOutput;
/// use cortex_ai::flow::condition::Condition;
/// use cortex_ai::flow::processor::Processor;
/// use cortex_ai::FlowComponent;
/// use cortex_ai::FlowError;
/// use std::error::Error;
/// use std::fmt;
/// use std::pin::Pin;
/// use std::future::Future;
/// use flume::{Sender, Receiver};
///
/// #[derive(Clone, Debug)]
/// struct MyData(String);
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
///
/// impl FlowComponent for MySource {
///     type Input = ();
///     type Output = MyData;
///     type Error = MyError;
/// }
///
/// impl Source for MySource {
///     fn stream(&self) -> Pin<Box<dyn Future<Output = Result<SourceOutput<Self::Output, Self::Error>, Self::Error>> + Send>> {
///         Box::pin(async move {
///             let (tx, rx) = flume::bounded(10);
///             let (feedback_tx, _) = flume::bounded(10);
///             Ok(SourceOutput { receiver: rx, feedback: feedback_tx })
///         })
///     }
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
///     fn evaluate(&self, input: Self::Input) -> Pin<Box<dyn Future<Output = Result<(bool, Option<Self::Output>), Self::Error>> + Send>> {
///         Box::pin(async move { Ok((true, Some(false))) })
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let (_, shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
///     
///     let flow = Flow::<MyData, MyError, bool>::new()
///         .source(MySource)
///         .process(MyProcessor)
///         .when(MyCondition)
///         .process(MyProcessor)
///         .otherwise()
///         .process(MyProcessor)
///         .end();
///
///     let _ = flow.run_stream(shutdown_rx).await;
/// }
/// ```
pub struct Flow<DataType, ErrorType, OutputType> {
    pub(crate) source:
        Option<Box<dyn Source<Input = (), Output = DataType, Error = ErrorType> + Send + Sync>>,
    pub(crate) stages: Vec<Stage<DataType, ErrorType, OutputType>>,
}

impl<DataType, ErrorType, OutputType> Flow<DataType, ErrorType, OutputType>
where
    DataType: Clone + Send + Sync + 'static,
    OutputType: Send + Sync + 'static,
    ErrorType: Error + Send + Sync + Clone + 'static + From<FlowError>,
{
    /// Creates a new empty Flow.
    ///
    /// # Returns
    ///
    /// A new instance of `Flow` with no source or stages configured
    #[must_use]
    pub fn new() -> Self {
        Self {
            source: None,
            stages: Vec::new(),
        }
    }

    /// Sets the data source for the flow.
    ///
    /// # Arguments
    ///
    /// * `source` - The source that will produce data for the flow
    ///
    /// # Returns
    ///
    /// The flow builder for method chaining
    #[must_use]
    pub fn source<SourceType>(mut self, source: SourceType) -> Self
    where
        SourceType:
            Source<Input = (), Output = DataType, Error = ErrorType> + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

    /// Adds a processor stage to the flow.
    ///
    /// # Arguments
    ///
    /// * `processor` - The processor to add to the flow
    ///
    /// # Returns
    ///
    /// The flow builder for method chaining
    #[must_use]
    pub fn process<ProcessorType>(mut self, processor: ProcessorType) -> Self
    where
        ProcessorType: Processor<Input = DataType, Output = DataType, Error = ErrorType>
            + Send
            + Sync
            + 'static,
    {
        self.stages.push(Stage::Process(Box::new(processor)));
        self
    }

    /// Starts building a conditional branch in the flow.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition that determines which branch to take
    ///
    /// # Returns
    ///
    /// A `BranchBuilder` for constructing the conditional branches
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
        BranchBuilder::new(Box::new(condition), self)
    }

    /// Executes the flow asynchronously, processing data from the source through all stages.
    ///
    /// # Arguments
    ///
    /// * `shutdown` - A broadcast receiver for graceful shutdown signaling
    ///
    /// # Returns
    ///
    /// A Result containing either a vector of processed data items or an error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The flow source is not set
    /// * Any stage in the flow returns an error during processing
    /// * The task execution fails
    #[instrument(skip(self))]
    pub async fn run_stream(
        mut self,
        shutdown: broadcast::Receiver<()>,
    ) -> Result<Vec<DataType>, ErrorType> {
        info!("Starting flow execution");
        let source = self.source.take().ok_or_else(|| {
            error!("Flow source not set");
            ErrorType::from(FlowError("Flow source not set".to_string()))
        })?;

        debug!("Initializing source stream");
        let source_output = source.stream().await?;
        let receiver = source_output.receiver;
        let feedback = source_output.feedback;
        let mut results = Vec::new();
        let mut shutdown_rx = shutdown;
        let mut handles = Vec::new();

        let stages = Arc::new(self.stages);
        info!("Starting message processing loop");

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    warn!("Received shutdown signal");
                    break;
                }
                item = receiver.recv_async() => {
                    if let Ok(item) = item {
                        let feedback = feedback.clone();
                        let stages = Arc::clone(&stages);

                        debug!("Spawning task for data processing");
                        let handle = tokio::spawn(async move {
                            let mut current_item = match item {
                                Ok(data) => {
                                    debug!("Processing new item");
                                    data
                                },
                                Err(e) => {
                                    error!("Source error: {:?}", e);
                                    let _ = feedback.send(Err(e.clone()));
                                    return Err(e);
                                }
                            };

                            for stage in stages.iter() {
                                match stage {
                                    Stage::Process(processor) => {
                                        debug!("Executing processor stage");
                                        current_item = match processor.process(current_item).await {
                                            Ok(data) => data,
                                            Err(e) => {
                                                error!("Processor error: {:?}", e);
                                                let _ = feedback.send(Err(e.clone()));
                                                return Err(e);
                                            }
                                        };
                                    }
                                    Stage::Branch(branch) => {
                                        debug!("Evaluating branch condition");
                                        let (condition_met, _) = match branch.condition.evaluate(current_item.clone()).await {
                                            Ok(result) => result,
                                            Err(e) => {
                                                error!("Branch condition error: {:?}", e);
                                                let _ = feedback.send(Err(e.clone()));
                                                return Err(e);
                                            }
                                        };

                                        let stages = if condition_met {
                                            debug!("Taking then branch");
                                            &branch.then_branch
                                        } else {
                                            debug!("Taking else branch");
                                            &branch.else_branch
                                        };

                                        for stage in stages {
                                            if let Stage::Process(processor) = stage {
                                                current_item = match processor.process(current_item).await {
                                                    Ok(data) => data,
                                                    Err(e) => {
                                                        error!("Branch processor error: {:?}", e);
                                                        let _ = feedback.send(Err(e.clone()));
                                                        return Err(e);
                                                    }
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                            debug!("Data processing completed successfully");
                            let _ = feedback.send(Ok(current_item.clone()));
                            Ok(current_item)
                        });
                        handles.push(handle);
                    } else {
                        debug!("Source channel closed");
                        break;
                    }
                }
            }
        }

        debug!("Collecting results from all tasks");
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    error!("Task error: {:?}", e);
                    return Err(e);
                }
                Err(e) => {
                    error!("Task join error: {:?}", e);
                    return Err(ErrorType::from(FlowError(e.to_string())));
                }
            }
        }

        debug!("Flow execution completed successfully");
        Ok(results)
    }
}

impl<DataType, ErrorType, OutputType> Default for Flow<DataType, ErrorType, OutputType>
where
    DataType: Clone + Send + Sync + 'static,
    OutputType: Send + Sync + 'static,
    ErrorType: Error + Send + Sync + Clone + 'static + From<FlowError>,
{
    fn default() -> Self {
        Self::new()
    }
}
