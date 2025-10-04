/// Example of using the workflow engine with database integration
use degov_engine::{Step, StepType, Workflow, WorkflowEngine};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create workflow engine with 2 worker threads
    let engine = WorkflowEngine::new(2).await?;

    println!("=== Database Integration Example ===\n");

    // Create a workflow that uses the database
    let mut workflow = Workflow::new(
        "db-workflow".to_string(),
        "Database Workflow".to_string(),
    );

    // Step 1: Store data in database
    workflow.steps.push(Step {
        id: "step1".to_string(),
        name: "Store in Database".to_string(),
        step_type: StepType::Script {
            code: r#"
                export default async function(ctx) {
                    const db = KV.openKv();
                    // Store some data in the database (all ops are async)
                    await db.set("user:1", { name: "Alice", age: 30 });
                    await db.set("user:2", { name: "Bob", age: 25 });
                    await db.set("counter", 0);
                    
                    console.log("Stored users in database");
                }   
            "#
            .to_string(),
        },
        params: HashMap::new(),
    });

    // Step 2: Read and modify data
    workflow.steps.push(Step {
        id: "step2".to_string(),
        name: "Read and Update".to_string(),
        step_type: StepType::Script {
            code: r#"
                // Read data from database (async)
                export default async function(ctx) {
                    const db = KV.openKv();
                    const user1 = await db.get("user:1");
                    console.log("User 1:", JSON.stringify(user1));
                }
            "#
            .to_string(),
        },
        params: HashMap::new(),
    });

    // Register the workflow
    engine.register_workflow(workflow)?;

    // Start workflow execution
    let execution_id = engine
        .start_workflow("db-workflow", "exec-db-1".to_string(), HashMap::new())
        .await?;

    println!("\nStarted workflow execution: {}\n", execution_id);

    // Wait for completion
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Get execution status
    if let Some(execution) = engine.get_execution(&execution_id) {
        println!("\n=== Execution Complete ===");
        println!("State: {:?}", execution.state);
        println!("Steps completed: {}", execution.current_step);
    }

    // Shutdown engine
    engine.shutdown().await;

    Ok(())
}

