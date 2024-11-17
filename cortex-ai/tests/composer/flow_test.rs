use crate::helpers::{
    init_tracing, EmptySource, ErrorProcessor, ErrorSource, PassthroughProcessor, SkipProcessor,
    SkipSource, StreamErrorSource, TestSource,
};
use cortex_ai::Flow;
use flume::bounded;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;

#[cfg(test)]
mod flow_tests {
    use cortex_ai::{
        flow::types::SourceOutput, FlowComponent, FlowError, FlowFuture, Processor, Sink, Source,
    };
    use flume::Receiver;

    use super::*;
    use crate::helpers::{run_flow_with_timeout, TestCondition, TestSink};

    // Move struct definitions to the top
    struct MultiSource {
        rx: Receiver<Result<String, FlowError>>,
        feedback: flume::Sender<Result<String, FlowError>>,
    }

    impl FlowComponent for MultiSource {
        type Input = ();
        type Output = String;
        type Error = FlowError;
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

        fn on_feedback(&self, result: Result<Self::Output, Self::Error>) {
            info!("Feedback received: {:?}", result);
        }
    }

    #[tokio::test]
    async fn it_should_error_when_source_not_set() {
        init_tracing();
        info!("Starting source not set test");
        // Given
        let flow = Flow::<String, FlowError, String>::new();

        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);

