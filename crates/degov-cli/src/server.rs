use axum::Router;
use degov_server::Server;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}};
use tokio::sync::broadcast;

pub async fn start_server(server: Server) -> Result<(), Box<dyn std::error::Error>> {
    // Create shutdown channel
    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    
    // Start the engine's RPC server for workers (on 8080)
    let engine = server.state().engine.clone();

    let listen_addr = std::env::var("ENGINE_LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    let engine_addr = listen_addr.parse()?;
    let mut shutdown_rx1 = shutdown_tx.subscribe();
    
    let engine_handle = tokio::spawn(async move {
        tokio::select! {
            result = degov_engine::engine::run_server(engine, engine_addr) => {
                if let Err(e) = result {
                    tracing::error!("Engine RPC server error: {}", e);
                }
            }
            _ = shutdown_rx1.recv() => {
                tracing::info!("Engine RPC server shutting down...");
            }
        }
    });

    // Start the HTTP API server (on 3030)
    let mut shutdown_rx2 = shutdown_tx.subscribe();
    let http_handle = tokio::spawn(async move {
        tokio::select! {
            result = start_http_server(server) => {
                if let Err(e) = result {
                    tracing::error!("HTTP API server error: {}", e);
                }
            }
            _ = shutdown_rx2.recv() => {
                tracing::info!("HTTP API server shutting down...");
            }
        }
    });

    // Wait for shutdown signal
    wait_for_shutdown_signal().await;
    
    tracing::info!("Shutdown signal received, gracefully shutting down...");
    
    // Send shutdown signal to all servers
    let _ = shutdown_tx.send(());
    
    // Wait for servers to shutdown with timeout
    let shutdown_timeout = tokio::time::Duration::from_secs(30);
    tokio::select! {
        _ = engine_handle => {
            tracing::info!("Engine RPC server stopped");
        }
        _ = tokio::time::sleep(shutdown_timeout) => {
            tracing::warn!("Engine RPC server shutdown timed out");
        }
    }
    
    tokio::select! {
        _ = http_handle => {
            tracing::info!("HTTP API server stopped");
        }
        _ = tokio::time::sleep(shutdown_timeout) => {
            tracing::warn!("HTTP API server shutdown timed out");
        }
    }
    
    tracing::info!("Server shutdown complete");
    
    Ok(())
}

async fn wait_for_shutdown_signal() {
    use tokio::signal;
    
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal");
        },
    }
}

async fn start_http_server(server: Server) -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("[::]:3030").await?;

    println!("HTTP API listening on http://{:?}", listener.local_addr()?);
    println!("Engine RPC listening on http://127.0.0.1:8080");

    let mut app = Router::new();

    // Add API routes with workflow service
    app = degov_server::api::add_api_routes(app, server.workflow_service()).await;
    
    // Add admin UI routes
    app = add_infra_admin_routes(app);

    axum::serve(listener, app.layer(CorsLayer::very_permissive()))
        .await?;

    Ok(())
}

fn add_infra_admin_routes(mut app: Router) -> Router {
    let spa_path = "./apps/infra-admin/dist";
    
    // Create a ServeDir with fallback to index.html for SPA routing
    let serve_dir = ServeDir::new(spa_path)
        .not_found_service(ServeFile::new(format!("{}/index.html", spa_path)));
    
    // Mount the SPA service at /admin with fallback
    app = app.nest_service("/admin", serve_dir.clone())
             .fallback_service(serve_dir);

    app
}

