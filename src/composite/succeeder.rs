use crate::{BehaviorArc, BehaviorNode, NodeResult};
use std::sync::Arc;

/// Always succeedes.
pub struct Succeeder<B> {
    child: Option<BehaviorArc<B>>,
}

impl<B> std::fmt::Debug for Succeeder<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Succeeder")
            .field("child", &self.child)
            .finish()
    }
}

impl<B> Default for Succeeder<B> {
    fn default() -> Self {
        Self { child: None }
    }
}

impl<B> Succeeder<B> {
    pub fn new(child: BehaviorArc<B>) -> Self {
        Self { child: Some(child) }
    }
}

impl<B: 'static> BehaviorNode<B> for Succeeder<B> {
    fn tick(self: Arc<Self>, blackboard: &mut B) -> crate::NodeResult<B> {
        if let Some(child) = self.child.as_ref() {
            match child.clone().tick(blackboard) {
                NodeResult::Failure | NodeResult::Success => NodeResult::Success,
                NodeResult::Running(resume) => NodeResult::Running(Succeeder::new(resume).arc()),
            }
        } else {
            NodeResult::Success
        }
    }
}

#[cfg(test)]
mod tests {}
