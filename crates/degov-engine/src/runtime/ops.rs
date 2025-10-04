use super::database::KvStore;
use std::{cell::RefCell, sync::Arc};

// Import deno_core through rustyscript
use rustyscript::deno_core;
use deno_core::op2;

thread_local! {
    static KV_STORE: RefCell<Option<Arc<KvStore>>> = RefCell::new(None);
}

/// Set the KV store for the current thread
pub fn set_kv_store(store: Arc<KvStore>) {
    KV_STORE.with(|kv| {
        *kv.borrow_mut() = Some(store);
    });
}

/// Get the KV store for the current thread
fn get_kv_store() -> Option<Arc<KvStore>> {
    KV_STORE.with(|kv| kv.borrow().clone())
}

/// Op to get a value from the KV store
#[op2(async)]
#[serde]
async fn op_kv_get(#[string] key: String) -> Option<Vec<u8>> {
    let store = get_kv_store()?;
    store.get(&key).await
}

/// Op to set a value in the KV store
#[op2(async)]
async fn op_kv_set(#[string] key: String, #[string] value: String) {
    if let Some(store) = get_kv_store() {
        let value_bytes = value.into_bytes();
        store.set(key, value_bytes).await;
    }
}

// Define the KV extension
deno_core::extension!(
    kv_extension,
    ops = [op_kv_get, op_kv_set],
    esm_entry_point = "ext:kv_extension/kv.js",
    esm = [dir "src/runtime/js", "kv.js"],
);
