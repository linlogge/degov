use connect_rpc_build::{connect_rpc_codegen, AxumConnectGenSettings};

fn main() {
    let settings = AxumConnectGenSettings::from_directory_recursive("proto")
        .expect("failed to glob proto files");

    connect_rpc_codegen(settings).unwrap();
}
