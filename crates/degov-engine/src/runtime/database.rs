use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::debug;

/// Simple in-memory key-value store
#[derive(Clone)]
pub struct KvStore {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl KvStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let data = self.data.read().ok()?;
        let result = data.get(key).cloned();
        debug!("KV GET: key={}, found={}", key, result.is_some());
        result
    }

    pub async fn set(&self, key: String, value: Vec<u8>) {
        debug!("KV SET: key={}, size={} bytes", key, value.len());
        if let Ok(mut data) = self.data.write() {
            data.insert(key, value);
        }
    }

    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.data.read().map(|d| d.len()).unwrap_or(0)
    }
}
