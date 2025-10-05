# degov-engine

A production-grade distributed workflow engine built on FoundationDB with horizontal scaling capabilities.

## Overview

This workflow engine provides a robust foundation for building distributed, stateful applications with:

- **Atomic State Transitions**: Leverages FoundationDB's ACID transactions for consistency
- **Horizontal Scaling**: Multiple workers can process tasks concurrently
- **Distributed Locking**: Ensures only one executor per workflow instance at a time
- **Task Queue with Leases**: Prevents duplicate execution with timeout-based leases
- **Idempotency**: Safe to retry any action without side effects
- **Event Logging**: Complete audit trail of all workflow operations
- **Graceful Failure Handling**: Dead letter queue for permanently failed tasks

## Architecture

### Core Components

#### 1. Workflow Definition (`model.rs`)
- States and transitions
- Actions (Script, Task, HTTP, Delay)
- Conditions for transitions
- Compensation actions for rollback

#### 2. Storage Layer (`storage.rs`)
- Key-value schema in FoundationDB
- Optimistic locking with versionstamps
- Priority-based task queue
- Worker registration and health checks
- Event log (append-only)

#### 3. Engine (`engine.rs`)
- Workflow registration and validation
- Instance lifecycle management (create, pause, resume, cancel)
- State transition orchestration
- Task execution coordination

#### 4. Runtime (`runtime/`)
- Deno/JavaScript execution environment
- Worker pool for script execution
- Sandboxed execution with timeouts

## Key Design Principles

### 1. One Transaction = One State Transition
Every state transition happens within a single FoundationDB transaction, ensuring atomicity.

```rust
// Atomically transition state and create tasks
self.storage.update_instance_state(instance_id, |instance| {
    instance.current_state = new_state;
    Ok(())
}).await?;
```

### 2. Task Claiming with Leases
Workers claim tasks with a lease timeout (default 30s). If a worker crashes, the lease expires and another worker can reclaim the task.

```rust
pub struct TaskLease {
    pub worker_id: WorkerId,
    pub claimed_at: i64,
    pub expires_at: i64,
    pub heartbeat_at: i64,
}
```

### 3. Idempotent Operations
Every task has an idempotency key. If a task is retried, the stored result is returned instead of re-executing.

```rust
// Check idempotency before execution
if let Some(result) = storage.get_task_result(&task.idempotency_key).await? {
    return Ok(()); // Already executed
}
```

### 4. No Shared Mutable State
All coordination happens through FoundationDB. Workers are stateless and can be added/removed dynamically.

### 5. Distributed Locking
Workflow instances can be locked to ensure exclusive processing:

```rust
// Try to acquire lock with TTL
if storage.try_lock_instance(instance_id, worker_id, ttl_ms).await? {
    // Process the instance
    storage.unlock_instance(instance_id, worker_id).await?;
}
```

## Data Model

### FDB Schema

```
/degov/workflow/
  workflows/{workflow_id}                          -> WorkflowDefinition
  instances/{instance_id}                          -> InstanceState
  instance_index/{workflow_id}/{instance_id}       -> empty (for listing)
  tasks/{priority}/{scheduled_at}/{task_id}        -> Task (ordered queue)
  task_by_id/{task_id}                             -> Task
  task_idempotency/{idempotency_key}               -> TaskResult
  workers/{worker_id}                              -> Worker
  events/{instance_id}/{timestamp}/{event_id}      -> EventLog
  locks/{instance_id}                              -> (WorkerId, expires_at)
```

### State Machine

```
WorkflowDefinition
├── States (HashMap<String, StateDefinition>)
│   ├── on_enter: Option<Action>
│   ├── on_exit: Option<Action>
│   └── is_terminal: bool
└── Transitions (Vec<Transition>)
    ├── from/to states
    ├── event trigger
    ├── condition (JavaScript)
    ├── action
    └── compensation (for rollback)
```

## Usage

### 1. Define a Workflow

```rust
use degov_engine::*;
use std::collections::HashMap;

let mut states = HashMap::new();

states.insert("created".to_string(), StateDefinition {
    name: "created".to_string(),
    is_terminal: false,
    on_enter: Some(Action::Script {
        code: r#"
            export default function(context) {
                console.log("Order created:", context.order_id);
                return { validated: true };
            }
        "#.to_string(),
        language: "javascript".to_string(),
    }),
    on_exit: None,
    timeout_seconds: Some(300),
});

states.insert("completed".to_string(), StateDefinition {
    name: "completed".to_string(),
    is_terminal: true,
    on_enter: None,
    on_exit: None,
    timeout_seconds: None,
});

let transitions = vec![
    Transition {
        from: "created".to_string(),
        to: "completed".to_string(),
        event: "complete".to_string(),
        condition: Some("context.validated === true".to_string()),
        action: None,
        compensation: None,
    },
];

let workflow = WorkflowDefinition {
    id: "order-workflow".to_string(),
    name: "Order Workflow".to_string(),
    version: 1,
    initial_state: "created".to_string(),
    states,
    transitions,
    created_at: chrono::Utc::now().timestamp_millis(),
};
```

### 2. Initialize Engine

