use std::path::PathBuf;

use clap::{Parser, Subcommand, builder::styling};
use clap_cargo::style;
use degov_server::Server;

mod validate;

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
    /// Validate DSL YAML files
    Validate {
        #[command(subcommand)]
        command: ValidateCommands,
    },
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
    /// Server commands
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
}

#[derive(Subcommand)]
enum ValidateCommands {
    /// Validate a single YAML file
    File {
        /// Path to the YAML file
        path: PathBuf,
        
        /// Show verbose output including warnings
        #[arg(short, long)]
        verbose: bool,
        
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Validate all definitions in a directory
    Dir {
        /// Path to the directory
        path: PathBuf,
        
        /// Show verbose output including warnings
        #[arg(short, long)]
        verbose: bool,
        
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Validate by NSID
    Nsid {
        /// NSID to validate (e.g., de.berlin/business-registration)
        nsid: String,
        
        /// Root directory for services (defaults to ./services)
        #[arg(short, long, default_value = "services")]
        root: PathBuf,
        
        /// Show verbose output including warnings
        #[arg(short, long)]
        verbose: bool,
        
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Validate all services in the services directory
    All {
        /// Root directory for services (defaults to ./services)
        #[arg(short, long, default_value = "services")]
        root: PathBuf,
        
        /// Show verbose output including warnings
        #[arg(short, long)]
        verbose: bool,
        
        /// Output in JSON format
        #[arg(long)]
        json: bool,
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

#[derive(Subcommand)]
enum ServerCommands {
    /// Start the server
    Start {
        #[arg(short, long)]
        did: String,
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Validate { command } => {
            handle_validate_command(command);
        }
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
        Commands::Server { command } => match command {
            ServerCommands::Start { did } => {
                println!("Starting server with DID: {}", did);
                let server = Server::new(did);
                let result = server.start().await;
                if let Err(e) = result {
                    println!("Error: {}", e);
                }
            }
        },
    }
}

fn handle_validate_command(command: &ValidateCommands) {
    match command {
        ValidateCommands::File { path, verbose, json } => {
            if !path.exists() {
                eprintln!("Error: File not found: {}", path.display());
                std::process::exit(1);
            }
            
            let result = validate::validate_file(path);
            let results = vec![result];
            
            if *json {
                validate::print_results_json(&results);
            } else {
                validate::print_results(&results, *verbose);
            }
            
            if results.iter().any(|r| !r.success) {
                std::process::exit(1);
            }
        }
        ValidateCommands::Dir { path, verbose, json } => {
            if !path.exists() {
                eprintln!("Error: Directory not found: {}", path.display());
                std::process::exit(1);
            }
            
            let results = validate::validate_directory(path);
            
            if *json {
                validate::print_results_json(&results);
            } else {
                validate::print_results(&results, *verbose);
            }
            
            if results.iter().any(|r| !r.success) {
                std::process::exit(1);
            }
        }
        ValidateCommands::Nsid { nsid, root, verbose, json } => {
            if !root.exists() {
                eprintln!("Error: Root directory not found: {}", root.display());
                std::process::exit(1);
            }
            
            let results = validate::validate_by_nsid(nsid, root);
            
            if *json {
                validate::print_results_json(&results);
            } else {
                validate::print_results(&results, *verbose);
            }
            
            if results.iter().any(|r| !r.success) {
                std::process::exit(1);
            }
        }
        ValidateCommands::All { root, verbose, json } => {
            if !root.exists() {
                eprintln!("Error: Root directory not found: {}", root.display());
                std::process::exit(1);
            }
            
            let results = validate::validate_all(root);
            
            if *json {
                validate::print_results_json(&results);
            } else {
                validate::print_results(&results, *verbose);
            }
            
            if results.iter().any(|r| !r.success) {
                std::process::exit(1);
            }
        }
    }
}
