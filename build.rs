// build.rs
fn main() {
    tonic_build::configure()
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap())
                .join("ethscope_descriptor.bin"),
        )
        .compile(&["proto/ethscope.proto"], &["proto"])
        .expect("failed to compile proto/ethscope.proto");
}