use std::path::PathBuf;

use clap::{Parser, Subcommand, builder::styling};
use clap_cargo::style;

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
    /// DRD (DeGov Resource Definition) commands
    Drd {
        #[command(subcommand)]
        command: DrdCommands,
    },
    /// Workflow commands
    Wf {
        #[command(subcommand)]
        command: WfCommands,
    },
}

#[derive(Subcommand)]
enum DrdCommands {
    /// Check DRD files for validity
    Check,
    /// Display contents of DRD file
    Cat {
        #[arg(short, long)]
        path: PathBuf,
    },
}

#[derive(Subcommand)]
enum WfCommands {
    /// Run a workflow
    Run,
    /// Deploy a workflow
    Deploy,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Drd { command } => match command {
            DrdCommands::Check => println!("Checking DRD..."),
            DrdCommands::Cat { path } => {
                let contents = std::fs::read_to_string(path).unwrap();
                println!("{}", contents);
            }
        },
        Commands::Wf { command } => match command {
            WfCommands::Run => println!("Running workflow..."),
            WfCommands::Deploy => println!("Deploying workflow..."),
        },
    }
}
