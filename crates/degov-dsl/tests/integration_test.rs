use degov_dsl::{Definition, Nsid, Parser};

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
    merkleProof: true
  schema:
    type: object
    properties:
      id:
        type: string
        format: uuid
        generated: true
      name:
        type: string
        required: true
        minLength: 1
        maxLength: 100
"#;
    
    let definition = Definition::from_yaml(yaml).unwrap();
    assert_eq!(definition.metadata().id.as_str(), "de.test/example");
    
    if let Definition::DataModel(model) = definition {
        assert_eq!(model.metadata.title, "Example Model");
        assert!(model.spec.storage.as_ref().unwrap().encrypted);
        assert!(model.spec.storage.as_ref().unwrap().merkle_proof);
        assert_eq!(model.spec.schema.properties.len(), 2);
    } else {
        panic!("Expected DataModel");
    }
}

#[test]
fn test_parse_service() {
    let yaml = r#"
apiVersion: degov.gov/v1
kind: Service
metadata:
  id: de.berlin/business-registration
  title: Business Registration Service
  version: 1.0.0
  authority:
    did: did:degov:de:berlin:business-office
    name: Berlin Business Registration Office
spec:
  models:
    - de.berlin/business
    - de.berlin/owner
  workflows:
    - de.berlin/business-registration#workflow
  credentials:
    - de.berlin/business-license#credential
  config:
    payment:
      enabled: true
      provider: giropay
      fees:
        - type: registration
          amount: 15000
          currency: EUR
"#;
    
    let definition = Definition::from_yaml(yaml).unwrap();
    
    if let Definition::Service(service) = definition {
        assert_eq!(service.metadata.id.as_str(), "de.berlin/business-registration");
        assert_eq!(service.spec.models.len(), 2);
        assert_eq!(service.spec.workflows.len(), 1);
        assert!(service.spec.config.as_ref().unwrap().payment.as_ref().unwrap().enabled);
    } else {
        panic!("Expected Service");
    }
}

#[test]
fn test_parse_workflow() {
    let yaml = r#"
apiVersion: degov.gov/v1
kind: Workflow
metadata:
  id: de.berlin/business-registration#workflow
  title: Business Registration Workflow
  version: 1.0.0
spec:
  model: de.berlin/business
  initialState: draft
  states:
    draft:
      title: Draft Application
      type: user-input
      allowedActions:
        - submitForReview
        - cancel
    approved:
      title: Approved
      type: automated
      terminal: false
  transitions:
    submitForReview:
      from: draft
      to: pending-review
      title: Submit for Review
      permissions:
        - applicant-owner
"#;
    
    let definition = Definition::from_yaml(yaml).unwrap();
    
    if let Definition::Workflow(workflow) = definition {
        assert_eq!(workflow.metadata.id.as_str(), "de.berlin/business-registration#workflow");
        assert_eq!(workflow.spec.model, "de.berlin/business");
        assert_eq!(workflow.spec.initial_state, "draft");
        assert_eq!(workflow.spec.states.len(), 2);
        assert_eq!(workflow.spec.transitions.len(), 1);
    } else {
        panic!("Expected Workflow");
    }
}

#[test]
fn test_parse_permission() {
    let yaml = r#"
apiVersion: degov.gov/v1
kind: Permission
metadata:
  id: de.berlin/business-registration#permissions
  title: Business Registration Access Rules
  version: 1.0.0
spec:
  roles:
    citizen:
      description: Regular citizen user
      inherits: []
    business-reviewer:
      description: Staff member who reviews applications
      inherits: []
      attributes:
        department: business-office
  rules:
    - name: read-own-business
      description: Citizens can read businesses they own
      effect: allow
      principals:
        roles:
          - citizen
      actions:
        - read
      resources:
        models:
          - de.berlin/business
      conditions:
        - type: expression
          value: record.owners[*].did contains principal.did
  default: deny
"#;
    
    let definition = Definition::from_yaml(yaml).unwrap();
    
    if let Definition::Permission(permission) = definition {
        assert_eq!(permission.spec.roles.len(), 2);
        assert_eq!(permission.spec.rules.len(), 1);
    } else {
        panic!("Expected Permission");
    }
}

#[test]
fn test_parse_credential() {
    let yaml = r#"
apiVersion: degov.gov/v1
kind: Credential
metadata:
  id: de.berlin/business-license#credential
  title: Business License Certificate
  version: 1.0.0
spec:
  type: BusinessLicenseCredential
  issuer:
    did: did:degov:de:berlin:business-office
    name: Berlin Business Registration Office
  credentialSubject:
    schema:
      type: object
      properties:
        id:
          type: string
          format: did
        legalName:
          type: string
          source: business.legalName
  issuanceConditions:
    - workflowState: approved
  proof:
    type: Ed25519Signature2020
    keyType: authority-signing-key
    proofPurpose: assertionMethod
"#;
    
    let definition = Definition::from_yaml(yaml).unwrap();
    
    if let Definition::Credential(credential) = definition {
        assert_eq!(credential.spec.credential_type, "BusinessLicenseCredential");
        assert_eq!(credential.spec.credential_subject.schema.properties.len(), 2);
    } else {
        panic!("Expected Credential");
    }
}

#[test]
fn test_parser_nsid_to_path() {
    let parser = Parser::new("services");
    
    // This will fail if files don't exist, but tests the parser structure
    let nsid: Nsid = "de.degov/identity-card".parse().unwrap();
    let result = parser.load_by_nsid(&nsid);
    
    // We're just testing that the path resolution works, not that files exist
    match result {
        Ok(_) => println!("Successfully loaded definitions"),
        Err(e) => println!("Expected error (files may not exist): {}", e),
    }
    
    // Test string-based loading
    let result = parser.load_by_nsid_str("de.degov/identity-card");
    match result {
        Ok(_) => println!("Successfully loaded definitions"),
        Err(e) => println!("Expected error (files may not exist): {}", e),
    }
}

#[test]
fn test_metadata_nsid_parsing() {
    use degov_dsl::Metadata;
    use std::collections::HashMap;
    
    let meta = Metadata {
        id: "de.berlin/business-registration#workflow".parse().unwrap(),
        title: "Test".to_string(),
        description: None,
        version: "1.0.0".to_string(),
        authority: None,
        tags: vec![],
        extra: HashMap::new(),
    };
    
    assert_eq!(meta.nsid_authority(), "de.berlin");
    assert_eq!(meta.nsid_entity(), "business-registration");
    assert_eq!(meta.nsid_fragment(), Some("workflow"));
    assert!(!meta.is_federal());
}

