//! JavaScript runtime using rquickjs

use crate::error::{RuntimeError, RuntimeResult};
use crate::types::{RuntimeType, TaskDefinition};
use async_trait::async_trait;
use rquickjs::{Context, Runtime as QjsRuntime};
use std::time::Duration;
use tokio::time::timeout;

/// JavaScript runtime implementation using rquickjs
pub struct JavaScriptRuntime {
    timeout_duration: Duration,
}

impl JavaScriptRuntime {
    /// Create a new JavaScript runtime
    pub fn new() -> Self {
        Self {
            timeout_duration: Duration::from_secs(30),
        }
    }

    /// Create a new JavaScript runtime with custom timeout
    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self {
            timeout_duration: Duration::from_millis(timeout_ms),
        }
    }

    /// Execute JavaScript code synchronously (internal)
    fn execute_sync(&self, code: &str, input: &[u8]) -> RuntimeResult<Vec<u8>> {
        // Create a new runtime for each execution (isolation)
        let runtime = QjsRuntime::new().map_err(|e| {
            RuntimeError::JavaScript(format!("Failed to create runtime: {}", e))
        })?;

        let context = Context::full(&runtime).map_err(|e| {
            RuntimeError::JavaScript(format!("Failed to create context: {}", e))
        })?;

        context.with(|ctx| {
            // Convert input bytes to JSON string
            let input_str = String::from_utf8_lossy(input);
            
            // Inject input as global variable
            let input_code = format!("globalThis.input = {};", input_str);
            ctx.eval::<(), _>(input_code).map_err(|e| {
                RuntimeError::JavaScript(format!("Failed to inject input: {}", e))
            })?;

            // Execute the user code
            let result: rquickjs::Value = ctx.eval(code).map_err(|e| {
                RuntimeError::JavaScript(format!("Execution error: {}", e))
            })?;

            // Convert result to JSON
            let json_result: Option<rquickjs::String> = ctx
                .json_stringify(result)
                .map_err(|e| {
                    RuntimeError::JavaScript(format!("Failed to stringify result: {}", e))
                })?;

            let json_str = json_result
                .map(|s| s.to_string().map_err(|e| RuntimeError::JavaScript(format!("Failed to convert result: {}", e))))
                .transpose()?
                .unwrap_or_else(|| "null".to_string());

            Ok(json_str.into_bytes())
        })
    }
}

impl Default for JavaScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::Runtime for JavaScriptRuntime {
    async fn execute(&self, task: &TaskDefinition, input: &[u8]) -> RuntimeResult<Vec<u8>> {
        let code = String::from_utf8(task.code.clone()).map_err(|e| {
            RuntimeError::InvalidCode(format!("Invalid UTF-8 in JavaScript code: {}", e))
        })?;

        let timeout_duration = if task.timeout_ms > 0 {
            Duration::from_millis(task.timeout_ms)
        } else {
            self.timeout_duration
        };

        let input = input.to_vec();
        let code_clone = code.clone();

        // Execute in a blocking task with timeout
        let result = timeout(timeout_duration, tokio::task::spawn_blocking(move || {
            let rt = JavaScriptRuntime::new();
            rt.execute_sync(&code_clone, &input)
        }))
        .await
        .map_err(|_| RuntimeError::Timeout(task.timeout_ms))?
        .map_err(|e| RuntimeError::JavaScript(format!("Task execution error: {}", e)))?;

        result
    }

    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::JavaScript
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_execution() {
        use super::super::Runtime as _;
        let runtime = JavaScriptRuntime::new();
        let task = TaskDefinition {
            name: "test".to_string(),
            runtime_type: RuntimeType::JavaScript,
            code: b"input.value * 2".to_vec(),
            timeout_ms: 5000,
            retry_policy: None,
        };

        let input = br#"{"value": 21}"#;
        let result = runtime.execute(&task, input).await.unwrap();
        let result_str = String::from_utf8(result).unwrap();
        
        assert_eq!(result_str, "42");
    }

    #[tokio::test]
    async fn test_timeout() {
        use super::super::Runtime as _;
        let runtime = JavaScriptRuntime::new();
        let task = TaskDefinition {
            name: "test".to_string(),
            runtime_type: RuntimeType::JavaScript,
            code: b"while(true) {}".to_vec(),
            timeout_ms: 100,
            retry_policy: None,
        };

        let input = br#"{}"#;
        let result = runtime.execute(&task, input).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RuntimeError::Timeout(_)));
    }
}

