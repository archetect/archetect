fn main() {
    configure_windows();
}

fn configure_windows() {
    static MANIFEST: &str = "pkg/windows/Manifest.xml";

    let Ok(target_os) = std::env::var("CARGO_CFG_TARGET_OS") else { return };
    let Ok(target_env) = std::env::var("CARGO_CFG_TARGET_ENV") else { return };

    let Ok(mut manifest) = std::env::current_dir() else { return };
    println!("Current Directory: {:?}", manifest);
    manifest.push(MANIFEST);
    let Some(manifest) = manifest.to_str() else { return };

    println!("manifest: {}", manifest);

    if !(target_os == "windows" && target_env == "msvc") {
        println!("Not Windows on MSVC; skipping");
        return;
    }

    println!("cargo:rerun-if-changed={}", MANIFEST);
    // Embed the Windows application manifest file.
    println!("cargo:rustc-link-arg-bin=archetect=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bin=archetect=/MANIFESTINPUT:{manifest}");
}