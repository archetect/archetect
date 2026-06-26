use std::env;
use std::path::PathBuf;

const SELF_PROTO: &str = "specs/archetect.proto";
const SELF_DIR: &str = "specs";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", SELF_PROTO);

    // Make the build self-contained: prost/tonic need a `protoc` binary. Rather
    // than requiring a system install (CI, contributors, `cargo install`), fall
    // back to a vendored protoc. An explicit `PROTOC` env var still wins, so
    // distro packagers can override.
    if env::var_os("PROTOC").is_none() {
        env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }

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
