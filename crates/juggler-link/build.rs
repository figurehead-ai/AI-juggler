//! Compiles the Fleet protocol (`proto/`) with `protox` — a pure-Rust
//! protobuf compiler, so neither developers nor CI need `protoc`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto");
    let descriptors = protox::compile(["juggler/link/v1/link.proto"], ["proto"])?;
    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_fds(descriptors)?;
    Ok(())
}
