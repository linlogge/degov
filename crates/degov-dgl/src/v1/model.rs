use crate::prelude::*;

pub fn create_model_node_def() -> NodeDef {
    NodeDef::new("model")
        .with_description("Model type definition")
        .with_child(create_string_type_node_def())
        .with_child(create_integer_type_node_def())
}

fn create_string_type_node_def() -> NodeDef {
    NodeDef::new("string")
        .with_description("String type definition")
        .with_argument(ArgumentDef::new("id", ValueType::String))
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("description", PropertyDef::new(ValueType::String))
}

fn create_integer_type_node_def() -> NodeDef {
    NodeDef::new("integer")
        .with_description("Integer type definition")
        .with_argument(ArgumentDef::new("id", ValueType::String))
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("description", PropertyDef::new(ValueType::String))
}
