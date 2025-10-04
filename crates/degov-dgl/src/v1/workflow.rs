use crate::prelude::*;

pub fn create_workflow_node_def() -> NodeDef {
    NodeDef::new("workflow")
        .with_description("Workflow type definition")
        .with_child(create_step_node_def())
}

fn create_step_node_def() -> NodeDef {
    NodeDef::new("step")
        .with_description("Step type definition")
        .with_argument(ArgumentDef::new("name", ValueType::String))
        .with_property("description", PropertyDef::new(ValueType::String))
        .with_child(create_step_script_node_def())
}

fn create_step_script_node_def() -> NodeDef {
    NodeDef::new("script")
        .with_description("Script type definition")
        .with_property("path", PropertyDef::new(ValueType::String))
}
