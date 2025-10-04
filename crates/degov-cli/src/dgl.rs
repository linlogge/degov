use std::path::PathBuf;

use clap::Subcommand;
use miette::IntoDiagnostic;

#[derive(Subcommand)]
pub enum DglCommands {
    /// Check infrastructure files for validity
    Cat { path: PathBuf },
}

pub fn handle_dgl_command(command: &DglCommands) -> miette::Result<()> {
    match command {
        DglCommands::Cat { path } => {
            let contents = std::fs::read_to_string(path).into_diagnostic()?;
            degov_dsl::syntax::cat_text_ansi(&contents);
            Ok(())
        }
    }
}