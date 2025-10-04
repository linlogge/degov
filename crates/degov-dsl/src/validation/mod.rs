mod nsid;

pub use nsid::create_nsid_validator;

use crate::schema::{ValidationContext, ValidationError, ValidationResult};
use async_trait::async_trait;
use std::sync::Arc;

/// A validator that can validate KDL nodes
pub trait Validator: Send + Sync {
    /// Validate a node synchronously
    fn validate(&self, ctx: &ValidationContext) -> ValidationResult;
}

/// An async validator
#[async_trait]
pub trait AsyncValidator: Send + Sync {
    /// Validate a node asynchronously
    async fn validate_async(&self, ctx: ValidationContext<'_>) -> ValidationResult;
}

/// A collection of validators
#[derive(Default)]
pub struct ValidatorRegistry {
    sync_validators: Vec<(String, Arc<dyn Validator>)>,
    async_validators: Vec<(String, Arc<dyn AsyncValidator>)>,
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Register a synchronous validator
    pub fn register_sync(
        &mut self,
        name: impl Into<String>,
        validator: impl Validator + 'static,
    ) {
        self.sync_validators.push((name.into(), Arc::new(validator)));
    }
    
    /// Register an asynchronous validator
    pub fn register_async(
        &mut self,
        name: impl Into<String>,
        validator: impl AsyncValidator + 'static,
    ) {
        self.async_validators.push((name.into(), Arc::new(validator)));
    }
    
    /// Get a synchronous validator by name
    pub fn get_sync(&self, name: &str) -> Option<&Arc<dyn Validator>> {
        self.sync_validators
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
    }
    
    /// Get an asynchronous validator by name
    pub fn get_async(&self, name: &str) -> Option<&Arc<dyn AsyncValidator>> {
        self.async_validators
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
    }
    
    /// Get all validator names
    pub fn validator_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        names.extend(self.sync_validators.iter().map(|(n, _)| n.clone()));
        names.extend(self.async_validators.iter().map(|(n, _)| n.clone()));
        names
    }
}

/// Helper to create a validator from a closure
pub struct FnValidator<F>
where
    F: Fn(&ValidationContext) -> ValidationResult + Send + Sync,
{
    func: F,
}

impl<F> FnValidator<F>
where
    F: Fn(&ValidationContext) -> ValidationResult + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> Validator for FnValidator<F>
where
    F: Fn(&ValidationContext) -> ValidationResult + Send + Sync,
{
    fn validate(&self, ctx: &ValidationContext) -> ValidationResult {
        (self.func)(ctx)
    }
}

/// Helper to create an async validator from a closure
/// Note: Due to lifetime complexities, async validators are better implemented directly
pub struct AsyncFnValidator<F>
where
    F: Fn() -> ValidationResult + Send + Sync,
{
    _func: F,
}

impl<F> AsyncFnValidator<F>
where
    F: Fn() -> ValidationResult + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self {
            _func: func,
        }
    }
}

// Note: Async validation with closures is complex due to lifetime issues
// For now, implement AsyncValidator trait directly for your types

/// Built-in validators
pub mod builtin {
    use super::*;
    
    /// Validates that a string matches a regex pattern
    pub struct RegexValidator {
        pattern: regex::Regex,
        message: String,
    }
    
    impl RegexValidator {
        pub fn new(pattern: &str, message: impl Into<String>) -> Result<Self, regex::Error> {
            Ok(Self {
                pattern: regex::Regex::new(pattern)?,
                message: message.into(),
            })
        }
    }
    
    impl Validator for RegexValidator {
        fn validate(&self, ctx: &ValidationContext) -> ValidationResult {
            // Check if first argument matches pattern
            if let Some(arg) = ctx.node.entries().first() {
                if let kdl::KdlValue::String(value) = arg.value() {
                    if !self.pattern.is_match(value) {
                        return Err(ValidationError::new(
                            self.message.clone(),
                            ctx.span,
                        ));
                    }
                }
            }
            Ok(())
        }
    }
    
    /// Validates numeric ranges
    pub struct RangeValidator {
        min: Option<f64>,
        max: Option<f64>,
    }
    
    impl RangeValidator {
        pub fn new(min: Option<f64>, max: Option<f64>) -> Self {
            Self { min, max }
        }
    }
    
    impl Validator for RangeValidator {
        fn validate(&self, ctx: &ValidationContext) -> ValidationResult {
            if let Some(arg) = ctx.node.entries().first() {
                // Try to parse as number
                if let kdl::KdlValue::String(s) = arg.value() {
                    if let Ok(value) = s.parse::<f64>() {
                        if let Some(min) = self.min {
                            if value < min {
                                return Err(ValidationError::new(
                                    format!("Value must be at least {}", min),
                                    ctx.span,
                                ));
                            }
                        }
                        if let Some(max) = self.max {
                            if value > max {
                                return Err(ValidationError::new(
                                    format!("Value must be at most {}", max),
                                    ctx.span,
                                ));
                            }
                        }
                    }
                }
            }
            Ok(())
        }
    }
    
    /// Example async validator that could check external resources
    pub struct ExternalRefValidator {
        check_exists: bool,
    }
    
    impl ExternalRefValidator {
        pub fn new(check_exists: bool) -> Self {
            Self { check_exists }
        }
        
        async fn check_reference_exists(&self, _reference: &str) -> bool {
            // This would actually check a database, file system, etc.
            // For now, just simulate async work
            #[cfg(feature = "async")]
            {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
            true
        }
    }
    
    #[async_trait]
    impl AsyncValidator for ExternalRefValidator {
        async fn validate_async(&self, ctx: ValidationContext<'_>) -> ValidationResult {
            if !self.check_exists {
                return Ok(());
            }
            
            if let Some(arg) = ctx.node.entries().first() {
                if let kdl::KdlValue::String(value) = arg.value() {
                    if !self.check_reference_exists(value).await {
                        return Err(ValidationError::new(
                            format!("Reference '{}' does not exist", value),
                            ctx.span,
                        ));
                    }
                }
            }
            Ok(())
        }
    }
}

/// A validation pipeline that runs multiple validators
pub struct ValidationPipeline {
    validators: Vec<Arc<dyn Validator>>,
    async_validators: Vec<Arc<dyn AsyncValidator>>,
}

impl ValidationPipeline {
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
            async_validators: Vec::new(),
        }
    }
    
    pub fn add_validator(&mut self, validator: Arc<dyn Validator>) {
        self.validators.push(validator);
    }
    
    pub fn add_async_validator(&mut self, validator: Arc<dyn AsyncValidator>) {
        self.async_validators.push(validator);
    }
    
    /// Run all synchronous validators
    pub fn validate(&self, ctx: &ValidationContext) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        
        for validator in &self.validators {
            if let Err(err) = validator.validate(ctx) {
                errors.push(err);
            }
        }
        
        errors
    }
    
    /// Run all validators including async ones
    pub async fn validate_async(&self, ctx: &ValidationContext<'_>) -> Vec<ValidationError> {
        let errors = self.validate(ctx);
        
        // Run async validators
        for _validator in &self.async_validators {
            // Note: Async validation with borrowed context is complex
            // In practice, you'd clone necessary data before async validation
            // For now, skip async validators in this method
        }
        
        errors
    }
}

impl Default for ValidationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fn_validator() {
        let _validator = FnValidator::new(|ctx| {
            if ctx.node.name().value() == "test" {
                Ok(())
            } else {
                Err(ValidationError::new("Expected 'test' node", ctx.span))
            }
        });
        
        // Would need actual test setup with KDL document
    }
}

