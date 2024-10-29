use super::BranchBuilder;
use crate::flow::{
    condition::Condition,
    processor::Processor,
    source::Source,
    stage::{BranchStage, Stage}, // Added BranchStage import
};
use std::error::Error;
use std::fmt;
use tokio::sync::broadcast;

// Define a custom error type for Flow
#[derive(Debug, Clone)]
pub struct FlowError(String);

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Flow error: {}", self.0)
    }
}

impl Error for FlowError {}

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
    #[must_use]
    pub fn new() -> Self {
        Self {
            source: None,
            stages: Vec::new(),
        }
    }

    #[must_use]
    pub fn source<SourceType>(mut self, source: SourceType) -> Self
    where
        SourceType:
            Source<Input = (), Output = DataType, Error = ErrorType> + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

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

    async fn process_item(
        &self,
        mut item: DataType,
        feedback: &flume::Sender<Result<DataType, ErrorType>>,
    ) -> Result<DataType, ErrorType> {
        for stage in &self.stages {
            item = match stage {
                Stage::Process(processor) => processor.process(item).await?,
                Stage::Branch(branch) => self.process_branch(branch, item).await?,
            }
        }
        let _ = feedback.send(Ok(item.clone()));
        Ok(item)
    }

    async fn process_branch(
        &self,
        branch: &BranchStage<DataType, ErrorType, OutputType>,
        item: DataType,
    ) -> Result<DataType, ErrorType> {
        let (condition_met, _) = branch.condition.evaluate(item.clone()).await?;
        let stages = if condition_met {
            &branch.then_branch
        } else {
            &branch.else_branch
        };

        let mut current_item = item;
        for stage in stages {
            if let Stage::Process(processor) = stage {
                current_item = processor.process(current_item).await?;
            }
        }
        Ok(current_item)
    }

    async fn handle_source_item(
        &self,
        item: Result<DataType, ErrorType>,
        feedback: &flume::Sender<Result<DataType, ErrorType>>,
    ) -> Result<Option<DataType>, ErrorType> {
        match item {
            Ok(data) => {
                let result = self.process_item(data, feedback).await;
                if let Err(e) = &result {
                    let _ = feedback.send(Err(e.clone()));
                }
                result.map(Some)
            }
            Err(e) => {
                let _ = feedback.send(Err(e.clone()));
                Err(e)
            }
        }
    }

    /// Runs the flow until completion or error.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - No source is set for the flow
    /// - The source fails to initialize or stream data
    /// - Any processor in the flow returns an error
    /// - Any condition evaluation fails
    pub async fn run_stream(
        mut self,
        shutdown: broadcast::Receiver<()>,
    ) -> Result<Vec<DataType>, ErrorType> {
        let source = self.source.take().ok_or_else(|| {
            // Use take() to move out of Option
            ErrorType::from(FlowError("Flow source not set".to_string()))
        })?;
        let source_output = source.stream().await?;
        let receiver = source_output.receiver;
        let feedback = source_output.feedback;
        let mut results = Vec::new();
        let mut shutdown_rx = shutdown;

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    println!("Received shutdown signal");
                    break;
                }
                item = receiver.recv_async() => {
                    if let Ok(item) = item {
                        match self.handle_source_item(item, &feedback).await {
                            Ok(Some(result)) => results.push(result),
                            Ok(None) => continue,
                            Err(e) => return Err(e),
                        }
                    } else {
                        println!("Source channel closed");
                        break;
                    }
                }
            }
        }

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
