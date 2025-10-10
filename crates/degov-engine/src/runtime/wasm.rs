//! WASM runtime using wasmtime

use crate::error::{RuntimeError, RuntimeResult};
use crate::types::{RuntimeType, TaskDefinition};
use async_trait::async_trait;
use std::time::Duration;
use tokio::time::timeout;
use wasmtime::*;
use wasmtime_wasi::WasiCtxBuilder;

/// WASM runtime implementation using wasmtime
pub struct WasmRuntime {
    engine: Engine,
    timeout_duration: Duration,
}

impl WasmRuntime {
    /// Create a new WASM runtime
    pub fn new() -> RuntimeResult<Self> {
        let mut config = Config::new();
        config.async_support(true);
        config.wasm_component_model(false); // Enable when ready for component model

        let engine = Engine::new(&config)
            .map_err(|e| RuntimeError::Wasm(format!("Failed to create engine: {}", e)))?;

        Ok(Self {
            engine,
            timeout_duration: Duration::from_secs(30),
        })
    }

    /// Create a new WASM runtime with custom timeout
    pub fn with_timeout(timeout_ms: u64) -> RuntimeResult<Self> {
        let mut runtime = Self::new()?;
        runtime.timeout_duration = Duration::from_millis(timeout_ms);
        Ok(runtime)
    }

    /// Execute WASM module
    async fn execute_wasm(&self, wasm_bytes: &[u8], input: &[u8]) -> RuntimeResult<Vec<u8>> {
        // Create a new store for each execution
        let mut linker = Linker::new(&self.engine);
        
        // Add WASI support (wasmtime 37 API)
        // Note: For wasmtime 37, WASI linker setup is done differently
        // This is a simplified version that compiles but may need adjustments for full WASI support
        
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build();

        let mut store = Store::new(&self.engine, wasi);

        // Load the WASM module
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| RuntimeError::Wasm(format!("Failed to load module: {}", e)))?;

        // Instantiate the module
        let instance = linker
            .instantiate_async(&mut store, &module)
            .await
            .map_err(|e| RuntimeError::Wasm(format!("Failed to instantiate: {}", e)))?;

        // Look for the execute function
        let execute_func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "execute")
            .map_err(|e| {
                RuntimeError::Wasm(format!("Failed to find 'execute' function: {}", e))
            })?;

        // Allocate memory for input (simplified - real implementation would use proper memory management)
        let input_ptr = 0; // This would need proper memory allocation
        let input_len = input.len() as i32;

        // Call the function
        let result_ptr = execute_func
            .call_async(&mut store, (input_ptr, input_len))
            .await
            .map_err(|e| RuntimeError::Wasm(format!("Execution error: {}", e)))?;

        // For now, return a simple result
        // Real implementation would read from WASM memory
        Ok(vec![result_ptr as u8])
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default WASM runtime")
    }
}

#[async_trait]
impl super::Runtime for WasmRuntime {
    async fn execute(&self, task: &TaskDefinition, input: &[u8]) -> RuntimeResult<Vec<u8>> {
        let timeout_duration = if task.timeout_ms > 0 {
            Duration::from_millis(task.timeout_ms)
        } else {
            self.timeout_duration
        };

        // Execute with timeout
        let result = timeout(
            timeout_duration,
            self.execute_wasm(&task.code, input),
        )
        .await
        .map_err(|_| RuntimeError::Timeout(task.timeout_ms))??;

        Ok(result)
    }

    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Wasm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = WasmRuntime::new();
        assert!(runtime.is_ok());
    }
}

