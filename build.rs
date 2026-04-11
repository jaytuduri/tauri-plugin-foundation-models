use std::env;
use std::path::PathBuf;
use std::process::Command;

const COMMANDS: &[&str] = &[
    "availability",
    "generate",
    "generate_stream",
    "create_session",
    "respond",
    "respond_stream",
    "close_session",
    "resolve_tool_call",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let swift_dir = manifest_dir.join("swift");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=swift/Package.swift");
    println!("cargo:rerun-if-changed=swift/Sources");

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let swift_config = if profile == "release" { "release" } else { "debug" };

    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let triple = match arch.as_str() {
        "aarch64" => "arm64-apple-macosx",
        "x86_64" => "x86_64-apple-macosx",
        other => panic!("unsupported macOS arch: {other}"),
    };

    let build_dir = out_dir.join("swift-build");
    std::fs::create_dir_all(&build_dir).expect("create swift build dir");

    let status = Command::new("swift")
        .current_dir(&swift_dir)
        .args([
            "build",
            "-c",
            swift_config,
            "--triple",
            triple,
            "--build-path",
        ])
        .arg(&build_dir)
        .status()
        .expect("failed to invoke `swift build` — is Xcode command line tools installed?");

    if !status.success() {
        panic!("swift build failed");
    }

    // SwiftPM places artifacts under <build-path>/<triple>/<config>/
    let lib_dir = build_dir.join(triple).join(swift_config);
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=AppleIntelligenceFFI");

    // Link required system frameworks.
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=FoundationModels");

    // Swift runtime (needed because we link a Swift static archive).
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
}
