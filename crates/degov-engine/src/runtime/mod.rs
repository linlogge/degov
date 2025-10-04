mod database;
mod ops;

pub use database::KvStore;

use crate::error::{EngineError, Result};
use rustyscript::{json_args, Module, Runtime as RustyRuntime, RuntimeOptions};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tracing::{debug, error};

/// Result of script execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Runtime manager that handles script execution via rustyscript
pub struct DenoRuntime {
    /// Shared KV store for database operations
    kv_store: Arc<KvStore>,
    /// Semaphore to limit concurrent executions
    semaphore: Arc<Semaphore>,
}

impl DenoRuntime {
    /// Create a new runtime manager
    pub async fn new(pool_size: usize) -> Result<Self> {
        let kv_store = Arc::new(KvStore::new());
        let semaphore = Arc::new(Semaphore::new(pool_size));

        Ok(Self {
            kv_store,
            semaphore,
        })
    }

    /// Create a new runtime instance with KV store extension
    fn create_runtime(kv_store: Arc<KvStore>) -> Result<RustyRuntime> {
        // Create the KV extension with the store in OpState
        let kv_extension = ops::kv_extension::init();

        let mut runtime = RustyRuntime::new(RuntimeOptions {
            timeout: Duration::from_secs(30),
            extensions: vec![kv_extension],
            ..Default::default()
        })
        .map_err(|e| EngineError::RuntimeError(format!("Failed to create runtime: {}", e)))?;

        // Store the KV store in the runtime's OpState
        runtime.register_function("__internal_setup_kv", move |_args| {
            // This is a workaround - we'll pass the store via the actual ops
            Ok(serde_json::Value::Null)
        })
        .map_err(|e| EngineError::RuntimeError(format!("Failed to setup KV: {}", e)))?;

        // Store KV in a global for the ops to access
        ops::set_kv_store(kv_store);

        Ok(runtime)
    }

    /// Execute a script with the given context
    pub async fn execute_script(
        &self,
        code: &str,
        context: serde_json::Value,
    ) -> Result<ScriptResult> {
        // Acquire a permit to limit concurrent executions
        let _permit = self.semaphore.acquire().await
            .map_err(|e| EngineError::RuntimeError(format!("Failed to acquire semaphore: {}", e)))?;

        let kv_store = self.kv_store.clone();
        let code = code.to_string();

        // Execute the script in a blocking task
        let result = tokio::task::spawn_blocking(move || {
            // Create a new runtime for this execution
            let mut runtime = Self::create_runtime(kv_store)?;

            debug!("Executing script with context: {:?}", context);

            let module = Module::new("user_script.js", &code);

            // Load the module
            let module_handle = runtime.load_module(&module)
                .map_err(|e| EngineError::ScriptError(format!("Failed to load module: {:?}", e)))?;

            // Call the default export function
            let exec_result: std::result::Result<serde_json::Value, rustyscript::Error> =
                runtime.call_function(Some(&module_handle), "default", json_args!(context));

            let script_result = match exec_result {
                Ok(value) => ScriptResult {
                    success: true,
                    output: Some(value),
                    error: None,
                },
                Err(e) => {
                    error!("Script execution error: {:?}", e);
                    ScriptResult {
                        success: false,
                        output: None,
                        error: Some(format!("{:?}", e)),
                    }
                }
            };

            Ok::<ScriptResult, EngineError>(script_result)
        })
        .await
        .map_err(|e| EngineError::RuntimeError(format!("Task join error: {}", e)))?;

        result
    }

    /// Shutdown the runtime
    pub async fn shutdown(&self) {
        debug!("Shutting down runtime manager");
        // Nothing to clean up in this implementation
    }
}

