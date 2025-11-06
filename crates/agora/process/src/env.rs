use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::{Process, Signal};

#[async_trait]
pub trait Environment: Send + Sync {
    fn id(&self) -> u64;
    fn get_next_process_id(&self) -> u64;
    fn get_process(&self, id: u64) -> Option<Arc<dyn Process>>;
    fn add_process(&self, id: u64, proc: Arc<dyn Process>);
    fn remove_process(&self, id: u64);
    fn process_count(&self) -> usize;
    async fn can_spawn_next_process(&self) -> Result<Option<()>>;
    fn send(&self, id: u64, signal: Signal);
}

#[async_trait]
pub trait Environments: Send + Sync {
    type Env: Environment;

    async fn create(&self, id: u64) -> Result<Arc<Self::Env>>;
    async fn get(&self, id: u64) -> Option<Arc<Self::Env>>;
}

#[derive(Clone)]
pub struct DegovEnvironment {
    environment_id: u64,
    next_process_id: Arc<AtomicU64>,
    processes: Arc<DashMap<u64, Arc<dyn Process>>>,
}

impl DegovEnvironment {
    pub fn new(id: u64) -> Self {
        Self {
            environment_id: id,
            processes: Arc::new(DashMap::new()),
            next_process_id: Arc::new(AtomicU64::new(1)),
        }
    }
}

#[async_trait]
impl Environment for DegovEnvironment {
    fn get_process(&self, id: u64) -> Option<Arc<dyn Process>> {
        self.processes.get(&id).map(|x| x.clone())
    }

    fn add_process(&self, id: u64, proc: Arc<dyn Process>) {
        self.processes.insert(id, proc);
    }

    fn remove_process(&self, id: u64) {
        self.processes.remove(&id);
    }

    fn process_count(&self) -> usize {
        self.processes.len()
    }

    fn send(&self, id: u64, signal: Signal) {
        if let Some(proc) = self.processes.get(&id) {
            proc.send(signal);
        }
    }

    fn get_next_process_id(&self) -> u64 {
        self.next_process_id.fetch_add(1, Ordering::Relaxed)
    }

    fn id(&self) -> u64 {
        self.environment_id
    }

    async fn can_spawn_next_process(&self) -> Result<Option<()>> {
        // Don't impose any limits to process spawning
        Ok(Some(()))
    }
}

#[derive(Clone, Default)]
pub struct DegovEnvironments {
    envs: Arc<DashMap<u64, Arc<DegovEnvironment>>>,
}

#[async_trait]
impl Environments for DegovEnvironments {
    type Env = DegovEnvironment;
    async fn create(&self, id: u64) -> Result<Arc<Self::Env>> {
        let env = Arc::new(DegovEnvironment::new(id));
        self.envs.insert(id, env.clone());
        Ok(env)
    }

    async fn get(&self, id: u64) -> Option<Arc<Self::Env>> {
        self.envs.get(&id).map(|e| e.clone())
    }
}
