use clap::{Parser, Subcommand, builder::styling};
use clap_cargo::style;
use degov_server::Server;
use miette::bail;

mod dgl;
mod infrastructure;
mod validate;
mod server;

use dgl::{DglCommands, handle_dgl_command};
use infrastructure::{InfrastructureCommands, handle_infrastructure_command};

#[derive(Parser)]
#[command(author, version, long_about = None)]
#[command(about = "CLI for managing DeGov application stacks")]
#[command(styles = CLAP_STYLING)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

pub const CLAP_STYLING: styling::Styles = styling::Styles::styled()
    .header(style::HEADER)
    .usage(style::USAGE)
    .literal(style::LITERAL)
    .placeholder(style::PLACEHOLDER)
    .error(style::ERROR)
    .valid(style::VALID)
    .invalid(style::INVALID);

#[derive(Subcommand)]
enum Commands {
    Dgl {
        #[command(subcommand)]
        command: DglCommands,
    },
    Infrastructure {
        #[command(subcommand)]
        command: InfrastructureCommands,
    },
    /// Server commands
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
    /// Worker commands
    Worker {
        #[command(subcommand)]
        command: WorkerCommands,
    },
}

#[derive(Subcommand)]
enum ServerCommands {
    /// Start the server
    Start {
        #[arg(short, long)]
        did: String,
    },
}

#[derive(Subcommand)]
enum WorkerCommands {
    /// Start a worker
    Start {
        /// Engine URL to connect to
        #[arg(short, long, default_value = "http://127.0.0.1:8080")]
        engine_url: String,
        
        /// Worker polling interval in milliseconds
        #[arg(short, long, default_value = "500")]
        poll_interval: u64,
        
        /// Heartbeat interval in seconds
        #[arg(long, default_value = "10")]
        heartbeat_interval: u64,
    },
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Server { command } => match command {
            ServerCommands::Start { did } => {
                println!("Starting server with DID: {}", did);
                // Initialize logging
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::INFO)
                    .init();

                let server = Server::new(did).await.map_err(|e| {
                    miette::miette!("Failed to initialize server: {}", e)
                })?;
                if let Err(e) = server::start_server(server).await {
                    bail!("Server error: {}", e);
                }
            }
        },
        Commands::Worker { command } => match command {
            WorkerCommands::Start { engine_url, poll_interval, heartbeat_interval } => {
                use std::time::Duration;
                
                // Initialize logging
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::INFO)
                    .init();
                
                println!("Starting worker...");
                println!("  Engine URL: {}", engine_url);
                println!("  Poll interval: {}ms", poll_interval);
                println!("  Heartbeat interval: {}s", heartbeat_interval);
                
                let worker = degov_engine::Worker::new(engine_url)
                    .await
                    .map_err(|e| miette::miette!("Failed to create worker: {}", e))?
                    .with_poll_interval(Duration::from_millis(*poll_interval))
                    .with_heartbeat_interval(Duration::from_secs(*heartbeat_interval));
                
                println!("Worker ID: {}", worker.id());
                
                if let Err(e) = worker.run().await {
                    bail!("Worker error: {}", e);
                }
            }
        },
        Commands::Infrastructure { command } => handle_infrastructure_command(command)?,
        Commands::Dgl { command } => handle_dgl_command(command)?,
    }

    Ok(())
}
