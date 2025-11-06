use std::net::SocketAddr;

use axum::{Router, routing::get};
use futures::future::BoxFuture;
use tokio::{
    sync::{mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel}, watch},
};
pub use tokio_util::sync::CancellationToken;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

mod error;

use crate::error::{FrontdoorError, Result};

pub struct ServerBuilder {
    listen_address: Option<SocketAddr>,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            listen_address: None,
        }
    }

    pub fn with_listen_address(mut self, listen_address: SocketAddr) -> Self {
        self.listen_address = Some(listen_address);
        self
    }

    pub fn build(self) -> Result<Server> {
        let listen_address = self
            .listen_address
            .ok_or(FrontdoorError::MissingListenAddress)?;

        Ok(Server { listen_address })
    }
}

pub struct ServicesConfig {
    services: Vec<ServiceConfig>,
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self { services: Vec::new() }
    }
}

pub struct ServiceConfig {
    name: String,
    url: String,
}

pub struct ServerConfig {
    listen_address: SocketAddr,
}

pub struct Server {
    listen_address: SocketAddr,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            listen_address: config.listen_address,
        }
    }

    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    pub fn serve(self, services: ServicesConfig) -> Serve {
        Serve { server: self, services_config: services }
    }

    pub fn serve_watch(self) -> (ConfigSender, ServeWatch) {
        let (tx, rx) = unbounded_channel();
        (ConfigSender { tx }, ServeWatch { server: self, services_config_rx: rx })
    }
}

#[derive(Clone)]
pub struct ConfigSender {
    tx: UnboundedSender<ServicesConfig>,
}

impl ConfigSender {
    pub fn send(self, config: ServicesConfig) -> anyhow::Result<()> {
        self.tx.send(config)?;
        Ok(())
    }
}

pub struct ServiceHandler {
    listen_address: SocketAddr,
    config: ServicesConfig,
}

impl ServiceHandler {
    pub fn try_new(listen_address: SocketAddr, config: ServicesConfig) -> anyhow::Result<Self> {
        Ok(Self { listen_address, config })
    }

    pub async fn run(&self, cancel_token: tokio_util::sync::CancellationToken) -> anyhow::Result<()> {
        let ServiceHandler { listen_address, config } = self;

        let router = Router::new()
            .route("/health", get(|| async { "OK" }))
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive());

        let listener = tokio::net::TcpListener::bind(listen_address).await?;

        let local_addr = listener.local_addr();

        match local_addr {
            Ok(listen_address) => {
                info!("Axum server listening on {}", listen_address);
            }
            Err(e) => {
                error!("Failed to get local address: {}", e);
            }
        }

        let signal = async move {
            cancel_token.cancelled().await;
        };

        axum::serve(listener, router)
                .with_graceful_shutdown(signal)
                .await
                .map_err(anyhow::Error::from)?;

        Ok(())
    }
}

pub struct Serve {
    server: Server,
    services_config: ServicesConfig,
}

impl Serve {
    pub fn new(server: Server, services_config: ServicesConfig) -> Self {
        Self { server, services_config }
    }

    pub fn with_graceful_shutdown<F>(self, signal: F) -> WithGracefulShutdown<F, Self>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        WithGracefulShutdown::new(signal, self)
    }
}

impl ServeWatchWithGracefulShutdown for Serve {
    async fn run(self, cancel_token: tokio_util::sync::CancellationToken) -> anyhow::Result<()> {
        let Serve { server, services_config } = self;

        let handler = ServiceHandler::try_new(server.listen_address, services_config)?;
        handler.run(cancel_token).await?;

        Ok(())
    }
}

pub struct ServeWatch {
    server: Server,
    services_config_rx: UnboundedReceiver<ServicesConfig>,
}

impl ServeWatch {
    pub fn with_graceful_shutdown<F>(self, signal: F) -> WithGracefulShutdown<F, Self>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        WithGracefulShutdown::new(signal, self)
    }
}

impl ServeWatchWithGracefulShutdown for ServeWatch {
    async fn run(self, cancel_token: tokio_util::sync::CancellationToken) -> anyhow::Result<()> {
        let ServeWatch {
            server,
            mut services_config_rx,
        } = self;

        info!("Starting server");

        let config = services_config_rx.recv().await.ok_or(anyhow::Error::msg("No services config received"))?;

        let mut handler = ServiceHandler::try_new(server.listen_address, config)?;

        loop {
            let child_token = cancel_token.child_token();

            tokio::select! {
                res = handler.run(child_token.clone()) => {
                    if let Err(ref e) = res {
                        error!("Failed to run service handler: {}", e);
                    }
                    return res;
                }
                config = services_config_rx.recv() => {
                    if let Some(config) = config {
                        match ServiceHandler::try_new(server.listen_address, config) {
                            Ok(new_handler) => {
                                handler = new_handler;
                            }
                            Err(e) => {
                                error!("Failed to create new service handler: {}", e);
                            }
                        }
                    }

                    child_token.cancel();
                }
            }
        }
    }
}

pub trait ServeWatchWithGracefulShutdown: Send + 'static {
    fn run(self, cancel_token: tokio_util::sync::CancellationToken) -> impl Future<Output = anyhow::Result<()>> + Send + 'static;
}

impl IntoFuture for Serve {
    type Output = anyhow::Result<()>;
    type IntoFuture = BoxFuture<'static, anyhow::Result<()>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.run(CancellationToken::new()).await })
    }
}

pub struct WithGracefulShutdown<F, S> {
    signal: F,
    serve: S,
}

impl<F, S> WithGracefulShutdown<F, S>
where
    F: Future<Output = ()> + Send + 'static,
    S: ServeWatchWithGracefulShutdown,
{
    pub fn new(signal: F, serve: S) -> Self {
        Self { signal, serve }
    }

    async fn run(self) -> anyhow::Result<()> {
        let Self { signal, serve } = self;

        let cancel_token = CancellationToken::new();

        let (signal_tx, signal_rx) = watch::channel(());

        tokio::spawn(async move {
            signal.await;
            info!("Received graceful shutdown signal. Telling tasks to shutdown");
            drop(signal_rx);
        });

        let serve_handle = tokio::spawn(serve.run(cancel_token.clone()));

        signal_tx.closed().await;

        cancel_token.cancel();

        let _ = serve_handle.await;

        info!("Server shutdown complete");

        Ok(())
    }
}

impl<F, S> IntoFuture for WithGracefulShutdown<F, S>
where
    F: Future<Output = ()> + Send + 'static,
    S: ServeWatchWithGracefulShutdown,
{
    type Output = anyhow::Result<()>;
    type IntoFuture = BoxFuture<'static, anyhow::Result<()>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.run().await })
    }
}
