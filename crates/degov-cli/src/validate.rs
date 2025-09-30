use colored::Colorize;
use degov_dsl::{Definition, Nsid, Parser};
use std::path::{Path, PathBuf};

/// Validation result for a single file or definition
#[derive(Debug)]
pub struct ValidationResult {
    pub path: Option<PathBuf>,
    pub nsid: Option<Nsid>,
    pub success: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn success(path: Option<PathBuf>, nsid: Option<Nsid>) -> Self {
        Self {
            path,
            nsid,
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn failure(path: Option<PathBuf>, nsid: Option<Nsid>, errors: Vec<String>) -> Self {
        Self {
            path,
            nsid,
            success: false,
            errors,
            warnings: Vec::new(),
        }
    }
}

/// Validate a single YAML file
pub fn validate_file(path: impl AsRef<Path>) -> ValidationResult {
    let path_buf = path.as_ref().to_path_buf();
    
    match Definition::from_file(&path_buf) {
        Ok(def) => {
            let nsid = def.metadata().id.clone();
            let mut result = ValidationResult::success(Some(path_buf.clone()), Some(nsid));
            
            // Additional validation checks
            validate_definition(&def, &mut result);
            
            result
        }
        Err(e) => {
            ValidationResult::failure(
                Some(path_buf),
                None,
                vec![format!("Parse error: {}", e)],
            )
        }
    }
}

/// Validate a definition and add any warnings
fn validate_definition(def: &Definition, result: &mut ValidationResult) {
    let metadata = def.metadata();
    
    // Check version format
    if !metadata.version.contains('.') {
        result.warnings.push(format!(
            "Version '{}' does not follow semantic versioning",
            metadata.version
        ));
    }
    
    // Check if title is empty
    if metadata.title.is_empty() {
        result.warnings.push("Title is empty".to_string());
    }
    
    // Check if description is missing
    if metadata.description.is_none() {
        result.warnings.push("Description is missing".to_string());
    }
    
    // Type-specific validations
    match def {
        Definition::Service(service) => {
            if service.spec.models.is_empty() {
                result.warnings.push("Service has no models defined".to_string());
            }
        }
        Definition::DataModel(model) => {
            if model.spec.schema.properties.is_empty() {
                result.warnings.push("DataModel has no properties defined".to_string());
            }
        }
        Definition::Workflow(workflow) => {
            if workflow.spec.states.is_empty() {
                result.errors.push("Workflow has no states defined".to_string());
                result.success = false;
            }
            if workflow.spec.transitions.is_empty() {
                result.warnings.push("Workflow has no transitions defined".to_string());
            }
        }
        Definition::Permission(permission) => {
            if permission.spec.roles.is_empty() {
                result.warnings.push("Permission has no roles defined".to_string());
            }
        }
        Definition::Credential(credential) => {
            if credential.spec.credential_subject.schema.properties.is_empty() {
                result.warnings.push("Credential has no subject properties".to_string());
            }
        }
    }
}

/// Validate all YAML files in a directory
pub fn validate_directory(path: impl AsRef<Path>) -> Vec<ValidationResult> {
    let path_ref = path.as_ref();
    let parser = Parser::new(path_ref.to_path_buf());
    let mut results = Vec::new();
    
    match parser.load_directory(path_ref) {
        Ok(definitions) => {
            for def in definitions {
                let nsid = def.metadata().id.clone();
                let mut result = ValidationResult::success(None, Some(nsid));
                validate_definition(&def, &mut result);
                results.push(result);
            }
            
            if results.is_empty() {
                results.push(ValidationResult::failure(
                    Some(path.as_ref().to_path_buf()),
                    None,
                    vec!["No valid definitions found in directory".to_string()],
                ));
            }
        }
        Err(e) => {
            results.push(ValidationResult::failure(
                Some(path.as_ref().to_path_buf()),
                None,
                vec![format!("Failed to load directory: {}", e)],
            ));
        }
    }
    
    results
}

/// Validate by NSID
pub fn validate_by_nsid(nsid: &str, root: impl AsRef<Path>) -> Vec<ValidationResult> {
    let parser = Parser::new(root.as_ref().to_path_buf());
    
    match parser.load_by_nsid_str(nsid) {
        Ok(definitions) => {
            let mut results = Vec::new();
            for def in definitions {
                let nsid = def.metadata().id.clone();
                let mut result = ValidationResult::success(None, Some(nsid));
                validate_definition(&def, &mut result);
                results.push(result);
            }
            
            if results.is_empty() {
                results.push(ValidationResult::failure(
                    None,
                    None,
                    vec![format!("No definitions found for NSID: {}", nsid)],
                ));
            }
            
            results
        }
        Err(e) => {
            vec![ValidationResult::failure(
                None,
                None,
                vec![format!("Failed to load NSID '{}': {}", nsid, e)],
            )]
        }
    }
}

/// Discover and validate all services in a directory tree
pub fn validate_all(root: impl AsRef<Path>) -> Vec<ValidationResult> {
    let parser = Parser::new(root.as_ref().to_path_buf());
    
    match parser.discover_services() {
        Ok(definitions) => {
            let mut results = Vec::new();
            for def in definitions {
                let nsid = def.metadata().id.clone();
                let mut result = ValidationResult::success(None, Some(nsid));
                validate_definition(&def, &mut result);
                results.push(result);
            }
            
            if results.is_empty() {
                results.push(ValidationResult::failure(
                    None,
                    None,
                    vec!["No definitions found".to_string()],
                ));
            }
            
            results
        }
        Err(e) => {
            vec![ValidationResult::failure(
                None,
                None,
                vec![format!("Failed to discover services: {}", e)],
            )]
        }
    }
}

/// Print validation results in a human-readable format
pub fn print_results(results: &[ValidationResult], verbose: bool) {
    let total = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let failed = total - successful;
    
    println!();
    println!("{}", "Validation Results".bold());
    println!("{}", "===================".bold());
    println!();
    
    for result in results {
        if result.success {
            let icon = "✓".green().bold();
            let path_or_nsid = result.path.as_ref()
                .map(|p| p.display().to_string())
                .or_else(|| result.nsid.as_ref().map(|n| n.to_string()))
                .unwrap_or_else(|| "<unknown>".to_string());
            
            println!("{} {}", icon, path_or_nsid.green());
            
            if verbose && !result.warnings.is_empty() {
                for warning in &result.warnings {
                    println!("  {} {}", "⚠".yellow(), warning.yellow());
                }
            }
        } else {
            let icon = "✗".red().bold();
            let path_or_nsid = result.path.as_ref()
                .map(|p| p.display().to_string())
                .or_else(|| result.nsid.as_ref().map(|n| n.to_string()))
                .unwrap_or_else(|| "<unknown>".to_string());
            
            println!("{} {}", icon, path_or_nsid.red());
            
            for error in &result.errors {
                println!("  {} {}", "✗".red(), error.red());
            }
            
            if verbose && !result.warnings.is_empty() {
                for warning in &result.warnings {
                    println!("  {} {}", "⚠".yellow(), warning.yellow());
                }
            }
        }
    }
    
    println!();
    println!("{}", "Summary".bold());
    println!("{}", "-------".bold());
    println!("Total:      {}", total);
    println!("Successful: {}", successful.to_string().green());
    
    if failed > 0 {
        println!("Failed:     {}", failed.to_string().red());
    } else {
        println!("Failed:     {}", failed);
    }
    
    println!();
    
    if failed == 0 {
        println!("{}", "All validations passed! ✓".green().bold());
    } else {
        println!("{}", format!("{} validation(s) failed", failed).red().bold());
    }
}

/// Print results in JSON format
pub fn print_results_json(results: &[ValidationResult]) {
    use std::collections::HashMap;
    
    let mut output = HashMap::new();
    output.insert("total", results.len());
    output.insert("successful", results.iter().filter(|r| r.success).count());
    output.insert("failed", results.iter().filter(|r| !r.success).count());
    
    let results_json: Vec<_> = results.iter().map(|r| {
        let mut obj = HashMap::new();
        
        if let Some(ref path) = r.path {
            obj.insert("path", path.display().to_string());
        }
        
        if let Some(ref nsid) = r.nsid {
            obj.insert("nsid", nsid.to_string());
        }
        
        obj.insert("success", if r.success { "true" } else { "false" }.to_string());
        
        if !r.errors.is_empty() {
            obj.insert("errors", r.errors.join("; "));
        }
        
        if !r.warnings.is_empty() {
            obj.insert("warnings", r.warnings.join("; "));
        }
        
        obj
    }).collect();
    
    println!("{{");
    println!("  \"summary\": {{");
    println!("    \"total\": {},", output["total"]);
    println!("    \"successful\": {},", output["successful"]);
    println!("    \"failed\": {}", output["failed"]);
    println!("  }},");
    println!("  \"results\": [");
    
    for (i, result_map) in results_json.iter().enumerate() {
        println!("    {{");
        let mut first = true;
        for (key, value) in result_map {
            if !first {
                println!(",");
            }
            print!("      \"{}\": \"{}\"", key, value);
            first = false;
        }
        println!();
        if i < results_json.len() - 1 {
            println!("    }},");
        } else {
            println!("    }}");
        }
    }
    
    println!("  ]");
    println!("}}");
}