```rust
use foundationdb as fdb;

let _network = unsafe { fdb::boot() };
let db = fdb::Database::default()?;

// Create engine with 4 worker threads
let engine = WorkflowEngine::new(db, 4).await?;

// Register workflow
engine.register_workflow(workflow).await?;

// Start worker
engine.start_worker().await?;
```

### 3. Create and Execute Instances

```rust
// Create workflow instance
let instance_id = engine.create_instance(
    "order-workflow",
    Some("order-123".to_string()),
    serde_json::json!({
        "order_id": "ORD-123",
        "customer_id": "CUST-456",
        "total": 99.99
    }),
).await?;

// Trigger event to move workflow forward
engine.trigger_event(
    &instance_id,
    "complete",
    Some(serde_json::json!({
        "payment_confirmed": true
    })),
).await?;

// Check instance status
if let Some(instance) = engine.get_instance(&instance_id).await? {
    println!("Status: {:?}", instance.status);
    println!("Current state: {}", instance.current_state);
}
```

### 4. Lifecycle Management

```rust
// Pause instance
engine.pause_instance(&instance_id).await?;

// Resume instance
engine.resume_instance(&instance_id).await?;

// Cancel instance
engine.cancel_instance(&instance_id).await?;

// Get event log
let events = engine.get_events(&instance_id).await?;
for event in events {
    println!("{:?}: {:?}", event.timestamp, event.event_type);
}
```

### 5. Graceful Shutdown

```rust
// Shutdown worker and runtime
engine.shutdown().await?;
```

## Action Types

### 1. Script Action
Execute JavaScript/TypeScript code with context:

```rust
Action::Script {
    code: r#"
        export default function(context) {
            return { result: context.value * 2 };
        }
    "#.to_string(),
    language: "javascript".to_string(),
}
```

### 2. Task Action
Custom task handler (extensible):

```rust
Action::Task {
    task_type: "send_email".to_string(),
    payload: serde_json::json!({
        "to": "user@example.com",
        "subject": "Order confirmation"
    }),
}
```

### 3. HTTP Action
Make HTTP requests (placeholder for future implementation):

```rust
Action::Http {
    url: "https://api.example.com/webhook".to_string(),
    method: "POST".to_string(),
    headers: headers_map,
    body: Some(serde_json::json!({"event": "order_created"})),
}
```

### 4. Delay Action
Schedule delayed execution:

```rust
Action::Delay {
    seconds: 3600, // 1 hour delay
}
```

## Scaling

### Horizontal Scaling

Multiple workers can run on different machines:

```rust
// Worker 1
let engine1 = WorkflowEngine::new(db.clone(), 4).await?;
engine1.start_worker().await?;

// Worker 2 (different machine)
let engine2 = WorkflowEngine::new(db.clone(), 4).await?;
engine2.start_worker().await?;
```

Workers automatically coordinate through FDB:
- Task claiming with leases prevents duplicate work
- Worker health checks via heartbeat timestamps
- Failed workers are detected when heartbeats expire

### Task Priority

Higher priority tasks are processed first:

```rust
task.priority = 10; // Higher values = higher priority
```

Tasks are stored with `-priority` as part of the key, ensuring high-priority tasks are claimed first.

## Error Handling

### Retry Logic

Tasks are automatically retried on failure:

```rust
pub struct Task {
    pub retry_count: u32,
    pub max_retries: u32, // Default: 3
    // ...
}
```

After `max_retries`, tasks move to the dead letter queue for manual intervention.

### Dead Letter Queue

Failed tasks that exceed retries are marked as `DeadLetter`:

```rust
if task.retry_count >= task.max_retries {
    task.status = TaskStatus::DeadLetter;
}
```

### Compensation/Rollback

Transitions can have compensation actions for rollback:

```rust
Transition {
    from: "payment_processing".to_string(),
    to: "cancelled".to_string(),
    event: "cancel".to_string(),
    compensation: Some(Action::Script {
        code: "export default function(ctx) { /* refund */ }".to_string(),
        language: "javascript".to_string(),
    }),
    // ...
}
```

## Testing

Run the distributed workflow example:

```bash
cargo run --example distributed_workflow --release
```

This demonstrates:
- Multiple workflow instances executing in parallel
- State transitions with actions
- Pause/resume functionality
- Cancellation
- Event logging
- Worker coordination

## Performance Considerations

### Transaction Limits
- FDB transactions have a 5-second default timeout
- Keep state transitions atomic and fast
- Batch operations when possible

### Worker Pool Size
- Recommended: 2-4x CPU cores for I/O-bound tasks
- 1x CPU cores for CPU-bound tasks
- Each Deno isolate uses ~10-20MB memory

### Task Queue Optimization
- Priority ordering ensures high-priority work is done first
- Lease timeouts prevent stuck tasks
- Task claiming is O(log n) due to ordered keys

## Future Enhancements

- [ ] HTTP action implementation
- [ ] Workflow versioning and migration
- [ ] Metrics and observability (Prometheus)
- [ ] Workflow composition (sub-workflows)
- [ ] Conditional branching (parallel/choice states)
- [ ] Scheduled/cron workflows
- [ ] Saga pattern for distributed transactions
- [ ] GraphQL API for workflow management

## License

Part of the DeGov project.