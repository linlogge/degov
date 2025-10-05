use degov_engine::*;
use foundationdb as fdb;
use std::collections::HashMap;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting distributed workflow example");

    // Initialize FoundationDB
    let _network = unsafe { fdb::boot() };
    let db = fdb::Database::default()?;

    // Create workflow engine with 4 workers
    let engine = WorkflowEngine::new(db, 4).await?;

    // Define an order processing workflow
    let workflow = create_order_workflow();
    engine.register_workflow(workflow).await?;

    info!("Registered order processing workflow");

    // Start worker
    engine.start_worker().await?;

    info!("Started worker: {}", engine.worker_id());

    // Create multiple workflow instances to demonstrate parallel execution
    let mut instances = Vec::new();

    for i in 1..=3 {
        let instance_id = engine
            .create_instance(
                "order-processing",
                None, // Let engine generate unique ID
                serde_json::json!({
                    "order_id": format!("ORD-{:03}", i),
                    "customer_id": format!("CUST-{}", i),
                    "items": [
                        {"sku": "ITEM-001", "quantity": 2, "price": 29.99},
                        {"sku": "ITEM-002", "quantity": 1, "price": 49.99}
                    ],
                    "total": 109.97
                }),
            )
            .await?;

        instances.push(instance_id.clone());
        info!("Created workflow instance: {}", instance_id);
    }

    // Trigger events to move workflows through states
    for instance_id in &instances {
        info!("Triggering payment_received event for {}", instance_id);
        engine
            .trigger_event(
                instance_id,
                "payment_received",
                Some(serde_json::json!({
                    "payment_method": "credit_card",
                    "transaction_id": "TXN-12345"
                })),
            )
            .await?;
    }

    // Trigger fulfillment
    for instance_id in &instances {
        info!("Triggering items_shipped event for {}", instance_id);
        engine
            .trigger_event(
                instance_id,
                "items_shipped",
                Some(serde_json::json!({
                    "tracking_number": "TRACK-98765",
                    "carrier": "FedEx"
                })),
            )
            .await?;
    }

    // Mark as delivered
    for instance_id in &instances {
        info!("Triggering order_delivered event for {}", instance_id);
        engine
            .trigger_event(
                instance_id,
                "order_delivered",
                Some(serde_json::json!({
                    "delivered_at": chrono::Utc::now().to_rfc3339()
                })),
            )
            .await?;
    }

    // Check final state
    for instance_id in &instances {
        if let Some(instance) = engine.get_instance(instance_id).await? {
            info!(
                "Instance {} - State: {}, Status: {:?}",
                instance_id, instance.current_state, instance.status
            );

            // Get events
            let events = engine.get_events(instance_id).await?;
            info!("Instance {} has {} events", instance_id, events.len());

            for event in events.iter().take(5) {
                info!(
                    "  Event: {:?} at {}",
                    event.event_type, event.timestamp
                );
            }
        }
    }

    // Demonstrate pause/resume with a new instance
    info!("\nDemonstrating pause/resume functionality");
    let pause_instance_id = engine
        .create_instance(
            "order-processing",
            None, // Let engine generate unique ID
            serde_json::json!({
                "order_id": "ORD-PAUSE",
                "customer_id": "CUST-PAUSE",
                "items": [{"sku": "ITEM-001", "quantity": 1, "price": 10.00}],
                "total": 10.00
            }),
        )
        .await?;
    info!("Created instance for pause test: {}", pause_instance_id);

    // Let it start, then pause 
    
    engine.pause_instance(&pause_instance_id).await?;
    info!("Paused instance: {}", pause_instance_id);

    if let Some(instance) = engine.get_instance(&pause_instance_id).await? {
        info!("Instance status after pause: {:?}", instance.status);
    }

    engine.resume_instance(&pause_instance_id).await?;
    info!("Resumed instance: {}", pause_instance_id);

    if let Some(instance) = engine.get_instance(&pause_instance_id).await? {
        info!("Instance status after resume: {:?}", instance.status);
    }

    // Demonstrate cancellation with a new instance
    info!("\nDemonstrating cancellation");
    let cancel_instance_id = engine
        .create_instance(
            "order-processing",
            None, // Let engine generate unique ID
            serde_json::json!({
                "order_id": "ORD-CANCEL",
                "customer_id": "CUST-CANCEL",
                "items": [{"sku": "ITEM-001", "quantity": 1, "price": 10.00}],
                "total": 10.00
            }),
        )
        .await?;
    
    engine.cancel_instance(&cancel_instance_id).await?;
    info!("Cancelled instance: {}", cancel_instance_id);

    if let Some(instance) = engine.get_instance(&cancel_instance_id).await? {
        info!("Instance status after cancellation: {:?}", instance.status);
    }

    // Wait a bit more to ensure all tasks are processed
    tokio::time::sleep(tokio::time::Duration::from_secs(100)).await;

    // Graceful shutdown
    info!("\nShutting down workflow engine");
    engine.shutdown().await?;

    info!("Example completed successfully!");

    Ok(())
}

