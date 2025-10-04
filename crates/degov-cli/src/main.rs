use std::path::PathBuf;

use clap::{Parser, Subcommand, builder::styling};
use clap_cargo::style;
use degov_server::Server;
use miette::{IntoDiagnostic, bail};

mod validate;
mod infrastructure;
mod dgl;

use infrastructure::{InfrastructureCommands, handle_infrastructure_command};
use dgl::{DglCommands, handle_dgl_command};

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
}

#[derive(Subcommand)]
enum ServerCommands {
    /// Start the server
    Start {
        #[arg(short, long)]
        did: String,
    }
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Server { command } => match command {
            ServerCommands::Start { did } => {
                println!("Starting server with DID: {}", did);
                let server = Server::new(did);
                if let Err(e) = server.start().await {
                    bail!("Server error: {}", e);
                }
            }
        },
        Commands::Infrastructure { command } => handle_infrastructure_command(command)?,
        Commands::Dgl { command } => handle_dgl_command(command)?,
    }

    Ok(())
}
