//! Workflow definition registry

use crate::types::{WorkflowDefinition, WorkflowId};
use std::collections::HashMap;

/// In-memory registry of workflow definitions
pub struct WorkflowRegistry {
    definitions: HashMap<WorkflowId, WorkflowDefinition>,
}

impl WorkflowRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    /// Register a workflow definition
    pub fn register(&mut self, definition: WorkflowDefinition) {
        self.definitions.insert(definition.id, definition);
    }

    /// Get a workflow definition
    pub fn get(&self, id: &WorkflowId) -> Option<&WorkflowDefinition> {
        self.definitions.get(id)
    }

    /// Check if a workflow is registered
    pub fn contains(&self, id: &WorkflowId) -> bool {
        self.definitions.contains_key(id)
    }

    /// Remove a workflow definition
    pub fn unregister(&mut self, id: &WorkflowId) -> Option<WorkflowDefinition> {
        self.definitions.remove(id)
    }

    /// List all workflow IDs
    pub fn list(&self) -> Vec<WorkflowId> {
        self.definitions.keys().copied().collect()
    }
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}


