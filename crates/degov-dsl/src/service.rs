use crate::metadata::{ApiVersion, Metadata};
use serde::{Deserialize, Serialize};

/// Service definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Service {
    #[serde(rename = "apiVersion")]
    pub api_version: ApiVersion,
    
    #[serde(skip)]
    pub kind: String, // Always "Service"
    
    pub metadata: Metadata,
    
    pub spec: ServiceSpec,
}

/// Specification for a service
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceSpec {
    /// References to data models used by this service
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
    
    /// References to workflows
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workflows: Vec<String>,
    
    /// References to credentials that can be issued
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credentials: Vec<String>,
    
    /// Service-level configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ServiceConfig>,
}

/// Service configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    /// Payment configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment: Option<PaymentConfig>,
    
    /// Notification settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<NotificationConfig>,
    
    /// Inter-authority data requests
    #[serde(default, rename = "federatedRequests", skip_serializing_if = "Vec::is_empty")]
    pub federated_requests: Vec<FederatedRequest>,
    
    /// Accessibility settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityConfig>,
}

/// Payment configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaymentConfig {
    pub enabled: bool,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fees: Vec<Fee>,
}

/// Fee definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Fee {
    #[serde(rename = "type")]
    pub fee_type: String,
    
    /// Amount in cents
    pub amount: u64,
    
    pub currency: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Notification configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<EmailConfig>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sms: Option<SmsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailConfig {
    pub enabled: bool,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmsConfig {
    pub enabled: bool,
}

/// Federated request configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FederatedRequest {
    pub service: String,
    pub authority: String, // DID
    pub purpose: String,
    
    #[serde(rename = "requiredConsent")]
    pub required_consent: bool,
}

/// Accessibility configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccessibilityConfig {
    #[serde(rename = "wcagLevel")]
    pub wcag_level: String,
    
    #[serde(rename = "bitvCompliant")]
    pub bitv_compliant: bool,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_service() {
        let yaml = r#"
apiVersion: degov.gov/v1
kind: Service
metadata:
  id: de.berlin/test-service
  title: Test Service
  version: 1.0.0
spec:
  models:
    - de.berlin/business
  workflows:
    - de.berlin/test#workflow
  config:
    payment:
      enabled: true
      provider: giropay
"#;
        
        let service: Service = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(service.metadata.id.as_str(), "de.berlin/test-service");
        assert_eq!(service.spec.models.len(), 1);
        assert!(service.spec.config.as_ref().unwrap().payment.as_ref().unwrap().enabled);
    }
}

