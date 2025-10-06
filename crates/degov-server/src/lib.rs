use degov_core::did::DIDBuf;
use degov_storage::{boot, Database, MerkleSearchTree};
use axum::Router;
use tower_http::{cors::CorsLayer, services::ServeDir};

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

        let listener = tokio::net::TcpListener::bind("[::]:3030").await.unwrap();

        println!("listening on http://{:?}", listener.local_addr().unwrap());

        let mut app = Router::new();

        app = degov_api::add_api_routes(app).await;
        app = add_infra_admin_routes(app);

        axum::serve(listener, app.layer(CorsLayer::very_permissive()))
            .await
            .unwrap();

        Ok(())
    }
}

fn add_infra_admin_routes(mut app: Router) -> Router {
    let static_files = ServeDir::new("./apps/infra-admin/dist");

    app = app.nest_service("/admin", static_files);

    app
}
