fn main() {
    tauri_build::build();
    // Swift runtime dylibs live in the dyld shared cache; add the rpath so the
    // linker can find them at runtime (cargo:rustc-link-arg in a dependency's
    // build.rs does not propagate to the final binary).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
