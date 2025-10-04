use degov_engine::{Step, StepType, Workflow, WorkflowEngine};
use std::collections::HashMap;

#[tokio::test]
async fn test_simple_workflow() {
    // Initialize tracing for tests
    let _ = tracing_subscriber::fmt::try_init();

    // Create a workflow engine with 2 workers
    let engine = WorkflowEngine::new(2).await.unwrap();

    // Create a workflow
    let mut workflow = Workflow::new("test-workflow".to_string(), "Test Workflow".to_string());

    // Add a set step
    let mut set_params = HashMap::new();
    set_params.insert("x".to_string(), serde_json::json!(10));
    set_params.insert("y".to_string(), serde_json::json!(20));

    workflow.steps.push(Step {
        id: "step1".to_string(),
        name: "Set Variables".to_string(),
        step_type: StepType::Set,
        params: set_params,
    });

    // Add a log step
    workflow.steps.push(Step {
        id: "step2".to_string(),
        name: "Log Variables".to_string(),
        step_type: StepType::Log,
        params: HashMap::new(),
    });

    // Register the workflow
    engine.register_workflow(workflow).unwrap();

    // Start execution
    let execution_id = engine
        .start_workflow("test-workflow", "exec-1".to_string(), HashMap::new())
        .await
        .unwrap();

    // Wait for completion
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Check execution state
    let execution = engine.get_execution(&execution_id).unwrap();
    assert_eq!(execution.state, degov_engine::ExecutionState::Completed);
    assert_eq!(execution.variables.get("x"), Some(&serde_json::json!(10)));
    assert_eq!(execution.variables.get("y"), Some(&serde_json::json!(20)));

    engine.shutdown().await;
}

#[tokio::test]
async fn test_script_execution() {
    let _ = tracing_subscriber::fmt::try_init();

    let engine = WorkflowEngine::new(2).await.unwrap();

    let mut workflow = Workflow::new("script-workflow".to_string(), "Script Workflow".to_string());

    // Add a set step
    let mut set_params = HashMap::new();
    set_params.insert("a".to_string(), serde_json::json!(5));
    set_params.insert("b".to_string(), serde_json::json!(15));

    workflow.steps.push(Step {
        id: "step1".to_string(),
        name: "Set Variables".to_string(),
        step_type: StepType::Set,
        params: set_params,
    });

    // Add a script step
    workflow.steps.push(Step {
        id: "step2".to_string(),
        name: "Calculate Sum".to_string(),
        step_type: StepType::Script {
            code: "const result = context.a + context.b; console.log('Sum:', result);".to_string(),
        },
        params: HashMap::new(),
    });

    engine.register_workflow(workflow).unwrap();

    let execution_id = engine
        .start_workflow("script-workflow", "exec-2".to_string(), HashMap::new())
        .await
        .unwrap();

    // Wait for completion
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let execution = engine.get_execution(&execution_id).unwrap();
    assert_eq!(execution.state, degov_engine::ExecutionState::Completed);

    engine.shutdown().await;
}

