# Degov Workflow Engine

A production-grade distributed workflow engine with state machine-based workflows, worker coordination over RPC, and FoundationDB persistence with transactional guarantees.

## Features

- **State Machine Workflows**: User-definable states with transitions, guards, and actions
- **Distributed Execution**: Worker coordination with round-robin task scheduling
- **Fault Tolerance**: FoundationDB persistence with ACID transactions for crash recovery
- **Multiple Runtimes**: JavaScript (rquickjs) and WASM (wasmtime) task execution
- **RPC Communication**: Custom Connect-RPC protocol for worker-engine communication
- **Extensible Design**: Trait-based runtime system for adding new execution environments

## Architecture

```
┌─────────────────┐
│  WorkflowEngine │  ← Manages workflows, schedules tasks
└────────┬────────┘
         │ RPC
    ┌────┴─────┬─────────┐
    │          │         │
┌───▼────┐ ┌──▼─────┐ ┌─▼──────┐
│ Worker │ │ Worker │ │ Worker │  ← Execute tasks (JS/WASM)
└────────┘ └────────┘ └────────┘
         │
    ┌────▼──────────┐
    │ FoundationDB  │  ← Persistent state with transactions
    └───────────────┘
```

## Components

### 1. WorkflowEngine

Central coordinator that:
- Registers workflow definitions
- Starts workflow instances
- Manages worker registration and heartbeats
- Schedules tasks to workers (round-robin)
- Handles workflow state transitions

### 2. Worker

Task executor that:
- Connects to engine via RPC
- Polls for pending tasks
- Executes JavaScript and WASM code
- Reports task completion
- Sends periodic heartbeats

### 3. State Machine

User-definable workflow logic with:
- Named states with enter/exit actions
- Event-triggered transitions
- Optional guard conditions
- Support for sequential and branching flows

### 4. Persistence Layer

FoundationDB-backed storage with:
- Atomic task enqueue/dequeue
- Workflow state persistence
- Worker registration tracking
- Transactional consistency guarantees

## Usage

### Initialize the Engine

```rust
use degov_engine::{WorkflowEngine, foundationdb};
use std::sync::Arc;

// Initialize FoundationDB
degov_engine::init()?;
let db = foundationdb::Database::default()?;

// Create engine
let engine = Arc::new(
    WorkflowEngine::new(db, "127.0.0.1:8080".parse()?).await?
);
```

### Define a Workflow

```rust
use degov_engine::{
    StateMachine, State, Transition, Action, TaskDefinition, RuntimeType
};

let state_machine = StateMachine::builder()
    .initial_state("start")
    .add_state(
        State::new("start")
            .on_enter(Action::execute_task(TaskDefinition {
                name: "process_data".to_string(),
                runtime_type: RuntimeType::JavaScript,
                code: b"input.value * 2".to_vec(),
                timeout_ms: 5000,
                retry_policy: None,
            }))
            .add_transition(Transition::new("complete", "end"))
    )
    .add_state(State::new("end"))
    .build()?;
```

### Register and Start Workflow

```rust
use degov_engine::{WorkflowDefinition, WorkflowId};
use chrono::Utc;

let workflow_def = WorkflowDefinition {
    id: WorkflowId::new(),
    name: "Data Processing".to_string(),
    description: Some("Process data workflow".to_string()),
    state_machine,
    created_at: Utc::now(),
};

let workflow_id = engine.register_workflow(workflow_def).await?;

let instance = engine
    .start_workflow(&workflow_id, serde_json::json!({"value": 42}))
    .await?;
```

### Start a Worker

```rust
use degov_engine::Worker;

let worker = Worker::new("http://127.0.0.1:8080").await?;
worker.run().await?;
```

## State Machine Features

### Guards

Add conditions to transitions:

```rust
use degov_engine::Guard;

let transition = Transition::new("approve", "approved")
    .with_guard(Guard::new(|ctx| {
        ctx.get("amount")
            .and_then(|v| v.as_f64())
            .map(|amount| amount < 1000.0)
            .unwrap_or(false)
    }));
```

### Actions

Execute logic on state enter/exit:

```rust
State::new("processing")
    .on_enter(Action::set_data("status", json!("started")))
    .on_enter(Action::log("Processing started"))
    .on_exit(Action::log("Processing completed"))
```

## Task Runtimes

### JavaScript (rquickjs)

```rust
TaskDefinition {
    name: "calculate".to_string(),
    runtime_type: RuntimeType::JavaScript,
    code: br#"
        const result = input.a + input.b;
        result
    "#.to_vec(),
    timeout_ms: 5000,
    retry_policy: None,
}
```

### WASM (wasmtime)

```rust
TaskDefinition {
    name: "process".to_string(),
    runtime_type: RuntimeType::Wasm,
    code: wasm_module_bytes,
    timeout_ms: 10000,
    retry_policy: Some(RetryPolicy::default()),
}
```

## Fault Tolerance

### Engine Crashes
- Workflow states persisted in FoundationDB
- On restart, reload from database
- Reschedule orphaned tasks

### Worker Crashes
- Heartbeat monitoring detects failures
- Tasks reassigned via atomic FDB operations
- No duplicate execution

### Network Partitions
- RPC retry with exponential backoff
- Task dequeue is transactional
- State consistency maintained

## Configuration

### Worker Settings

```rust
let worker = Worker::new("http://engine:8080")
    .await?
    .with_poll_interval(Duration::from_millis(100))
    .with_heartbeat_interval(Duration::from_secs(5));
```

### Retry Policies

```rust
use degov_engine::RetryPolicy;

let retry = RetryPolicy {
    max_attempts: 3,
    initial_delay_ms: 1000,
    max_delay_ms: 60000,
    backoff_multiplier: 2.0,
};
```

## Quick Start

Run the complete example (engine + worker in one process):

```bash
# Make sure FoundationDB is running
cargo run --example simple_workflow
```

This will:
1. Start the workflow engine on `http://127.0.0.1:8080`
2. Spawn a worker that connects to the engine
3. Register a simple workflow with three states
4. Execute the workflow with JavaScript tasks
5. Show real-time progress and state transitions

## Examples

See `examples/` directory for complete examples:
- `simple_workflow.rs` - Complete workflow with engine and worker running together
- More examples coming soon

## License

See workspace LICENSE file.


