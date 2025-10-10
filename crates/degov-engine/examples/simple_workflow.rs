//! Simple workflow example
//!
//! This example demonstrates how to create and run a simple workflow with the degov-engine.
//! Both the engine and worker run in the same process using tokio for concurrency.
//!
//! To run this example:
//! 1. Make sure FoundationDB is running
//! 2. Run: cargo run --example simple_workflow

use degov_engine::{
    Action, RuntimeType, State, StateMachine, TaskDefinition, Transition, WorkflowDefinition,
    WorkflowEngine, WorkflowId, Worker,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 Starting workflow engine example...\n");

    // Initialize the workflow engine library
    degov_engine::init()?;

    // Connect to FoundationDB
    let db = foundationdb::Database::default()?;

    // Create the workflow engine
    let engine = Arc::new(WorkflowEngine::new(db, "127.0.0.1:8080".parse()?).await?);
    println!("✅ Workflow engine created\n");

    // Register a simple workflow
    let workflow_id = register_workflow(engine.clone()).await?;
    println!("✅ Workflow registered with ID: {}\n", workflow_id);

    // Spawn the engine server in a background task
    let engine_clone = engine.clone();
    let engine_handle = tokio::spawn(async move {
        println!("🎯 Starting engine RPC server on http://127.0.0.1:8080...");
        if let Err(e) = engine_clone.run().await {
            eprintln!("❌ Engine error: {}", e);
        }
    });

    // Wait a moment for the server to start
    sleep(Duration::from_secs(1)).await;

    // Spawn a worker in a background task
    let worker_handle = tokio::spawn(async move {
        println!("👷 Starting worker...");
        match Worker::new("http://127.0.0.1:8080").await {
            Ok(worker) => {
                println!("✅ Worker started with ID: {}\n", worker.id());
                if let Err(e) = worker.run().await {
                    eprintln!("❌ Worker error: {}", e);
                }
            }
            Err(e) => eprintln!("❌ Failed to create worker: {}", e),
        }
    });

    // Wait for worker to register
    sleep(Duration::from_secs(2)).await;

    // Start a workflow instance
    println!("🎬 Starting workflow instance...");
    let instance = engine
        .start_workflow(
            &workflow_id,
            serde_json::json!({
                "name": "Alice",
                "value": 42
            }),
        )
        .await?;

    println!("✅ Workflow instance started: {}", instance.id);
    println!("   Current state: {}", instance.current_state);
    println!("   Status: {:?}\n", instance.status);

    // Transition the workflow through states
    sleep(Duration::from_secs(3)).await;
    
    println!("⚡ Triggering transition: 'next'");
    match engine.transition_workflow(&instance.id, "next").await {
        Ok(new_state) => {
            println!("✅ Transitioned to state: {}\n", new_state);
        }
        Err(e) => {
            eprintln!("❌ Transition error: {}\n", e);
        }
    }

    sleep(Duration::from_secs(3)).await;

    println!("⚡ Triggering transition: 'done'");
    match engine.transition_workflow(&instance.id, "done").await {
        Ok(new_state) => {
            println!("✅ Transitioned to state: {}\n", new_state);
        }
        Err(e) => {
            eprintln!("❌ Transition error: {}\n", e);
        }
    }

    // Let tasks execute
    println!("⏳ Waiting for tasks to complete...");
    sleep(Duration::from_secs(5)).await;

    // Check final workflow state
    if let Ok(Some(final_instance)) = engine.persistence().workflows().get_instance(&instance.id).await {
        println!("\n📊 Final workflow state:");
        println!("   ID: {}", final_instance.id);
        println!("   Current state: {}", final_instance.current_state);
        println!("   Status: {:?}", final_instance.status);
        println!("   Context: {}", final_instance.context);
    }

    println!("\n✨ Example completed! Press Ctrl+C to exit.\n");

    // Keep running to show the system working
    tokio::select! {
        _ = engine_handle => println!("Engine task completed"),
        _ = worker_handle => println!("Worker task completed"),
        _ = tokio::signal::ctrl_c() => {
            println!("\n👋 Shutting down gracefully...");
        }
    }

    Ok(())
}

async fn register_workflow(engine: Arc<WorkflowEngine>) -> Result<WorkflowId, Box<dyn std::error::Error>> {
    // Create a state machine with three states
    let state_machine = StateMachine::builder()
        .initial_state("start")
        .add_state(
            State::new("start")
                .on_enter(Action::log("Workflow started".to_string()))
                .on_enter(Action::execute_task(TaskDefinition {
                    name: "greet".to_string(),
                    runtime_type: RuntimeType::JavaScript,
                    code: br#"
                        const name = input.name || "World";
                        const greeting = "Hello, " + name + "!";
                        console.log(greeting);
                        greeting
                    "#
                    .to_vec(),
                    timeout_ms: 5000,
                    retry_policy: None,
                }))
                .add_transition(Transition::new("next", "processing")),
        )
        .add_state(
            State::new("processing")
                .on_enter(Action::log("Processing data".to_string()))
                .on_enter(Action::execute_task(TaskDefinition {
                    name: "process".to_string(),
                    runtime_type: RuntimeType::JavaScript,
                    code: br#"
                        const value = input.value || 0;
                        const result = {
                            original: value,
                            doubled: value * 2,
                            squared: value * value,
                            processed: true,
                            timestamp: Date.now()
                        };
                        console.log("Processed:", JSON.stringify(result));
                        result
                    "#
                    .to_vec(),
                    timeout_ms: 5000,
                    retry_policy: None,
                }))
                .add_transition(Transition::new("done", "end")),
        )
        .add_state(
            State::new("end")
                .on_enter(Action::log("Workflow completed".to_string()))
        )
        .build()?;

    let workflow_def = WorkflowDefinition {
        id: WorkflowId::new(),
        name: "Simple Demo Workflow".to_string(),
        description: Some("A demonstration workflow with greeting and data processing".to_string()),
        state_machine,
        created_at: chrono::Utc::now(),
    };

    let workflow_id = engine.register_workflow(workflow_def).await?;
    Ok(workflow_id)
}
