use crate::{BehaviorArc, BehaviorNode, NodeResult};
use std::sync::Arc;

pub struct Selector<B> {
    pub(crate) sub: Arc<[BehaviorArc<B>]>,
}

impl<B> std::fmt::Debug for Selector<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("Selector<{:p}>", self.sub.as_ref()))
            .field("sub", &self.sub)
            .finish()
    }
}

impl<B, I: Into<BehaviorArc<B>>> FromIterator<I> for Selector<B> {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self {
            sub: Arc::from(iter.into_iter().map(Into::into).collect::<Vec<_>>()),
        }
    }
}

impl<B: 'static> Selector<B> {
    pub(crate) fn resume(
        seq: Arc<[BehaviorArc<B>]>,
        index: usize,
        resume: BehaviorArc<B>,
    ) -> BehaviorArc<B> {
        SelectorResume { seq, resume, index }.arc()
    }
}

impl<B: 'static> BehaviorNode<B> for Selector<B> {
    fn tick(self: Arc<Self>, blackboard: &mut B) -> NodeResult<B> {
        for (idx, sub) in self.sub.iter().enumerate() {
            match sub.clone().tick(blackboard) {
                NodeResult::Failure => {}
                NodeResult::Success => return NodeResult::Success,
                NodeResult::Running(resume) => {
                    return NodeResult::Running(Self::resume(self.sub.clone(), idx, resume))
                }
            }
        }
        NodeResult::Failure
    }
}

pub(crate) struct SelectorResume<B> {
    pub(crate) seq: Arc<[BehaviorArc<B>]>,
    pub(crate) resume: BehaviorArc<B>,
    pub(crate) index: usize,
}

impl<B> std::fmt::Debug for SelectorResume<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("SelectorResume<{:p}>", self.seq.as_ref()))
            .field("resume", &self.resume)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

impl<B: 'static> BehaviorNode<B> for SelectorResume<B> {
    fn tick(self: Arc<Self>, blackboard: &mut B) -> NodeResult<B> {
        // Tick the node we want to resume on
        match self.resume.clone().tick(blackboard) {
            NodeResult::Failure => {}
            NodeResult::Success => return NodeResult::Success,
            NodeResult::Running(resume) => {
                return NodeResult::Running(Selector::resume(self.seq.clone(), self.index, resume))
            }
        }
        for (idx, sub) in self.seq.iter().enumerate().skip(self.index + 1) {
            match sub.clone().tick(blackboard) {
                NodeResult::Failure => {}
                NodeResult::Success => return NodeResult::Success,
                NodeResult::Running(resume) => {
                    return NodeResult::Running(Selector::resume(self.seq.clone(), idx, resume))
                }
            }
        }
        NodeResult::Failure
    }
}

#[cfg(test)]
mod tests {
    use assert2::check;

    use super::*;
    use crate::{
        composite::tests::{test_with_context, Context},
        BehaviorNode, BehaviorRunner, NodeResult,
    };

    #[derive(Debug)]
    struct TwoStepPush {
        payload: i32,
        fail: bool,
        step: usize,
    }

    impl BehaviorNode<Context> for TwoStepPush {
        fn tick(self: Arc<Self>, context: &mut Context) -> NodeResult<Context> {
            if self.step < 1 {
                NodeResult::Running(
                    Self {
                        payload: self.payload,
                        fail: self.fail,
                        step: self.step + 1,
                    }
                    .arc(),
                )
            } else {
                context.stack.push(self.payload);
                if self.fail {
                    NodeResult::Failure
                } else {
                    NodeResult::Success
                }
            }
        }
    }

    #[test]
    fn selector_skips_tried_node() {
        let runner = BehaviorRunner::new(
            [
                TwoStepPush {
                    payload: 1,
                    fail: true,
                    step: 0,
                }
                .arc(),
                TwoStepPush {
                    payload: 2,
                    fail: false,
                    step: 0,
                }
                .arc(),
            ]
            .into_iter()
            .collect::<Selector<_>>()
            .arc(),
        );
        let (res, context) = test_with_context(|| Context { stack: Vec::new() }, runner, 10);
        check!(res == Some(true));
        check!(context.stack == vec![1, 2]);
    }
}
