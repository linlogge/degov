//! DeGov DGL v1 Schema Implementation
//!
//! This module provides the complete schema definition for the DeGov DGL v1,
//! supporting DataModel, Service, Workflow, Permission, and Credential definitions.
use crate::validation::create_nsid_validator;

use crate::prelude::*;

/// Create the complete DeGov DGL v1 schema
pub fn create_schema() -> Schema {
    let kind_enum = EnumDef::new(vec![
        "DataModel".to_string(),
        "Service".to_string(),
        "Workflow".to_string(),
        "Permission".to_string(),
        "Credential".to_string(),
    ])
    .with_description("The kind of the object");

    let root = NodeDef::default();

    let root = root.with_property(
        "id",
        PropertyDef::new(ValueType::Custom {
            name: "nsid".to_string(),
            validator: Some("nsid".to_string()),
        }).with_description(r#"The ID of the document. Must be a valid NSID."#)
        .required(),
    );

    let definition = NodeDef::new("definition")
        .with_description(
            r#"
Definition containing a kind property and a set of properties.

- kind: The kind of the definition, one of DataModel, Service, Workflow, Permission, Credential
    "#,
        )
        .with_property(
            "kind",
            PropertyDef {
                ty: ValueType::Enum("kind".to_string()),
                required: true,
                default: None,
                description: None,
                suggestions: Vec::new(),
            },
        )
        .with_child_conditional(|_,node| {
            NodeDef::get_node_property_value(node, "kind") == Some("DataModel".to_string())
        }, create_model_node_def());

    let root = root.with_child(definition);

    let mut schema = Schema::new("degov-dgl-v1", root);
    schema.define_enum("kind", kind_enum);
    schema.register_type_validator("nsid", create_nsid_validator());

    schema
}

fn create_model_node_def() -> NodeDef {
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
