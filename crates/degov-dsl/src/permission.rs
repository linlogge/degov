use crate::metadata::{ApiVersion, Metadata};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Permission definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Permission {
    #[serde(rename = "apiVersion")]
    pub api_version: ApiVersion,
    
    #[serde(skip)]
    pub kind: String, // Always "Permission"
    
    pub metadata: Metadata,
    
    pub spec: PermissionSpec,
}

/// Specification for permissions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PermissionSpec {
    /// Role definitions
    #[serde(default)]
    pub roles: IndexMap<String, Role>,
    
    /// Access control rules
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<Rule>,
    
    /// Default policy (allow or deny)
    #[serde(default = "default_deny")]
    pub default: DefaultPolicy,
    
    /// Audit configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit: Option<AuditConfig>,
}

fn default_deny() -> DefaultPolicy {
    DefaultPolicy::Deny
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultPolicy {
    Allow,
    Deny,
}

/// Role definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Role {
    pub description: String,
    
    /// Roles this role inherits from
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inherits: Vec<String>,
    
    /// Attributes associated with this role
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub attributes: IndexMap<String, String>,
}

/// Access control rule
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub name: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Effect of this rule (allow or deny)
    pub effect: Effect,
    
    /// Principals this rule applies to
    pub principals: Principals,
    
    /// Actions this rule applies to
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    
    /// Resources this rule applies to
    pub resources: Resources,
    
    /// Conditions that must be met
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    Allow,
    Deny,
}

/// Principals (who the rule applies to)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Principals {
    /// Roles
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    
    /// Authorities (DIDs)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authorities: Vec<String>,
    
    /// Attributes to exclude
    #[serde(default, rename = "excludeAttributes", skip_serializing_if = "IndexMap::is_empty")]
    pub exclude_attributes: IndexMap<String, String>,
}

/// Resources the rule applies to
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Resources {
    /// Wildcard - all resources
    Wildcard(Vec<String>),
    
    /// Specific resources
    Specific {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        models: Vec<String>,
        
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        fields: Vec<String>,
        
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        workflows: Vec<String>,
        
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        transitions: Vec<String>,
    },
}

/// Condition that must be met for rule to apply
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Condition {
    #[serde(rename = "type")]
    pub condition_type: ConditionType,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    
    #[serde(rename = "grantedBy", skip_serializing_if = "Option::is_none")]
    pub granted_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConditionType {
    Expression,
    Consent,
    Time,
}

/// Audit configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditConfig {
    #[serde(default, rename = "logAllAccess")]
    pub log_all_access: bool,
    
    #[serde(default, rename = "logDenials")]
    pub log_denials: bool,
    
    /// Actions that are considered sensitive and should always be logged
    #[serde(default, rename = "sensitiveActions", skip_serializing_if = "Vec::is_empty")]
    pub sensitive_actions: Vec<String>,
    
    /// Audit log retention period (ISO 8601 duration)
    #[serde(rename = "retentionPeriod")]
    pub retention_period: String,
}


