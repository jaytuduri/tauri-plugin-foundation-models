fn main() {
    tauri_build::build();
    // cargo:rustc-link-arg in a dependency's build.rs does not propagate to
    // the final binary, so the rpath for Swift runtime dylibs must be set here.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
