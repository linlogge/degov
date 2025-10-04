use degov_lsp::start_server;

#[tokio::main]
async fn main() {
    // Initialize tracing to write to stderr (stdout is used for LSP protocol)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::ERROR)
        .init();

    start_server().await;
}
