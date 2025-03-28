use std::sync::Arc;
pub mod builder;
pub mod diff;
pub mod digest;
mod node;
mod node_iter;
mod page;
mod tree;
pub mod visitor;

pub use node::*;
pub use page::*;
pub use tree::*;

use foundationdb::{Database, options::TransactionOption};
use serde::{Deserialize, Serialize};

// Example key and value types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Key(String);

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Value(String);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let network = unsafe { foundationdb::boot() };

    let db = Arc::new(Database::default().unwrap());

    /* let tree = FdbMerkleSearchTree::<Key, Value>::new(
        db.clone(),
        "example_tree".to_string(),
        digest::siphash::SipHasher::default(),
    );

    // Begin a transaction
    let tx = db.create_trx()?;
    tx.set_option(TransactionOption::RetryLimit(10))?;

    // Insert some key-value pairs
    tree.upsert(
        &tx,
        Key("apple".to_string()),
        &Value("red fruit".to_string()),
    )
    .await?;
    tree.upsert(
        &tx,
        Key("banana".to_string()),
        &Value("yellow fruit".to_string()),
    )
    .await?;
    tree.upsert(
        &tx,
        Key("cherry".to_string()),
        &Value("small red fruit".to_string()),
    )
    .await?;

    // Get the root hash to verify the tree state
    let hash1 = tree.root_hash(&tx).await?;
    println!("Root hash after first inserts: {:?}", hash1.as_ref());

    // Commit the transaction
    tx.commit()
        .await
        .map_err(|e| Box::new(e))
        .map_err(|e| e.to_string())?;

    // Start a new transaction
    let tx = db.create_trx()?;

    // Update a value
    tree.upsert(
        &tx,
        Key("banana".to_string()),
        &Value("yellow curved fruit".to_string()),
    )
    .await?;

    // Get the updated root hash
    let hash2 = tree.root_hash(&tx).await?;
    println!("Root hash after update: {:?}", hash2.as_ref());

    // The root hash should be different after the update
    assert_ne!(hash1.as_ref(), hash2.as_ref());

    // Commit the transaction
    tx.commit().await.map_err(|e| Box::new(e))
        .map_err(|e| e.to_string())?;
 */
    // shutdown the client
    drop(network);

    Ok(())
}
