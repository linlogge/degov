use degov_rpc_build::{degov_rpc_codegen, AxumConnectGenSettings};

fn main() {
    let settings = AxumConnectGenSettings::from_directory_recursive("proto")
        .expect("failed to glob proto files");

    degov_rpc_codegen(settings).unwrap();
}
