//! State definition for state machines

use super::{Context, Transition};
use crate::error::WorkflowResult;
use crate::types::TaskDefinition;
use serde::{Deserialize, Serialize};

/// A state in the state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    name: String,
    #[serde(default)]
    on_enter: Vec<Action>,
    #[serde(default)]
    on_exit: Vec<Action>,
    #[serde(default)]
    transitions: Vec<Transition>,
}

impl State {
    /// Create a new state
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            on_enter: Vec::new(),
            on_exit: Vec::new(),
            transitions: Vec::new(),
        }
    }

    /// Get the state name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Add an on_enter action
    pub fn on_enter(mut self, action: Action) -> Self {
        self.on_enter.push(action);
        self
    }

    /// Add an on_exit action
    pub fn on_exit(mut self, action: Action) -> Self {
        self.on_exit.push(action);
        self
    }

    /// Add a transition
    pub fn add_transition(mut self, transition: Transition) -> Self {
        self.transitions.push(transition);
        self
    }

    /// Get on_enter actions
    pub fn on_enter_actions(&self) -> &[Action] {
        &self.on_enter
    }

    /// Get on_exit actions
    pub fn on_exit_actions(&self) -> &[Action] {
        &self.on_exit
    }

    /// Get all transitions
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    /// Find a transition that matches the event and passes guards
    pub fn find_transition(&self, event: &str, ctx: &Context) -> Option<&Transition> {
        self.transitions
            .iter()
            .find(|t| t.matches(event, ctx))
    }
}

/// Actions that can be executed during state transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Execute a task (will be enqueued for workers)
    ExecuteTask(TaskDefinition),
    
    /// Set a value in the context
    SetData {
        key: String,
        value: serde_json::Value,
    },
    
    /// Log a message (for debugging)
    Log { message: String },
    
    /// No-op action
    NoOp,
}

impl Action {
    /// Execute the action on the context
    pub async fn execute(&self, ctx: &mut Context) -> WorkflowResult<()> {
        match self {
            Action::ExecuteTask(_task) => {
                // Task execution is handled by the engine
                // This is just a placeholder for validation
                Ok(())
            }
            Action::SetData { key, value } => {
                ctx.set(key, value.clone());
                Ok(())
            }
            Action::Log { message } => {
                tracing::info!("State action log: {}", message);
                Ok(())
            }
            Action::NoOp => Ok(()),
        }
    }

    /// Create an ExecuteTask action
    pub fn execute_task(task: TaskDefinition) -> Self {
        Action::ExecuteTask(task)
    }

    /// Create a SetData action
    pub fn set_data(key: impl Into<String>, value: serde_json::Value) -> Self {
        Action::SetData {
            key: key.into(),
            value,
        }
    }

    /// Create a Log action
    pub fn log(message: impl Into<String>) -> Self {
        Action::Log {
            message: message.into(),
        }
    }
}

