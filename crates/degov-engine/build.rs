use degov_rpc_build::{AxumConnectGenSettings, degov_rpc_codegen};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    degov_rpc_codegen(AxumConnectGenSettings::from_directory_recursive("proto")?)?;

    Ok(())
}
