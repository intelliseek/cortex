# Cortex Flow

[![Crates.io](https://img.shields.io/crates/v/cortex-ai.svg)](https://crates.io/crates/cortex-ai)
[![Documentation](https://docs.rs/cortex-ai/badge.svg)](https://docs.rs/cortex-ai)
[![License: MIT](https://img.shields.io/badge/License-Apache-blue.svg)](https://opensource.org/license/apache-2-0)
[![Coverage Status](https://codecov.io/gh/cortex-ai/cortex-ai/branch/main/graph/badge.svg)](https://codecov.io/gh/cortex-ai/cortex-ai)

A high-performance, asynchronous data processing pipeline library in Rust. Implemented to be used in RAG applications providing ready to use OpenAI compatible APIs, especially OpenRouter.ai.

## Features

- 🚀 Async design for high throughput
- 🔄 Flexible flow composition with branching
- 🔧 Modular and extensible architecture
- 📊 Built-in feedback mechanism
- 🛡️ Type-safe data processing
- 📈 Performance benchmarking
- 🧪 Comprehensive testing and documentation
- 📚 Example processors, sources, conditions and best practices
- 🔒 Safe Rust

## Installation

Add this to your `Cargo.toml`:
```toml
[dependencies]
cortex-ai = "0.1.0"
```

## Quick Start
```rust
use cortex_ai::{Flow, Source, Processor, Condition};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flow = Flow::new()
        .source(MySource)
        .process(MyProcessor)
        .when(MyCondition)
        .process(ThenProcessor)
        .otherwise()
        .process(ElseProcessor)
        .end();
    flow.run_stream(shutdown_rx).await?;
    Ok(())
}
```


## Usage Examples

### Simple Processing Flow

```rust
let flow = Flow::new()
    .source(DataSource::new())
    .process(DataProcessor::new())
    .run_stream(shutdown_rx)
    .await?;
```


### Conditional Branching

```rust
let flow = Flow::new()
    .source(DataSource::new())
    .when(ValidationCondition::new())
    .process(ValidDataProcessor::new())
    .otherwise()
    .process(InvalidDataProcessor::new())
    .end();
```


## Performance

Benchmark results for different flow configurations:

- Simple Flow: ~1M messages/sec
- Branching Flow: ~800K messages/sec
- Complex Flow: ~500K messages/sec
- Concurrent Flow: ~700K messages/sec

Run benchmarks with:

```bash
just bench
```

## Development

### Prerequisites

- Rust 1.80 or higher
- Cargo
- just

### Building

```bash
just build
```

### Testing

```bash
just test
```

### Formatting & Linting

```bash
just check
```

### Running the sample project

```bash
just sample
```

### Project Layout
```
cortex-ai/
├── src/
│ ├── composer/ # Flow composition
│ ├── flow/ # Core flow components
│ └── lib.rs # Public API   
├── tests/ # unit tests
└── benches/ # performance benchmarks

cortex-processors/
├── src/
│ └── processors/ # Processor implementations
│ └── lib.rs # Public API   
├── tests/ # unit tests
└── benches/ # performance benchmarks

cortex-sources/
├── src/
│ └── sources/ # Source implementations
│ └── lib.rs # Public API   
├── tests/ # unit tests
└── benches/ # performance benchmarks

cortex-conditions/
├── src/
│ └── conditions/ # Condition implementations
│ └── lib.rs # Public API   
├── tests/ # unit tests
└── benches/ # performance benchmarks
```

## Implementing a New Processor


To add a new processor to your flow, implement the `Processor` trait:

```rust
use cortex_ai::{FlowComponent, Processor, FlowFuture};
// 1. Define your processor struct
#[derive(Clone)]
struct MultiplicationProcessor {
    // Add any configuration or state here
    multiplier: i32,
}
// 2. Implement FlowComponent to define types
impl FlowComponent for MultiplicationProcessor {
    type Input = i32; // Input type
    type Output = i32; // Output type
    type Error = MyError; // Error type
}
// 3. Implement the Processor trait
impl Processor for MultiplicationProcessor {
    fn process(&self, input: Self::Input) -> FlowFuture<', Self::Output, Self::Error> {
        let multiplier = self.multiplier;
        Box::pin(async move {
            // Process your data here

            Ok(input * multiplier)
        })
    }
}
// 4. Use in a flow
let flow = Flow::new()
    .source(MySource::new())
    .process(MultiplicationProcessor { multiplier: 2 })
    .process(AnotherProcessor::new());
```


### Best Practices
```
1. Make your processor `Clone` if possible
2. Keep processing functions pure and stateless
3. Use appropriate error types
4. Document input/output requirements
5. Add unit tests for your processor
```
### Example: String Processor

```rust
#[derive(Clone)]
struct StringTransformer;

impl FlowComponent for StringTransformer {
    type Input = String;
    type Output = String;
    type Error = ProcessError;
}
impl Processor for StringTransformer {
    fn process(&self, input: Self::Input) -> FlowFuture<', Self::Output, Self::Error> {
        Box::pin(async move {
            Ok(input.to_uppercase())
        })
    }
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_string_transformer() {
        let processor = StringTransformer;
        let result = processor.process("hello".to_string()).await;
        assert_eq!(result.unwrap(), "HELLO");
    }
}
```

## Implementing a New Source

To create a new data source for your flow, implement the `Source` trait:

```rust
use cortex_ai::{FlowComponent, Source, FlowFuture};

// 1. Define your source struct
#[derive(Clone)]
struct MySource {
    // Add any configuration or state here
}

// 2. Implement FlowComponent to define types
impl FlowComponent for MySource {
    type Input = MyInput; // Input type
    type Output = MyOutput; // Output type
    type Error = MyError; // Error type
}

// 3. Implement the Source trait
impl Source for MySource {
    fn source(&self) -> FlowFuture<', Self::Output, Self::Error> {
        Box::pin(async move {
            // Source your data here

            Ok(MyOutput)
        })
    }
}

// 4. Use in a flow
let flow = Flow::new()
    .source(MySource::new())
    .process(MyProcessor)
    .run_stream(shutdown_rx)
    .await?;
```

### Example: Kafka Source
```rust
use cortex_ai::{FlowComponent, Source, FlowFuture, SourceOutput};
use flume::bounded;

// 1. Define your source struct
#[derive(Clone)]
struct KafkaSource {
    topic: String,
    broker: String,
}

// 2. Implement FlowComponent to define types
impl FlowComponent for KafkaSource {
    type Input = (); // Source input is always ()
    type Output = String; // Define what your source produces
    type Error = SourceError; // Your error type
}

// 3. Implement the Source trait
impl Source for KafkaSource {
    fn stream(&self) -> FlowFuture<', SourceOutput<Self::Output, Self::Error>, Self::Error> {
        let topic = self.topic.clone();
        let broker = self.broker.clone();
        Box::pin(async move {
            // Create channels for data and feedback
            let (tx, rx) = bounded(1000);
            let (feedback_tx, feedback_rx) = bounded(1000);
            // Handle feedback in a separate task
            tokio::spawn(async move {
                while let Ok(result) = feedback_rx.recv_async().await {
                    match result {
                        Ok(data) => {
                            // Successfully processed message
                            println!("Message processed: {}", data);
                            // Here you could commit offset in Kafka
                        }
                        Err(e) => {
                            // Processing failed
                            println!("Processing failed: {}", e);
                            // Here you could implement retry logic
                        }
                    }
                }
            });
            // Set up your actual source (e.g., Kafka consumer)
            // let consumer = KafkaConsumer::new(broker, topic);
            // Start producing messages
            tokio::spawn(async move {
                loop {
                    // Fetch message from your source
                    // let msg = consumer.fetch().await?;
                    let msg = "sample message".to_string();
                    if tx.send(Ok(msg)).is_err() {
                        break; // Channel closed
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

// 4. Use in a flow
let source = KafkaSource {
    topic: "my-topic".to_string(),
    broker: "localhost:9092".to_string(),
};
let flow = Flow::new()
    .source(source)
    .process(MyProcessor::new());
```

### Best Practices for Sources
```
1. Feedback Handling
   - Always process feedback to track message status
   - Implement proper commit/acknowledgment logic
   - Handle errors appropriately

2. Channel Sizing
   - Choose appropriate buffer sizes
   - Consider backpressure mechanisms
   - Monitor channel capacity

3. Error Handling
   - Use specific error types
   - Provide context in errors
   - Consider retry strategies

4. Resource Management
   - Clean up resources when channel closes
   - Handle shutdown gracefully
   - Monitor resource usage
```

## Implementing a New Condition

Here's an example of implementing a condition that checks if a key exists in Redis:

```rust
use cortex_ai::{FlowComponent, Condition, ConditionFuture};
use redis::AsyncCommands;
use std::sync::Arc;

// 1. Define your condition struct
#[derive(Clone)]
struct RedisExistsCondition {
    client: Arc<redis::Client>,
    prefix: String,
}
impl RedisExistsCondition {
    pub fn new(redis_url: &str, prefix: &str) -> redis::RedisResult<Self> {
        Ok(Self {
            client: Arc::new(redis::Client::open(redis_url)?),
            prefix: prefix.to_string(),
        })
    }
}

// 2. Define custom error type
#[derive(Debug, thiserror::Error)]
pub enum RedisConditionError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

// 3. Implement FlowComponent to define types
impl FlowComponent for RedisExistsCondition {
    type Input = String; // Input key to check
    type Output = String; // Pass through the key
    type Error = RedisConditionError; // Custom error type
}

// 4. Implement the Condition trait
impl Condition for RedisExistsCondition {
        fn evaluate(&self, input: Self::Input) -> ConditionFuture<', Self::Output, Self::Error> {
        let client = self.client.clone();
        let key = format!("{}:{}", self.prefix, input);
        Box::pin(async move {
            // Get Redis connection
            let mut conn = client.get_async_connection().await?;
            // Check if key exists
            let exists: bool = conn.exists(&key).await?;
            // Return condition result and optional output
            Ok((exists, Some(input)))
        })
    }
}

// 5. Use in a flow
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let condition = RedisExistsCondition::new(
        "redis://localhost:6379",
        "myapp:cache"
    )?;
    
    let flow = Flow::new()
        .source(MessageSource::new())
        .when(condition)
        .process(CacheHitProcessor::new())
        .otherwise()
        .process(CacheMissProcessor::new())
        .end();

    flow.run_stream(shutdown_rx).await?;

    Ok(())
}
// 6. Test the condition
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_redis_condition() {
        let condition = RedisExistsCondition::new(
            "redis://localhost:6379",
            "test"
        ).unwrap();
        // Set up test data
        let mut conn = condition.client.get_async_connection().await.unwrap();
        conn.set("test:key1", "value1").await.unwrap();
        // Test existing key
        let result = condition.evaluate("key1".to_string()).await.unwrap();
        assert!(result.0); // Should exist
        assert_eq!(result.1, Some("key1".to_string()));
        // Test non-existing key
        let result = condition.evaluate("key2".to_string()).await.unwrap();
        assert!(!result.0); // Should not exist
        assert_eq!(result.1, Some("key2".to_string()));
    }
}
```


### Best Practices for Conditions

1. **Error Handling**:
   - Use custom error types with thiserror
   - Provide meaningful error messages
   - Handle all possible failure cases

2. **Resource Management**:
   - Share connections using Arc when possible
   - Handle connection failures gracefully
   - Clean up resources properly

3. **Testing**:
   - Test both positive and negative cases
   - Test error conditions
   - Use test fixtures or mocks for external services

4. **Performance**:
   - Cache connections when possible
   - Use connection pools for databases
   - Consider timeout settings

### Example: Complex Condition

```rust
// Combine multiple conditions
#[derive(Clone)]
struct CombinedCondition<T> {
    conditions: Vec<T>,
    require_all: bool,
}

impl<T> CombinedCondition<T>
where
    T: Condition + Clone,
    T::Input: Clone,
    T::Output: Clone,
    T::Error: From<&'static str>,
{
    fn new(conditions: Vec<T>, require_all: bool) -> Self {
        Self {
            conditions,
            require_all,
        }
    }
}
impl<T> FlowComponent for CombinedCondition<T>
where
    T: Condition + Clone,
    T::Input: Clone,
    T::Output: Clone,
    T::Error: From<&'static str>,
{
    type Input = T::Input;
    type Output = T::Output;
    type Error = T::Error;
}

impl<T> Condition for CombinedCondition<T>
where
    T: Condition + Clone,
    T::Input: Clone,
    T::Output: Clone,
    T::Error: From<&'static str>,
{
    fn evaluate(&self, input: Self::Input) -> ConditionFuture<', Self::Output, Self::Error> {
        let conditions = self.conditions.clone();
        let require_all = self.require_all;
        let input_clone = input.clone();
        Box::pin(async move {
            let mut results = Vec::new();
            for condition in conditions {
                let (result, _ ) = condition.evaluate(input_clone.clone()).await?;
                results.push(result);
            }
            let final_result = if require_all {
                results.iter().all(|&r| r)
            } else {
                results.iter().any(|&r| r)
            };
            Ok((final_result, Some(input)))
        })
    }
}
```

## Using Combined Conditions

Here's how to use multiple conditions together:

```rust
use cortex_ai::{Flow, FlowComponent, Condition, ConditionFuture};

// Individual conditions
#[derive(Clone)]
struct RedisExistsCondition {
// ... implementation from previous example ...
}

#[derive(Clone)]
struct RateLimitCondition {
    max_requests: u32,
    window_secs: u64,
}

impl FlowComponent for RateLimitCondition {
    type Input = String;
    type Output = String;
    type Error = RedisConditionError;
}

impl Condition for RateLimitCondition {
    fn evaluate(&self, input: Self::Input) -> ConditionFuture<', Self::Output, Self::Error> {
        Box::pin(async move {
            // Check rate limit logic here
            Ok((true, Some(input)))
        })
    }
}

#[derive(Clone)]
struct WhitelistCondition {
    allowed_prefixes: Vec<String>,
}

impl FlowComponent for WhitelistCondition {
    type Input = String;
    type Output = String;
    type Error = RedisConditionError;
}

impl Condition for WhitelistCondition {
    fn evaluate(&self, input: Self::Input) -> ConditionFuture<', Self::Output, Self::Error> {
        let allowed = self.allowed_prefixes.iter()
            .any(|prefix| input.starts_with(prefix));
        Box::pin(async move {
            Ok((allowed, Some(input)))
        })
    }
}

// Usage example
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create individual conditions
    let redis_condition = RedisExistsCondition::new(
        "redis://localhost:6379",
        "myapp:cache",
    )?;

    let rate_limit = RateLimitCondition {
        max_requests: 100,
        window_secs: 60,
    };

    let whitelist = WhitelistCondition {
        allowed_prefixes: vec!["user_".to_string(), "admin_".to_string()],
    };

    // Combine conditions - ALL must be true
    let strict_condition = CombinedCondition::new(
        vec![
            redis_condition,
            rate_limit,
            whitelist,
        ],
        true, // require_all = true
    );
    // Combine conditions - ANY can be true
    let relaxed_condition = CombinedCondition::new(
        vec![
            redis_condition,
            rate_limit,
            whitelist,
        ],
        false, // require_all = false
    );
    // Use in flows
    let strict_flow = Flow::new()
        .source(MessageSource::new())
        .when(strict_condition)
        .process(AllChecksPassedProcessor::new())
        .otherwise()
        .process(SomeCheckFailedProcessor::new())
        .end();

    let relaxed_flow = Flow::new()
        .source(MessageSource::new())
        .when(relaxed_condition)
        .process(AnyCheckPassedProcessor::new())
        .otherwise()
        .process(AllChecksFailedProcessor::new())
        .end();

    // Run both flows
    tokio::join!(
        strict_flow.run_stream(strict_shutdown_rx),
        relaxed_flow.run_stream(relaxed_shutdown_rx),
    );
    Ok(())
}
// Testing combined conditions
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_combined_conditions() {
        // Set up test conditions
        let redis_condition = RedisExistsCondition::new(
            "redis://localhost:6379",
            "test",
        ).unwrap();
        let rate_limit = RateLimitCondition {
            max_requests: 100,
            window_secs: 60,
        };
        let whitelist = WhitelistCondition {
            allowed_prefixes: vec!["user_".to_string()],
        };
        // Test strict condition (ALL must pass)
        let strict = CombinedCondition::new(
            vec![redis_condition.clone(), rate_limit.clone(), whitelist.clone()],
            true,
        );
        // Test relaxed condition (ANY can pass)
        let relaxed = CombinedCondition::new(
            vec![redis_condition, rate_limit, whitelist],
            false,
        );
        // Set up test data
        let mut conn = redis::Client::open("redis://localhost:6379")
            .unwrap()
            .get_async_connection()
            .await
            .unwrap();

        conn.set("test:user_123", "value").await.unwrap();
        // Test valid input
        let result = strict.evaluate("user_123".to_string()).await.unwrap();
        assert!(result.0); // All conditions should pass
        // Test invalid input
        let result = strict.evaluate("invalid_456".to_string()).await.unwrap();
        assert!(!result.0); // Should fail whitelist check
        // Test relaxed condition
        let result = relaxed.evaluate("invalid_456".to_string()).await.unwrap();
        assert!(result.0); // Should pass rate limit check even if others fail
    }
}
```


### Common Use Cases for Combined Conditions

1. **Access Control**:
   ```rust
   let access_check = CombinedCondition::new(vec![
       AuthTokenCondition::new(),
       RoleCondition::new(),
       PermissionCondition::new(),
   ], true);
   ```

2. **Data Validation**:
   ```rust
   let validation = CombinedCondition::new(vec![
       SchemaValidation::new(),
       BusinessRuleValidation::new(),
       DataIntegrityCheck::new(),
   ], true);
   ```

3. **Routing Logic**:
   ```rust
   let routing = CombinedCondition::new(vec![
       CapacityCheck::new(),
       LoadBalancingCheck::new(),
       HealthCheck::new(),
   ], false);
   ```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

### Guidelines

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by modern data processing pipelines
- Built with Rust's async ecosystem
- Performance-focused design

## Contact

- GitHub: [@yaman](https://github.com/yaman)