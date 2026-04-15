use std::env;
use std::path::PathBuf;

const SELF_PROTO: &str = "specs/archetect.proto";
const SELF_DIR: &str = "specs";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", SELF_PROTO);

    // tonic 0.14 split prost-based codegen into `tonic-prost-build`.
    // `configure()` lives there now; tonic-build itself is the
    // transport-agnostic codegen primitive.
    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("archetect.bin"))
        .build_server(true)
        .build_client(true)
        .compile_protos(&[SELF_PROTO], &[SELF_DIR])?;

    Ok(())
}
