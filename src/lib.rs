use std::{future::Future, time::Duration};

/// Timer service provider trait.
pub trait Timer: Future {
    /// Create new timer.
    ///
    /// # Parameters
    /// * `duration` - Timer expiration interval
    fn new(duration: Duration) -> Self;
}

/// Timer service provider trait with more configiration.
pub trait TimerWithContext: Timer {
    /// timer extra context parameter.
    type Context;
    /// Create new timer with context parameter
    ///
    /// # Parameters
    /// * `duration` - Timer expiration interval
    /// * `context` - See [`TimerWithContext::Context`]
    fn new_with_context<C>(duration: Duration, context: C) -> Self
    where
        C: AsMut<Self::Context>;
}

/// Timer implementation using hashed timewheel algorithm
pub mod hashed;
