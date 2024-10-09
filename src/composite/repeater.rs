use crate::{BehaviorArc, BehaviorNode, NodeResult};
use std::fmt::Debug;
use std::sync::Arc;

/// Repeats its child infintely
pub struct Repeated<B> {
    resume: Option<BehaviorArc<B>>,
    child: BehaviorArc<B>,
}

impl<B> Debug for Repeated<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Repeated")
            .field("child", &self.child)
            .finish()
    }
}

impl<B> Repeated<B> {
    pub fn new(child: BehaviorArc<B>) -> Self {
        Self {
            child,
            resume: None,
        }
    }
}

impl<B: 'static> BehaviorNode<B> for Repeated<B> {
    fn tick(self: Arc<Self>, blackboard: &mut B) -> NodeResult<B> {
        if let Some(resume) = self.resume.as_ref() {
            if let NodeResult::Running(resume) = resume.clone().tick(blackboard) {
                return NodeResult::Running(
                    Self {
                        resume: Some(resume),
                        child: self.child.clone(),
                    }
                    .arc(),
                );
            }
        }
        if let NodeResult::Running(resume) = self.child.clone().tick(blackboard) {
            return NodeResult::Running(
                Self {
                    resume: Some(resume),
                    child: self.child.clone(),
                }
                .arc(),
            );
        }

        // Restart, cuz we never end
        NodeResult::Running(Arc::new(Self {
            child: self.child.clone(),
            resume: None,
        }))
    }
}

/// Repeats its child a set number of times
pub struct LimitedRepeated<B> {
    child: BehaviorArc<B>,
    limit: usize,
    completed: usize,
    resume: Option<BehaviorArc<B>>,
}

impl<B> LimitedRepeated<B> {
    pub fn new(limit: usize, child: BehaviorArc<B>) -> Self {
        Self {
            child,
            limit,
            completed: 0,
            resume: None,
        }
    }
}

impl<B> Debug for LimitedRepeated<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LimitedRepeated")
            .field("child", &self.child)
            .field("limit", &self.limit)
            .field("completed", &self.completed)
            .finish()
    }
}

impl<B: 'static> BehaviorNode<B> for LimitedRepeated<B> {
    fn tick(self: Arc<Self>, blackboard: &mut B) -> NodeResult<B> {
        let mut completed = self.completed;

        if completed >= self.limit {
            return NodeResult::Success;
        }

        if let Some(resume) = self.resume.as_ref() {
            match resume.clone().tick(blackboard) {
                NodeResult::Running(resume) => {
                    return NodeResult::Running(
                        Self {
                            resume: Some(resume),
                            child: self.child.clone(),
                            limit: self.limit,
                            completed: self.completed,
                        }
                        .arc(),
                    )
                }
                _ => {
                    completed += 1;
                }
            }
        }
        match self.child.clone().tick(blackboard) {
            NodeResult::Running(resume) => {
                return NodeResult::Running(
                    Self {
                        resume: Some(resume),
                        child: self.child.clone(),
                        limit: self.limit,
                        completed,
                    }
                    .arc(),
                )
            }
            _ => {
                completed += 1;
            }
        }

        // Restart until we've completed the repetitions
        NodeResult::Running(Arc::new(Self {
            child: self.child.clone(),
            resume: None,
            limit: self.limit,
            completed,
        }))
    }
}

/// Repeats its child until its child fails
pub struct RepeatedUntilFailure<B> {
    resume: Option<BehaviorArc<B>>,
    child: BehaviorArc<B>,
}

impl<B> RepeatedUntilFailure<B> {
    pub fn new(child: BehaviorArc<B>) -> Self {
        Self {
            child,
            resume: None,
        }
    }
}

impl<B> Debug for RepeatedUntilFailure<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepeatedUntilFailure")
            .field("child", &self.child)
            .finish()
    }
}

impl<B: 'static> BehaviorNode<B> for RepeatedUntilFailure<B> {
    fn tick(self: Arc<Self>, blackboard: &mut B) -> NodeResult<B> {
        if let Some(resume) = self.resume.as_ref() {
            match resume.clone().tick(blackboard) {
                NodeResult::Running(resume) => {
                    return NodeResult::Running(
                        Self {
                            resume: Some(resume),
                            child: self.child.clone(),
                        }
                        .arc(),
                    );
                }
                NodeResult::Failure => return NodeResult::Success,
                _ => (),
            }
        }
        match self.child.clone().tick(blackboard) {
            NodeResult::Running(resume) => NodeResult::Running(
                Self {
                    resume: Some(resume),
                    child: self.child.clone(),
                }
                .arc(),
            ),
            NodeResult::Success => {
                // Restart whenever we succeed
                NodeResult::Running(Arc::new(Self {
                    child: self.child.clone(),
                    resume: None,
                }))
            }
            NodeResult::Failure => {
                // We have encounted a failure, so *succeed*
                NodeResult::Success
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        composite::{
            tests::{test_with_context, Context},
            Sequence,
        },
        BehaviorRunner,
    };

    use super::*;
    use assert2::check;

    #[derive(Debug)]
    struct Push1;
    impl BehaviorNode<Context> for Push1 {
        fn tick(self: Arc<Self>, context: &mut Context) -> NodeResult<Context> {
            context.stack.push(1);
            NodeResult::Success
        }
    }

    #[derive(Debug)]
    struct FibPush;
    impl BehaviorNode<Context> for FibPush {
        fn tick(self: Arc<Self>, context: &mut Context) -> NodeResult<Context> {
            if context.stack.len() < 2 {
                context.stack.push(1);
            } else {
                let len = context.stack.len();
                let a = context.stack[len - 2];
                let b = context.stack[len - 1];
                context.stack.push(a + b);
            }
            NodeResult::Success
        }
    }

    #[derive(Debug)]
    struct IsCapped {
        cap: i32,
    }
    impl BehaviorNode<Context> for IsCapped {
        fn tick(self: Arc<Self>, context: &mut Context) -> NodeResult<Context> {
            if context.stack.iter().any(|v| *v > self.cap) {
                NodeResult::Failure
            } else {
                NodeResult::Success
            }
        }
    }

    #[test]
    fn limited_repeat_repeats_to_limit() {
        let runner = BehaviorRunner::new(LimitedRepeated::new(3, Push1.arc()).arc());
        let (res, context) = test_with_context(|| Context { stack: Vec::new() }, runner, 3);
        check!(res == Some(true));
        check!(context.stack == vec![1, 1, 1]);
    }

    #[test]
    fn repeat_until_failure_stops_on_failure() {
        let runner = BehaviorRunner::new(
            RepeatedUntilFailure::new(
                vec![FibPush.arc(), IsCapped { cap: 100 }.arc()]
                    .into_iter()
                    .collect::<Sequence<_>>()
                    .arc(),
            )
            .arc(),
        );
        let (res, context) = test_with_context(|| Context { stack: Vec::new() }, runner, 11);
        check!(res == Some(true));
        check!(context.stack == vec![1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144]);
    }
}
