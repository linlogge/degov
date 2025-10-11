use std::sync::Arc;
use degov_core::did::DIDBuf;
use degov_engine::WorkflowEngine;

mod error;
mod services;

pub use error::ServerError;
pub use services::WorkflowService;

/// Application state containing all services and components
/// This is the core state that holds all business logic components
pub struct AppState {
    pub engine: Arc<WorkflowEngine>,
    pub did: DIDBuf,
}

impl AppState {
    pub async fn new(did: String) -> Result<Self, Box<dyn std::error::Error>> {
        let did = DIDBuf::from_string(did)?;
        
        // Initialize database and workflow engine
        let db = foundationdb::Database::from_path("/usr/local/etc/foundationdb/fdb.cluster")?;
        let engine_addr = "127.0.0.1:8080".parse()?;
        let engine = Arc::new(WorkflowEngine::new(db, engine_addr).await?);
        
        Ok(Self {
            engine,
            did,
        })
    }
}

/// Main server state with all services
/// This is what should be passed to the API layer
pub struct Server {
    state: Arc<AppState>,
    workflow_service: Arc<WorkflowService>,
    network: foundationdb::api::NetworkAutoStop,
}

impl Server {
    pub async fn new<T: Into<String>>(did: T) -> Result<Self, Box<dyn std::error::Error>> {
        let network = unsafe { foundationdb::boot() };
        
        let state = Arc::new(AppState::new(did.into()).await?);
        let workflow_service = Arc::new(WorkflowService::new(state.clone()));
        
        Ok(Self { state, workflow_service, network })
    }

    /// Get the workflow service for API handlers
    pub fn workflow_service(&self) -> Arc<WorkflowService> {
        self.workflow_service.clone()
    }

    /// Get the application state
    pub fn state(&self) -> Arc<AppState> {
        self.state.clone()
    }
}

