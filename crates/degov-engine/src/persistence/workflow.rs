//! Workflow persistence

use super::{build_key, keys};
use crate::error::{PersistenceError, PersistenceResult};
use crate::types::{WorkflowDefinition, WorkflowId, WorkflowInstance, WorkflowStatus};
use chrono::Utc;
use foundationdb::{Database, Transaction};
use std::sync::Arc;

/// Workflow storage operations
#[derive(Clone)]
pub struct WorkflowStore {
    db: Arc<Database>,
}

impl WorkflowStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a workflow definition
    pub async fn save_definition(&self, definition: &WorkflowDefinition) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        self.save_definition_tx(&tx, definition).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Save a workflow definition within a transaction
    pub async fn save_definition_tx(
        &self,
        tx: &Transaction,
        definition: &WorkflowDefinition,
    ) -> PersistenceResult<()> {
        let key = build_key(keys::WORKFLOW_DEF_PREFIX, &definition.id.to_string());
        let value = serde_json::to_vec(definition)?;
        tx.set(&key, &value);
        Ok(())
    }

    /// Get a workflow definition
    pub async fn get_definition(&self, id: &WorkflowId) -> PersistenceResult<Option<WorkflowDefinition>> {
        let tx = self.db.create_trx()?;
        let result = self.get_definition_tx(&tx, id).await?;
        tx.cancel();
        Ok(result)
    }

    /// Get a workflow definition within a transaction
    pub async fn get_definition_tx(
        &self,
        tx: &Transaction,
        id: &WorkflowId,
    ) -> PersistenceResult<Option<WorkflowDefinition>> {
        let key = build_key(keys::WORKFLOW_DEF_PREFIX, &id.to_string());
        let bytes = tx.get(&key, false).await?;
        
        match bytes {
            Some(data) => {
                let definition = serde_json::from_slice(data.as_ref())?;
                Ok(Some(definition))
            }
            None => Ok(None),
        }
    }

    /// Save a workflow instance
    pub async fn save_instance(&self, instance: &WorkflowInstance) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        self.save_instance_tx(&tx, instance).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Save a workflow instance within a transaction
    pub async fn save_instance_tx(
        &self,
        tx: &Transaction,
        instance: &WorkflowInstance,
    ) -> PersistenceResult<()> {
        let key = build_key(keys::WORKFLOW_PREFIX, &instance.id.to_string());
        let value = serde_json::to_vec(instance)?;
        tx.set(&key, &value);
        Ok(())
    }

    /// Get a workflow instance
    pub async fn get_instance(&self, id: &WorkflowId) -> PersistenceResult<Option<WorkflowInstance>> {
        let tx = self.db.create_trx()?;
        let result = self.get_instance_tx(&tx, id).await?;
        tx.cancel();
        Ok(result)
    }

    /// Get a workflow instance within a transaction
    pub async fn get_instance_tx(
        &self,
        tx: &Transaction,
        id: &WorkflowId,
    ) -> PersistenceResult<Option<WorkflowInstance>> {
        let key = build_key(keys::WORKFLOW_PREFIX, &id.to_string());
        let bytes = tx.get(&key, false).await?;
        
        match bytes {
            Some(data) => {
                let instance = serde_json::from_slice(data.as_ref())?;
                Ok(Some(instance))
            }
            None => Ok(None),
        }
    }

    /// Update workflow state
    pub async fn update_state(
        &self,
        id: &WorkflowId,
        state: &str,
        status: WorkflowStatus,
    ) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        let mut instance = self.get_instance_tx(&tx, id).await?
            .ok_or_else(|| PersistenceError::NotFound(id.to_string()))?;
        
        instance.current_state = state.to_string();
        instance.status = status;
        instance.updated_at = Utc::now();
        
        if matches!(status, WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled) {
            instance.completed_at = Some(Utc::now());
        }
        
        self.save_instance_tx(&tx, &instance).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Update workflow context data
    pub async fn update_context(
        &self,
        id: &WorkflowId,
        context: serde_json::Value,
    ) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        let mut instance = self.get_instance_tx(&tx, id).await?
            .ok_or_else(|| PersistenceError::NotFound(id.to_string()))?;
        
        instance.context = context;
        instance.updated_at = Utc::now();
        
        self.save_instance_tx(&tx, &instance).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Delete a workflow instance
    pub async fn delete_instance(&self, id: &WorkflowId) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        let key = build_key(keys::WORKFLOW_PREFIX, &id.to_string());
        tx.clear(&key);
        tx.commit().await?;
        Ok(())
    }
}


