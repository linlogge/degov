//! Production-grade workflow engine with state machine-based workflows
//!
//! This crate provides a distributed workflow engine with:
//! - User-definable state machines with transitions, guards, and actions
//! - Worker coordination over RPC with round-robin scheduling
//! - FoundationDB persistence with transactional guarantees
//! - JavaScript (rquickjs) and WASM (wasmtime) task execution
//! - Failure recovery and fault tolerance
//!
//! # Example
//!
//! ```no_run
//! use degov_engine::{WorkflowEngine, Worker, StateMachine, State, Transition, RuntimeType, TaskDefinition};
//! use foundationdb::Database;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize FoundationDB
//!     foundationdb::boot().await;
//!     let db = Database::default()?;
//!
//!     // Create a simple workflow
//!     let state_machine = StateMachine::builder()
//!         .initial_state("start")
//!         .add_state(
//!             State::new("start")
//!                 .add_transition(Transition::new("begin", "processing"))
//!         )
//!         .add_state(
//!             State::new("processing")
//!                 .add_transition(Transition::new("complete", "end"))
//!         )
//!         .add_state(State::new("end"))
//!         .build()?;
//!
//!     // Start the engine
//!     let engine = WorkflowEngine::new(db, "127.0.0.1:8080".parse()?).await?;
//!     
//!     // Start a worker
//!     let worker = Worker::new("http://127.0.0.1:8080").await?;
//!     
//!     Ok(())
//! }
//! ```

// Core modules
pub mod engine;
pub mod error;
pub mod persistence;
pub mod runtime;
pub mod state_machine;
pub mod types;
pub mod worker;

// Re-exports for public API
pub use engine::{TaskScheduler, WorkflowEngine, WorkflowRegistry};
pub use error::{
    EngineError, PersistenceError, Result, RpcError, RuntimeError, WorkflowError, WorkflowResult,
};
pub use persistence::PersistenceLayer;
pub use runtime::{JavaScriptRuntime, Runtime, WasmRuntime};
pub use state_machine::{Action, Context, Guard, State, StateMachine, Transition};
pub use types::{
    RetryPolicy, RuntimeType, TaskDefinition, TaskExecution, TaskId, TaskResult, TaskStatus,
    WorkerHealthStatus, WorkerInfo, WorkerId, WorkerStats, WorkflowDefinition, WorkflowId,
    WorkflowInstance, WorkflowStatus,
};
pub use worker::{TaskExecutor, Worker};

// Re-export foundationdb for convenience
pub use foundationdb;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
