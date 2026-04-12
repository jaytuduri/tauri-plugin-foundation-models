# Architecture

This document describes the internal architecture of `tauri-plugin-apple-intelligence`.

## Layer diagram

```
 ┌─────────────────────────────────┐
 │        Frontend (JS/TS)         │  guest-js/index.ts
 │   invoke() / Channel / listen() │
 └──────────────┬──────────────────┘
                │  Tauri IPC
 ┌──────────────▼──────────────────┐
 │         Rust plugin core        │  src/commands.rs, src/session.rs
 │   commands, session bookkeeping │
 └──────────────┬──────────────────┘
                │  C ABI (FFI)
 ┌──────────────▼──────────────────┐
 │     Swift static library        │  swift/Sources/AppleIntelligenceFFI/
 │  FoundationModels, SessionStore │
 └──────────────┬──────────────────┘
                │
 ┌──────────────▼──────────────────┐
 │  Apple FoundationModels.framework│  macOS 26+ system framework
 │    On-device language model      │
 └─────────────────────────────────┘
```

## Source layout

```
├── build.rs                 # Drives `swift build`, links frameworks
├── src/
│   ├── lib.rs               # Plugin init, handler registration
│   ├── commands.rs          # Tauri commands (the IPC surface)
│   ├── session.rs           # Completion/stream bookkeeping (pending maps)
│   ├── error.rs             # Error enum, serialisation
│   └── ffi.rs               # `extern "C"` declarations for the Swift lib
├── swift/
│   ├── Package.swift        # SwiftPM manifest (static lib, macOS 26+)
│   └── Sources/AppleIntelligenceFFI/
│       ├── Bridge.swift     # @_cdecl functions — the C ABI surface
│       ├── SessionStore.swift  # Thread-safe session registry
│       ├── ToolDispatcher.swift # Tool call routing + DynamicTool
│       └── Types.swift      # Callback typedefs shared across files
├── guest-js/
│   └── index.ts             # TypeScript API consumed by app frontends
├── permissions/             # Tauri v2 permission TOML files
└── examples/
    ├── smoke.rs             # Rust-only FFI smoke test
    └── chat-app/            # Full Tauri chat app example
```

## How it works

### Build pipeline

`build.rs` invokes `swift build` to compile the Swift package into a static library (`libAppleIntelligenceFFI.a`). It then tells Cargo to link that library plus the `Foundation` and `FoundationModels` system frameworks and the Swift concurrency runtime.

### Request flow (one-shot generation)

1. Frontend calls `generate(prompt)` which invokes the Tauri command `plugin:apple-intelligence|generate`.
2. `commands::generate` creates an ephemeral session via `ai_create_session`, calls `ai_respond`, and closes the session.
3. On the Swift side, `ai_respond` looks up the `LanguageModelSession` in `SessionStore`, calls `session.respond(to:)`, and fires the completion callback with the result.
4. The Rust side receives the callback on a `oneshot::channel`, returns the text to the frontend.

### Request flow (streaming)

Same as above, but uses `ai_respond_stream`. The Swift side calls `session.streamResponse(to:)` and emits incremental deltas via a token callback. The Rust side forwards each delta to the frontend through a Tauri `Channel<String>`.

### Session management

Sessions are numeric IDs. The Swift `SessionStore` holds a map of `UInt64 -> LanguageModelSession`. The Rust side doesn't store sessions — it passes the ID through to Swift for every operation.

### Tool calling

1. Tools are declared when creating a session (`createSession({ tools: [...] })`).
2. On the Swift side, each tool becomes a `DynamicTool` that conforms to the `Tool` protocol.
3. When the model invokes a tool, `DynamicTool.call()` dispatches through `ToolDispatcher`, which fires a C callback into Rust.
4. Rust emits a Tauri event (`apple-intelligence://tool-call`) to the frontend.
5. The frontend's `registerToolHandlers` listener invokes the matching handler, then calls `resolve_tool_call` to send the result back.
6. The result flows back to Swift via `ai_resolve_tool_call`, which resumes the suspended `CheckedContinuation`.

### v1 tool argument limitation

FoundationModels requires tool argument types to conform to `@Generable` at compile time. Since schemas come from the frontend at runtime, v1 uses a single `input: String` argument. Tool handlers receive a raw string and must parse it themselves. Structured argument schemas are planned for v2.

## Concurrency model

- Swift functions dispatch work on `Task.detached` to avoid blocking the C caller.
- Rust uses `tokio::sync::oneshot` for completions and `tokio::sync::mpsc` for streaming tokens.
- `SessionStore` and `ToolDispatcher` use `NSLock` for thread safety.
- `PENDING_COMPLETIONS` and `PENDING_STREAMS` are `Lazy<Mutex<HashMap>>` singletons keyed by a monotonic context ID.

## Error propagation

Swift errors are converted to strings in `errorMessage()`. Well-known errors (`exceededContextWindowSize`, `unsupportedLanguageOrLocale`) are mapped to specific Rust `Error` variants in `map_native_error()`. All errors serialize to strings for the frontend via `Serialize for Error`.
