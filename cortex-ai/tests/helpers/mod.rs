use cortex_ai::{
    composer::flow::FlowError, Condition, ConditionFuture, Flow, FlowComponent, FlowFuture,
    Processor, Source,
};
use flume::bounded;
use std::error::Error;
use std::fmt;
use std::time::Duration;
use tokio::sync::broadcast;

// Error Types
#[derive(Debug)]
pub struct TestError(pub String);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Test error: {}", self.0)
    }
}

impl Error for TestError {}

impl From<FlowError> for TestError {
    fn from(error: FlowError) -> Self {
        TestError(error.to_string())
    }
}

// Test Components
#[derive(Debug)]
pub struct TestProcessor;
pub struct TestCondition;
pub struct TestSource {
    pub data: String,
}

impl FlowComponent for TestProcessor {
    type Input = String;
    type Output = String;
    type Error = TestError;
}

impl Processor for TestProcessor {
    fn process<'a>(&'a self, input: Self::Input) -> FlowFuture<'a, Self::Output, Self::Error> {
        Box::pin(async move { Ok(format!("processed_{}", input)) })
    }
}

impl FlowComponent for TestCondition {
    type Input = String;
    type Output = String;
    type Error = TestError;
}

impl Condition for TestCondition {
    fn evaluate<'a>(
        &'a self,
        input: Self::Input,
    ) -> ConditionFuture<'a, Self::Output, Self::Error> {
        Box::pin(async move {
            let condition_met = input.contains("test");
            Ok((condition_met, Some(input)))
        })
    }
}

impl FlowComponent for TestSource {
    type Input = ();
    type Output = String;
    type Error = TestError;
}

impl Source for TestSource {
    fn stream<'a>(
        &'a self,
    ) -> FlowFuture<'a, flume::Receiver<Result<Self::Output, Self::Error>>, Self::Error> {
        let data = self.data.clone();
        Box::pin(async move {
            let (tx, rx) = bounded(1);
            tx.send(Ok(data)).unwrap();
            drop(tx);
            Ok(rx)
        })
    }
}

// Helper Functions
pub async fn run_flow_with_timeout<DataType, ErrorType, OutputType>(
    flow: Flow<DataType, ErrorType, OutputType>,
    timeout: Duration,
) -> Result<Vec<DataType>, ErrorType>
where
    DataType: Clone + Send + Sync + 'static,
    OutputType: Send + Sync + 'static,
    ErrorType: Error + Send + Sync + 'static + From<FlowError>,
{
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let handle = tokio::spawn(async move { flow.run_stream(shutdown_rx).await });

    tokio::time::sleep(timeout).await;
    let _ = shutdown_tx.send(());

    handle.await.unwrap()
}
