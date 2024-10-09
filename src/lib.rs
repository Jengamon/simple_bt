//! Create a simple behavior tree implementation

pub mod composite;

use std::sync::Arc;

#[derive(Debug)]
pub enum NodeResult<B> {
    /// The node is still running
    ///
    /// This contains the node to be ticked
    Running(BehaviorArc<B>),
    /// The node succeeded
    Success,
    /// The node failed
    Failure,
}

pub type BehaviorArc<B> = Arc<dyn BehaviorNode<B>>;

// This is our main "behavior tree" trait.
// all nodes implement this trait.

pub trait BehaviorNode<B>: std::fmt::Debug + Send + Sync {
    fn tick(self: Arc<Self>, context: &mut B) -> NodeResult<B>;

    fn arc(self) -> BehaviorArc<B>
    where
        Self: Sized + Send + Sync + 'static,
    {
        Arc::new(self)
    }
}

#[derive(Debug)]
/// Takes care of executing a behavior tree
pub struct BehaviorRunner<B> {
    tree: BehaviorArc<B>,
    current_tick: Option<BehaviorArc<B>>,
}

impl<B> BehaviorRunner<B> {
    pub fn new(tree: BehaviorArc<B>) -> Self {
        Self {
            tree,
            current_tick: None,
        }
    }

    pub fn from_node<N>(node: N) -> Self
    where
        N: BehaviorNode<B> + 'static,
    {
        Self {
            tree: Arc::new(node),
            current_tick: None,
        }
    }

    pub fn into_inner(self) -> BehaviorArc<B> {
        self.current_tick.unwrap_or(self.tree)
    }

    pub fn is_running(&self) -> bool {
        self.current_tick.is_some()
    }

    fn tick_node(&mut self, node: &Arc<dyn BehaviorNode<B>>, context: &mut B) -> Option<bool> {
        match node.clone().tick(context) {
            NodeResult::Running(nbp) => {
                self.current_tick = Some(nbp);
                None
            }
            NodeResult::Success => Some(true),
            NodeResult::Failure => Some(false),
        }
    }

    // returns None -> still running
    // return Some(p) -> p true success, p false failure
    pub fn proceed(&mut self, context: &mut B) -> Option<bool> {
        if let Some(bp) = self.current_tick.take() {
            self.tick_node(&bp, context)
        } else {
            let node = self.tree.clone();
            self.tick_node(&node, context)
        }
    }
}
