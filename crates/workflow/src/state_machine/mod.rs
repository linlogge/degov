//! State machine implementation for workflows

mod context;
mod state;
mod transition;

pub use context::Context;
pub use state::{Action, State};
pub use transition::{Guard, Transition};

use crate::error::{WorkflowError, WorkflowResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State machine that defines workflow behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachine {
    states: HashMap<String, State>,
    initial_state: String,
}

impl StateMachine {
    /// Create a new builder for constructing a state machine
    pub fn builder() -> StateMachineBuilder {
        StateMachineBuilder::new()
    }

    /// Get the initial state
    pub fn initial_state(&self) -> &str {
        &self.initial_state
    }

    /// Get a state by name
    pub fn get_state(&self, name: &str) -> Option<&State> {
        self.states.get(name)
    }

    /// Attempt a state transition based on an event
    pub async fn transition(&self, ctx: &mut Context, event: &str) -> WorkflowResult<String> {
        let current_state_name = ctx.current_state();
        let current_state = self
            .states
            .get(current_state_name)
            .ok_or_else(|| WorkflowError::InvalidState(current_state_name.to_string()))?;

        // Find a matching transition
        let transition = current_state
            .find_transition(event, ctx)
            .ok_or_else(|| WorkflowError::TransitionNotAllowed {
                from: current_state_name.to_string(),
                event: event.to_string(),
            })?;

        let target_state_name = transition.target_state();

        // Verify target state exists
        let target_state = self
            .states
            .get(target_state_name)
            .ok_or_else(|| WorkflowError::InvalidState(target_state_name.to_string()))?;

        // Execute on_exit actions for current state
        for action in current_state.on_exit_actions() {
            action.execute(ctx).await?;
        }

        // Update context state
        ctx.set_state(target_state_name.to_string());

        // Execute on_enter actions for target state
        for action in target_state.on_enter_actions() {
            action.execute(ctx).await?;
        }

        Ok(target_state_name.to_string())
    }

    /// Validate that the state machine is well-formed
    pub fn validate(&self) -> WorkflowResult<()> {
        // Check initial state exists
        if !self.states.contains_key(&self.initial_state) {
            return Err(WorkflowError::InvalidDefinition(format!(
                "Initial state '{}' not found",
                self.initial_state
            )));
        }

        // Validate all transition targets exist
        for (state_name, state) in &self.states {
            for transition in state.transitions() {
                if !self.states.contains_key(transition.target_state()) {
                    return Err(WorkflowError::InvalidDefinition(format!(
                        "State '{}' has transition to non-existent state '{}'",
                        state_name,
                        transition.target_state()
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Builder for constructing state machines
pub struct StateMachineBuilder {
    states: HashMap<String, State>,
    initial_state: Option<String>,
}

impl StateMachineBuilder {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            initial_state: None,
        }
    }

    /// Set the initial state
    pub fn initial_state(mut self, name: impl Into<String>) -> Self {
        self.initial_state = Some(name.into());
        self
    }

    /// Add a state to the machine
    pub fn add_state(mut self, state: State) -> Self {
        self.states.insert(state.name().to_string(), state);
        self
    }

    /// Build the state machine
    pub fn build(self) -> WorkflowResult<StateMachine> {
        let initial_state = self
            .initial_state
            .ok_or_else(|| WorkflowError::InvalidDefinition("No initial state set".to_string()))?;

        if self.states.is_empty() {
            return Err(WorkflowError::InvalidDefinition(
                "State machine has no states".to_string(),
            ));
        }

        let machine = StateMachine {
            states: self.states,
            initial_state,
        };

        machine.validate()?;
        Ok(machine)
    }
}

impl Default for StateMachineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let sm = StateMachine::builder()
            .initial_state("start")
            .add_state(
                State::new("start")
                    .add_transition(Transition::new("next", "processing")),
            )
            .add_state(State::new("processing").add_transition(Transition::new("done", "end")))
            .add_state(State::new("end"))
            .build()
            .unwrap();

        assert_eq!(sm.initial_state(), "start");
        assert!(sm.get_state("start").is_some());
        assert!(sm.get_state("processing").is_some());
        assert!(sm.get_state("end").is_some());
    }

    #[test]
    fn test_validation_missing_initial() {
        let result = StateMachine::builder()
            .initial_state("nonexistent")
            .add_state(State::new("start"))
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_validation_missing_target() {
        let result = StateMachine::builder()
            .initial_state("start")
            .add_state(
                State::new("start")
                    .add_transition(Transition::new("next", "nonexistent")),
            )
            .build();

        assert!(result.is_err());
    }
}
