# degov-engine

A lightweight workflow engine with Deno runtime support for executing JavaScript/TypeScript scripts.

## Architecture

The engine is designed with clear separation of concerns:

### 1. Workflow Engine (`engine.rs`)
- Manages workflow registration and execution
- Maintains execution state
- Orchestrates step execution
- Emits events for monitoring

### 2. Deno Runtime (`runtime/`)
- **Worker Pool** (`pool.rs`): Manages multiple Deno isolates
- **Worker** (`worker.rs`): Individual Deno isolate for script execution
- Handles async script execution
- Provides isolation between executions

### 3. Workflow Model (`workflow.rs`)
- Workflow definition structures
- Execution state management
- Step types (Script, Set, Log)

## Design Decisions for Throughput

### Worker Pool Architecture
- Multiple Deno isolates running in separate tokio tasks
- Work stealing via shared channel (mpsc)
- Each worker maintains its own JsRuntime
- Non-blocking task distribution

### Why This Approach?
1. **Parallelism**: Multiple scripts can execute simultaneously
2. **Isolation**: Each worker has its own V8 isolate
3. **No Startup Overhead**: Workers are reused for multiple executions
4. **Scalability**: Pool size can be tuned based on workload

### Trade-offs
- Memory: Each isolate has overhead (~10-20MB)
- Optimal pool size depends on:
  - CPU cores available
  - Script execution time
  - Memory constraints
- Recommended: 2-4x CPU cores for I/O bound scripts, 1x for CPU bound

## Usage

```rust
use degov_engine::{WorkflowEngine, Workflow};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create engine with 4 workers
    let engine = WorkflowEngine::new(4).await?;

    // Define workflow
    let workflow = Workflow::from_json(r#"{
        "id": "my-workflow",
        "name": "My Workflow",
        "steps": [
            {
                "id": "step1",
                "name": "Calculate",
                "type": {
                    "script": {
                        "code": "return { result: 42 };"
                    }
                }
            }
        ]
    }"#)?;

    // Register and execute
    engine.register_workflow(workflow)?;
    let exec_id = engine.start_workflow(
        "my-workflow",
        "exec-1".to_string(),
        HashMap::new()
    ).await?;

    // Get status
    if let Some(execution) = engine.get_execution(&exec_id) {
        println!("State: {:?}", execution.state);
    }

    engine.shutdown().await;
    Ok(())
}
```

## Features

- ✅ Async workflow execution
- ✅ Worker pool with Deno isolates
- ✅ Script execution (JavaScript/TypeScript)
- ✅ Variable management across steps
- ✅ Event system for monitoring
- ✅ Parallel execution support

## Comparison with Acts

This engine is inspired by [acts](../acts) but simplified:

- **Acts**: Full-featured with YAML definitions, branches, catches, timeouts
- **degov-engine**: Focused on core execution with Deno runtime

Key differences:
- Uses Deno instead of QuickJS (better TypeScript support)
- Worker pool architecture for throughput
- Simpler step model
- JSON-based workflow definitions

## Future Enhancements

- [ ] TypeScript execution support
- [ ] Step conditions and branches
- [ ] Error handling and retries
- [ ] Workflow persistence
- [ ] Distributed execution
- [ ] Metrics and observability

