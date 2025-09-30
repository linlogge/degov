# DeGov YAML DSL Specification

**Version:** 1.0  
**Date:** September 30, 2025

## 1. Introduction

This document proposes a YAML-based Domain-Specific Language (DSL) for the DeGov framework. The DSL enables government authorities to declaratively define data models, workflows, permissions, and integrations without writing code. The language is designed to be:

- **Human-readable**: Non-technical administrators can understand and modify definitions
- **Version-controllable**: YAML files can be tracked in git for auditing and rollback
- **Composable**: Definitions can reference and extend each other
- **Type-safe**: Strong typing prevents common configuration errors
- **Extensible**: Supports custom logic via embedded JavaScript

## 2. Core Concepts

### 2.1 Naming Convention (AT Protocol Lexicon Style)

All entities in the DeGov DSL follow the AT Protocol Lexicon naming convention using reverse DNS with slash separators and hash fragments for types:

**Format:** `{authority}/{entity-name}` or `{authority}/{entity-name}#{type}`

**Examples:**
- `de.bund/person` - Federal person model (main definition)
- `de.berlin/business` - Berlin business model
- `de.bayern/building-permit` - Bavaria building permit
- `de.berlin/business-registration#workflow` - Workflow within service
- `de.berlin/business-registration#permissions` - Permission rules
- `de.berlin/business-license#credential` - Credential definition
- `com.example/custom-service#plugin` - Third-party plugin

**Authority Hierarchy:**
- **Federal (de.bund/)**: Base models shared across all German authorities
- **State (de.{state}/)**: State-specific models (e.g., `de.berlin/`, `de.bayern/`)
- **Municipal (de.{city}/)**: City-specific models
- **Third-party (com.{company}/)**: Private company extensions

**Hash Fragment Types:**
- `#workflow` - Workflow/state machine definition
- `#permissions` - Access control rules
- `#credential` - Verifiable credential schema
- `#plugin` - Plugin definition
- No hash fragment means the main model/entity definition

