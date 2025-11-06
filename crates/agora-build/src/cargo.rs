use crate::{BuildOutput, OwnedRustBuild};
use std::path::Path;
use thiserror::Error;
use tokio::process::Command;

/// Error types for Cargo builds
#[derive(Debug, Error)]
pub enum CargoBuildError {
    #[error("Failed to execute cargo command: {0}")]
    CommandExecution(String),
    #[error("Cargo build failed with exit code {exit_code}\n\nstdout:\n{stdout}\n\nstderr:\n{stderr}")]
    BuildFailed {
        exit_code: i32,
        stdout: String,
        stderr: String,
    },
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Build a Rust service using Cargo
pub(crate) async fn build_cargo(name: &str, rust_build: &OwnedRustBuild) -> Result<BuildOutput, CargoBuildError> {
    let work_dir = rust_build
        .path
        .as_ref()
        .map(|p| p.as_path())
        .unwrap_or_else(|| Path::new("."));

    // Validate that the path exists
    if !work_dir.exists() {
        return Err(CargoBuildError::InvalidPath(format!(
            "Build path does not exist: {}",
            work_dir.display()
        )));
    }

    // Build the cargo command
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .current_dir(work_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Add target if specified
    if let Some(target) = &rust_build.target {
        cmd.arg("--target").arg(target);
        tracing::info!("Building Rust service '{}' with target: {}", name, target);
    } else {
        tracing::info!("Building Rust service '{}' in directory: {}", name, work_dir.display());
    }

    // Execute the command
    let output = cmd
        .output()
        .await
        .map_err(|e| CargoBuildError::CommandExecution(format!("Failed to spawn cargo: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let success = output.status.success();

    if !success {
        tracing::error!(
            "Cargo build failed for service '{}':\nstdout: {}\nstderr: {}",
            name,
            stdout,
            stderr
        );
        return Err(CargoBuildError::BuildFailed {
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
        });
    }

    tracing::info!("Successfully built Rust service '{}'", name);

    // Determine the output path based on target
    // Cargo uses the exact target name in the directory structure
    let output_path = if let Some(target) = &rust_build.target {
        work_dir
            .join("target")
            .join(target)
            .join("release")
            .join(name)
    } else {
        work_dir
            .join("target")
            .join("release")
            .join(name)
    };

    Ok(BuildOutput {
        service_name: name.to_string(),
        success: true,
        output_path: if output_path.exists() { Some(output_path) } else { None },
        stdout,
        stderr,
    })
}

