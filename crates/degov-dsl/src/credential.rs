use crate::metadata::{ApiVersion, Authority, Metadata};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Credential definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Credential {
    #[serde(rename = "apiVersion")]
    pub api_version: ApiVersion,
    
    #[serde(skip)]
    pub kind: String, // Always "Credential"
    
    pub metadata: Metadata,
    
    pub spec: CredentialSpec,
}

/// Specification for a credential
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialSpec {
    /// W3C Verifiable Credential type
    #[serde(rename = "type")]
    pub credential_type: String,
    
    /// Issuer information
    pub issuer: Authority,
    
    /// Credential subject schema
    #[serde(rename = "credentialSubject")]
    pub credential_subject: CredentialSubject,
    
    /// Issuance conditions
    #[serde(default, rename = "issuanceConditions", skip_serializing_if = "Vec::is_empty")]
    pub issuance_conditions: Vec<IssuanceCondition>,
    
    /// Cryptographic proof configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<ProofConfig>,
    
    /// Revocation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revocation: Option<RevocationConfig>,
    
    /// Display configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<DisplayConfig>,
    
    /// Portability configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portability: Option<PortabilityConfig>,
}

/// Credential subject definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialSubject {
    pub schema: CredentialSchema,
}

/// Schema for credential subject
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    
    #[serde(default)]
    pub properties: IndexMap<String, CredentialProperty>,
}

/// Property in credential subject
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialProperty {
    #[serde(rename = "type")]
    pub property_type: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Source field from the data model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    
    /// Computed value (JavaScript)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed: Option<String>,
    
    /// Format hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Issuance condition
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IssuanceCondition {
    /// Workflow state condition
    WorkflowState {
        #[serde(rename = "workflowState")]
        workflow_state: String,
    },
    
    /// Payment status condition
    PaymentStatus {
        #[serde(rename = "paymentStatus")]
        payment_status: String,
    },
    
    /// Custom condition
    Custom {
        condition: String,
    },
}

/// Proof configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProofConfig {
    #[serde(rename = "type")]
    pub proof_type: String,
    
    #[serde(rename = "keyType")]
    pub key_type: String,
    
    #[serde(rename = "proofPurpose")]
    pub proof_purpose: String,
}

/// Revocation configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevocationConfig {
    #[serde(rename = "type")]
    pub revocation_type: String,
    
    #[serde(rename = "statusListUrl")]
    pub status_list_url: String,
    
    /// Conditions under which credential should be revoked
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,
}

/// Display configuration for credential
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
    pub title: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    
    #[serde(rename = "backgroundColor", skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    
    #[serde(rename = "textColor", skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    
    /// Properties to display
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<DisplayProperty>,
    
    /// QR code configuration
    #[serde(rename = "qrCode", skip_serializing_if = "Option::is_none")]
    pub qr_code: Option<QrCodeConfig>,
}

/// Display property
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayProperty {
    pub label: String,
    pub path: String,
    
    #[serde(rename = "displayFormat", skip_serializing_if = "Option::is_none")]
    pub display_format: Option<String>,
}

/// QR code configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QrCodeConfig {
    pub enabled: bool,
    pub content: String,
}

/// Portability configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PortabilityConfig {
    /// Export formats supported
    #[serde(default, rename = "exportFormats", skip_serializing_if = "Vec::is_empty")]
    pub export_formats: Vec<String>,
    
    /// Who this credential can be shared with
    #[serde(default, rename = "shareableWith", skip_serializing_if = "Vec::is_empty")]
    pub shareable_with: Vec<String>,
}


