//! The composite nodes here share an API with [`super::Sequence`] and
//! [`super::Selector`], but instead of polling each node individually
//! "in sequence", all nodes are polled each poll step.

use std::sync::Arc;

use crate::{BehaviorArc, BehaviorNode, NodeResult};

pub struct ParallelSequence<B> {
    pub(crate) sub: Arc<[BehaviorArc<B>]>,
}

impl<B> std::fmt::Debug for ParallelSequence<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("ParallelSequence<{:p}>", self.sub.as_ref()))
            .field("sub", &self.sub)
            .finish()
    }
}

impl<B, I: Into<BehaviorArc<B>>> FromIterator<I> for ParallelSequence<B> {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self {
            sub: Arc::from(iter.into_iter().map(Into::into).collect::<Vec<_>>()),
        }
    }
}

impl<B: 'static> BehaviorNode<B> for ParallelSequence<B> {
    fn tick(self: Arc<Self>, context: &mut B) -> NodeResult<B> {
        let mut new_children = vec![];
        for child in self.sub.iter() {
            match child.clone().tick(context) {
                NodeResult::Failure => return NodeResult::Failure,
                NodeResult::Success => {}
                NodeResult::Running(node) => {
                    new_children.push(node);
                }
            }
        }

        if new_children.is_empty() {
            NodeResult::Success
        } else {
            NodeResult::Running(
                Self {
                    sub: Arc::from(new_children),
                }
                .arc(),
            )
        }
    }
}

pub struct ParallelSelector<B> {
    pub(crate) sub: Arc<[BehaviorArc<B>]>,
}

impl<B> std::fmt::Debug for ParallelSelector<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("ParallelSelector<{:p}>", self.sub.as_ref()))
            .field("sub", &self.sub)
            .finish()
    }
}

impl<B, I: Into<BehaviorArc<B>>> FromIterator<I> for ParallelSelector<B> {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self {
            sub: Arc::from(iter.into_iter().map(Into::into).collect::<Vec<_>>()),
        }
    }
}

impl<B: 'static> BehaviorNode<B> for ParallelSelector<B> {
    fn tick(self: Arc<Self>, context: &mut B) -> NodeResult<B> {
        let mut new_children = vec![];
        for child in self.sub.iter() {
            match child.clone().tick(context) {
                NodeResult::Success => return NodeResult::Success,
                NodeResult::Failure => {}
                NodeResult::Running(node) => {
                    new_children.push(node);
                }
            }
        }

        if new_children.is_empty() {
            NodeResult::Failure
        } else {
            NodeResult::Running(
                Self {
                    sub: Arc::from(new_children),
                }
                .arc(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use assert2::check;

    use crate::BehaviorRunner;

    use super::*;

    #[derive(Default, PartialEq, Debug)]
    struct Diem {
        day: usize,
        paydays: Vec<usize>,
    }

    #[derive(Debug, Default)]
    struct PaydayWait {
        index: usize,
        payload: usize,
        time: usize,
        terminal: bool,
    }

    impl BehaviorNode<Diem> for PaydayWait {
        fn tick(self: Arc<Self>, context: &mut Diem) -> NodeResult<Diem> {
            if self.time < context.day {
                let paydays = &mut context.paydays;
                let size_diff = self.index.abs_diff(paydays.len());
                if paydays.len() <= self.index {
                    paydays.extend(vec![0; size_diff + 1]);
                }
                paydays[self.index] = self.payload;
                if self.terminal {
                    NodeResult::Failure
                } else {
                    NodeResult::Success
                }
            } else {
                NodeResult::Running(self)
            }
        }
    }

    #[test]
    fn parallel_sequence_test() {
        let mut runner = BehaviorRunner::from_node(
            [
                PaydayWait {
                    index: 0,
                    payload: 19,
                    time: 5,
                    ..Default::default()
                }
                .arc(),
                PaydayWait {
                    index: 1,
                    payload: 42,
                    ..Default::default()
                }
                .arc(),
                PaydayWait {
                    index: 3,
                    time: 13,
                    ..Default::default()
                }
                .arc(),
            ]
            .into_iter()
            .collect::<ParallelSequence<_>>(),
        );

        let mut diem = Diem::default();
        check!(runner.proceed(&mut diem) == None);
        check!(diem == Diem::default());
        diem.day = 1;
        check!(runner.proceed(&mut diem) == None);
        check!(
            diem == Diem {
                paydays: vec![0, 42],
                day: 1,
            }
        );
        diem.day = 6;
        check!(runner.proceed(&mut diem) == None);
        check!(
            diem == Diem {
                paydays: vec![19, 42],
                day: 6,
            }
        );
        diem.day = 21;
        check!(runner.proceed(&mut diem) == Some(true));
        check!(
            diem == Diem {
                paydays: vec![19, 42, 0, 0],
                day: 21,
            }
        );
    }

    #[test]
    fn parallel_selector_test() {
        let mut runner = BehaviorRunner::from_node(
            [
                PaydayWait {
                    index: 0,
                    payload: 19,
                    time: 5,
                    ..Default::default()
                }
                .arc(),
                PaydayWait {
                    index: 1,
                    payload: 42,
                    terminal: true,
                    ..Default::default()
                }
                .arc(),
                PaydayWait {
                    index: 3,
                    time: 13,
                    ..Default::default()
                }
                .arc(),
            ]
            .into_iter()
            .collect::<ParallelSelector<_>>(),
        );

        let mut diem = Diem::default();
        check!(runner.proceed(&mut diem) == None);
        check!(diem == Diem::default());
        diem.day = 1;
        check!(runner.proceed(&mut diem) == None);
        check!(
            diem == Diem {
                paydays: vec![0, 42],
                day: 1,
            }
        );
        diem = Diem {
            day: 6,
            ..Default::default()
        };
        check!(runner.proceed(&mut diem) == Some(true));
        check!(
            diem == Diem {
                paydays: vec![19],
                day: 6,
            }
        );
        diem = Diem {
            day: 21,
            ..Default::default()
        };
        check!(runner.proceed(&mut diem) == Some(true));
        check!(
            diem == Diem {
                paydays: vec![19],
                day: 21,
            }
        );
    }
}
