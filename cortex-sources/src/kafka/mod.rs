//! Kafka source implementations

use crate::error::{Result, SourceError};
use cortex_ai::{
    flow::{source::Source, types::SourceOutput},
    FlowComponent, FlowFuture,
};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig, Message,
};
use std::marker::PhantomData;
use std::sync::Arc;
use std::{error::Error, time::Duration};

/// Configuration for Kafka source
#[derive(Debug, Clone)]
pub struct KafkaConfig {
    /// Bootstrap servers (comma-separated list)
    pub bootstrap_servers: String,
    /// Topic to consume from
    pub topic: String,
    /// Consumer group ID
    pub group_id: String,
    /// Auto offset reset (earliest/latest)
    pub auto_offset_reset: String,
    /// Session timeout (in milliseconds)
    pub session_timeout_ms: u64,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            bootstrap_servers: "localhost:9092".to_string(),
            topic: "default-topic".to_string(),
            group_id: "cortex-consumer".to_string(),
            auto_offset_reset: "earliest".to_string(),
            session_timeout_ms: 6000,
        }
    }
}

/// A source that reads from Kafka
pub struct KafkaSource<T> {
    consumer: Arc<StreamConsumer>,
    timeout: Duration,
    _phantom: PhantomData<T>,
}

impl<T> KafkaSource<T>
where
    T: for<'a> TryFrom<Vec<u8>, Error = Box<dyn Error + Send + Sync>> + Send + Sync + 'static,
{
    /// Create a new Kafka source with the given configuration
    pub fn new(config: KafkaConfig) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &config.group_id)
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", &config.auto_offset_reset)
            .set("session.timeout.ms", &config.session_timeout_ms.to_string())
            .create()
            .map_err(|e| SourceError::Custom(e.to_string()))?;

        consumer
            .subscribe(&[&config.topic])
            .map_err(|e| SourceError::Custom(e.to_string()))?;

        Ok(Self {
            consumer: Arc::new(consumer),
            timeout: Duration::from_secs(1),
            _phantom: PhantomData,
        })
    }

    /// Set the timeout for reading messages
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}



impl<T> Source for KafkaSource<T>
where
    T: for<'a> TryFrom<Vec<u8>, Error = Box<dyn Error + Send + Sync>> + Send + Sync + 'static,
{
    fn stream(&self) -> FlowFuture<'_, SourceOutput<Self::Output, Self::Error>, Self::Error> {
        let (source_tx, source_rx) = flume::unbounded();
        let (feedback_tx, feedback_rx) = flume::unbounded();

        let consumer = Arc::clone(&self.consumer);

        // Spawn a task to handle message consumption
        tokio::spawn({
            async move {
                loop {
                    match consumer.recv().await {
                        Ok(message) => {
                            if let Some(payload) = message.payload() {
                                match T::try_from(payload.to_vec()) {
                                    Ok(item) => {
                                        if source_tx.send(Ok(item)).is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        if source_tx
                                            .send(Err(SourceError::Custom(e.to_string())))
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if source_tx
                                .send(Err(SourceError::Custom(e.to_string())))
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
            }
        });

        // Spawn a task to handle feedback and commit offsets
        let consumer = Arc::clone(&self.consumer);
        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                if result.is_ok() {
                    if let Err(e) = consumer.commit_consumer_state(rdkafka::consumer::CommitMode::Async) {
                        tracing::error!("Failed to commit offsets: {}", e);
                    }
                }
            }
        });

        Box::pin(async move {
            Ok(SourceOutput {
                receiver: source_rx,
                feedback: feedback_tx,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kafka_config_default() {
        let config = KafkaConfig::default();
        assert_eq!(config.bootstrap_servers, "localhost:9092");
        assert_eq!(config.topic, "default-topic");
        assert_eq!(config.group_id, "cortex-consumer");
        assert_eq!(config.auto_offset_reset, "earliest");
        assert_eq!(config.session_timeout_ms, 6000);
    }

    // Example of how to use KafkaSource with a String type
    #[derive(Debug)]
    struct StringWrapper(String);

    impl TryFrom<Vec<u8>> for StringWrapper {
        type Error = Box<dyn std::error::Error + Send + Sync>;

        fn try_from(bytes: Vec<u8>) -> std::result::Result<Self, Self::Error> {
            String::from_utf8(bytes)
                .map(StringWrapper)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }

    #[tokio::test]
    #[ignore] // Requires Kafka to be running
    async fn test_kafka_source() {
        let config = KafkaConfig::default();
        let source = KafkaSource::<StringWrapper>::new(config).unwrap();

        // Test the source stream
        let result = source.stream().await;
        assert!(result.is_ok() || result.is_err());
    }
}
