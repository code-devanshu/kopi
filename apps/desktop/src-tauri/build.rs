fn main() {
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let mut attributes = tauri_build::Attributes::new();
    if windows {
        // The linker embeds the manifest below instead, in every binary.
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    }
    tauri_build::try_build(attributes).expect("failed to run the Tauri build script");
    if windows {
        embed_manifest();
    }
}

/// Embeds the manifest that asks for Common Controls v6 in the test binaries too, not just the
/// app. Tauri links against v6, so a test binary without it can't start
/// (`STATUS_ENTRYPOINT_NOT_FOUND`).
fn embed_manifest() {
    let manifest = std::env::current_dir()
        .expect("no current directory")
        .join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    // Fail on linker warnings, such as a manifest it couldn't read.
    println!("cargo:rustc-link-arg=/WX");
}
