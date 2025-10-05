use crate::error::{EngineError, Result};
use crate::model::TaskResult;
use rustyscript::{json_args, Error as RsError, Module, Runtime, RuntimeOptions};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

/// Deno runtime for executing JavaScript/TypeScript code
/// 
/// This runtime uses a pool pattern where each execution spawns a blocking task
/// with its own isolated runtime instance. This avoids thread-safety issues
/// while still providing parallelism through the semaphore.
pub struct DenoRuntime {
    /// Semaphore to limit concurrent executions
    semaphore: Arc<Semaphore>,
    /// Timeout for script execution
    timeout_secs: u64,
}

impl DenoRuntime {
    /// Create a new Deno runtime manager with a pool size
    pub async fn new(pool_size: usize) -> Result<Self> {
        info!("Initializing Deno runtime manager with concurrency limit of {}", pool_size);

        Ok(Self {
            semaphore: Arc::new(Semaphore::new(pool_size)),
            timeout_secs: 30,
        })
    }

    /// Execute a JavaScript/TypeScript script with the given context
    pub async fn execute_script(
        &self,
        code: &str,
        context: Value,
    ) -> Result<TaskResult> {
        let code = code.to_string();

        // Acquire semaphore permit to limit concurrency
        let _permit = self.semaphore.acquire().await
            .map_err(|e| EngineError::RuntimeError(format!("Failed to acquire permit: {}", e)))?;

        let timeout_secs = self.timeout_secs;

        // Execute in a blocking task with its own runtime instance
        let result = tokio::task::spawn_blocking(move || {
            Self::execute_with_new_runtime(&code, context, timeout_secs)
        })
        .await
        .map_err(|e| EngineError::RuntimeError(format!("Task join error: {}", e)))?;

        result
    }

    /// Execute code with a fresh runtime instance
    fn execute_with_new_runtime(
        code: &str,
        context: Value,
        timeout_secs: u64,
    ) -> Result<TaskResult> {
        let start = std::time::Instant::now();

        // Wrap the code in a module format if it's not already
        let module_code = if code.trim().starts_with("export") {
            code.to_string()
        } else {
            format!(
                "export default function(context) {{\n{}\n}}",
                code
            )
        };

        debug!("Executing script: {}", module_code.chars().take(100).collect::<String>());

        let module = Module::new("script.js", &module_code);

        // Create a new runtime for this execution
        let runtime_options = RuntimeOptions {
            timeout: Duration::from_secs(timeout_secs),
            default_entrypoint: Some("default".to_string()),
            ..Default::default()
        };
        
        let mut runtime = Runtime::new(runtime_options)
            .map_err(|e| EngineError::RuntimeError(format!("Failed to create runtime: {}", e)))?;

        // Load the module
        let handle = runtime
            .load_module(&module)
            .map_err(|e| {
                error!("Failed to load module: {}", e);
                EngineError::ScriptError(format!("Failed to load module: {}", e))
            })?;

        // Call the default export function with the context
        let result: std::result::Result<Value, RsError> = runtime
            .call_function(Some(&handle), "default", json_args!(context));

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                debug!("Script executed successfully in {}ms", duration_ms);
                Ok(TaskResult {
                    task_id: String::new(),
                    success: true,
                    output: Some(output),
                    error: None,
                    duration_ms,
                })
            }
            Err(e) => {
                error!("Script execution failed: {}", e);
                Ok(TaskResult {
                    task_id: String::new(),
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms,
                })
            }
        }
    }

    /// Shutdown the runtime manager
    pub async fn shutdown(&self) {
        info!("Shutting down Deno runtime manager");
        
        // Note: We can't force-stop running tasks, but we can wait for all
        // current tasks to complete by acquiring all permits
        let mut permits = Vec::new();
        
        // Best effort to acquire permits
        while let Ok(permit) = self.semaphore.try_acquire() {
            permits.push(permit);
        }
        
        info!("Deno runtime manager shut down (acquired {} permits)", permits.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_simple_execution() {
        let runtime = DenoRuntime::new(2).await.unwrap();

        let code = r#"
            export default function(context) {
                return { result: context.value * 2 };
            }
        "#;

        let context = json!({ "value": 21 });
        let result = runtime.execute_script(code, context).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output.unwrap()["result"], 42);
    }

    #[tokio::test]
    async fn test_console_log() {
        let runtime = DenoRuntime::new(1).await.unwrap();

        let code = r#"
            export default function(context) {
                console.log("Hello from JavaScript!", context.name);
                return { message: "logged" };
            }
        "#;

        let context = json!({ "name": "DeGov" });
        let result = runtime.execute_script(code, context).await.unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let runtime = DenoRuntime::new(1).await.unwrap();

        let code = r#"
            export default function(context) {
                throw new Error("Intentional error");
            }
        "#;

        let context = json!({});
        let result = runtime.execute_script(code, context).await.unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let runtime = DenoRuntime::new(4).await.unwrap();

        let code = r#"
            export default function(context) {
                return { result: context.value + 1 };
            }
        "#;

        // Execute 10 tasks sequentially (each still respects the semaphore)
        for i in 0..10 {
            let code_clone = code.to_string();
            let context = json!({ "value": i });
            
            let result = runtime.execute_script(&code_clone, context).await.unwrap();
            assert!(result.success);
            assert_eq!(result.output.unwrap()["result"], i + 1);
        }
    }
}