        // When
        let result = flow.run_stream(shutdown_rx).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Flow error: No source configured"
        );
    }

    #[tokio::test]
    async fn it_should_handle_source_stream_error() {
        init_tracing();
        info!("Starting source stream error test");
        // Given
        let flow = Flow::<String, FlowError, String>::new()
            .source(StreamErrorSource)
            .process(PassthroughProcessor)
            .sink(TestSink);

        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);

        // When
        let result = flow.run_stream(shutdown_rx).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Source error: Stream initialization error"
        );
    }

    #[tokio::test]
    async fn it_should_handle_processor_error() {
        init_tracing();
        info!("Starting processor error test");
        // Given
        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
        let flow = Flow::<String, FlowError, String>::new()
            .source(TestSource {
                data: "test_input".to_string(),
                feedback: feedback_tx,
            })
            .process(ErrorProcessor)
            .sink(TestSink);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Process error: Processing failed"
        );
    }

    #[tokio::test]
    async fn it_should_handle_empty_source() {
        init_tracing();
        info!("Starting empty source test");
        // Given
        let flow = Flow::<String, FlowError, String>::new()
            .source(EmptySource)
            .process(PassthroughProcessor)
            .sink(TestSink);

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
        let flow = Flow::<String, FlowError, String>::default();

        // When
        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);
        let result = flow.run_stream(shutdown_rx).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Flow error: No source configured"
        );
    }

    #[tokio::test]
    async fn it_should_send_error_feedback_for_source_errors() {
        init_tracing();
        info!("Starting send error feedback for source errors test");
        // Given
        let (feedback_tx, feedback_rx) = bounded::<Result<String, FlowError>>(1);
        let feedback_results = Arc::new(Mutex::new(Vec::<Result<String, FlowError>>::new()));
        let feedback_results_clone = feedback_results.clone();

        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
            }
        });

        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(ErrorSource {
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Source error: Source error"
        );

        let feedback_results = feedback_results.lock().unwrap();
        assert_eq!(feedback_results.len(), 1);
        assert!(matches!(
            &feedback_results[0],
            Err(e) if e.to_string() == "Source error: Source error"
        ));
    }

    #[tokio::test]
    async fn it_should_handle_skipped_items() {
        init_tracing();
        info!("Starting handle skipped items test");

        let (feedback_tx, feedback_rx) = bounded::<Result<String, FlowError>>(1);
        let feedback_results = Arc::new(Mutex::new(Vec::<Result<String, FlowError>>::new()));
        let feedback_results_clone = feedback_results.clone();

        // Create a oneshot channel to signal when feedback is received
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let done_tx = Arc::new(Mutex::new(Some(done_tx)));

        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
                if results.len() == 1 {
                    // Signal completion and break
                    if let Some(tx) = done_tx.lock().unwrap().take() {
                        tx.send(()).ok();
                    }
                    break;
                }
            }
        });

        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(SkipSource {
                feedback: feedback_tx,
            })
            .process(SkipProcessor)
            .sink(TestSink);

        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Wait for feedback with timeout
        tokio::select! {
            _ = done_rx => {},
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                panic!("Feedback timeout");
            }
        }

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "skipped");

        let feedback_results = feedback_results.lock().unwrap();
        assert_eq!(feedback_results.len(), 1);
        assert!(matches!(
            &feedback_results[0],
            Ok(msg) if msg == "skipped"
        ));
    }

    #[tokio::test]
    async fn it_should_set_source() {
        init_tracing();
        info!("Starting set source test");
        // Given
        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
        let flow = Flow::<String, FlowError, String>::new()
            .source(TestSource {
                data: "test".to_string(),
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

        // When
        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);
        let result = flow.run_stream(shutdown_rx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn it_should_handle_source_item_error() {
        init_tracing();
        info!("Starting handle source item error test");
        // Given
        let (feedback_tx, feedback_rx) = bounded::<Result<String, FlowError>>(1);
        let feedback_results = Arc::new(Mutex::new(Vec::<Result<String, FlowError>>::new()));
        let feedback_results_clone = feedback_results.clone();

        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
            }
        });

        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(ErrorSource {
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Source error: Source error"
        );

        let feedback_results = feedback_results.lock().unwrap();
        assert_eq!(feedback_results.len(), 1);
        assert!(matches!(
            &feedback_results[0],
            Err(e) if e.to_string() == "Source error: Source error"
        ));
    }

    #[tokio::test]
    async fn it_should_return_empty_vec_when_no_items_processed() {
        init_tracing();
        info!("Starting return empty vec when no items processed test");
        // Given
        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(EmptySource)
            .process(PassthroughProcessor)
            .sink(TestSink);

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
        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
        let flow = Flow::<String, FlowError, String>::new()
            .source(TestSource {
                data: "test".to_string(),
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

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
        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
        let source_data = "test".to_string();
        let flow = Flow::<String, FlowError, String>::new()
            .source(TestSource {
                data: source_data.clone(),
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

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
        let (feedback_tx, feedback_rx) = bounded::<Result<String, FlowError>>(1);
        let feedback_results = Arc::new(Mutex::new(Vec::<Result<String, FlowError>>::new()));
        let feedback_results_clone = feedback_results.clone();

        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
            }
        });

        let test_data = "test_data".to_string();
        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(TestSource {
                data: test_data.clone(),
                feedback: feedback_tx.clone(), // Clone feedback sender
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], test_data);

        // Wait longer for feedback processing
        tokio::time::sleep(Duration::from_millis(100)).await; // Increased wait time

        let feedback_results = feedback_results.lock().unwrap();
        assert_eq!(feedback_results.len(), 1);
        assert!(matches!(
            &feedback_results[0],
            Ok(data) if data == &test_data
        ));
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

        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(MultiSource {
                rx,
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

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
        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
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
            .end()
            .sink(TestSink);

        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[should_panic(expected = "Flow must have at least one processor or condition before sink")]
    async fn it_should_require_processor_before_sink() {
        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
        let _: Flow<String, FlowError, String> = Flow::new()
            .source(TestSource {
                data: "test".to_string(),
                feedback: feedback_tx,
            })
            .sink(TestSink);
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot add processor after sink")]
    async fn it_should_not_allow_processor_after_sink() {
        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
        let _: Flow<String, FlowError, String> = Flow::new()
            .source(TestSource {
                data: "test".to_string(),
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink)
            .process(PassthroughProcessor);
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot add condition after sink")]
    async fn it_should_not_allow_condition_after_sink() {
        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
        let _: Flow<String, FlowError, String> = Flow::new()
            .source(TestSource {
                data: "test".to_string(),
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink)
            .when(TestCondition)
            .process(PassthroughProcessor)
            .otherwise()
            .end();
    }

    #[tokio::test]
    async fn it_should_error_when_sink_not_set() {
        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(TestSource {
                data: "test".to_string(),
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor);

        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);
        let result = flow.run_stream(shutdown_rx).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Flow error: No sink configured"
        );
    }

    #[tokio::test]
    async fn it_should_handle_sink_error() {
        init_tracing();
        info!("Starting sink error test");

        struct ErrorSink;

        impl FlowComponent for ErrorSink {
            type Input = String;
            type Output = String;
            type Error = FlowError;
        }

        impl Sink for ErrorSink {
            fn sink(&self, _input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error> {
                Box::pin(async move { Err(FlowError::Sink("Sink error".to_string())) })
            }
        }

        let (feedback_tx, feedback_rx) = bounded::<Result<String, FlowError>>(1);
        let feedback_results = Arc::new(Mutex::new(Vec::new()));
        let feedback_results_clone = feedback_results.clone();

        // Create a oneshot channel to signal when feedback is received
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let done_tx = Arc::new(Mutex::new(Some(done_tx)));

        // Spawn feedback handler
        tokio::spawn(async move {
            if let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
                if let Some(tx) = done_tx.lock().unwrap().take() {
                    tx.send(()).ok();
                }
            }
        });

        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(TestSource {
                data: "test".to_string(),
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(ErrorSink);

        // Wait for feedback first
        let result = tokio::select! {
            r = run_flow_with_timeout(flow, Duration::from_millis(100)) => r,
            _ = done_rx => {
                // Feedback received, now check the result
                let feedback_results = feedback_results.lock().unwrap();
                assert_eq!(feedback_results.len(), 1);
                assert!(matches!(
                    &feedback_results[0],
                    Err(e) if e.to_string().contains("Sink error")
                ));
                Err(FlowError::Sink("Sink error".to_string()))
            }
        };

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Sink error"));
    }

    #[tokio::test]
    async fn it_should_handle_concurrent_processing() {
        init_tracing();
        info!("Starting concurrent processing test");

        let (tx, rx) = bounded(10);
        let (feedback_tx, feedback_rx) = bounded(10);
        let feedback_results = Arc::new(Mutex::new(Vec::new()));
        let feedback_results_clone = feedback_results.clone();

        tokio::spawn(async move {
            while let Ok(result) = feedback_rx.recv_async().await {
                let mut results = feedback_results_clone.lock().unwrap();
                results.push(result);
            }
        });

        // Send multiple items concurrently
        for i in 0..5 {
            tx.send(Ok(format!("item{}", i))).unwrap();
        }
        drop(tx);

        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(MultiSource {
                rx,
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 5);

        let feedback_results = feedback_results.lock().unwrap();
        assert_eq!(feedback_results.len(), 5);
    }

    #[tokio::test]
    async fn it_should_handle_shutdown_during_processing() {
        init_tracing();
        info!("Starting shutdown during processing test");

        let (tx, rx) = bounded(100);
        let (feedback_tx, _) = bounded(100);

        // Send many items to ensure processing is ongoing
        for i in 0..50 {
            tx.send(Ok(format!("item{}", i))).unwrap();
        }
        drop(tx);

        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(MultiSource {
                rx,
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

        let handle = tokio::spawn(async move { flow.run_stream(shutdown_rx).await });

        // Send shutdown signal immediately
        shutdown_tx.send(()).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
        // Some items might be processed before shutdown
        assert!(result.unwrap().len() < 50);
    }

    #[tokio::test]
    async fn it_should_handle_slow_processor() {
        init_tracing();
        info!("Starting slow processor test");

        struct SlowProcessor;

        impl FlowComponent for SlowProcessor {
            type Input = String;
            type Output = String;
            type Error = FlowError;
        }

        impl Processor for SlowProcessor {
            fn process(&self, input: Self::Input) -> FlowFuture<'_, Self::Output, Self::Error> {
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok(input)
                })
            }
        }

        let (feedback_tx, _) = bounded::<Result<String, FlowError>>(1);
        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(TestSource {
                data: "test".to_string(),
                feedback: feedback_tx,
            })
            .process(SlowProcessor)
            .sink(TestSink);

        let result = run_flow_with_timeout(flow, Duration::from_millis(200)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn it_should_handle_channel_closure() {
        init_tracing();
        info!("Starting channel closure test");

        let (tx, rx) = bounded(1);
        let (feedback_tx, _) = bounded(1);

        // Drop sender without sending anything
        drop(tx);

        let flow: Flow<String, FlowError, String> = Flow::new()
            .source(MultiSource {
                rx,
                feedback: feedback_tx,
            })
            .process(PassthroughProcessor)
            .sink(TestSink);

        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
