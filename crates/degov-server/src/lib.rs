use degov_core::did::DIDBuf;
use degov_storage::boot;
use axum::Router;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}};

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
    let spa_path = "./apps/infra-admin/dist";
    
    // Create a ServeDir with fallback to index.html for SPA routing
    // This follows the pattern from the example: using_serve_dir_with_assets_fallback
    let serve_dir = ServeDir::new(spa_path)
        .not_found_service(ServeFile::new(format!("{}/index.html", spa_path)));
    
    // Mount the SPA service at /admin with fallback
    app = app.nest_service("/admin", serve_dir.clone())
             .fallback_service(serve_dir);

    app
}

