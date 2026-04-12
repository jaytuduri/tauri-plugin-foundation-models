# Contributing

## Prerequisites

- macOS 26 or later
- Apple Intelligence enabled (System Settings > Apple Intelligence & Siri)
- Xcode 26 command line tools (`xcode-select --install`)
- Rust 1.77+ with the `aarch64-apple-darwin` or `x86_64-apple-darwin` target
- Node.js 18+

## Building

```sh
# Build the Swift static library and Rust crate
cargo build

# Build the JavaScript bindings
npm install && npm run build
```

The build process is driven by `build.rs`, which:
1. Runs `swift build` on the `swift/` package to produce `libAppleIntelligenceFFI.a`
2. Links the static library, `Foundation.framework`, `FoundationModels.framework`, and the Swift concurrency runtime

## Testing

### Smoke test

The Rust smoke test exercises the FFI layer directly without Tauri:

```sh
cargo run --example smoke
```

This checks availability, creates a session, runs a one-shot respond, runs a streaming respond, and closes the session.

### Chat app example

A full Tauri app lives in `examples/chat-app/`:

```sh
cd examples/chat-app
npm install
npm run tauri dev
```

## Project structure

See [ARCHITECTURE.md](./ARCHITECTURE.md) for a detailed breakdown of the source layout and how the layers interact.

## Adding a new command

1. **Swift**: Add the `@_cdecl` function in `swift/Sources/AppleIntelligenceFFI/Bridge.swift`.
2. **Rust FFI**: Declare the `extern "C"` signature in `src/ffi.rs`.
3. **Rust command**: Add the `#[command]` function in `src/commands.rs` and register it in the `invoke_handler` in `src/lib.rs`.
4. **Permissions**: Add a TOML file in `permissions/` and include the permission in `permissions/default.toml`.
5. **JavaScript**: Add the public function in `guest-js/index.ts`.
6. **Build JS**: Run `npm run build` to regenerate `dist-js/`.

## Adding a new permission scope

Permission files live in `permissions/`. Each file grants access to one or more commands. The `default.toml` bundles all permissions for typical use. For fine-grained access, apps can reference individual permission identifiers like `apple-intelligence:allow-generate`.

## Code conventions

- Rust follows standard `rustfmt` formatting.
- Swift uses Apple's standard Swift style.
- All strings crossing the C ABI boundary are UTF-8, NUL-terminated, heap-allocated with `strdup()`, and freed with `ai_free_string()`.
- Error strings from Swift must stay in sync with `map_native_error()` in `src/commands.rs`.

## Common issues

### `swift build` fails with "no such module 'FoundationModels'"

You need macOS 26+ and Xcode 26 command line tools. Run `xcode-select --install` and make sure you're on the latest beta.

### Linker errors about Swift concurrency symbols

The build links against `/usr/lib/swift` for the system Swift runtime. Do not add the toolchain's `swift-5.5/` back-deployment stubs path — those lack Swift 6 symbols.

### "Apple Intelligence not available" at runtime

Check System Settings > Apple Intelligence & Siri. The feature must be enabled and the on-device model must be downloaded. If `availability()` returns `modelNotReady`, wait and try again.
