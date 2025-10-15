use connectare_build::{connectare_codegen, ConnectareGenSettings};

fn main() {
    let settings = ConnectareGenSettings::from_directory_recursive("proto")
        .expect("failed to glob proto files");

    connectare_codegen(settings).unwrap();
}
