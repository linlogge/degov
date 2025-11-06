mod cargo;

use crate::cargo::build_cargo;
use dgv_core::v1::service::ServiceBuild;
use std::path::PathBuf;
use thiserror::Error;

pub use cargo::CargoBuildError;

/// Error types for the application builder
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("Cargo build error: {0}")]
    Cargo(#[from] CargoBuildError),
    #[error("Build failed for service: {0}")]
    ServiceFailed(String),
}

/// Result type for build operations
pub type BuildResult<T> = Result<T, BuildError>;

/// Application builder that can build multiple services concurrently
pub struct AppBuilder {
    services: Vec<(String, OwnedServiceBuild)>,
}

/// Owned version of RustBuild for internal use
#[derive(Debug, Clone)]
pub(crate) struct OwnedRustBuild {
    pub path: Option<PathBuf>,
    pub target: Option<String>,
}

/// Owned version of ServiceBuild for storage in the builder
#[derive(Debug, Clone)]
enum OwnedServiceBuild {
    Rust(OwnedRustBuild),
}


impl<'a> From<ServiceBuild<'a>> for OwnedServiceBuild {
    fn from(build: ServiceBuild<'a>) -> Self {
        match build {
            ServiceBuild::Rust(rust_build) => OwnedServiceBuild::Rust(OwnedRustBuild {
                path: rust_build.path.map(|p| p.into_owned()),
                target: rust_build.target.map(|t| t.into_owned()),
            }),
        }
    }
}

impl AppBuilder {
    /// Create a new application builder
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    /// Add a service to be built
    pub fn add_service<'a>(&mut self, name: String, build: ServiceBuild<'a>) {
        self.services.push((name, build.into()));
    }

    /// Build all services concurrently
    pub async fn build_all(&self) -> BuildResult<Vec<BuildOutput>> {
        let mut tasks = Vec::new();

        for (name, service_build) in &self.services {
            let name = name.clone();
            let build = service_build.clone();
            tasks.push(tokio::spawn(async move {
                build_service(&name, &build).await
            }));
        }

        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(output)) => results.push(output),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(BuildError::ServiceFailed(format!("Task join error: {}", e))),
            }
        }

        Ok(results)
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Output from a build operation
#[derive(Debug, Clone)]
pub struct BuildOutput {
    pub service_name: String,
    pub success: bool,
    pub output_path: Option<PathBuf>,
    pub stdout: String,
    pub stderr: String,
}

/// Build a single service based on its build configuration
async fn build_service(name: &str, build: &OwnedServiceBuild) -> BuildResult<BuildOutput> {
    match build {
        OwnedServiceBuild::Rust(rust_build) => {
            let output = build_cargo(name, &rust_build).await?;
            Ok(output)
        }
    }
}

