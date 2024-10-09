//! Implementations of composite nodes

// We do a little thin runner so nodes are thick

mod inverter;
mod parallel;
mod repeater;
mod selector;
mod sequence;
mod succeeder;

#[allow(unused_imports)]
pub use inverter::Inverter;
#[allow(unused_imports)]
pub use parallel::{ParallelSelector, ParallelSequence};
#[allow(unused_imports)]
pub use repeater::{LimitedRepeated, Repeated, RepeatedUntilFailure};
#[allow(unused_imports)]
pub use selector::Selector;
#[allow(unused_imports)]
pub use sequence::Sequence;
#[allow(unused_imports)]
pub use succeeder::Succeeder;

// Utilities for testing
#[cfg(test)]
mod tests {
    use crate::BehaviorRunner;

    pub(super) struct Context {
        pub stack: Vec<i32>,
    }

    pub(super) fn test_with_context<F>(
        init_context: F,
        mut runner: BehaviorRunner<Context>,
        running_limit: usize,
    ) -> (Option<bool>, Context)
    where
        F: FnOnce() -> Context,
    {
        let mut context = init_context();
        let mut running_count = 0;
        let mut res = runner.proceed(&mut context);
        while runner.is_running() && res.is_none() {
            if running_count >= running_limit {
                return (None, context);
            }
            running_count += 1;
            res = runner.proceed(&mut context);
        }
        (res, context)
    }
}
