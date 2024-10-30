use cortex_ai::{
    composer::flow::FlowError, flow::types::SourceOutput, Condition, ConditionFuture, Flow,
    FlowComponent, FlowFuture, Processor, Source,
};
use criterion::{criterion_group, criterion_main, Criterion};
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct BenchError(String);

impl fmt::Display for BenchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bench error: {}", self.0)
    }
}

impl Error for BenchError {}

impl From<FlowError> for BenchError {
    fn from(error: FlowError) -> Self {
        Self(error.to_string())
    }
}

#[derive(Clone)]
struct BenchProcessor;

impl FlowComponent for BenchProcessor {
    type Input = String;
    type Output = String;
    type Error = BenchError;
}

impl Processor for BenchProcessor {
    fn process(&self, input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error> {
        Box::pin(async move { Ok(format!("processed_{input}")) })
    }
}

#[derive(Clone)]
struct BenchCondition;

impl FlowComponent for BenchCondition {
    type Input = String;
    type Output = String;
    type Error = BenchError;
}

impl Condition for BenchCondition {
    fn evaluate(&self, input: Self::Input) -> ConditionFuture<'_, Self::Output, Self::Error> {
        Box::pin(async move {
            let condition_met = input.contains('5');
            Ok((condition_met, Some(input)))
        })
    }
}

#[derive(Clone, Debug)]
struct QueueSource {
    producer: flume::Sender<Result<String, BenchError>>,
    consumer: flume::Receiver<Result<String, BenchError>>,
    processed_count: Arc<AtomicUsize>,
}

impl QueueSource {
    fn new() -> Self {
        let (producer, consumer) = flume::bounded(1000);
        Self {
            producer,
            consumer,
            processed_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl FlowComponent for QueueSource {
    type Input = ();
    type Output = String;
    type Error = BenchError;
}

impl Source for QueueSource {
    fn stream(&self) -> FlowFuture<'_, SourceOutput<Self::Output, Self::Error>, Self::Error> {
        let consumer = self.consumer.clone();
        let processed_count = self.processed_count.clone();

        Box::pin(async move {
            let (feedback_tx, feedback_rx) = flume::bounded::<Result<String, BenchError>>(1000);

            let feedback_count = processed_count.clone();
            tokio::spawn(async move {
                while let Ok(result) = feedback_rx.recv_async().await {
                    if result.is_ok() {
                        feedback_count.fetch_add(1, Ordering::SeqCst);
                    }
                }
            });

            Ok(SourceOutput {
                receiver: consumer,
                feedback: feedback_tx,
            })
        })
    }
}

#[derive(Debug, Clone)]
enum BenchmarkConfig {
    Simple,
    Branching,
    Complex,
    Concurrent,
}

fn flow_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("flow_throughput");
    group.sampling_mode(criterion::SamplingMode::Flat);
    let measurement_time = Duration::from_secs(10);
    group.measurement_time(measurement_time);
    group.warm_up_time(Duration::from_secs(1));

    // Fixed number of messages per iteration
    let messages: usize = 10_000;
    group.throughput(criterion::Throughput::Bytes(
        u64::try_from(messages).unwrap(),
    ));

    for config_type in &[
        BenchmarkConfig::Simple,
        BenchmarkConfig::Branching,
        BenchmarkConfig::Complex,
        BenchmarkConfig::Concurrent,
    ] {
        let config_name = format!("{:?}_throughput", config_type);
        let config_type = config_type.clone();

        group.bench_function(config_name, |b| {
            let source = QueueSource::new();
            let producer = source.producer.clone();
            let processed_count = source.processed_count.clone();

            let flow = match config_type {
                BenchmarkConfig::Simple => Flow::new().source(source).process(BenchProcessor),
                BenchmarkConfig::Branching => Flow::new()
                    .source(source)
                    .when(BenchCondition)
                    .process(BenchProcessor)
                    .otherwise()
                    .process(BenchProcessor)
                    .end(),
                BenchmarkConfig::Complex => {
                    let flow = Flow::new().source(source).process(BenchProcessor);

                    let flow = flow
                        .when(BenchCondition)
                        .process(BenchProcessor)
                        .otherwise()
                        .process(BenchProcessor)
                        .end();

                    flow.when(BenchCondition)
                        .process(BenchProcessor)
                        .otherwise()
                        .process(BenchProcessor)
                        .end()
                }
                BenchmarkConfig::Concurrent => Flow::new()
                    .source(source)
                    .process(BenchProcessor)
                    .process(BenchProcessor)
                    .process(BenchProcessor),
            };

            let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
            let flow_handle = rt.spawn(async move { flow.run_stream(shutdown_rx).await });

            b.to_async(&rt).iter_custom(|_iters| {
                let producer = producer.clone();
                let processed_count = processed_count.clone();

                async move {
                    processed_count.store(0, Ordering::SeqCst);
                    let start = Instant::now();

                    for i in 0..messages {
                        if producer.send(Ok(format!("message_{i}"))).is_err() {
                            break;
                        }
                    }

                    while processed_count.load(Ordering::SeqCst) < messages {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }

                    start.elapsed()
                }
            });

            let _ = shutdown_tx.send(());
            let _ = rt.block_on(flow_handle);

            // First convert to i32 (which has From implementation), then to f64
            let messages_per_sec =
                i32::try_from(messages / measurement_time.as_secs() as usize).unwrap_or(i32::MAX);
            let throughput = f64::from(messages_per_sec);
            println!(
                "Config: {config_type:?}, Messages: {messages}, Throughput: {throughput:.2} messages/sec",
            );
        });
    }

    group.finish();
}

criterion_group!(benches, flow_benchmark);
criterion_main!(benches);
