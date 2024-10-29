use cortex_pipeline::{
    Condition, ConditionFuture, Pipeline, PipelineComponent, PipelineFuture, Processor, Source,
};

use std::error::Error;
use std::fmt;

// Custom error type for our example
#[derive(Debug)]
struct ExampleError(String);

impl fmt::Display for ExampleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Example error: {}", self.0)
    }
}

impl Error for ExampleError {}

// We'll create some example components
struct ExampleSource;
struct ExampleProcessor;
struct ExampleCondition;

// Implement the necessary traits (simplified for demonstration)
impl PipelineComponent for ExampleSource {
    type Input = ();
    type Output = String;
    type Error = ExampleError;
}

impl Source for ExampleSource {
    fn stream<'a>(
        &'a self,
    ) -> PipelineFuture<'a, flume::Receiver<Result<Self::Output, Self::Error>>, Self::Error> {
        Box::pin(async move {
            let (tx, rx) = flume::bounded(10);
            // Example: send one message
            tx.send(Ok("Sample data".to_string())).unwrap();
            Ok(rx)
        })
    }
}

impl PipelineComponent for ExampleProcessor {
    type Input = String;
    type Output = String;
    type Error = ExampleError;
}

impl Processor for ExampleProcessor {
    fn process<'a>(&'a self, input: Self::Input) -> PipelineFuture<'a, Self::Output, Self::Error> {
        Box::pin(async move { Ok(format!("Processed: {}", input)) })
    }
}

// Add this implementation before implementing Condition
impl PipelineComponent for ExampleCondition {
    type Input = String;
    type Output = bool;
    type Error = ExampleError;
}

impl Condition for ExampleCondition {
    fn evaluate<'a>(
        &'a self,
        input: Self::Input,
    ) -> ConditionFuture<'a, (bool, Option<Self::Output>), Self::Error> {
        Box::pin(async move {
            let condition_met = input.contains("Sample");
            Ok((
                condition_met,
                if condition_met {
                    Some((true, Some(true)))
                } else {
                    None
                },
            ))
        })
    }
}

// For the tokio macro error, we'll use the runtime directly instead
fn main() -> Result<(), Box<dyn Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let pipeline = Pipeline::new()
            .source(ExampleSource)
            .when(ExampleCondition)
            .process(ExampleProcessor)
            .otherwise()
            .process(ExampleProcessor)
            .end()
            .process(ExampleProcessor);

        pipeline.run_stream().await?;
        Ok(())
    })
}
