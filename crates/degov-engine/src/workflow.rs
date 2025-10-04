/// Core workflow data structures
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A workflow consists of a series of steps to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub outputs: HashMap<String, serde_json::Value>,
    pub steps: Vec<Step>,
}

/// A step in the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub step_type: StepType,
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

/// Type of step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepType {
    /// Execute a script (JavaScript/TypeScript)
    Script { code: String },
    /// Set variables
    Set,
    /// Log output
    Log,
}

/// Runtime state of a workflow execution
#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub execution_id: String,
    pub state: ExecutionState,
    pub variables: HashMap<String, serde_json::Value>,
    pub current_step: usize,
}

/// State of workflow execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Result of a step execution
#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl Workflow {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            steps: Vec::new(),
        }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl WorkflowExecution {
    pub fn new(workflow_id: String, execution_id: String) -> Self {
        Self {
            workflow_id,
            execution_id,
            state: ExecutionState::Pending,
            variables: HashMap::new(),
            current_step: 0,
        }
    }

    pub fn set_variable(&mut self, key: String, value: serde_json::Value) {
        self.variables.insert(key, value);
    }

    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }
}

