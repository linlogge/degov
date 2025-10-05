use crate::prelude::*;

pub fn create_workflow_node_def() -> NodeDef {
    NodeDef::new("workflow")
        .with_description("Workflow type definition")
        .with_child(create_states_node_def())
        .with_child(create_transitions_node_def())
}

fn create_states_node_def() -> NodeDef {
    NodeDef::new("states")
        .with_description("States available in the workflow")
        .with_child(create_state_node_def())
}

fn create_state_node_def() -> NodeDef {
    NodeDef::new("state")
        .with_description("State available in the workflow")
        .with_argument(ArgumentDef::new("name", ValueType::String))
        .with_property("description", PropertyDef::new(ValueType::String))
        .with_property("type", PropertyDef::new(ValueType::String))
}

fn create_transitions_node_def() -> NodeDef {
    NodeDef::new("transitions")
        .with_description("Transitions available in the workflow")
        .with_child(create_transition_node_def())
}

fn create_transition_node_def() -> NodeDef {
    NodeDef::new("transition")
        .with_description("Transition available in the workflow")
        .with_argument(ArgumentDef::new("name", ValueType::String))
        .with_property("description", PropertyDef::new(ValueType::String))
        .with_property("from", PropertyDef::new(ValueType::String))
        .with_property("to", PropertyDef::new(ValueType::String))
}
