use crate::helpers::{run_flow_with_timeout, TestCondition, TestError, TestProcessor, TestSource};
use cortex_ai::{Condition, ConditionFuture, Flow, FlowComponent, FlowFuture};
use std::time::Duration;

#[cfg(test)]
mod branch_builder_tests {
    use cortex_ai::Source;
    use flume::bounded;

    use super::*;

    #[tokio::test]
    async fn it_should_execute_then_branch_when_condition_is_met() {
        // Given
        let flow = Flow::new()
            .source(TestSource {
                data: "test_input".to_string(),
            })
            .when(TestCondition)
            .process(TestProcessor)
            .otherwise()
            .process(TestProcessor)
            .end();

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100))
            .await
            .unwrap();

        // Then
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "processed_test_input");
    }

    #[tokio::test]
    async fn it_should_execute_else_branch_when_condition_is_not_met() {
        // Given
        let flow = Flow::new()
            .source(TestSource {
                data: "no_match".to_string(),
            })
            .when(TestCondition)
            .process(TestProcessor)
            .otherwise()
            .process(TestProcessor)
            .end();

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100))
            .await
            .unwrap();

        // Then
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "processed_no_match");
    }

    #[tokio::test]
    async fn it_should_handle_condition_evaluation_error() {
        // Given
        struct ErrorCondition;

        impl FlowComponent for ErrorCondition {
            type Input = String;
            type Output = String;
            type Error = TestError;
        }

        impl Condition for ErrorCondition {
            fn evaluate<'a>(
                &'a self,
                _input: Self::Input,
            ) -> ConditionFuture<'a, Self::Output, Self::Error> {
                Box::pin(async move { Err(TestError("Condition error".to_string())) })
            }
        }

        let flow = Flow::new()
            .source(TestSource {
                data: "test_input".to_string(),
            })
            .when(ErrorCondition)
            .process(TestProcessor)
            .otherwise()
            .process(TestProcessor)
            .end();

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100)).await;

        // Then
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Test error: Condition error"
        );
    }

    #[tokio::test]
    async fn it_should_execute_complete_flow() {
        // Given
        let flow = Flow::new()
            .source(TestSource {
                data: "test_input".to_string(),
            })
            .when(TestCondition)
            .process(TestProcessor)
            .otherwise()
            .process(TestProcessor)
            .end();

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100))
            .await
            .unwrap();

        // Then
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn it_should_process_multiple_items_in_branch() {
        // Given
        struct MultiItemSource;

        impl FlowComponent for MultiItemSource {
            type Input = ();
            type Output = String;
            type Error = TestError;
        }

        impl Source for MultiItemSource {
            fn stream<'a>(
                &'a self,
            ) -> FlowFuture<'a, flume::Receiver<Result<Self::Output, Self::Error>>, Self::Error>
            {
                Box::pin(async move {
                    let (tx, rx) = bounded(2);
                    tx.send(Ok("test_1".to_string())).unwrap();
                    tx.send(Ok("no_match".to_string())).unwrap();
                    drop(tx);
                    Ok(rx)
                })
            }
        }

        let flow = Flow::new()
            .source(MultiItemSource)
            .when(TestCondition)
            .process(TestProcessor)
            .otherwise()
            .process(TestProcessor)
            .end();

        // When
        let result = run_flow_with_timeout(flow, Duration::from_millis(100))
            .await
            .unwrap();

        // Then
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "processed_test_1");
        assert_eq!(result[1], "processed_no_match");
    }
}
