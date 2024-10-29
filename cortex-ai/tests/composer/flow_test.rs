use crate::helpers::{run_flow_with_timeout, TestError, TestSource};
use cortex_ai::{Flow, FlowComponent, FlowFuture, Processor, Source};
use flume::bounded;
use std::time::Duration;

#[cfg(test)]
mod flow_tests {
    use super::*;

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
        struct ErrorSource;

        impl FlowComponent for ErrorSource {
            type Input = ();
            type Output = String;
            type Error = TestError;
        }

        impl Source for ErrorSource {
            fn stream<'a>(
                &'a self,
            ) -> FlowFuture<'a, flume::Receiver<Result<Self::Output, Self::Error>>, Self::Error>
            {
                Box::pin(async move { Err(TestError("Source stream error".to_string())) })
            }
        }

        let flow = Flow::<String, TestError, String>::new().source(ErrorSource);
        let (_, shutdown_rx) = tokio::sync::broadcast::channel(1);

        // When
        let result = flow.run_stream(shutdown_rx).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Test error: Source stream error"
        );
    }

    #[tokio::test]
    async fn it_should_handle_processor_error() {
        // Given
        struct ErrorProcessor;

        impl FlowComponent for ErrorProcessor {
            type Input = String;
            type Output = String;
            type Error = TestError;
        }

        impl Processor for ErrorProcessor {
            fn process<'a>(
                &'a self,
                _input: Self::Input,
            ) -> FlowFuture<'a, Self::Output, Self::Error> {
                Box::pin(async move { Err(TestError("Process error".to_string())) })
            }
        }

        let flow = Flow::<String, TestError, String>::new()
            .source(TestSource {
                data: "test_input".to_string(),
            })
            .process(ErrorProcessor);

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Test error: Process error");
    }

    #[tokio::test]
    async fn it_should_handle_empty_source() {
        // Given
        struct EmptySource;

        impl FlowComponent for EmptySource {
            type Input = ();
            type Output = String;
            type Error = TestError;
        }

        impl Source for EmptySource {
            fn stream<'a>(
                &'a self,
            ) -> FlowFuture<'a, flume::Receiver<Result<Self::Output, Self::Error>>, Self::Error>
            {
                Box::pin(async move {
                    let (tx, rx) = bounded(1);
                    drop(tx); // Close channel immediately
                    Ok(rx)
                })
            }
        }

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
}
