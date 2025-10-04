use degov_core::did::DIDBuf;
use degov_storage::{boot, Database, MerkleSearchTree};

pub struct Server {
    did: DIDBuf,
}

impl Server {
    pub fn new<T: Into<String>>(did: T) -> Self {
        let did = DIDBuf::from_string(did.into()).unwrap();
        Self { did }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let network = unsafe { boot() };

        let result = self.start_inner().await;

        drop(network);

        result
    }

    pub async fn start_inner(&self) -> Result<(), Box<dyn std::error::Error>> {
        /* let db = Database::from_path("./fdb.cluster")?;
        let mut mst = MerkleSearchTree::new(db).await?;
 */
        
        degov_api::start_server().await;

        Ok(())
    }
}