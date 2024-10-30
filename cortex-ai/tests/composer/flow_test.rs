use crate::helpers::{
    init_tracing, EmptySource, ErrorProcessor, ErrorSource, PassthroughProcessor, SkipProcessor,
    SkipSource, StreamErrorSource, TestError, TestSource,
};
use cortex_ai::Flow;
use flume::bounded;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;

#[cfg(test)]
mod flow_tests {
    use cortex_ai::{flow::types::SourceOutput, FlowComponent, FlowFuture, Source};
    use flume::Receiver;

    use super::*;
    use crate::helpers::{run_flow_with_timeout, TestCondition};

    // Move struct definitions to the top
    struct MultiSource {
        rx: Receiver<Result<String, TestError>>,
        feedback: flume::Sender<Result<String, TestError>>,
    }

    impl FlowComponent for MultiSource {
        type Input = ();
        type Output = String;
        type Error = TestError;
    }

    impl Source for MultiSource {
        fn stream(&self) -> FlowFuture<'_, SourceOutput<Self::Output, Self::Error>, Self::Error> {
            let rx = self.rx.clone();
            let feedback = self.feedback.clone();
            Box::pin(async move {
                Ok(SourceOutput {
                    receiver: rx,
                    feedback,
                })
            })
        }
    }

    #[tokio::test]
    async fn it_should_error_when_source_not_set() {
        init_tracing();
        info!("Starting source not set test");
        // Given
        let flow = Flow::<String, TestError, String>::new();
        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);

        // When
        let result = flow.run_stream(shutdown_rx).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Test error: Flow error: Flow source not set"
        );
    }

    #[tokio::test]
    async fn it_should_handle_source_stream_error() {
        init_tracing();
        info!("Starting source stream error test");
        // Given
        let flow = Flow::<String, TestError, String>::new().source(StreamErrorSource);
        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);

        // When
        let result = flow.run_stream(shutdown_rx).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Test error: Stream initialization error"
        );
    }

    #[tokio::test]
    async fn it_should_handle_processor_error() {
        init_tracing();
        info!("Starting processor error test");
        // Given
        let (feedback_tx, _) = bounded::<Result<String, TestError>>(1);
        let flow = Flow::<String, TestError, String>::new()
            .source(TestSource {
                data: "test_input".to_string(),
                feedback: feedback_tx,
            })
            .process(ErrorProcessor);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Test error: Processing failed"
        );
    }

    #[tokio::test]
    async fn it_should_handle_empty_source() {
        init_tracing();
        info!("Starting empty source test");
        // Given
        let flow = Flow::<String, TestError, String>::new().source(EmptySource);
        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);

        // When
        let result = flow.run_stream(shutdown_rx).await;

        // Then
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn it_should_create_flow_using_default() {
        init_tracing();
        info!("Starting flow using default test");
        // Given
        let flow = Flow::<String, TestError, String>::default();

        // When
        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);
        let result = flow.run_stream(shutdown_rx).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Test error: Flow error: Flow source not set"
        );
    }

    #[tokio::test]
    async fn it_should_send_error_feedback_for_source_errors() {
        init_tracing();
        info!("Starting send error feedback for source errors test");
        // Given
        let (feedback_tx, feedback_rx) = bounded::<Result<String, TestError>>(1);
        let feedback_results = Arc::new(Mutex::new(Vec::new()));
        let feedback_results_clone = feedback_results.clone();

        // Spawn a task to collect feedback
        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
            }
        });

        let flow: Flow<String, TestError, String> = Flow::new()
            .source(ErrorSource {
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Test error: Source error");

        // Wait a bit for feedback processing
        tokio::time::sleep(Duration::from_millis(50)).await;

        let feedback_results = feedback_results.lock().unwrap();
        assert_eq!(feedback_results.len(), 1);
        assert!(matches!(
            &feedback_results[0],
            Err(e) if e.to_string() == "Test error: Source error"
        ));
        drop(feedback_results);
    }

    #[tokio::test]
    async fn it_should_handle_skipped_items() {
        init_tracing();
        info!("Starting handle skipped items test");
        // Given
        let (feedback_tx, feedback_rx) = bounded::<Result<String, TestError>>(1);
        let feedback_results = Arc::new(Mutex::new(Vec::new()));
        let feedback_results_clone = feedback_results.clone();

        // Spawn a task to collect feedback
        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
            }
        });

        let flow: Flow<String, TestError, String> = Flow::new()
            .source(SkipSource {
                feedback: feedback_tx,
            })
            .process(SkipProcessor);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "skipped");

        // Wait a bit for feedback processing
        tokio::time::sleep(Duration::from_millis(50)).await;

        let feedback_results = feedback_results.lock().unwrap();
        assert_eq!(feedback_results.len(), 1);
        assert!(matches!(
            &feedback_results[0],
            Ok(msg) if msg == "skipped"
        ));
        drop(feedback_results);
    }

    #[tokio::test]
    async fn it_should_set_source() {
        init_tracing();
        info!("Starting set source test");
        // Given
        let (feedback_tx, _) = bounded::<Result<String, TestError>>(1);
        let flow = Flow::<String, TestError, String>::new();

        // When
        let flow = flow.source(TestSource {
            data: "test".to_string(),
            feedback: feedback_tx,
        });

        // Then
        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);
        let result = flow.run_stream(shutdown_rx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn it_should_handle_source_item_error() {
        init_tracing();
        info!("Starting handle source item error test");
        // Given
        let (feedback_tx, feedback_rx) = bounded::<Result<String, TestError>>(1);
        let feedback_results = Arc::new(Mutex::new(Vec::new()));
        let feedback_results_clone = feedback_results.clone();

        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
            }
        });

        let flow: Flow<String, TestError, String> = Flow::new()
            .source(ErrorSource {
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_err());
        let feedback_results = feedback_results.lock().unwrap();
        assert_eq!(feedback_results.len(), 1);
        assert!(matches!(
            &feedback_results[0],
            Err(e) if e.to_string() == "Test error: Source error"
        ));
        drop(feedback_results);
    }

    #[tokio::test]
    async fn it_should_return_empty_vec_when_no_items_processed() {
        init_tracing();
        info!("Starting return empty vec when no items processed test");
        // Given
        let flow: Flow<String, TestError, String> = Flow::new().source(EmptySource);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn it_should_add_processor_to_stages() {
        init_tracing();
        info!("Starting add processor to stages test");
        // Given
        let (feedback_tx, _) = bounded::<Result<String, TestError>>(1);
        let flow = Flow::<String, TestError, String>::new()
            .source(TestSource {
                data: "test".to_string(),
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "test"); // PassthroughProcessor doesn't modify the input
    }

    #[tokio::test]
    async fn it_should_preserve_source_after_setting() {
        init_tracing();
        info!("Starting preserve source after setting test");
        // Given
        let (feedback_tx, _) = bounded::<Result<String, TestError>>(1);
        let source_data = "test".to_string();
        let flow = Flow::<String, TestError, String>::new().source(TestSource {
            data: source_data.clone(),
            feedback: feedback_tx,
        });

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], source_data); // Verify source data is preserved
    }

    #[tokio::test]
    async fn it_should_handle_source_item_with_feedback() {
        init_tracing();
        info!("Starting handle source item with feedback test");
        // Given
        let (feedback_tx, feedback_rx) = bounded::<Result<String, TestError>>(1);
        let feedback_results = Arc::new(Mutex::new(Vec::new()));
        let feedback_results_clone = feedback_results.clone();

        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
            }
        });

        let test_data = "test_data".to_string();
        let flow: Flow<String, TestError, String> = Flow::new()
            .source(TestSource {
                data: test_data.clone(),
                feedback: feedback_tx, // Pass feedback channel to TestSource
            })
            .process(PassthroughProcessor);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], test_data); // Verify data is processed and not skipped

        // Wait a bit for feedback processing
        tokio::time::sleep(Duration::from_millis(50)).await;

        let feedback_results = feedback_results.lock().unwrap();
        assert_eq!(feedback_results.len(), 1);
        assert!(matches!(
            &feedback_results[0],
            Ok(data) if data == &test_data
        ));
        drop(feedback_results);
    }

    #[tokio::test]
    async fn it_should_process_multiple_items() {
        init_tracing();
        info!("Starting process multiple items test");
        // Given
        let (tx, rx) = bounded(2);
        let (feedback_tx, _) = bounded(2);

        tx.send(Ok("item1".to_string())).unwrap();
        tx.send(Ok("item2".to_string())).unwrap();
        drop(tx);

        let flow: Flow<String, TestError, String> = Flow::new()
            .source(MultiSource {
                rx,
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 2); // Verify multiple items are processed
        assert_eq!(results, vec!["item1".to_string(), "item2".to_string()]);
    }

    #[tokio::test]
    async fn it_should_show_tracing_metrics() {
        // Set up tracing with a more detailed configuration
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter("cortex_ai=info,test=info")
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL) // Show enter/exit of spans
            .try_init();

        if subscriber.is_err() {
            println!("Warning: tracing already initialized");
        }

        // Run a flow with all components to generate tracing data
        let (feedback_tx, _) = bounded::<Result<String, TestError>>(1);
        let test_condition = TestCondition; // Create instance first
        let flow = Flow::new()
            .source(TestSource {
                data: "test_data".to_string(),
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .when(test_condition) // Use the instance
            .process(PassthroughProcessor)
            .otherwise()
            .process(PassthroughProcessor)
            .end();

        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;
        assert!(result.is_ok());
    }
}
