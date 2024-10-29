use super::BranchBuilder;
use crate::flow::{condition::Condition, processor::Processor, source::Source, stage::Stage};
use std::error::Error;
use std::fmt;
use tokio::sync::broadcast;

// Define a custom error type for Flow
#[derive(Debug)]
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
    ErrorType: Error + Send + Sync + 'static + From<FlowError>,
{
    pub fn new() -> Self {
        Self {
            source: None,
            stages: Vec::new(),
        }
    }

    pub fn source<SourceType>(mut self, source: SourceType) -> Self
    where
        SourceType:
            Source<Input = (), Output = DataType, Error = ErrorType> + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

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

    pub async fn run_stream(
        self,
        shutdown: broadcast::Receiver<()>,
    ) -> Result<Vec<DataType>, ErrorType> {
        let source = self
            .source
            .ok_or_else(|| ErrorType::from(FlowError("Flow source not set".to_string())))?;
        let receiver = source.stream().await?;
        let mut results = Vec::new();
        let mut shutdown_rx = shutdown;

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    println!("Received shutdown signal");
                    break;
                }
                item = receiver.recv_async() => {
                    match item {
                        Ok(item) => {
                            let mut current_item = item?;
                            for stage in &self.stages {
                                match stage {
                                    Stage::Process(processor) => {
                                        current_item = processor.process(current_item).await?;
                                    }
                                    Stage::Branch(branch) => {
                                        let (condition_met, _) = branch.condition.evaluate(current_item.clone()).await?;
                                        let stages = if condition_met {
                                            &branch.then_branch
                                        } else {
                                            &branch.else_branch
                                        };

                                        for stage in stages {
                                            if let Stage::Process(processor) = stage {
                                                current_item = processor.process(current_item).await?;
                                            }
                                        }
                                    }
                                }
                            }
                            results.push(current_item);
                        }
                        Err(_) => {
                            // Channel is closed, break the loop
                            println!("Source channel closed");
                            break;
                        }
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
    ErrorType: Error + Send + Sync + 'static + From<FlowError>,
{
    fn default() -> Self {
        Self::new()
    }
}
