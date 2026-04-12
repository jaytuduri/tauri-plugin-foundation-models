# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A Tauri v2 plugin that exposes Apple's on-device FoundationModels framework (Apple Intelligence) to Tauri apps on macOS 26+. The focus is on building a solid, robust, versatile, and easy-to-use plugin — not the example app.

## Build Commands

```bash
# Build the Rust plugin (also compiles the Swift static library via build.rs)
cargo build

# Build the TypeScript guest-js bindings
npm run build          # runs rollup, outputs to dist-js/

# Check Rust code
cargo check
cargo clippy
```

Requires: Xcode command-line tools, macOS 26+ SDK, Rust 1.77+, Swift 5.10+.

## Architecture

Three layers connected by a C ABI boundary:

### Swift Layer (`swift/Sources/AppleIntelligenceFFI/`)
Static library compiled via SwiftPM during `cargo build` (driven by `build.rs`). Wraps `FoundationModels` framework:
- `Bridge.swift` — `@_cdecl` exported C functions (`ai_availability`, `ai_create_session`, `ai_respond`, `ai_respond_stream`, etc.). All data crosses the boundary as JSON-encoded C strings. Error strings must stay in sync with `map_native_error()` in `commands.rs`.
- `SessionStore.swift` — Thread-safe singleton registry of `LanguageModelSession` instances keyed by numeric ID.
- `ToolDispatcher.swift` — Suspends Swift async when the model invokes a tool, waits for resolution from the Rust/JS side via `ai_resolve_tool_call`. Contains `DynamicTool` — see v1 limitation below.
- `Types.swift` — C callback typedefs shared across Swift files.

### Rust Layer (`src/`)
Tauri plugin glue:
- `ffi.rs` — `extern "C"` declarations matching the Swift exports. Status 0 = success.
- `commands.rs` — Tauri `#[command]` handlers. Manages async completion/streaming via callback trampolines that deliver results through `session.rs` channels.
- `session.rs` — `PENDING_COMPLETIONS` / `PENDING_STREAMS` maps linking C callback context IDs to tokio oneshot/mpsc channels.
- `error.rs` — Typed error enum with well-known variants (`ContextWindowExceeded`, `UnsupportedLanguageOrLocale`) pattern-matched from Swift error strings.

### TypeScript Layer (`guest-js/index.ts`)
Frontend API consumed by Tauri app developers. Key exports: `availability()`, `generate()`, `generateStream()`, `createSession()` (returns `Session` class), `registerToolHandlers()`.

### Data Flow
`guest-js → Tauri IPC invoke → Rust commands → C FFI → Swift FoundationModels → C callbacks → Rust channels → IPC response → guest-js`

For streaming: tokens flow through `TokenCallback` trampoline → mpsc channel → Tauri `Channel<String>` back to JS.

For tool calls: Swift suspends via `withCheckedContinuation` → C callback → Tauri event → JS handler → `resolve_tool_call` invoke → Swift continuation resumed.

## Known Limitation (v1)

FoundationModels' `Tool` protocol requires an associated `Arguments: @Generable` type at compile time. Dynamic schemas from JSON aren't supported without private macro internals. Current workaround: `DynamicTool` accepts a single free-form `input: String` parameter. Tool handlers on the JS side receive the raw string. Structured arguments are a v2 goal.

## Permissions

Tauri permission files live in `permissions/`. The `default.toml` grants all eight commands. The `build.rs` COMMANDS array must match the handlers registered in `lib.rs`.

## Conventions

- All cross-boundary data is JSON-encoded C strings (`*const c_char` / `UnsafePointer<CChar>`). Swift-allocated strings must be freed via `ai_free_string`.
- Error string literals in `Bridge.swift` `errorMessage()` and `commands.rs` `map_native_error()` must stay in sync.
- camelCase for all Serde-serialized types (`#[serde(rename_all = "camelCase")]`).
- The `build.rs` COMMANDS array must list every command exposed by the plugin.
