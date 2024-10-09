use crate::{BehaviorArc, BehaviorNode, NodeResult};
use std::sync::Arc;

/// Inverts the result of its child
///
/// Success becomes failure, failure success.
pub struct Inverter<B> {
    child: BehaviorArc<B>,
}

impl<B> std::fmt::Debug for Inverter<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inverter")
            .field("child", &self.child)
            .finish()
    }
}

impl<B> Inverter<B> {
    pub fn new(child: BehaviorArc<B>) -> Self {
        Self { child }
    }
}

impl<B: 'static> BehaviorNode<B> for Inverter<B> {
    fn tick(self: Arc<Self>, blackboard: &mut B) -> NodeResult<B> {
        match self.child.clone().tick(blackboard) {
            NodeResult::Success => NodeResult::Failure,
            NodeResult::Failure => NodeResult::Success,
            NodeResult::Running(resume) => NodeResult::Running(Inverter::new(resume).arc()),
        }
    }
}

#[cfg(test)]
mod tests {
    use assert2::check;

    use super::*;
    use crate::{
        composite::{
            tests::{test_with_context, Context},
            Succeeder,
        },
        BehaviorNode, BehaviorRunner, NodeResult,
    };

    #[derive(Debug)]
    struct SucceedAfterSteps {
        steps: u32,
        step: u32,
    }

    impl SucceedAfterSteps {
        fn new(steps: u32) -> Self {
            Self { steps, step: 0 }
        }
    }

    impl BehaviorNode<Context> for SucceedAfterSteps {
        fn tick(self: Arc<Self>, _context: &mut Context) -> crate::NodeResult<Context> {
            if self.step < self.steps {
                NodeResult::Running(
                    Self {
                        steps: self.steps,
                        step: self.step + 1,
                    }
                    .arc(),
                )
            } else {
                NodeResult::Success
            }
        }
    }

    #[test]
    fn inverter_inverts_properly() {
        let runner1 =
            BehaviorRunner::from_node(Inverter::<Context>::new(Succeeder::default().arc()));

        let (res, context) = test_with_context(|| Context { stack: Vec::new() }, runner1, 0);
        check!(res == Some(false));
        check!(context.stack == Vec::<i32>::new());

        let runner2 = BehaviorRunner::new(Inverter::new(SucceedAfterSteps::new(9).arc()).arc());

        let (res, context) = test_with_context(|| Context { stack: Vec::new() }, runner2, 9);
        check!(res == Some(false));
        check!(context.stack == Vec::<i32>::new());
    }
}
