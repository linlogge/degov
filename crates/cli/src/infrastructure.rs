use clap::Subcommand;

#[derive(Subcommand)]
pub enum InfrastructureCommands {
    /// Check infrastructure files for validity
    Check,
}

pub fn handle_infrastructure_command(command: InfrastructureCommands) -> miette::Result<()> {
    match command {
        InfrastructureCommands::Check => {
            Ok(())
        }
    }
}