fn create_order_workflow() -> WorkflowDefinition {
    let now = chrono::Utc::now().timestamp_millis();

    let mut states = HashMap::new();

    // Define states
    states.insert(
        "created".to_string(),
        StateDefinition {
            name: "created".to_string(),
            is_terminal: false,
            on_enter: Some(Action::Script {
                code: r#"
                    export default function(context) {
                        console.log("Order created:", context.order_id);
                        return { validated: true };
                    }
                "#
                .to_string(),
                language: "javascript".to_string(),
            }),
            on_exit: None,
            timeout_seconds: Some(300),
        },
    );

    states.insert(
        "payment_processing".to_string(),
        StateDefinition {
            name: "payment_processing".to_string(),
            is_terminal: false,
            on_enter: Some(Action::Script {
                code: r#"
                    export default function(context) {
                        console.log("Processing payment for order:", context.order_id);
                        console.log("Total amount:", context.total);
                        return { payment_status: "processing" };
                    }
                "#
                .to_string(),
                language: "javascript".to_string(),
            }),
            on_exit: None,
            timeout_seconds: Some(600),
        },
    );

    states.insert(
        "fulfillment".to_string(),
        StateDefinition {
            name: "fulfillment".to_string(),
            is_terminal: false,
            on_enter: Some(Action::Script {
                code: r#"
                    export default function(context) {
                        console.log("Fulfilling order:", context.order_id);
                        console.log("Tracking:", context.tracking_number);
                        return { fulfillment_status: "in_progress" };
                    }
                "#
                .to_string(),
                language: "javascript".to_string(),
            }),
            on_exit: None,
            timeout_seconds: Some(86400),
        },
    );

    states.insert(
        "completed".to_string(),
        StateDefinition {
            name: "completed".to_string(),
            is_terminal: true,
            on_enter: Some(Action::Script {
                code: r#"
                    export default function(context) {
                        console.log("Order completed:", context.order_id);
                        return { completed_at: new Date().toISOString() };
                    }
                "#
                .to_string(),
                language: "javascript".to_string(),
            }),
            on_exit: None,
            timeout_seconds: None,
        },
    );

    states.insert(
        "cancelled".to_string(),
        StateDefinition {
            name: "cancelled".to_string(),
            is_terminal: true,
            on_enter: Some(Action::Script {
                code: r#"
                    export default function(context) {
                        console.log("Order cancelled:", context.order_id);
                        return { cancelled_at: new Date().toISOString() };
                    }
                "#
                .to_string(),
                language: "javascript".to_string(),
            }),
            on_exit: None,
            timeout_seconds: None,
        },
    );

    // Define transitions
    let transitions = vec![
        Transition {
            from: "created".to_string(),
            to: "payment_processing".to_string(),
            event: "payment_received".to_string(),
            condition: Some("context.total > 0".to_string()),
            action: Some(Action::Script {
                code: r#"
                    export default function(context) {
                        console.log("Validating payment for:", context.total);
                        return { payment_validated: true };
                    }
                "#
                .to_string(),
                language: "javascript".to_string(),
            }),
            compensation: Some(Action::Script {
                code: r#"
                    export default function(context) {
                        console.log("Refunding payment for order:", context.order_id);
                        return { refunded: true };
                    }
                "#
                .to_string(),
                language: "javascript".to_string(),
            }),
        },
        Transition {
            from: "payment_processing".to_string(),
            to: "fulfillment".to_string(),
            event: "items_shipped".to_string(),
            condition: None,
            action: Some(Action::Script {
                code: r#"
                    export default function(context) {
                        console.log("Shipping items for order:", context.order_id);
                        return { shipped: true };
                    }
                "#
                .to_string(),
                language: "javascript".to_string(),
            }),
            compensation: None,
        },
        Transition {
            from: "fulfillment".to_string(),
            to: "completed".to_string(),
            event: "order_delivered".to_string(),
            condition: None,
            action: Some(Action::Script {
                code: r#"
                    export default function(context) {
                        console.log("Order delivered:", context.order_id);
                        return { delivery_confirmed: true };
                    }
                "#
                .to_string(),
                language: "javascript".to_string(),
            }),
            compensation: None,
        },
        Transition {
            from: "created".to_string(),
            to: "cancelled".to_string(),
            event: "cancel".to_string(),
            condition: None,
            action: None,
            compensation: None,
        },
        Transition {
            from: "payment_processing".to_string(),
            to: "cancelled".to_string(),
            event: "cancel".to_string(),
            condition: None,
            action: None,
            compensation: None,
        },
    ];

    WorkflowDefinition {
        id: "order-processing".to_string(),
        name: "Order Processing Workflow".to_string(),
        version: 1,
        initial_state: "created".to_string(),
        states,
        transitions,
        created_at: now,
    }
}
