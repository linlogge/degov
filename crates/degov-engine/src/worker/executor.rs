//! Task executor

use crate::error::{EngineError, Result, RuntimeError};
use crate::runtime::Runtime;
use crate::types::{RuntimeType, TaskDefinition};
use std::collections::HashMap;

/// Task executor that manages different runtimes
pub struct TaskExecutor {
    runtimes: HashMap<RuntimeType, Box<dyn Runtime>>,
}

impl TaskExecutor {
    /// Create a new task executor
    pub fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
        }
    }

    /// Register a runtime
    pub fn register_runtime(&mut self, runtime_type: RuntimeType, runtime: Box<dyn Runtime>) {
        self.runtimes.insert(runtime_type, runtime);
    }

    /// Execute a task
    pub async fn execute(&self, task: &TaskDefinition, input: &[u8]) -> Result<Vec<u8>> {
        let runtime = self
            .runtimes
            .get(&task.runtime_type)
            .ok_or_else(|| {
                EngineError::Runtime(RuntimeError::RuntimeNotAvailable(
                    task.runtime_type.as_str().to_string(),
                ))
            })?;

        runtime
            .execute(task, input)
            .await
            .map_err(EngineError::Runtime)
    }

    /// Get supported runtime types
    pub fn supported_runtimes(&self) -> Vec<RuntimeType> {
        self.runtimes.keys().copied().collect()
    }

    /// Check if a runtime is supported
    pub fn supports_runtime(&self, runtime_type: &RuntimeType) -> bool {
        self.runtimes.contains_key(runtime_type)
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}


