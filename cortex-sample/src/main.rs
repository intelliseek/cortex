use cortex_ai::{
    Condition, ConditionFuture, Flow, FlowComponent, FlowError, FlowFuture, Processor, Sink,
};
use cortex_sources::kafka::{KafkaConfig, KafkaSource};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::marker::PhantomData;
use tokio::sync::broadcast;

// Define our ClickBehavior type
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickBehavior {
    user_id: String,
    event_type: String,
    session_id: String,
}

// Implement TryFrom for ClickBehavior to deserialize from Kafka messages
impl TryFrom<Vec<u8>> for ClickBehavior {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        serde_json::from_slice(&bytes).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

// Generic processor that filters by a field
struct FieldFilter<T, F>
where
    F: Fn(&T) -> String,
{
    field_extractor: F,
    expected_value: String,
    _phantom: PhantomData<T>,
}

impl<T, F> FieldFilter<T, F>
where
    F: Fn(&T) -> String,
{
    #[must_use]
    pub const fn new(field_extractor: F, expected_value: String) -> Self {
        Self {
            field_extractor,
            expected_value,
            _phantom: PhantomData,
        }
    }
}

impl<T, F> FlowComponent for FieldFilter<T, F>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(&T) -> String + Send + Sync + 'static,
{
    type Input = T;
    type Output = T;
    type Error = FlowError;
}

impl<T, F> Processor for FieldFilter<T, F>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(&T) -> String + Send + Sync + 'static,
{
    fn process(&self, input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error> {
        let expected = self.expected_value.clone();
        let actual = (self.field_extractor)(&input);

        Box::pin(async move {
            if actual == expected {
                Ok(input)
            } else {
                Err(FlowError::Process(format!(
                    "Field mismatch: expected {expected}, got {actual}"
                )))
            }
        })
    }
}

// Generic condition that checks a field
#[derive(Clone)]
struct FieldCondition<T, F>
where
    F: Fn(&T) -> String,
{
    field_extractor: F,
    expected_value: String,
    _phantom: PhantomData<T>,
}

impl<T, F> FieldCondition<T, F>
where
    F: Fn(&T) -> String,
{
    #[must_use]
    pub const fn new(field_extractor: F, expected_value: String) -> Self {
        Self {
            field_extractor,
            expected_value,
            _phantom: PhantomData,
        }
    }
}

impl<T, F> FlowComponent for FieldCondition<T, F>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(&T) -> String + Send + Sync + 'static,
{
    type Input = T;
    type Output = bool;
    type Error = FlowError;
}

impl<T, F> Condition for FieldCondition<T, F>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(&T) -> String + Send + Sync + 'static,
{
    fn evaluate(&self, input: Self::Input) -> ConditionFuture<'_, Self::Output, Self::Error> {
        let expected = self.expected_value.clone();
        let actual = (self.field_extractor)(&input);

        Box::pin(async move { Ok((actual == expected, Some(true))) })
    }
}

// Implement a simple sink
struct PrintSink;

impl FlowComponent for PrintSink {
    type Input = ClickBehavior;
    type Output = ClickBehavior;
    type Error = FlowError;
}

impl Sink for PrintSink {
    fn sink(&self, input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error> {
        println!(
            "Consumed event - User: {}, Event: {}, Session: {}",
            input.user_id, input.event_type, input.session_id
        );
        Box::pin(async move { Ok(input) })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Configure Kafka source
    let kafka_config = KafkaConfig {
        bootstrap_servers: "localhost:9092".to_string(),
        topic: "click-events".to_string(),
        group_id: "click-processor".to_string(),
        auto_offset_reset: "earliest".to_string(),
        session_timeout_ms: 6000,
    };

    // Create Kafka source for ClickBehavior - pass reference to config
    let source = KafkaSource::<ClickBehavior>::new(&kafka_config)?;

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    // Build and run the flow
    let flow = Flow::new()
        .source(source)
        .process(FieldFilter::new(
            |click: &ClickBehavior| click.event_type.clone(),
            "button_click".to_string(),
        ))
        .when(FieldCondition::new(
            |click: &ClickBehavior| click.session_id.clone(),
            "session123".to_string(),
        ))
        .process(FieldFilter::new(
            |click: &ClickBehavior| click.event_type.clone(),
            "purchase".to_string(),
        ))
        .otherwise()
        .process(FieldFilter::new(
            |click: &ClickBehavior| click.event_type.clone(),
            "view".to_string(),
        ))
        .end()
        .sink(PrintSink);

    // Run the flow
    let handle = tokio::spawn(async move {
        match flow.run_stream(shutdown_rx).await {
            Ok(results) => {
                println!("Processed {} events", results.len());
                for result in results {
                    println!(
                        "User: {}, Event: {}, Session: {}",
                        result.user_id, result.event_type, result.session_id
                    );
                }
            }
            Err(e) => eprintln!("Flow error: {e}"),
        }
    });

    // Let it run for a while
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    // Send shutdown signal
    let _ = shutdown_tx.send(());

    // Wait for the flow to complete
    handle.await?;

    Ok(())
}
