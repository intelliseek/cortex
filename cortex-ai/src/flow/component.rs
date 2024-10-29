use std::error::Error;

pub trait FlowComponent {
    type Input;
    type Output;
    type Error: Error + Send + Sync + 'static;
}
