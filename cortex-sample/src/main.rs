use cortex_ai::{
    composer::flow::FlowError, flow::types::SourceOutput, Condition, ConditionFuture, Flow,
    FlowComponent, FlowFuture, Processor, Source,
};
use std::error::Error;
use std::fmt;
use tokio::sync::broadcast;

// Custom error type for our example
#[derive(Debug, Clone)]
struct ExampleError(String);

impl fmt::Display for ExampleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Example error: {}", self.0)
    }
}

impl Error for ExampleError {}

impl From<FlowError> for ExampleError {
    fn from(error: FlowError) -> Self {
        Self(error.to_string())
    }
}

// We'll create some example components
struct ExampleSource;
struct ExampleProcessor;
struct ExampleCondition;

// Implement the necessary traits (simplified for demonstration)
impl FlowComponent for ExampleSource {
    type Input = ();
    type Output = String;
    type Error = ExampleError;
}

// Add FlowComponent implementation for ExampleProcessor
impl FlowComponent for ExampleProcessor {
    type Input = String;
    type Output = String;
    type Error = ExampleError;
}

impl Source for ExampleSource {
    fn stream(&self) -> FlowFuture<'_, SourceOutput<Self::Output, Self::Error>, Self::Error> {
        Box::pin(async move {
            let (tx, rx) = flume::bounded(10);
            let (feedback_tx, feedback_rx) = flume::bounded(10);

            // Example: send one message
            println!("Sending message");
            tx.send(Ok("Sample data".to_string())).unwrap();
            drop(tx);

            // Handle feedback
            tokio::spawn(async move {
                while let Ok(result) = feedback_rx.recv_async().await {
                    match result {
                        Ok(data) => println!("Processing succeeded: {data}"),
                        Err(e) => println!("Processing failed: {e}"),
                    }
                }
            });

            Ok(SourceOutput {
                receiver: rx,
                feedback: feedback_tx,
            })
        })
    }
}

impl Processor for ExampleProcessor {
    fn process(&self, input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error> {
        println!("Processing: {input}");
        Box::pin(async move { Ok(format!("Processed: {input}")) })
    }
}

impl FlowComponent for ExampleCondition {
    type Input = String;
    type Output = String;
    type Error = ExampleError;
}

impl Condition for ExampleCondition {
    fn evaluate(&self, input: Self::Input) -> ConditionFuture<'_, Self::Output, Self::Error> {
        Box::pin(async move {
            let condition_met = input.contains("Sample");
            println!("Condition met: {condition_met}");
            Ok((condition_met, Some(input)))
        })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let flow = Flow::new()
            .source(ExampleSource)
            .when(ExampleCondition)
            .process(ExampleProcessor)
            .otherwise()
            .process(ExampleProcessor)
            .end()
            .process(ExampleProcessor);

        let handle = tokio::spawn(async move { flow.run_stream(shutdown_rx).await });

        // Let it run for a while
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Try to send shutdown signal, ignore if flow already completed
        let _ = shutdown_tx.send(());

        // Wait for the flow to complete and handle any errors
        handle
            .await
            .map_err(|e| Box::new(ExampleError(e.to_string())) as Box<dyn Error>)?
            .map(|_results| ()) // Map Vec<String> to ()
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    })
}
