use std::marker::PhantomData;

use digest::{Hasher, RootHash, SipHasher};
use foundationdb::{Database, Transaction};

mod digest;
mod node;
mod page;

use node::Node;
use page::Page;
use siphasher::sip::SipHasher24;

const ROOT_KEY: &[u8] = b"root";

pub(crate) type DefaultHasher = SipHasher;

struct Mst<K, V, H = DefaultHasher, const N: usize = 16> {
    db: Database,
    hasher: H,
    tree_hasher: SipHasher24,
    root: Page<N, K>,
    root_hash: Option<RootHash>,
    _value_type: PhantomData<V>,
}

impl<K, V, H, const N: usize> Mst<K, V, H, N> {
    pub fn new(db: Database, hasher: H) -> Self {
        Self {
            db,
            hasher,
            tree_hasher: SipHasher24::new(),
            root: Page::new(0, vec![]),
            root_hash: None,
            _value_type: PhantomData,
        }
    }

    pub async fn insert(&mut self, key: K, value: V) -> Result<(), foundationdb::FdbBindingError> {
        let mut trx = self.db.run(|tx, _| {
            async move {
                let mut root = tx.get(ROOT_KEY, false).await?;

                Ok(())
            }
        });

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let network = unsafe { foundationdb::boot() };

    // Have fun with the FDB API
    insert_and_get()
        .await
        .expect("could not run the insert and get");

    // shutdown the client
    drop(network);
}

async fn insert_and_get() -> foundationdb::FdbResult<()> {
    let db = foundationdb::Database::default()?;

    let mut mst: Mst<&str, &str, SipHasher, 16> = Mst::new(db, SipHasher::default());

    mst.insert("hello", "world").await.unwrap();

    Ok(())
}
