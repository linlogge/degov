//! Transition logic for state machines

use super::Context;
use serde::{Deserialize, Serialize};

/// A transition between states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    event: String,
    target_state: String,
    #[serde(skip)]
    guard: Option<Guard>,
}

impl Transition {
    /// Create a new transition
    pub fn new(event: impl Into<String>, target_state: impl Into<String>) -> Self {
        Self {
            event: event.into(),
            target_state: target_state.into(),
            guard: None,
        }
    }

    /// Add a guard condition to this transition
    pub fn with_guard(mut self, guard: Guard) -> Self {
        self.guard = Some(guard);
        self
    }

    /// Get the event that triggers this transition
    pub fn event(&self) -> &str {
        &self.event
    }

    /// Get the target state
    pub fn target_state(&self) -> &str {
        &self.target_state
    }

    /// Check if this transition matches the event and passes guards
    pub fn matches(&self, event: &str, ctx: &Context) -> bool {
        if self.event != event {
            return false;
        }

        if let Some(guard) = &self.guard {
            guard.check(ctx)
        } else {
            true
        }
    }
}

/// Guard condition for transitions
#[derive(Clone)]
pub struct Guard {
    check_fn: std::sync::Arc<dyn Fn(&Context) -> bool + Send + Sync>,
}

impl Guard {
    /// Create a new guard from a function
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&Context) -> bool + Send + Sync + 'static,
    {
        Self {
            check_fn: std::sync::Arc::new(f),
        }
    }

    /// Check if the guard passes for the given context
    pub fn check(&self, ctx: &Context) -> bool {
        (self.check_fn)(ctx)
    }
}

impl std::fmt::Debug for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guard").finish()
    }
}

// Manual Serialize/Deserialize since we can't serialize closures
impl Serialize for Guard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Guards are not serializable, just serialize a placeholder
        serializer.serialize_none()
    }
}

impl<'de> Deserialize<'de> for Guard {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Create a default guard that always returns true
        Ok(Guard::new(|_| true))
    }
}


