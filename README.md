# tauri-plugin-foundation-models

Tauri v2 plugin for Apple's on-device [Foundation Models](https://developer.apple.com/documentation/foundationmodels) framework. Exposes text generation, streaming, stateful sessions, and tool calling to your macOS app via Rust or JavaScript.

> **Early release.** Requires macOS 26 and Apple Intelligence enabled. API may change.

## Requirements

- macOS 26+
- Apple Intelligence enabled (System Settings → Apple Intelligence & Siri)
- Xcode 26 command line tools
- Tauri v2

## Installation

`src-tauri/Cargo.toml`:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
tauri-plugin-foundation-models = { git = "https://github.com/jaytuduri/tauri-plugin-foundation-models" }
```

`src-tauri/src/lib.rs`:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_foundation_models::init())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

`src-tauri/capabilities/default.json`:

```json
{ "permissions": ["foundation-models:default"] }
```

JavaScript:

```sh
npm install github:jaytuduri/tauri-plugin-foundation-models
```

## Usage

```typescript
import { availability, generate, generateStream, createSession, registerToolHandlers } from 'tauri-plugin-foundation-models-api'

// Check availability first
const status = await availability()
if (!status.available) return

// One-shot generation
const reply = await generate('Summarise this article: ' + text)

// Streaming
const full = await generateStream('Explain recursion.', (chunk) => console.log(chunk))

// Stateful session
const session = await createSession({
  instructions: 'You are a concise technical assistant.',
})
const a = await session.respond('What is a monad?')
const b = await session.respond('Give me a TypeScript example.')
await session.close()

// Tool calling
const toolSession = await createSession({
  tools: [{
    name: 'get_weather',
    description: 'Get weather for a city. Pass args as JSON string, e.g. {"city":"London"}.',
    parametersSchema: {},
  }],
})

const unlisten = await registerToolHandlers({
  async get_weather(args) {
    const { city } = JSON.parse(args as string)
    return fetchWeather(city)
  },
})

const response = await toolSession.respond('What is the weather in Tokyo?')
unlisten()
```

## Building from source

```sh
cargo build          # builds Swift static library + Rust crate
npm install && npm run build  # builds JS bindings
cargo run --example smoke     # smoke test (requires macOS 26 + Apple Intelligence)
```

## Known limitations

- Tool arguments are passed as a single JSON-encoded string. Structured argument schemas are planned for v0.3.

## License

MIT OR Apache-2.0
