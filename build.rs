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
    "img_availability",
    "generate_image",
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
    // Weak-link FoundationModels so the binary loads on macOS < 26.
    // The #available guards in Bridge.swift prevent any actual calls.
    println!("cargo:rustc-link-arg=-weak_framework");
    println!("cargo:rustc-link-arg=FoundationModels");
    // Weak-link ImagePlayground (available macOS 15.4+; guards in ImageBridge.swift).
    println!("cargo:rustc-link-arg=-weak_framework");
    println!("cargo:rustc-link-arg=ImagePlayground");
    // CoreGraphics and ImageIO are needed for PNG encoding of generated images.
    println!("cargo:rustc-link-lib=framework=CoreGraphics");
    println!("cargo:rustc-link-lib=framework=ImageIO");

    // Swift concurrency runtime — resolved from the macOS dyld shared cache.
    // Do NOT add the toolchain's swift-5.5/ path: those are back-deployment
    // stubs that lack Swift 6 symbols (withCheckedContinuation(isolation:)).
    println!("cargo:rustc-link-lib=dylib=swift_Concurrency");
    println!("cargo:rustc-link-lib=dylib=swiftCore");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
}
