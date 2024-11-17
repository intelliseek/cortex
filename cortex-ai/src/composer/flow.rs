use super::BranchBuilder;
use crate::{
    flow::{
        condition::Condition,
        processor::Processor,
        sink::Sink,
        source::Source,
        stage::{BranchStage, Stage},
    },
    FlowError,
};
use std::{error::Error, sync::Arc};
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument, warn};

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
    pub(crate) has_sink: bool,
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
            has_sink: false,
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
        if !self.stages.is_empty() {
            panic!("Source must be the first component in a flow");
        }
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
        if self.source.is_none() {
            panic!("Flow must start with a source");
        }
        if self.has_sink {
            panic!("Cannot add processor after sink - sink must be the last component");
        }
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
        if self.source.is_none() {
            panic!("Flow must start with a source");
        }
        if self.has_sink {
            panic!("Cannot add condition after sink - sink must be the last component");
        }
        BranchBuilder::new(Box::new(condition), self)
    }

    /// Adds a sink stage to the flow.
    ///
    /// # Arguments
    ///
    /// * `sink` - The sink to add to the flow
    ///
    /// # Returns
    ///
    /// The flow builder for method chaining
    #[must_use]
    pub fn sink<SinkType>(mut self, sink: SinkType) -> Self
    where
        SinkType:
            Sink<Input = DataType, Output = DataType, Error = ErrorType> + Send + Sync + 'static,
    {
        if self.source.is_none() {
            panic!("Flow must start with a source");
        }
        if self.stages.is_empty() {
            panic!("Flow must have at least one processor or condition before sink");
        }
        if self.has_sink {
            panic!("Flow already has a sink component");
        }
        self.stages.push(Stage::Sink(Box::new(sink)));
        self.has_sink = true;
        self
    }

    fn validate(&self) -> Result<(), ErrorType> {
        if self.source.is_none() {
            return Err(ErrorType::from(FlowError::NoSource));
        }
        if !self.has_sink {
            return Err(ErrorType::from(FlowError::NoSink));
        }
        Ok(())
    }

    async fn process_item(
        item: DataType,
        stages: &[Stage<DataType, ErrorType, OutputType>],
        context: &ProcessingContext<DataType, ErrorType>,
    ) -> Result<DataType, ErrorType> {
        let mut current = item;
        for stage in stages {
            current = Self::execute_stage(current, stage, context).await?;
        }
        Ok(current)
    }

    // Single Responsibility: Execute a single stage
    async fn execute_stage(
        input: DataType,
        stage: &Stage<DataType, ErrorType, OutputType>,
        context: &ProcessingContext<DataType, ErrorType>,
    ) -> Result<DataType, ErrorType> {
        match stage {
            Stage::Process(processor) => Self::execute_processor(input, processor, context).await,
            Stage::Sink(sink) => Self::execute_sink(input, sink, context).await,
            Stage::Branch(branch) => Self::execute_branch(input, branch, context).await,
        }
    }

    // Single Responsibility: Execute a processor
    async fn execute_processor(
        input: DataType,
        processor: &Box<dyn Processor<Input = DataType, Output = DataType, Error = ErrorType> + Send + Sync>,
        context: &ProcessingContext<DataType, ErrorType>,
    ) -> Result<DataType, ErrorType> {
        debug!("Executing processor stage");
        let result = processor.process(input).await;
        // Only send feedback for errors
        if let Err(ref e) = result {
            let _ = context.send_feedback(Err(e.clone()));
        }
        result
    }

    async fn execute_sink(
        input: DataType,
        sink: &Box<dyn Sink<Input = DataType, Output = DataType, Error = ErrorType> + Send + Sync>,
        context: &ProcessingContext<DataType, ErrorType>,
    ) -> Result<DataType, ErrorType> {
        debug!("Executing sink stage");
        let result = sink.sink(input).await;
        // Send feedback and ensure it's received
        context.send_feedback(result.clone());
        result
    }

    async fn execute_branch(
        input: DataType,
        branch: &BranchStage<DataType, ErrorType, OutputType>,
        context: &ProcessingContext<DataType, ErrorType>,
    ) -> Result<DataType, ErrorType> {
        debug!("Evaluating branch condition");
        let (condition_met, _) = Self::evaluate_condition(&input, &branch.condition, context).await?;
        
        let stages = if condition_met {
            debug!("Taking then branch");
            &branch.then_branch
        } else {
            debug!("Taking else branch");
            &branch.else_branch
        };

        // Process stages iteratively
        let mut current = input;
        for stage in stages {
            current = match stage {
                Stage::Process(processor) => {
                    Self::execute_processor(current, processor, context).await?
                }
                Stage::Sink(sink) => {
                    Self::execute_sink(current, sink, context).await?
                }
                Stage::Branch(_) => {
                    warn!("Nested branches are not supported");
                    current
                }
            };
        }
        Ok(current)
    }

    async fn evaluate_condition(
        input: &DataType,
        condition: &Box<
            dyn Condition<Input = DataType, Output = OutputType, Error = ErrorType> + Send + Sync,
        >,
        context: &ProcessingContext<DataType, ErrorType>,
    ) -> Result<(bool, Option<OutputType>), ErrorType> {
        let result = condition.evaluate(input.clone()).await;
        if let Err(ref e) = result {
            let _ = context.send_feedback(Err(e.clone()));
        }
        result
    }

    // Main entry point
    #[instrument(skip(self))]
    pub async fn run_stream(
        mut self,
        shutdown: broadcast::Receiver<()>,
    ) -> Result<Vec<DataType>, ErrorType> {
        info!("Starting flow execution");
        self.validate()?;

        let source = self.source.take().unwrap();
        let source_output = source.stream().await?;

        let context = ProcessingContext {
            feedback: source_output.feedback,
        };

        let stages = Arc::new(self.stages);
        let mut results = Vec::new();
        let mut shutdown_rx = shutdown;

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    warn!("Received shutdown signal");
                    break;
                }
                item = source_output.receiver.recv_async() => {
                    match item {
                        Ok(Ok(data)) => {
                            let stages = Arc::clone(&stages);
                            let context = context.clone();
                            match Self::process_item(data, &stages, &context).await {
                                Ok(processed) => {
                                    // Don't send feedback here - only sinks send success feedback
                                    results.push(processed);
                                }
                                Err(e) => {
                                    error!("Processing error: {:?}", e);
                                    let _ = context.send_feedback(Err(e.clone()));
                                    return Err(e);
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            error!("Source error: {:?}", e);
                            let _ = context.send_feedback(Err(e.clone()));
                            return Err(e);
                        }
                        Err(_) => {
                            debug!("Source channel closed");
                            break;
                        }
                    }
                }
            }
        }

        debug!("Flow execution completed successfully");
        Ok(results)
    }
}

// Context for processing operations
#[derive(Clone)]
struct ProcessingContext<DataType, ErrorType> {
    feedback: flume::Sender<Result<DataType, ErrorType>>,
}

impl<DataType, ErrorType> ProcessingContext<DataType, ErrorType>
where
    DataType: Clone + Send + Sync + 'static,
    ErrorType: std::error::Error + Send + Sync + Clone + 'static,
{
    fn send_feedback(&self, result: Result<DataType, ErrorType>) {
        if let Err(e) = self.feedback.send(result) {
            error!("Failed to send feedback: {}", e);
        }
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
