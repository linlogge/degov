use crate::metadata::{ApiVersion, Metadata};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Workflow definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Workflow {
    #[serde(rename = "apiVersion")]
    pub api_version: ApiVersion,
    
    #[serde(skip)]
    pub kind: String, // Always "Workflow"
    
    pub metadata: Metadata,
    
    pub spec: WorkflowSpec,
}

/// Specification for a workflow
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowSpec {
    /// The data model this workflow operates on (NSID)
    pub model: String,
    
    /// Initial state when workflow starts
    #[serde(rename = "initialState")]
    pub initial_state: String,
    
    /// State definitions
    #[serde(default)]
    pub states: IndexMap<String, State>,
    
    /// Transition definitions
    #[serde(default)]
    pub transitions: IndexMap<String, Transition>,
    
    /// Escalation rules
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub escalations: Vec<Escalation>,
    
    /// Webhooks for external integrations
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub webhooks: Vec<Webhook>,
}

/// State definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct State {
    pub title: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    #[serde(rename = "type")]
    pub state_type: StateType,
    
    /// Actions allowed in this state
    #[serde(default, rename = "allowedActions", skip_serializing_if = "Vec::is_empty")]
    pub allowed_actions: Vec<String>,
    
    /// Actions to perform when entering this state
    #[serde(default, rename = "onEnter", skip_serializing_if = "Vec::is_empty")]
    pub on_enter: Vec<Action>,
    
    /// Actions to perform when exiting this state
    #[serde(default, rename = "onExit", skip_serializing_if = "Vec::is_empty")]
    pub on_exit: Vec<Action>,
    
    /// Timeout configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Timeout>,
    
    /// UI configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<StateUi>,
    
    /// Validations to run in this state
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validations: Option<StateValidations>,
    
    /// Whether this is a terminal state (no transitions out)
    #[serde(default)]
    pub terminal: bool,
    
    /// Periodic checks to perform while in this state
    #[serde(default, rename = "periodicChecks", skip_serializing_if = "Vec::is_empty")]
    pub periodic_checks: Vec<PeriodicCheck>,
    
    /// Retention settings for terminal states
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<StateRetention>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StateType {
    UserInput,
    Automated,
    Operational,
    Restricted,
    Terminal,
}

/// Action to perform
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Action {
    pub action: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_yaml::Value>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

/// Timeout configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Timeout {
    /// ISO 8601 duration
    pub duration: String,
    
    /// Action to perform on timeout
    pub action: String,
}

/// UI configuration for a state
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StateUi {
    #[serde(default, rename = "formSections", skip_serializing_if = "Vec::is_empty")]
    pub form_sections: Vec<String>,
    
    #[serde(default, rename = "showComments")]
    pub show_comments: bool,
    
    #[serde(default, rename = "allowEdits")]
    pub allow_edits: bool,
    
    #[serde(rename = "displayWarning", skip_serializing_if = "Option::is_none")]
    pub display_warning: Option<String>,
}

/// State validations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StateValidations {
    #[serde(default, rename = "onSubmit", skip_serializing_if = "Vec::is_empty")]
    pub on_submit: Vec<String>,
}

/// Periodic check configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeriodicCheck {
    /// ISO 8601 duration
    pub interval: String,
    
    pub action: String,
}

/// State retention configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StateRetention {
    #[serde(rename = "archiveAfter")]
    pub archive_after: String,
}

/// Transition definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Transition {
    /// Source state(s)
    #[serde(deserialize_with = "deserialize_from_field")]
    pub from: Vec<String>,
    
    /// Target state
    pub to: String,
    
    pub title: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Required permissions (role names)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    
    /// Validation functions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validations: Vec<TransitionValidation>,
    
    /// Side effects to perform
    #[serde(default, rename = "sideEffects", skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<String>,
    
    /// Whether a comment is required
    #[serde(default, rename = "requiresComment")]
    pub requires_comment: bool,
    
    /// Whether a signature is required
    #[serde(default, rename = "requiresSignature")]
    pub requires_signature: bool,
    
    /// Approval requirements
    #[serde(rename = "requiresApproval", skip_serializing_if = "Option::is_none")]
    pub requires_approval: Option<ApprovalRequirement>,
    
    /// Reason requirements
    #[serde(default, rename = "requiresReason", skip_serializing_if = "Vec::is_empty")]
    pub requires_reason: Vec<String>,
    
    /// Whether confirmation is required
    #[serde(default, rename = "confirmationRequired")]
    pub confirmation_required: bool,
    
    /// UI configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<TransitionUi>,
}

/// Helper to deserialize single string or array of strings
fn deserialize_from_field<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Deserialize;
    
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }
    
    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::String(s) => Ok(vec![s]),
        StringOrVec::Vec(v) => Ok(v),
    }
}

/// Transition validation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransitionValidation {
    pub name: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

/// Approval requirement
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApprovalRequirement {
    pub from: String,
    
    #[serde(rename = "minCount")]
    pub min_count: u32,
}

/// UI configuration for a transition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransitionUi {
    #[serde(rename = "commentPrompt", skip_serializing_if = "Option::is_none")]
    pub comment_prompt: Option<String>,
    
    #[serde(rename = "confirmationMessage", skip_serializing_if = "Option::is_none")]
    pub confirmation_message: Option<String>,
}

/// Escalation rule
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Escalation {
    pub name: String,
    pub condition: String,
    pub action: String,
}

/// Webhook configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Webhook {
    pub event: String,
    pub url: String,
    pub method: String,
    
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_yaml::Value>,
}

