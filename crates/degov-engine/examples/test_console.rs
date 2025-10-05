use degov_engine::*;
use foundationdb as fdb;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Testing console.log output ===\n");

    // Initialize FoundationDB
    let _network = unsafe { fdb::boot() };
    let db = fdb::Database::default()?;

    // Create workflow engine
    let engine = WorkflowEngine::new(db, 2).await?;

    // Create a simple workflow with console.log
    let mut states = HashMap::new();
    states.insert("start".to_string(), StateDefinition {
        name: "start".to_string(),
        is_terminal: false,
        on_enter: Some(Action::Script {
            code: r#"
                export default function(context) {
                    console.log("=== CONSOLE.LOG FROM JAVASCRIPT ===");
                    console.log("Context:", JSON.stringify(context));
                    console.log("This should appear in the terminal!");
                    return { processed: true };
                }
            "#.to_string(),
            language: "javascript".to_string(),
        }),
        on_exit: None,
        timeout_seconds: None,
    });

    states.insert("end".to_string(), StateDefinition {
        name: "end".to_string(),
        is_terminal: true,
        on_enter: None,
        on_exit: None,
        timeout_seconds: None,
    });

    let transitions = vec![
        Transition {
            from: "start".to_string(),
            to: "end".to_string(),
            event: "finish".to_string(),
            condition: None,
            action: None,
            compensation: None,
        },
    ];

    let workflow = WorkflowDefinition {
        id: "test-console".to_string(),
        name: "Console Test".to_string(),
        version: 1,
        initial_state: "start".to_string(),
        states,
        transitions,
        created_at: chrono::Utc::now().timestamp_millis(),
    };

    engine.register_workflow(workflow).await?;
    println!("Registered workflow\n");

    // Start worker
    engine.start_worker().await?;
    println!("Started worker\n");

    // Create instance
    let instance_id = engine.create_instance(
        "test-console",
        None,
        serde_json::json!({
            "test": "data",
            "number": 42
        }),
    ).await?;

    println!("Created instance: {}\n", instance_id);
    println!("Waiting for task execution...\n");

    // Wait for task to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    println!("\n=== Test complete ===");

    engine.shutdown().await?;
    Ok(())
}
