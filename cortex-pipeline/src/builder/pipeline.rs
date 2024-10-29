use std::error::Error;
use crate::pipeline::{
    stage::Stage,
    source::Source,
    processor::Processor,
    condition::Condition,
};
use super::BranchBuilder;

pub struct Pipeline<T, E> {
    pub(crate) source: Option<Box<dyn Source<Input = (), Output = T, Error = E>>>,
    pub(crate) stages: Vec<Stage<T, E>>,
}

impl<T, E> Pipeline<T, E> 
where
    T: Clone + Send + 'static,
    E: Error + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            source: None,
            stages: Vec::new(),
        }
    }

    pub fn source<S>(mut self, source: S) -> Self 
    where
        S: Source<Input = (), Output = T, Error = E> + 'static
    {
        self.source = Some(Box::new(source));
        self
    }

    pub fn process<P>(mut self, processor: P) -> Self
    where
        P: Processor<Input = T, Output = T, Error = E> + 'static
    {
        self.stages.push(Stage::Process(Box::new(processor)));
        self
    }

    pub fn when<C>(self, condition: C) -> BranchBuilder<T, E>
    where
        C: Condition<Input = T, Output = bool, Error = E> + 'static
    {
        BranchBuilder::new(Box::new(condition), self)
    }

    pub async fn run_stream(self) -> Result<(), Box<dyn Error>> {
        let source = self.source.ok_or("Pipeline source not set")?;
        let receiver = source.stream().await?;

        while let Ok(item) = receiver.recv_async().await {
            let mut current_item = item?;
            
            for stage in &self.stages {
                match stage {
                    Stage::Process(processor) => {
                        current_item = processor.process(current_item).await?;
                    }
                    Stage::Branch(branch) => {
                        let (condition_met, _) = branch.condition.evaluate(current_item.clone()).await?;
                        let stages = if condition_met {
                            &branch.then_branch
                        } else {
                            &branch.else_branch
                        };

                        for stage in stages {
                            if let Stage::Process(processor) = stage {
                                current_item = processor.process(current_item).await?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
} 