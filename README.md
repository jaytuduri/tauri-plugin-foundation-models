# tauri-plugin-apple-intelligence

Tauri v2 plugin for Apple's on-device AI frameworks on macOS 26+. Exposes text generation, streaming, stateful sessions, tool calling, and image generation to your macOS app via Rust or JavaScript.

- **Text** — [FoundationModels](https://developer.apple.com/documentation/foundationmodels): one-shot, streaming, stateful sessions, tool calling
- **Images** — [ImagePlayground](https://developer.apple.com/documentation/imageplayground): programmatic on-device image generation from text prompts

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
tauri-plugin-apple-intelligence = { git = "https://github.com/jaytuduri/tauri-plugin-apple-intelligence" }
```

`src-tauri/src/lib.rs`:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_apple_intelligence::init())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

`src-tauri/capabilities/default.json`:

```json
{ "permissions": ["apple-intelligence:default"] }
```

JavaScript:

```sh
npm install github:jaytuduri/tauri-plugin-apple-intelligence
```

## Usage

### Text generation

```typescript
import { availability, generate, generateStream, createSession, registerToolHandlers } from 'tauri-plugin-apple-intelligence-api'

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

### Image generation

```typescript
import { imgAvailability, generateImages } from 'tauri-plugin-apple-intelligence-api'

// Check availability and list styles
const status = await imgAvailability()
if (!status.available) return
console.log(status.styles) // [{ id: 'illustration' }, ...]

// Generate one image
await generateImages(
  [{ type: 'text', value: 'A cat wearing mittens in a snowy forest' }],
  (img) => {
    document.querySelector('img').src = `data:image/png;base64,${img.dataBase64}`
  }
)

// Generate up to 4 images with a specific style
const count = await generateImages(
  [{ type: 'text', value: 'Geometric abstract art' }],
  (img) => addImageToGrid(img),
  {
    styleId: status.styles[0].id,
    limit: 4,
    creationVariety: 'high',  // macOS 26.4+
  }
)
```

Image generation uses Apple's on-device models — no network required, no API key. Each generated image is delivered as a base64-encoded PNG via the `onImage` callback as it becomes available.

## Building from source

```sh
cargo build          # builds Swift static library + Rust crate
npm install && npm run build  # builds JS bindings
cargo run --example smoke     # smoke test (requires macOS 26 + Apple Intelligence)
```

## Known limitations

- **Tool arguments** are passed as a single JSON-encoded string. Structured argument schemas are planned for v0.3.
- **Image concepts** support `text` only. Image and drawing concepts (`ImagePlaygroundConcept.image(_:)`, `.drawing(_:)`) are not yet exposed through the FFI layer.
- **Image generation options** (`creationVariety`, `personalization`) require macOS 26.4+; they are silently ignored on macOS 26.0–26.3.

## License

MIT OR Apache-2.0
