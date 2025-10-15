use connectare_build::{ConnectareGenSettings, connectare_codegen};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    connectare_codegen(ConnectareGenSettings::from_directory_recursive("proto")?)?;

    Ok(())
}
