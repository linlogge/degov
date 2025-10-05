//! DeGov Workflow Engine
//!
//! A distributed workflow engine built on FoundationDB with support for:
//! - Atomic state transitions with ACID guarantees
//! - Distributed task execution with horizontal scaling
//! - Worker coordination with lease-based locking
//! - Event logging for audit trails
//! - Idempotent task execution
//! - Compensation/rollback support

mod engine;
mod error;
mod executers;
mod model;
mod storage;

pub use engine::WorkflowEngine;
pub use error::{EngineError, Result};
pub use executers::deno::DenoRuntime;
pub use model::*;
