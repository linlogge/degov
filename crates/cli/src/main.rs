use clap::{Parser, Subcommand, builder::styling};
use clap_cargo::style;

mod dgl;
mod infrastructure;
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
    
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    
    let cli = Cli::parse();

    match cli.command {
        
    }

    Ok(())
}
