use crate::metadata::{ApiVersion, Metadata};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Data model definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataModel {
    #[serde(rename = "apiVersion")]
    pub api_version: ApiVersion,
    
    #[serde(skip)]
    pub kind: String, // Always "DataModel"
    
    pub metadata: Metadata,
    
    pub spec: DataModelSpec,
}

/// Specification for a data model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataModelSpec {
    /// Models this one inherits from (NSIDs)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inherits: Vec<String>,
    
    /// Storage configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageConfig>,
    
    /// Schema definition
    pub schema: Schema,
    
    /// Indexes for efficient querying
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indexes: Vec<Index>,
    
    /// Computed/derived fields
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub computed: IndexMap<String, ComputedField>,
}

/// Storage configuration for a model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    /// Whether to encrypt data at rest
    #[serde(default)]
    pub encrypted: bool,
    
    /// Whether to generate Merkle proofs
    #[serde(default, rename = "merkleProof")]
    pub merkle_proof: bool,
    
    /// Data retention policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<RetentionPolicy>,
}

/// Data retention policy
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetentionPolicy {
    /// ISO 8601 duration (e.g., "P50Y" for 50 years)
    pub duration: String,
    
    /// What to do after deletion
    #[serde(rename = "afterDeletion")]
    pub after_deletion: RetentionAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RetentionAction {
    Delete,
    Anonymize,
    Archive,
}

/// Schema definition (JSON Schema-like)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Schema {
    #[serde(rename = "type")]
    pub schema_type: SchemaType,
    
    /// Properties for object types
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: IndexMap<String, Property>,
    
    /// Items definition for array types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Property>>,
    
    /// Required fields (for objects)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Object,
    Array,
    String,
    Number,
    Integer,
    Boolean,
}

/// Property definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Property {
    #[serde(rename = "type")]
    pub property_type: PropertyType,
    
    /// Reference to another model (for ref types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    
    /// Format hint (e.g., "uuid", "email", "did")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Whether field is required
    #[serde(default)]
    pub required: bool,
    
    /// Whether field can be null
    #[serde(default)]
    pub nullable: bool,
    
    /// Whether field is immutable after creation
    #[serde(default)]
    pub immutable: bool,
    
    /// Whether to create an index
    #[serde(default)]
    pub indexed: bool,
    
    /// Whether to encrypt this field
    #[serde(default)]
    pub encrypted: bool,
    
    /// Whether this is personally identifiable information
    #[serde(default)]
    pub pii: bool,
    
    /// Whether value is auto-generated
    #[serde(default)]
    pub generated: bool,
    
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_yaml::Value>,
    
    /// Enum values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    
    /// Pattern for string validation (regex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    
    /// Minimum length (strings)
    #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    
    /// Maximum length (strings)
    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    
    /// Minimum value (numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    
    /// Maximum value (numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    
    /// Minimum items (arrays)
    #[serde(rename = "minItems", skip_serializing_if = "Option::is_none")]
    pub min_items: Option<usize>,
    
    /// Maximum items (arrays)
    #[serde(rename = "maxItems", skip_serializing_if = "Option::is_none")]
    pub max_items: Option<usize>,
    
    /// Items definition for array types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Property>>,
    
    /// Properties for object types
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: IndexMap<String, Property>,
    
    /// Custom validations
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validations: Vec<Validation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyType {
    String,
    Number,
    Integer,
    Boolean,
    Date,
    Timestamp,
    Enum,
    Array,
    Object,
    #[serde(rename = "ref")]
    Ref,
}

/// Custom validation rule
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Validation {
    #[serde(rename = "type")]
    pub validation_type: ValidationType,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationType {
    Custom,
    DateRange,
    Pattern,
    Required,
}

/// Index definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Index {
    pub name: String,
    pub fields: Vec<String>,
    
    #[serde(default)]
    pub unique: bool,
}

/// Computed field definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ComputedField {
    #[serde(rename = "type")]
    pub field_type: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// JavaScript code to compute the value
    pub script: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_model() {
        let yaml = r#"
apiVersion: degov.gov/v1
kind: DataModel
metadata:
  id: de.test/example
  title: Example Model
  version: 1.0.0
spec:
  storage:
    encrypted: true
  schema:
    type: object
    properties:
      id:
        type: string
        format: uuid
      name:
        type: string
        required: true
"#;
        
        let model: DataModel = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(model.metadata.id.as_str(), "de.test/example");
        assert_eq!(model.spec.storage.as_ref().unwrap().encrypted, true);
        assert!(model.spec.schema.properties.contains_key("id"));
    }
}

