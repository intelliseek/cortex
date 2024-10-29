use crate::helpers::{run_flow_with_timeout, TestCondition, TestError, TestProcessor, TestSource};
use cortex_ai::{Condition, ConditionFuture, Flow, FlowComponent};
use std::time::Duration;

#[cfg(test)]
mod branch_builder_tests {
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
            fn evaluate(
                &self,
                _input: Self::Input,
            ) -> ConditionFuture<'_, Self::Output, Self::Error> {
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
}
