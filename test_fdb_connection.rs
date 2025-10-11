use foundationdb::api::NetworkAutoStop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing FoundationDB network...");
    let _network: NetworkAutoStop = unsafe { foundationdb::boot() };
    
    println!("Connecting to database...");
    let db = foundationdb::Database::from_path("/usr/local/etc/foundationdb/fdb.cluster")?;
    
    println!("Creating transaction...");
    let tx = db.create_trx()?;
    
    println!("Testing simple read...");
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let result = tx.get(b"test_key", false).await;
        println!("Read result: {:?}", result);
        tx.cancel();
    });
    
    println!("✓ Connection successful!");
    Ok(())
}

