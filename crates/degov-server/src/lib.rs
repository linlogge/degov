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
        let db = Database::from_path("./fdb.cluster")?;
        let mut mst = MerkleSearchTree::new(db).await?;

        let value = MerkleSearchTree::encode_value(&"tests".to_string())?;
        println!("Value (DAG-CBOR encoded): {:?}", value);
        println!("Value (DAG-CBOR encoded hex): {}", hex::encode(&value));

        println!("Calling put_typed...");
        mst.put_typed("test".to_string(), &"test".to_string()).await?;
        println!("put_typed succeeded");

        println!("Calling get_typed...");
        let value: Option<String> = mst.get_typed("test").await?;
        println!("Value Read: {:?}", value);

        let proof = mst.generate_proof("test").await?;
        println!("Proof: {:?}", proof);

        Ok(())
    }
}