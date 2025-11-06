use std::path::{Path, PathBuf};
use dgv_agora_build::AppBuilder;
use dgv_core::v1::service::{ServiceBuild, RustBuild};
use miette::{IntoDiagnostic, Result};
use std::borrow::Cow;

/// Handle the build command
pub async fn handle_build_command(path: PathBuf) -> Result<()> {
    // Check if path exists
    if !path.exists() {
        return Err(miette::miette!("Path does not exist: {}", path.display()));
    }

    // Determine if it's a file or directory
    let service_builds = if path.is_file() {
        // Single file - parse it (for now, create fake structs)
        vec![create_fake_service_build_from_file(&path)?]
    } else {
        // Directory - find all .dgl files and create fake structs
        find_and_create_fake_service_builds(&path)?
    };

    if service_builds.is_empty() {
        return Err(miette::miette!(
            "No service files found in: {}",
            path.display()
        ));
    }

    let count = service_builds.len();

    // Build all services concurrently
    let mut builder = AppBuilder::new();
    for (name, build) in service_builds {
        builder.add_service(name, build);
    }

    println!("Building {} service(s)...", count);
    let results = builder.build_all().await.into_diagnostic()?;

    // Report results
    let mut success_count = 0;
    let mut fail_count = 0;

    for result in results {
        if result.success {
            success_count += 1;
            println!("✓ Successfully built: {}", result.service_name);
            if let Some(output_path) = &result.output_path {
                println!("  Output: {}", output_path.display());
            }
        } else {
            fail_count += 1;
            eprintln!("✗ Failed to build: {}", result.service_name);
            if !result.stderr.is_empty() {
                eprintln!("  Error: {}", result.stderr);
            }
        }
    }

    println!("\nBuild summary: {} succeeded, {} failed", success_count, fail_count);

    if fail_count > 0 {
        return Err(miette::miette!("Build failed for {} service(s)", fail_count));
    }

    Ok(())
}

/// Create a fake ServiceBuild from a DGL file (temporary until parsing is implemented)
fn create_fake_service_build_from_file(file_path: &Path) -> Result<(String, ServiceBuild<'static>)> {
    // For now, create a fake service build
    // TODO: Parse the DGL file to extract actual service information
    
    // Extract service name from file path (without extension)
    let service_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown-service")
        .to_string();

    // Get the directory containing the file
    let base_dir = file_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Create a fake Rust build pointing to a common app directory
    // In the real implementation, this would be parsed from the DGL file
    let rust_build = RustBuild {
        path: Some(Cow::Owned(base_dir.join("app"))),
        target: Some(Cow::Owned("wasm32-wasip2".to_string())),
    };

    let service_build = ServiceBuild::Rust(rust_build);

    Ok((service_name, service_build))
}

/// Find all .dgl files in a directory and create fake service builds
fn find_and_create_fake_service_builds(dir_path: &Path) -> Result<Vec<(String, ServiceBuild<'static>)>> {
    let mut service_builds = Vec::new();

    // Look for .dgl files in the directory
    let entries = std::fs::read_dir(dir_path).into_diagnostic()?;

    for entry in entries {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "dgl" {
                    let (name, build) = create_fake_service_build_from_file(&path)?;
                    service_builds.push((name, build));
                }
            }
        }
    }

    Ok(service_builds)
}
