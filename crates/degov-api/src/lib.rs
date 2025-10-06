use axum::Router;
use tower_http::cors::CorsLayer;

mod error;
pub mod hello;

pub use error::Error;

pub async fn start_server() {
    println!("Starting DeGov API server");

    let mut app = Router::new();
    app = hello::add_routes(app);

    let listener = tokio::net::TcpListener::bind("[::]:3030")
        .await
        .unwrap();

    println!("listening on http://{:?}", listener.local_addr().unwrap());

    axum::serve(listener, app.layer(CorsLayer::very_permissive()))
        .await
        .unwrap();
}
