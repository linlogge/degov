//! Execution context for state machines

use crate::types::WorkflowId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context holds the runtime state during workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    workflow_id: WorkflowId,
    current_state: String,
    data: serde_json::Value,
    metadata: HashMap<String, String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Context {
    /// Create a new context
    pub fn new(workflow_id: WorkflowId, initial_state: String) -> Self {
        let now = Utc::now();
        Self {
            workflow_id,
            current_state: initial_state,
            data: serde_json::Value::Object(serde_json::Map::new()),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a context with initial data
    pub fn with_data(workflow_id: WorkflowId, initial_state: String, data: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            workflow_id,
            current_state: initial_state,
            data,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the workflow ID
    pub fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }

    /// Get the current state
    pub fn current_state(&self) -> &str {
        &self.current_state
    }

    /// Set the current state
    pub(crate) fn set_state(&mut self, state: String) {
        self.current_state = state;
        self.updated_at = Utc::now();
    }

    /// Get the data
    pub fn data(&self) -> &serde_json::Value {
        &self.data
    }

    /// Get mutable data
    pub fn data_mut(&mut self) -> &mut serde_json::Value {
        self.updated_at = Utc::now();
        &mut self.data
    }

    /// Set data value at path
    pub fn set(&mut self, key: &str, value: serde_json::Value) {
        if let serde_json::Value::Object(map) = &mut self.data {
            map.insert(key.to_string(), value);
            self.updated_at = Utc::now();
        }
    }

    /// Get data value at path
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        if let serde_json::Value::Object(map) = &self.data {
            map.get(key)
        } else {
            None
        }
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Get creation timestamp
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Get last update timestamp
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}


