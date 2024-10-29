use crate::helpers::{
    EmptySource, ErrorProcessor, ErrorSource, PassthroughProcessor, SkipProcessor, SkipSource,
    StreamErrorSource, TestError, TestSource,
};
use cortex_ai::Flow;
use flume::bounded;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(test)]
mod flow_tests {
    use super::*;
    use crate::helpers::run_flow_with_timeout;

    #[tokio::test]
    async fn it_should_error_when_source_not_set() {
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
        // Given
        let flow = Flow::<String, TestError, String>::new()
            .source(TestSource {
                data: "test_input".to_string(),
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
}
