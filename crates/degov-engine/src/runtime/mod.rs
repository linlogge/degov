//! Runtime abstraction for task execution

mod javascript;
mod wasm;

pub use javascript::JavaScriptRuntime;
pub use wasm::WasmRuntime;

use crate::error::RuntimeResult;
use crate::types::{RuntimeType, TaskDefinition};
use async_trait::async_trait;

/// Trait for task execution runtimes
#[async_trait]
pub trait Runtime: Send + Sync {
    /// Execute a task and return the output
    async fn execute(&self, task: &TaskDefinition, input: &[u8]) -> RuntimeResult<Vec<u8>>;

    /// Get the runtime type
    fn runtime_type(&self) -> RuntimeType;

    /// Check if this runtime can execute the given task
    fn can_execute(&self, task: &TaskDefinition) -> bool {
        task.runtime_type == self.runtime_type()
    }
}