The naming scheme provides:
- **Namespacing**: Prevents naming conflicts between authorities
- **Discoverability**: Clear ownership and authority (domain before slash)
- **Type Clarity**: Hash fragments distinguish between different aspects (#workflow, #permissions)
- **Inheritance**: Natural hierarchy for model inheritance
- **Portability**: Models can reference each other across authorities
- **AT Proto Lexicon Compatibility**: Matches Lexicon naming with `nsid#type` structure

### 2.2 Service Definition

A **Service** is the top-level container that represents a complete government service offering (e.g., Business Registration, Building Permit, Marriage Certificate).

### 2.3 Data Models

**Data Models** define the structure of entities and their fields with validation rules.

### 2.4 Workflows

**Workflows** define multi-step processes with states, transitions, and actions.

### 2.5 Permissions

**Permissions** define granular, attribute-based access control rules.

### 2.6 Credentials

**Credentials** define verifiable credentials that can be issued to citizens.

## 3. File Structure

The folder structure follows AT Protocol conventions using reverse DNS notation:

```
services/
├── de/
│   ├── bund/                       # Federal level
│   │   ├── identity-card/
│   │   │   ├── model.yaml         # Data model definition
│   │   │   ├── workflow.yaml      # Workflow definition
│   │   │   ├── permissions.yaml   # Permission rules
│   │   │   └── credential.yaml    # Credential definition
│   │   ├── passport/
│   │   │   └── model.yaml
│   │   └── person/
│   │       └── model.yaml         # Base person model (for inheritance)
│   ├── berlin/                     # Municipal level
│   │   ├── business-registration/
│   │   │   ├── service.yaml       # Service definition
│   │   │   ├── model.yaml
│   │   │   ├── workflow.yaml
│   │   │   └── permissions.yaml
│   │   └── building-permit/
│   │       ├── model.yaml
│   │       └── workflow.yaml
│   └── bayern/
│       └── business-license/
│           └── model.yaml
└── com/
    └── example/
        └── custom-service/
            └── model.yaml

# Naming Convention:
# - Federal: de.bund/<service-name>
# - State: de.<state>/<service-name>
# - Municipal: de.<city>/<service-name>
# - Third-party: com.<company>/<service-name>
```

## 4. DSL Syntax

### 4.1 Service Definition

**File:** `services/de/berlin/business-registration/service.yaml`

```yaml
apiVersion: degov.gov/v1
kind: Service
metadata:
  id: de.berlin/business-registration      # AT Protocol style with slash
  title: Business Registration Service
  description: Register a new business entity with the municipality
  version: 1.0.0
  authority:
    did: did:degov:de:berlin:business-office
    name: Berlin Business Registration Office
  tags:
    - business
    - registration
  
spec:
  # Reference to data models used by this service
  models:
    - de.berlin/business
    - de.berlin/owner
    - de.bund/address
  
  # Reference to workflows
  workflows:
    - de.berlin/business-registration#workflow
  
  # Reference to credentials that can be issued
  credentials:
    - de.berlin/business-license#credential
  
  # Service-level configuration
  config:
    # Payment configuration
    payment:
      enabled: true
      provider: giropay
      fees:
        - type: registration
          amount: 15000  # cents (€150)
          currency: EUR
          description: Business registration fee
    
    # Notification settings
    notifications:
      email:
        enabled: true
        templates:
          - application-received
          - application-approved
          - application-rejected
      sms:
        enabled: false
    
    # Inter-authority data requests
    federatedRequests:
      - service: identity-verification
        authority: did:degov:de:federal:bsi
        purpose: Verify citizen identity for business registration
        requiredConsent: true
    
    # Accessibility settings
    accessibility:
      wcagLevel: AA
      bitvCompliant: true
      languages:
        - de
        - en
        - tr
```

### 4.2 Data Model Definition

**File:** `services/de/berlin/business/model.yaml`

```yaml
apiVersion: degov.gov/v1
kind: DataModel
metadata:
  id: de.berlin/business                    # AT Protocol style with slash
  title: Business Entity
  description: Represents a registered business
  version: 1.0.0

spec:
  # Inheritance - inherit fields from base models
  inherits:
    - de.bund/legal-entity                   # Federal base model for legal entities
  
  # Storage configuration
  storage:
    encrypted: true
    merkleProof: true
    retention:
      duration: P50Y  # ISO 8601 duration (50 years)
      afterDeletion: anonymize
  
  # Schema definition
  schema:
    type: object
    properties:
      id:
        type: string
        format: uuid
        generated: true
        immutable: true
        indexed: true
        description: Unique business identifier
      
      legalName:
        type: string
        minLength: 1
        maxLength: 200
        required: true
        description: Official registered business name
        validations:
          - type: custom
            script: validateBusinessName
            message: Business name must not contain special characters
      
      legalForm:
        type: enum
        required: true
        values:
          - GmbH
          - UG
          - AG
          - e.K.
          - GbR
          - KG
          - OHG
        description: Legal form of the business
      
      foundingDate:
        type: date
        required: true
        validations:
          - type: dateRange
            min: "1900-01-01"
            max: "today"
        description: Date of business founding
      
      registrationNumber:
        type: string
        pattern: "^HRB-[0-9]{6}$"
        generated: true
        immutable: true
        indexed: true
        description: Official registration number (e.g., HRB-123456)
      
      businessAddress:
        type: ref
        ref: de.bund/address                 # Reference to federal address model
        required: true
        description: Primary business location
      
      owners:
        type: array
        items:
          type: ref
          ref: de.bund/person                # Reference to federal person model
        minItems: 1
        maxItems: 10
        required: true
        description: Business owners and their ownership percentages
      
      ownershipDetails:
        type: array
        items:
          type: object
          properties:
            person:
              type: ref
              ref: de.bund/person
            percentage:
              type: number
              min: 0
              max: 100
            role:
              type: enum
              values: [ceo, cfo, board-member, shareholder]
      
      industry:
        type: string
        required: true
        enum:
          - retail
          - manufacturing
          - services
          - technology
          - hospitality
          - healthcare
          - construction
          - other
        description: Primary industry sector
      
      taxId:
        type: string
        pattern: "^DE[0-9]{9}$"
        encrypted: true
        pii: true
        description: German tax identification number
      
      status:
        type: enum
        values:
          - draft
          - pending-review
          - active
          - suspended
          - dissolved
        default: draft
        indexed: true
        description: Current business registration status
      
      createdAt:
        type: timestamp
        generated: true
        immutable: true
        indexed: true
      
      updatedAt:
        type: timestamp
        generated: true
      
      createdBy:
        type: string
        format: did
        generated: true
        immutable: true
        description: DID of the user who created this record
      
      consentRecords:
        type: array
        items:
          type: object
          properties:
            purpose:
              type: string
            grantedAt:
              type: timestamp
            revokedAt:
              type: timestamp
              nullable: true
        description: Citizen consent audit trail
  
  # Indexes for efficient querying
  indexes:
    - name: by-registration-number
      fields: [registrationNumber]
      unique: true
    
    - name: by-status
      fields: [status, createdAt]
    
    - name: by-owner
      fields: [owners, status]
  
  # Computed/derived fields
  computed:
    age:
      type: integer
      description: Age of business in years
      script: |
        const now = new Date();
        const founding = new Date(record.foundingDate);
        return Math.floor((now - founding) / (365.25 * 24 * 60 * 60 * 1000));
    
    isFullyOwned:
      type: boolean
      description: Check if ownership percentages sum to 100%
      script: |
        const total = record.owners.reduce((sum, owner) => sum + owner.percentage, 0);
        return Math.abs(total - 100) < 0.01;
```

### 4.3 Workflow Definition

**File:** `services/de/berlin/business-registration/workflow.yaml`

```yaml
apiVersion: degov.gov/v1
kind: Workflow
metadata:
  id: de.berlin/business-registration#workflow  # Lexicon style with hash fragment
  title: Business Registration Workflow
  description: Multi-step process for registering a new business
  version: 1.0.0

spec:
  # The data model this workflow operates on
  model: de.berlin/business
  
  # Initial state when workflow starts
  initialState: draft
  
  # State machine definition
  states:
    draft:
      title: Draft Application
      description: Citizen is filling out the application
      type: user-input
      allowedActions:
        - submitForReview
        - saveDraft
        - cancel
      ui:
        formSections:
          - business-details
          - owner-information
          - address-details
      validations:
        onSubmit:
          - checkRequiredFields
          - validateOwnershipTotal
    
    pending-review:
      title: Pending Review
      description: Application is awaiting review by authority staff
      type: automated
      onEnter:
        - action: sendNotification
          params:
            recipient: applicant
            template: application-received
        - action: assignReviewer
          script: |
            // Assign to reviewer with lowest workload
            const reviewers = await db.query('staff', {
              role: 'business-reviewer',
              status: 'active'
            });
            const assignments = await db.query('assignments', {
              status: 'pending'
            });
            const workload = {};
            reviewers.forEach(r => workload[r.id] = 0);
            assignments.forEach(a => workload[a.reviewerId]++);
            const assignedTo = Object.keys(workload).reduce((a, b) => 
              workload[a] < workload[b] ? a : b
            );
            return { reviewerId: assignedTo };
      allowedActions:
        - approve
        - requestChanges
        - reject
      timeout:
        duration: P5D  # 5 business days
        action: escalateToSupervisor
    
    changes-requested:
      title: Changes Requested
      description: Authority has requested changes to the application
      type: user-input
      onEnter:
        - action: sendNotification
          params:
            recipient: applicant
            template: changes-requested
      allowedActions:
        - resubmit
        - cancel
      ui:
        showComments: true
        allowEdits: true
    
    approved:
      title: Approved
      description: Application has been approved
      type: automated
      onEnter:
        - action: generateRegistrationNumber
          script: |
            const year = new Date().getFullYear();
            const count = await db.count('business', {
              status: 'approved',
              createdAt: { year }
            });
            return `HRB-${year}${String(count + 1).padStart(6, '0')}`;
        - action: issueCredential
          credential: business-license
        - action: processPayment
          params:
            type: registration
        - action: sendNotification
          params:
            recipient: applicant
            template: application-approved
        - action: notifyPartners
          script: |
            // Notify tax authority about new business
            await federated.notify({
              authority: 'did:degov:de:federal:tax-office',
              event: 'business-registered',
              data: {
                businessId: record.id,
                taxId: record.taxId,
                legalName: record.legalName
              }
            });
      allowedActions:
        - issueModification
        - suspend
        - dissolve
      terminal: false
    
    rejected:
      title: Rejected
      description: Application has been rejected
      type: terminal
      onEnter:
        - action: sendNotification
          params:
            recipient: applicant
            template: application-rejected
        - action: logRejection
      allowedActions: []
      terminal: true
    
    active:
      title: Active Business
      description: Business is actively registered
      type: operational
      onEnter:
        - action: updatePublicRegistry
      allowedActions:
        - updateDetails
        - suspend
        - dissolve
      periodicChecks:
        - interval: P1Y  # Yearly
          action: annualComplianceCheck
    
    suspended:
      title: Suspended
      description: Business registration is temporarily suspended
      type: restricted
      onEnter:
        - action: sendNotification
          params:
            recipient: owner
            template: business-suspended
      allowedActions:
        - reinstate
        - dissolve
      ui:
        displayWarning: Business is currently suspended
    
    dissolved:
      title: Dissolved
      description: Business has been dissolved
      type: terminal
      onEnter:
        - action: archiveRecords
        - action: notifyPartners
        - action: revokeCredentials
      allowedActions: []
      terminal: true
      retention:
        archiveAfter: P30D
  
  # Transition definitions
  transitions:
    submitForReview:
      from: draft
      to: pending-review
      title: Submit for Review
      description: Submit application for authority review
      permissions:
        - applicant-owner
      validations:
        - name: checkAllFieldsComplete
          script: |
            const required = ['legalName', 'legalForm', 'businessAddress', 'owners'];
            for (const field of required) {
              if (!record[field]) {
                throw new Error(`Field ${field} is required`);
              }
            }
            return true;
        - name: validateOwnership
          script: |
            const total = record.owners.reduce((sum, o) => sum + o.percentage, 0);
            if (Math.abs(total - 100) > 0.01) {
              throw new Error('Ownership percentages must sum to 100%');
            }
            return true;
      sideEffects:
        - lockForEditing
        - createAuditEntry
    
    approve:
      from: pending-review
      to: approved
      title: Approve Application
      description: Approve the business registration
      permissions:
        - business-reviewer
        - supervisor
      requiresComment: false
      requiresSignature: true
      sideEffects:
        - notifyApplicant
        - createPublicRecord
    
    requestChanges:
      from: pending-review
      to: changes-requested
      title: Request Changes
      description: Request modifications to the application
      permissions:
        - business-reviewer
        - supervisor
      requiresComment: true
      ui:
        commentPrompt: Please specify what changes are needed
    
    reject:
      from: pending-review
      to: rejected
      title: Reject Application
      description: Reject the business registration application
      permissions:
        - supervisor
      requiresComment: true
      requiresApproval:
        from: supervisor
        minCount: 1
      sideEffects:
        - refundPayment
    
    resubmit:
      from: changes-requested
      to: pending-review
      title: Resubmit Application
      description: Resubmit application after making requested changes
      permissions:
        - applicant-owner
      validations:
        - checkAllFieldsComplete
    
    suspend:
      from: [active, approved]
      to: suspended
      title: Suspend Business
      description: Temporarily suspend business registration
      permissions:
        - supervisor
        - compliance-officer
      requiresComment: true
      requiresReason:
        - non-payment
        - compliance-violation
        - fraud-investigation
        - owner-request
    
    reinstate:
      from: suspended
      to: active
      title: Reinstate Business
      description: Restore suspended business to active status
      permissions:
        - supervisor
      requiresComment: true
    
    dissolve:
      from: [active, approved, suspended]
      to: dissolved
      title: Dissolve Business
      description: Permanently dissolve the business registration
      permissions:
        - applicant-owner
        - supervisor
      requiresComment: true
      confirmationRequired: true
      ui:
        confirmationMessage: This action cannot be undone. Are you sure?
    
    cancel:
      from: [draft, changes-requested]
      to: dissolved
      title: Cancel Application
      description: Cancel the registration application
      permissions:
        - applicant-owner
  
  # Escalation rules
  escalations:
    - name: escalateToSupervisor
      condition: stateAge > P5D && state == 'pending-review'
      action: |
        await notify.email({
          to: 'supervisor@business-office.berlin.de',
          subject: 'Application requires attention',
          template: 'escalation-notice',
          data: { applicationId: record.id }
        });
  
  # Webhooks for external integrations
  webhooks:
    - event: state-changed
      url: ${WEBHOOK_URL}/business-status
      method: POST
      headers:
        Authorization: Bearer ${WEBHOOK_SECRET}
      payload:
        businessId: "{{record.id}}"
        oldState: "{{transition.from}}"
        newState: "{{transition.to}}"
        timestamp: "{{timestamp}}"
```

### 4.4 Permission Definition

**File:** `services/de/berlin/business-registration/permissions.yaml`

```yaml
apiVersion: degov.gov/v1
kind: Permission
metadata:
  id: de.berlin/business-registration#permissions  # Lexicon style with hash fragment
  title: Business Registration Access Rules
  description: Defines who can access and modify business registration data
  version: 1.0.0

spec:
  # Define roles
  roles:
    citizen:
      description: Regular citizen user
      inherits: []
    
    applicant-owner:
      description: Citizen who owns or is applying for a business
      inherits: [citizen]
    
    business-reviewer:
      description: Staff member who reviews business applications
      inherits: []
      attributes:
        department: business-office
        clearanceLevel: standard
    
    supervisor:
      description: Supervisor who can approve/reject applications
      inherits: [business-reviewer]
      attributes:
        clearanceLevel: elevated
    
    compliance-officer:
      description: Officer who handles compliance and enforcement
      inherits: []
      attributes:
        department: compliance
        clearanceLevel: elevated
    
    system-admin:
      description: System administrator with full access
      inherits: []
      attributes:
        clearanceLevel: admin
  
  # Attribute-based access control rules
  rules:
    # Data access rules
    - name: read-own-business
      description: Citizens can read businesses they own
      effect: allow
      principals:
        roles: [citizen]
      actions:
        - read
      resources:
        models: [de.berlin/business]
      conditions:
        - type: expression
          value: record.owners[*].did contains principal.did
    
    - name: update-own-draft
      description: Owners can update their business while in draft state
      effect: allow
      principals:
        roles: [applicant-owner]
      actions:
        - update
      resources:
        models: [de.berlin/business]
      conditions:
        - type: expression
          value: record.status == 'draft' && record.createdBy == principal.did
    
    - name: reviewer-read-pending
      description: Reviewers can read pending applications
      effect: allow
      principals:
        roles: [business-reviewer]
      actions:
        - read
      resources:
        models: [de.berlin/business]
      conditions:
        - type: expression
          value: record.status in ['pending-review', 'changes-requested']
    
    - name: reviewer-update-status
      description: Reviewers can approve or request changes
      effect: allow
      principals:
        roles: [business-reviewer]
      actions:
        - transition
      resources:
        workflows: [de.berlin/business-registration#workflow]
        transitions: [requestChanges]
      conditions:
        - type: expression
          value: record.status == 'pending-review'
    
    - name: supervisor-approve
      description: Supervisors can approve applications
      effect: allow
      principals:
        roles: [supervisor]
      actions:
        - transition
      resources:
        workflows: [de.berlin/business-registration#workflow]
        transitions: [approve, reject]
      conditions: []
    
    - name: compliance-suspend
      description: Compliance officers can suspend businesses
      effect: allow
      principals:
        roles: [compliance-officer, supervisor]
      actions:
        - transition
      resources:
        workflows: [de.berlin/business-registration#workflow]
        transitions: [suspend, reinstate]
      conditions: []
    
    - name: admin-full-access
      description: System admins have full access
      effect: allow
      principals:
        roles: [system-admin]
      actions:
        - "*"
      resources:
        - "*"
      conditions: []
    
    # Field-level access control
    - name: hide-sensitive-fields
      description: Hide tax ID from reviewers without elevated clearance
      effect: deny
      principals:
        roles: [business-reviewer]
        excludeAttributes:
          clearanceLevel: elevated
      actions:
        - read
      resources:
        models: [de.berlin/business]
        fields: [taxId]
      conditions: []
    
    # Consent-based access
    - name: federated-data-access
      description: External authorities need explicit consent
      effect: allow
      principals:
        authorities:
          - did:degov:de:federal:tax-office
          - did:degov:de:berlin:statistics
      actions:
        - read
      resources:
        models: [de.berlin/business]
      conditions:
        - type: consent
          purpose: inter-authority-data-sharing
          grantedBy: record.createdBy
    
    # Time-based access
    - name: after-hours-restriction
      description: Restrict sensitive operations to business hours
      effect: deny
      principals:
        roles: [business-reviewer]
      actions:
        - transition
      resources:
        transitions: [approve, reject]
      conditions:
        - type: expression
          value: |
            const hour = new Date().getHours();
            const day = new Date().getDay();
            return hour < 8 || hour > 18 || day == 0 || day == 6;
  
  # Default deny - if no rule matches, deny access
  default: deny
  
  # Audit configuration
  audit:
    logAllAccess: true
    logDenials: true
    sensitiveActions:
      - update
      - delete
      - transition
    retentionPeriod: P10Y
```

### 4.5 Credential Definition

**File:** `services/de/berlin/business-license/credential.yaml`

```yaml
apiVersion: degov.gov/v1
kind: Credential
metadata:
  id: de.berlin/business-license#credential  # Lexicon style with hash fragment
  title: Business License Certificate
  description: Official verifiable credential for registered businesses
  version: 1.0.0

spec:
  # W3C Verifiable Credential type
  type: BusinessLicenseCredential
  
  # Issuer information
  issuer:
    did: did:degov:de:berlin:business-office
    name: Berlin Business Registration Office
    logo: https://berlin.de/assets/logo.svg
  
  # Credential schema
  credentialSubject:
    schema:
      type: object
      properties:
        id:
          type: string
          format: did
          description: DID of the business entity
        
        legalName:
          type: string
          description: Registered business name
          source: business.legalName
        
        legalForm:
          type: string
          description: Legal form
          source: business.legalForm
        
        registrationNumber:
          type: string
          description: Official registration number
          source: business.registrationNumber
        
        registrationDate:
          type: date
          description: Date of registration approval
          source: business.updatedAt
        
        businessAddress:
          type: object
          description: Primary business location
          source: business.businessAddress
        
        industry:
          type: string
          description: Primary industry sector
          source: business.industry
        
        status:
          type: string
          description: Current registration status
          source: business.status
        
        validUntil:
          type: date
          description: Credential expiration date
          computed: |
            const registrationDate = new Date(business.updatedAt);
            registrationDate.setFullYear(registrationDate.getFullYear() + 5);
            return registrationDate.toISOString();
  
  # Issuance conditions
  issuanceConditions:
    - workflowState: approved
    - paymentStatus: completed
  
  # Cryptographic signing
  proof:
    type: Ed25519Signature2020
    keyType: authority-signing-key
    proofPurpose: assertionMethod
  
  # Revocation
  revocation:
    type: StatusList2021
    statusListUrl: https://berlin.degov.de/status-lists/business-licenses
    conditions:
      - business.status in ['suspended', 'dissolved']
  
  # Credential rendering for display
  display:
    title: "{{credentialSubject.legalName}}"
    subtitle: Business License Certificate
    backgroundColor: "#0066CC"
    textColor: "#FFFFFF"
    
    properties:
      - label: Registration Number
        path: credentialSubject.registrationNumber
        displayFormat: monospace
      
      - label: Legal Form
        path: credentialSubject.legalForm
      
      - label: Registration Date
        path: credentialSubject.registrationDate
        displayFormat: date-long
      
      - label: Status
        path: credentialSubject.status
        displayFormat: badge
      
      - label: Valid Until
        path: credentialSubject.validUntil
        displayFormat: date-long
    
    qrCode:
      enabled: true
      content: https://verify.degov.de/credentials/{{credentialSubject.id}}
  
  # Portability - how this credential can be used elsewhere
  portability:
    exportFormats:
      - json-ld
      - jwt
      - pdf
    
    shareableWith:
      - banks
      - insurance-companies
      - government-agencies
      - business-partners
```

### 4.6 Plugin Definition

**File:** `services/de/berlin/tax-calculator/plugin.yaml`

```yaml
apiVersion: degov.gov/v1
kind: Plugin
metadata:
  id: de.berlin/tax-calculator#plugin      # Lexicon style with hash fragment
  title: Business Tax Calculator
  description: Calculate estimated business taxes
  version: 1.0.0
  author:
    name: Berlin Tax Office
    email: plugins@tax.berlin.de
    did: did:degov:de:berlin:tax-office

spec:
  # Plugin runtime configuration
  runtime:
    type: wasm  # or 'javascript'
    entrypoint: dist/plugin.wasm
    sandbox: true
    maxMemory: 128MB
    maxExecutionTime: 5000ms  # 5 seconds
  
  # API endpoints this plugin exposes
  endpoints:
    - path: /api/plugins/tax-calculator/estimate
      method: POST
      handler: calculateTaxEstimate
      permissions:
        - authenticated-user
      rateLimit:
        requests: 100
        period: 1h
  
  # Workflow actions this plugin provides
  workflowActions:
    - name: calculateBusinessTax
      description: Calculate tax obligations for a business
      inputs:
        revenue:
          type: number
          required: true
        expenses:
          type: number
          required: true
        legalForm:
          type: string
          required: true
      outputs:
        estimatedTax:
          type: number
        breakdown:
          type: object
  
  # Data access requirements
  permissions:
    read:
      models: [business]
      fields: [revenue, expenses, legalForm]
    
    write:
      models: []  # Read-only plugin
  
  # Dependencies
  dependencies:
    services:
      - name: tax-rates-api
        url: https://api.tax.berlin.de/v1
        authentication: oauth2
  
  # Configuration schema
  configuration:
    schema:
      type: object
      properties:
        taxRateOverride:
          type: number
          default: null
          description: Optional tax rate override for testing
        
        enableNotifications:
          type: boolean
          default: true
          description: Send notifications on tax calculation
```

## 5. Inheritance and References

### 5.1 Model Inheritance

Models can inherit from other models using reverse DNS notation. This allows for code reuse and establishing common base models.

**Base Model Example:** `services/de/bund/person/model.yaml`

```yaml
apiVersion: degov.gov/v1
kind: DataModel
metadata:
  id: de.bund/person                        # Federal base model
  title: Natural Person
  description: Base model for representing a natural person
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
        immutable: true
        indexed: true
      
      givenName:
        type: string
        required: true
        minLength: 1
        maxLength: 100
        description: First name(s)
      
      familyName:
        type: string
        required: true
        minLength: 1
        maxLength: 100
        description: Last name
      
      birthDate:
        type: date
        required: true
        pii: true
        description: Date of birth
      
      nationality:
        type: string
        required: true
        description: Nationality (ISO 3166-1 alpha-2)
      
      residenceAddress:
        type: ref
        ref: de.bund/address
        description: Current residence address
      
      email:
        type: string
        format: email
        pii: true
        description: Contact email
      
      phone:
        type: string
        pattern: "^\\+[1-9]\\d{1,14}$"
        pii: true
        description: Contact phone (E.164 format)
      
      did:
        type: string
        format: did
        immutable: true
        indexed: true
        description: Decentralized Identifier
      
      createdAt:
        type: timestamp
        generated: true
        immutable: true
      
      updatedAt:
        type: timestamp
        generated: true
  
  computed:
    fullName:
      type: string
      script: |
        return `${record.givenName} ${record.familyName}`;
    
    age:
      type: integer
      script: |
        const now = new Date();
        const birth = new Date(record.birthDate);
        const age = now.getFullYear() - birth.getFullYear();
        const monthDiff = now.getMonth() - birth.getMonth();
        if (monthDiff < 0 || (monthDiff === 0 && now.getDate() < birth.getDate())) {
          return age - 1;
        }
        return age;
```

**Derived Model Example:** `services/de/bund/citizen/model.yaml`

```yaml
apiVersion: degov.gov/v1
kind: DataModel
metadata:
  id: de.bund/citizen
  title: German Citizen
  description: Represents a German citizen with additional citizen-specific fields
  version: 1.0.0

spec:
  # Inherit all fields from de.bund/person
  inherits:
    - de.bund/person
  
  storage:
    encrypted: true
    retention:
      duration: P100Y
  
  schema:
    type: object
    properties:
      # All fields from de.bund/person are automatically included
      
      # Additional citizen-specific fields
      citizenId:
        type: string
        pattern: "^[0-9]{11}$"
        immutable: true
        indexed: true
        pii: true
        description: German citizen identification number
      
      taxId:
        type: string
        pattern: "^[0-9]{11}$"
        encrypted: true
        pii: true
        description: Tax identification number (Steuer-ID)
      
      socialSecurityNumber:
        type: string
        pattern: "^[0-9]{12}$"
        encrypted: true
        pii: true
        description: Social security number
      
      birthPlace:
        type: object
        required: true
        pii: true
        properties:
          city:
            type: string
          country:
            type: string
            pattern: "^[A-Z]{2}$"
        description: Place of birth
      
      identityCard:
        type: ref
        ref: de.bund/identity-card
        description: Reference to citizen's identity card
      
      passport:
        type: ref
        ref: de.bund/passport
        description: Reference to citizen's passport
      
      consentRecords:
        type: array
        items:
          type: object
          properties:
            authority:
              type: string
              format: did
            purpose:
              type: string
            grantedAt:
              type: timestamp
            revokedAt:
              type: timestamp
              nullable: true
```

**Multiple Inheritance Example:**

```yaml
apiVersion: degov.gov/v1
kind: DataModel
metadata:
  id: de.berlin/business-owner
  title: Business Owner
  description: A person who owns a business
  version: 1.0.0

spec:
  # Inherit from multiple base models
  inherits:
    - de.bund/person
    - de.berlin/taxpayer-mixin
  
  schema:
    type: object
    properties:
      businesses:
        type: array
        items:
          type: ref
          ref: de.berlin/business
        description: Businesses owned by this person
      
      ownershipHistory:
        type: array
        items:
          type: object
          properties:
            business:
              type: ref
              ref: de.berlin/business
            from:
              type: timestamp
            to:
              type: timestamp
              nullable: true
            percentage:
              type: number
```

### 5.2 References (Refs)

References create relationships between models, similar to AT Protocol's ref system. They enable graph-like data structures while maintaining data integrity.

#### 5.2.1 Single Reference

```yaml
businessAddress:
  type: ref
  ref: de.bund/address              # Reference to a single address record
  required: true
  description: Primary business location
```

#### 5.2.2 Array of References

```yaml
owners:
  type: array
  items:
    type: ref
    ref: de.bund/person             # Array of references to person records
  minItems: 1
  maxItems: 10
  description: Business owners
```

#### 5.2.3 Reference with Additional Properties

```yaml
ownershipDetails:
  type: array
  items:
    type: object
    properties:
      person:
        type: ref
        ref: de.bund/person         # Reference to person
      percentage:
        type: number
        min: 0
        max: 100
      startDate:
        type: date
      role:
        type: enum
        values: [ceo, cfo, shareholder, board-member]
```

#### 5.2.4 Polymorphic References

References can point to multiple types using union types:

```yaml
applicant:
  type: ref
  ref:
    oneOf:
      - de.bund/person              # Can be a person
      - de.berlin/business          # Or a business entity
  required: true
  description: Who is applying for this permit
```

#### 5.2.5 Nested References

```yaml
# In de.berlin/building-permit model
property:
  type: ref
  ref: de.berlin/property
  description: The property for which the permit is requested

# The de.berlin/property model contains:
owner:
  type: ref
  ref: de.bund/person
  description: Property owner

# This creates: building-permit -> property -> person (nested refs)
```

#### 5.2.6 Reference Resolution

When querying, you can control reference resolution depth:

```javascript
// Shallow - only returns reference IDs
const business = await db.get('de.berlin/business', businessId, {
  resolveRefs: false
});
// business.owners = ['did:degov:123...', 'did:degov:456...']

// Deep - resolves one level
const business = await db.get('de.berlin/business', businessId, {
  resolveRefs: true,
  depth: 1
});
// business.owners = [{ givenName: 'John', familyName: 'Doe', ... }, ...]

// Nested - resolves multiple levels
const permit = await db.get('de.berlin/building-permit', permitId, {
  resolveRefs: true,
  depth: 2
});
// permit.property.owner = { givenName: 'Jane', familyName: 'Smith', ... }
```

#### 5.2.7 Reference Validation

References are validated at runtime:

```yaml
# Ensures the referenced record exists and is accessible
owner:
  type: ref
  ref: de.bund/person
  validation:
    exists: true                    # Referenced record must exist
    permissions: read               # Current user must have read permission
    conditions:
      - record.age >= 18            # Additional validation on referenced record
```

#### 5.2.8 Bidirectional References

Define relationships in both directions:

```yaml
# In de.berlin/business model
owners:
  type: array
  items:
    type: ref
    ref: de.bund/person
  inverseName: ownedBusinesses     # Creates reverse relationship

# In de.bund/person model (automatically computed)
# ownedBusinesses is available as a computed field that finds
# all businesses referencing this person
```

### 5.3 Reference Examples

**Address Model:** `services/de/bund/address/model.yaml`

```yaml
apiVersion: degov.gov/v1
kind: DataModel
metadata:
  id: de.bund/address
  title: Address
  description: Standard German address format
  version: 1.0.0

spec:
  schema:
    type: object
    properties:
      id:
        type: string
        format: uuid
        generated: true
      
      street:
        type: string
        required: true
        description: Street name and number
      
      additionalInfo:
        type: string
        description: Additional address line (e.g., apartment number)
      
      postalCode:
        type: string
        required: true
        pattern: "^[0-9]{5}$"
        description: German postal code (5 digits)
      
      city:
        type: string
        required: true
        description: City name
      
      state:
        type: enum
        required: true
        values:
          - Baden-Württemberg
          - Bayern
          - Berlin
          - Brandenburg
          - Bremen
          - Hamburg
          - Hessen
          - Mecklenburg-Vorpommern
          - Niedersachsen
          - Nordrhein-Westfalen
          - Rheinland-Pfalz
          - Saarland
          - Sachsen
          - Sachsen-Anhalt
          - Schleswig-Holstein
          - Thüringen
        description: German state
      
      country:
        type: string
        default: DE
        pattern: "^[A-Z]{2}$"
        description: ISO 3166-1 alpha-2 country code
      
      coordinates:
        type: object
        properties:
          latitude:
            type: number
          longitude:
            type: number
        description: GPS coordinates
  
  computed:
    fullAddress:
      type: string
      script: |
        let addr = record.street;
        if (record.additionalInfo) addr += `, ${record.additionalInfo}`;
        addr += `, ${record.postalCode} ${record.city}`;
        return addr;
```

## 6. Advanced Features

### 6.1 Variable Interpolation

Variables can be interpolated using `${VAR_NAME}` or `{{path.to.value}}` syntax:

```yaml
message: "Hello {{citizen.firstName}}, your application ${applicationId} is ready."
url: "${API_BASE_URL}/businesses/{{business.id}}"
```

### 6.2 Conditional Logic

```yaml
conditions:
  - type: expression
    value: |
      if (business.revenue > 1000000) {
        return business.legalForm == 'AG' || business.legalForm == 'GmbH';
      }
      return true;
```

### 6.3 Multi-Language Support

```yaml
title:
  de: Gewerberegistrierung
  en: Business Registration
  tr: İşletme Kaydı

description:
  de: Registrieren Sie Ihr neues Unternehmen
  en: Register your new business
  tr: Yeni işletmenizi kaydedin
```

### 6.4 YAML Anchors and Reuse

```yaml
# Define reusable components
definitions:
  addressSchema: &addressSchema
    type: object
    properties:
      street:
        type: string
      city:
        type: string
      postalCode:
        type: string
        pattern: "^[0-9]{5}$"

# Use the reference
businessAddress:
  <<: *addressSchema
  required: true
```

### 6.5 Validation Functions

Built-in validation functions:

- `required`: Field must have a value
- `minLength(n)`, `maxLength(n)`: String length constraints
- `min(n)`, `max(n)`: Numeric range constraints
- `pattern(regex)`: Regex matching
- `email`, `url`, `uuid`, `did`: Format validators
- `dateRange(min, max)`: Date constraints
- `custom(script)`: Custom validation logic

### 6.6 Computed Fields

```yaml
computed:
  fullAddress:
    type: string
    script: |
      return `${record.street}, ${record.postalCode} ${record.city}`;
  
  isExpired:
    type: boolean
    script: |
      return new Date(record.validUntil) < new Date();
```

## 7. JavaScript Sandbox API

Custom scripts have access to a secure, sandboxed API:

### 7.1 Database Operations

```javascript
// Query records
const businesses = await db.query('de.berlin/business', {
  status: 'active',
  industry: 'technology'
});

// Get single record
const business = await db.get('de.berlin/business', businessId);

// Get with reference resolution
const business = await db.get('de.berlin/business', businessId, {
  resolveRefs: true,
  depth: 2  // Resolve nested references
});

// Create record
const newBusiness = await db.create('de.berlin/business', {
  legalName: 'Tech Startup GmbH',
  legalForm: 'GmbH',
  businessAddress: addressId,  // Reference to de.bund/address
  owners: [personId1, personId2]  // References to de.bund/person
});

// Update record
await db.update('de.berlin/business', businessId, {
  status: 'active'
});

// Count records
const count = await db.count('de.berlin/business', { status: 'pending' });
```

### 7.2 Cryptographic Operations

```javascript
// Hash data
const hash = await crypto.hash('sha256', data);

// Sign data
const signature = await crypto.sign(data, privateKey);

// Verify signature
const isValid = await crypto.verify(data, signature, publicKey);

// Generate DID
const did = await crypto.generateDID();
```

### 7.3 Notifications

```javascript
// Send email
await notify.email({
  to: citizen.email,
  subject: 'Application Approved',
  template: 'application-approved',
  data: { businessName: business.legalName }
});

// Send SMS
await notify.sms({
  to: citizen.phone,
  message: 'Your application has been approved'
});
```

### 7.4 Federated Operations

```javascript
// Request data from another authority
const taxRecord = await federated.request({
  authority: 'did:degov:de:federal:tax-office',
  service: 'tax-verification',
  data: { taxId: business.taxId }
});

// Notify another authority
await federated.notify({
  authority: 'did:degov:de:berlin:statistics',
  event: 'business-registered',
  data: { businessId: business.id }
});
```

### 7.5 Consent Management

```javascript
// Check if consent exists
const hasConsent = await consent.check({
  citizen: citizen.did,
  purpose: 'tax-verification',
  authority: 'did:degov:de:federal:tax-office'
});

// Request consent
await consent.request({
  citizen: citizen.did,
  purpose: 'background-check',
  authority: 'did:degov:de:police',
  explanation: 'Required for business license verification'
});
```

### 7.6 Credential Operations

```javascript
// Issue a credential
const credential = await credentials.issue({
  type: 'business-license',
  subject: business.id,
  claims: {
    registrationNumber: business.registrationNumber,
    legalName: business.legalName
  }
});

// Verify a credential
const isValid = await credentials.verify(credentialJWT);

// Revoke a credential
await credentials.revoke(credentialId, 'business-dissolved');
```

## 8. Validation and Type Safety

### 8.1 Schema Validation

The DeGov CLI provides validation tools:

```bash
# Validate a single model definition
degov validate services/de/berlin/business/model.yaml

# Validate entire service directory
degov validate services/de/berlin/business-registration/

# Validate by ID (NSID)
degov validate de.berlin/business-registration

# Validate specific type
degov validate de.berlin/business-registration#workflow

# Check for breaking changes
degov validate --check-compatibility de.berlin/business-registration --against v1.0.0
```

### 8.2 Type Checking

The system performs static type checking on:

- Field types and constraints
- Workflow state transitions
- Permission rule expressions
- JavaScript snippets (basic checks)

### 8.3 Linting

```bash
# Lint YAML files for common issues
degov lint services/

# Auto-fix common issues
degov lint --fix services/
```

## 9. Testing

### 9.1 Test Definitions

**File:** `services/de/berlin/business-registration/tests/workflow.test.yaml`

```yaml
apiVersion: degov.gov/v1
kind: Test
metadata:
  id: de.berlin/business-registration#test
  description: Test business registration workflow

spec:
  service: de.berlin/business-registration
  
  tests:
    - name: successful-registration
      description: Test complete registration flow
      steps:
        - action: create
          model: de.berlin.business
          data:
            legalName: Test GmbH
            legalForm: GmbH
            foundingDate: "2025-01-15"
            businessAddress:
              ref: de.bund.address
              data:
                street: Alexanderplatz 1
                postalCode: "10178"
                city: Berlin
                state: Berlin
                country: DE
            owners:
              - ref: de.bund.person
                data:
                  givenName: Max
                  familyName: Mustermann
                  birthDate: "1985-05-15"
                  nationality: DE
          as: applicant-user
          expect:
            status: 201
            record.status: draft
        
        - action: transition
          workflow: de.berlin/business-registration#workflow
          transition: submitForReview
          as: applicant-user
          expect:
            status: 200
            record.status: pending-review
        
        - action: transition
          workflow: de.berlin/business-registration#workflow
          transition: approve
          as: reviewer-user
          expect:
            status: 200
            record.status: approved
            record.registrationNumber: matches("^HRB-\\d{6}$")
        
        - action: verify
          credential: de.berlin/business-license#credential
          expect:
            issued: true
            valid: true
    
    - name: rejection-flow
      description: Test application rejection
      steps:
        - action: create
          model: de.berlin.business
          data:
            legalName: Invalid Business
            legalForm: GmbH
          as: applicant-user
        
        - action: transition
          transition: submitForReview
          as: applicant-user
        
        - action: transition
          transition: reject
          as: supervisor-user
          comment: Missing required documentation
          expect:
            status: 200
            record.status: rejected
    
    - name: permission-check
      description: Test that citizens cannot approve their own applications
      steps:
        - action: create
          model: de.berlin.business
          data:
            legalName: Test Business
            legalForm: GmbH
          as: applicant-user
        
        - action: transition
          transition: submitForReview
          as: applicant-user
        
        - action: transition
          transition: approve
          as: applicant-user  # Same user trying to approve
          expect:
            status: 403
            error: permission-denied
    
    - name: reference-resolution
      description: Test that references are properly resolved
      steps:
        - action: get
          model: de.berlin.business
          id: ${testBusinessId}
          resolveRefs: true
          depth: 2
          expect:
            status: 200
            record.owners[0].givenName: exists
            record.businessAddress.city: exists
```

### 9.2 Running Tests

```bash
# Run all tests for a service (by NSID)
degov test de.berlin/business-registration

# Run all tests in a directory
degov test services/de/berlin/business-registration/

# Run specific test file
degov test services/de/berlin/business-registration/tests/workflow.test.yaml

# Run specific test by NSID
degov test de.berlin/business-registration#test

# Run with coverage report
degov test --coverage de.berlin/business-registration

# Run tests with verbose output
degov test --verbose de.berlin/business-registration

# Run tests in watch mode (re-run on file changes)
degov test --watch services/de/berlin/
```

## 10. Deployment

### 10.1 Deployment Configuration

**File:** `deployment.yaml`

```yaml
apiVersion: degov.gov/v1
kind: Deployment
metadata:
  id: de.berlin/deployment#production
  environment: production

spec:
  authority:
    did: did:degov:de:berlin:business-office
  
  services:
    - id: de.berlin/business-registration
      version: 1.0.0
      enabled: true
    - id: de.berlin/building-permit
      version: 2.1.0
      enabled: true
    - id: de.berlin/tax-calculator#plugin
      version: 1.3.0
      enabled: true
  
  infrastructure:
    foundationdb:
      clusterFile: /etc/fdb/fdb.cluster
      encryption:
        enabled: true
        keyProvider: hsm
    
    network:
      p2p:
        port: 4001
        peers:
          - did:degov:de:federal:bsi
          - did:degov:de:berlin:statistics
      
      api:
        host: 0.0.0.0
        port: 8080
        tls:
          enabled: true
          certPath: /etc/ssl/certs/api.crt
          keyPath: /etc/ssl/private/api.key
  
  scaling:
    apiServers:
      min: 2
      max: 10
      targetCPU: 70
    
    workerProcesses:
      min: 2
      max: 20
  
  monitoring:
    prometheus:
      enabled: true
      port: 9090
    
    logging:
      level: info
      format: json
      outputs:
        - stdout
        - file:///var/log/degov/app.log
  
  backup:
    schedule: "0 2 * * *"  # Daily at 2 AM
    retention: P90D
    destination: s3://backups.berlin.degov.de/
```

### 10.2 Deployment Commands

```bash
# Deploy service by NSID
degov deploy de.berlin/business-registration --environment production

# Deploy from directory
degov deploy services/de/berlin/business-registration/ --environment production

# Deploy multiple services
degov deploy de.berlin/business-registration de.berlin/building-permit --environment production

# Rollback to previous version
degov rollback de.berlin/business-registration --environment production

# Check deployment status
degov status de.berlin/business-registration --environment production

# List all deployed services
degov list --environment production

# Compare deployed version with local
degov diff de.berlin/business-registration --environment production
```

## 11. Migration and Versioning

### 11.1 Schema Migrations

**File:** `services/de/berlin/business/migrations/001-add-industry-field.yaml`

```yaml
apiVersion: degov.gov/v1
kind: Migration
metadata:
  id: de.berlin/business#migration-001
  description: Add industry field to business model
  version: 001
  timestamp: 2025-09-30T10:00:00Z

spec:
  model: de.berlin/business
  
  up:
    - action: addField
      name: industry
      type: string
      default: "other"
      required: false
    
    - action: createIndex
      name: by-industry
      fields: [industry, status]
  
  down:
    - action: dropIndex
      name: by-industry
    
    - action: removeField
      name: industry
  
  dataTransformation:
    script: |
      // Infer industry from business name if possible
      const keywords = {
        technology: ['tech', 'software', 'IT', 'digital'],
        retail: ['shop', 'store', 'market'],
        hospitality: ['hotel', 'restaurant', 'café']
      };
      
      for (const [industry, terms] of Object.entries(keywords)) {
        for (const term of terms) {
          if (record.legalName.toLowerCase().includes(term)) {
            return { industry };
          }
        }
      }
      
      return { industry: 'other' };
```

### 11.2 Running Migrations

```bash
# Run pending migrations for all models
degov migrate up

# Run migrations for specific model
degov migrate up de.berlin/business

# Rollback last migration
degov migrate down de.berlin/business

# Rollback to specific version
degov migrate to de.berlin/business --version 003

# Check migration status
degov migrate status

# Check status for specific model
degov migrate status de.berlin/business

# Generate migration from model changes
degov migrate generate de.berlin/business --description "Add industry field"
```

## 12. Best Practices

### 12.1 Organizing Files

- **Follow reverse DNS structure**: Use `de/bund/`, `de/berlin/`, etc. for folder hierarchy
- **One model per directory**: Each model gets its own directory with `model.yaml`
- **Use meaningful NSIDs**: `de.berlin/business-registration`, not `service-1`
- **Use hash fragments for types**: `#workflow`, `#permissions`, `#credential`, `#plugin`
- **Version your schemas**: Include version in metadata
- **Document everything**: Add descriptions to all fields
- **Group related models**: Keep federal base models in `de/bund/`, derived models in state/city directories
- **Consistent naming**: Use kebab-case for entity names (e.g., `building-permit`, not `buildingPermit`)

### 12.2 Security

- **Minimize permissions**: Grant only necessary access
- **Encrypt sensitive data**: Use `encrypted: true` for PII
- **Validate all inputs**: Use both schema and custom validations
- **Audit critical actions**: Enable audit logging for sensitive operations
- **Sandbox custom code**: Always run JavaScript in sandboxed environment

### 12.3 Performance

- **Index frequently queried fields**: Add indexes for common query patterns
- **Limit computed fields**: Only compute what's necessary
- **Use pagination**: For large result sets
- **Cache static data**: Use computed fields for derived data
- **Optimize workflows**: Minimize state transitions

### 12.4 Maintainability

- **Keep scripts small**: Extract complex logic to plugins
- **Use references**: DRY principle for reusable components
- **Test thoroughly**: Write tests for all workflows
- **Version carefully**: Use semantic versioning
- **Document changes**: Maintain changelog

## 13. Examples

See the `examples/` directory for complete, working examples:

- `examples/business-registration/` - Complete business registration service
- `examples/building-permit/` - Building permit application workflow
- `examples/identity-verification/` - Citizen identity verification
- `examples/marriage-certificate/` - Marriage certificate issuance

## 14. Tooling

### 14.1 CLI Commands

```bash
# Initialize new service
degov init service de.berlin/my-service
degov init model de.berlin/my-model

# Generate boilerplate (using NSIDs)
degov generate model de.bund/citizen
degov generate workflow de.berlin/approval-process#workflow
degov generate credential de.berlin/license#credential
degov generate permissions de.berlin/my-service#permissions

# Scaffold entire service with models, workflows, permissions
degov scaffold service de.berlin/building-permit \
  --models property,owner,contractor \
  --workflow permit-approval \
  --credential building-permit-certificate

# Validate and lint
degov validate de.berlin/business
degov validate de.berlin/business-registration#workflow
degov validate services/de/berlin/
degov lint services/

# Test
degov test de.berlin/business-registration
degov test de.berlin/business-registration#test
degov test services/de/berlin/my-service/
degov test --watch services/de/berlin/

# Deploy
degov deploy de.berlin/my-service --environment staging
degov deploy de.berlin/my-service --environment production

# Monitor
degov logs de.berlin/my-service --follow
degov metrics de.berlin/my-service
degov status de.berlin/my-service

# Explore and query
degov list models  # List all models
degov list services  # List all services
degov show de.berlin/business  # Show model details
degov show de.berlin/business-registration#workflow  # Show specific type
degov graph de.berlin/business  # Visualize model relationships

# Import/Export
degov export de.berlin/business-registration --format json > export.json
degov import export.json --dry-run  # Preview import
degov import export.json  # Actually import
```

### 14.2 Visual Editor

The low-code builder provides a visual interface for creating and editing YAML definitions:

```bash
# Start visual editor
degov admin serve

# Opens browser at http://localhost:3000
```

Features:
- Drag-and-drop workflow designer
- Visual data model builder
- Permission rule editor
- Live validation and preview
- Export to YAML

## 15. Relationship Between IDs and DIDs

The DSL uses two types of identifiers that work together:

### 15.1 NSIDs (Namespaced Identifiers)

**Purpose:** Namespace and identify data models, services, workflows, etc. using AT Protocol Lexicon format

**Format:** `de.berlin/business-registration` or `de.berlin/business-registration#workflow`

**Usage:** Internal references within the DSL

```yaml
# Referencing a model
model: de.berlin/business

# Inheriting from a model
inherits:
  - de.bund/person

# Referencing a workflow
workflow: de.berlin/business-registration#workflow

# Referencing a credential
credential: de.berlin/business-license#credential
```

### 15.2 Decentralized Identifiers (DIDs)

**Purpose:** Identify actors (authorities, citizens) in the decentralized network

**Format:** `did:degov:de:berlin:business-office`

**Usage:** Authentication, authorization, and federated communication

```yaml
# Authority identification
authority:
  did: did:degov:de:berlin:business-office

# User identification (in records)
createdBy:
  type: string
  format: did
  description: DID of the user who created this record
```

### 15.3 Mapping Between NSIDs and DIDs

The system maintains a registry mapping NSIDs to authority DIDs:

```yaml
# Registry example
de.berlin/business-registration:
  authority: did:degov:de:berlin:business-office
  endpoint: https://api.berlin.degov.de
  publicKey: ...

de.bund/identity-card:
  authority: did:degov:de:bund:bsi
  endpoint: https://api.bsi.bund.de
  publicKey: ...
```

This enables:
- **Model ownership verification**: Only `did:degov:de:berlin:business-office` can modify `de.berlin/*` models
- **Federated queries**: Resolve model data from the owning authority
- **Trust establishment**: Verify cryptographic signatures on models and data

## 16. Complete Example: Building Permit Service

Here's a complete example showing how all the pieces fit together:

**Folder Structure:**
```
services/de/berlin/building-permit/
├── service.yaml
├── model.yaml
├── workflow.yaml
├── permissions.yaml
├── credential.yaml
└── tests/
    └── workflow.test.yaml
```

**service.yaml:**
```yaml
apiVersion: degov.gov/v1
kind: Service
metadata:
  id: de.berlin/building-permit
  title: Building Permit Application
  version: 1.0.0
  authority:
    did: did:degov:de:berlin:building-dept

spec:
  models:
    - de.berlin/building-permit-application
    - de.berlin/property
  workflows:
    - de.berlin/building-permit#workflow
  credentials:
    - de.berlin/building-permit#credential
```

**model.yaml:**
```yaml
apiVersion: degov.gov/v1
kind: DataModel
metadata:
  id: de.berlin/building-permit-application
  title: Building Permit Application
  version: 1.0.0

spec:
  inherits:
    - de.bund/application-base
  
  storage:
    encrypted: true
    retention:
      duration: P30Y
  
  schema:
    type: object
    properties:
      applicant:
        type: ref
        ref: de.bund.person
        required: true
      
      property:
        type: ref
        ref: de.berlin.property
        required: true
      
      projectType:
        type: enum
        values: [new-construction, renovation, extension, demolition]
        required: true
      
      description:
        type: string
        maxLength: 5000
        required: true
      
      estimatedCost:
        type: number
        min: 0
        required: true
      
      plans:
        type: array
        items:
          type: object
          properties:
            fileId:
              type: string
            fileName:
              type: string
            uploadedAt:
              type: timestamp
      
      status:
        type: enum
        values: [draft, submitted, under-review, approved, rejected, withdrawn]
        default: draft
        indexed: true
```

**Quick Implementation Script:**
```bash
# Create the service structure
degov scaffold service de.berlin/building-permit \
  --models property,building-permit-application \
  --workflow permit-approval \
  --credential building-permit-certificate

# Validate the generated files
degov validate de.berlin/building-permit

# Run tests
degov test de.berlin/building-permit

# Deploy to staging
degov deploy de.berlin/building-permit --environment staging

# After testing, deploy to production
degov deploy de.berlin/building-permit --environment production
```

## 17. Quick Reference

### 17.1 Common Field Types

| Type | Description | Example |
|------|-------------|---------|
| `string` | Text field | `"Max Mustermann"` |
| `number` | Numeric value | `42`, `3.14` |
| `integer` | Whole number | `42` |
| `boolean` | True/false | `true` |
| `date` | ISO 8601 date | `"2025-01-15"` |
| `timestamp` | ISO 8601 datetime | `"2025-01-15T10:30:00Z"` |
| `enum` | Fixed set of values | `values: [active, inactive]` |
| `array` | List of items | `items: { type: string }` |
| `object` | Nested structure | `properties: { ... }` |
| `ref` | Reference to another model | `ref: de.bund.person` |
| `uuid` | UUID format | `format: uuid` |
| `email` | Email format | `format: email` |
| `did` | Decentralized ID | `format: did` |

### 17.2 Common Field Properties

| Property | Description | Example |
|----------|-------------|---------|
| `required` | Field must be present | `required: true` |
| `default` | Default value | `default: active` |
| `immutable` | Cannot be changed after creation | `immutable: true` |
| `indexed` | Create database index | `indexed: true` |
| `encrypted` | Encrypt at rest | `encrypted: true` |
| `pii` | Personally identifiable information | `pii: true` |
| `generated` | Auto-generated by system | `generated: true` |
| `nullable` | Can be null | `nullable: true` |
| `pattern` | Regex validation | `pattern: "^[0-9]{5}$"` |
| `minLength` / `maxLength` | String length | `minLength: 1, maxLength: 100` |
| `min` / `max` | Numeric range | `min: 0, max: 100` |

### 17.3 Workflow State Types

| Type | Purpose | Example Use Case |
|------|---------|------------------|
| `user-input` | Awaiting user action | Draft application |
| `automated` | System processes automatically | Auto-approval checks |
| `operational` | Active/ongoing state | Active business |
| `restricted` | Limited functionality | Suspended account |
| `terminal` | Final state, no transitions out | Rejected, Dissolved |

### 17.4 Permission Effects

| Effect | Meaning |
|--------|---------|
| `allow` | Grant access if conditions match |
| `deny` | Deny access if conditions match (overrides allow) |

### 17.5 Common CLI Commands

```bash
# Development
degov init service de.city/my-service
degov generate model de.city/my-model
degov validate de.city/my-service
degov test de.city/my-service
degov test --watch services/de/city/

# Deployment
degov deploy de.city/my-service --environment production
degov status de.city/my-service --environment production
degov rollback de.city/my-service --environment production

# Exploration
degov list models
degov show de.city/my-model
degov show de.city/my-service#workflow
degov graph de.city/my-model

# Migrations
degov migrate generate de.city/my-model
degov migrate up de.city/my-model
degov migrate status
```

### 17.6 Naming Conventions Summary (NSIDs)

| Scope | Pattern | Example |
|-------|---------|---------|
| Federal | `de.bund/{name}` | `de.bund/person` |
| State | `de.{state}/{name}` | `de.bayern/business` |
| City | `de.{city}/{name}` | `de.berlin/building-permit` |
| Service | `de.{authority}/{service-name}` | `de.berlin/business-registration` |
| Workflow | `de.{authority}/{service}#workflow` | `de.berlin/business-registration#workflow` |
| Permissions | `de.{authority}/{service}#permissions` | `de.berlin/business-registration#permissions` |
| Credential | `de.{authority}/{credential}#credential` | `de.berlin/business-license#credential` |
| Plugin | `com.{company}/{plugin}#plugin` | `com.example/tax-calculator#plugin` |
| Migration | `de.{authority}/{model}#migration-{n}` | `de.berlin/business#migration-001` |
| Test | `de.{authority}/{service}#test` | `de.berlin/business-registration#test` |

### 17.7 Reference Resolution Examples

```javascript
// No resolution - returns IDs only
db.get('de.berlin/business', id, { resolveRefs: false })
// { owners: ['did:degov:...', 'did:degov:...'] }

// Shallow resolution - one level
db.get('de.berlin/business', id, { resolveRefs: true, depth: 1 })
// { owners: [{ givenName: 'John', ... }, { givenName: 'Jane', ... }] }

// Deep resolution - multiple levels
db.get('de.berlin/building-permit', id, { resolveRefs: true, depth: 2 })
// { property: { owner: { givenName: 'John', ... } } }
```

## 18. Conclusion

This YAML-based DSL provides a powerful, declarative way to define government services without writing code. It balances:

- **Simplicity**: Non-technical administrators can understand and modify definitions
- **Flexibility**: JavaScript extensions for complex logic
- **Security**: Built-in permission system and sandboxing
- **Interoperability**: Standard formats and federated architecture
- **Scalability**: Designed for high-availability, production deployments
- **AT Protocol Compatibility**: Reverse DNS naming and reference system

The DSL is the foundation of the DeGov framework, enabling rapid development and deployment of secure, citizen-centric government services.

### Key Features Recap:

1. **AT Proto Lexicon Naming** (`de.berlin/business#workflow`) - Clear namespacing, ownership, and type distinction
2. **NSIDs with Hash Fragments** - Separate concerns with `#workflow`, `#permissions`, `#credential`, etc.
3. **Inheritance** - Reuse common models across authorities
4. **References** - Create relationships between models
5. **Workflows** - Define multi-step processes with state machines
6. **Permissions** - Granular, attribute-based access control
7. **Credentials** - Issue verifiable certificates to citizens
8. **Federation** - Secure inter-authority data sharing
9. **Type Safety** - Comprehensive validation and type checking
10. **Extensibility** - JavaScript sandbox for custom logic
11. **Tooling** - Rich CLI and visual editor for development

For more examples and detailed documentation, see:
- `examples/` directory for complete working services
- `docs/api/` for REST API documentation
- `docs/guides/` for step-by-step tutorials
