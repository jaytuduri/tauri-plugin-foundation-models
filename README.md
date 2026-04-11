# tauri-plugin-apple-intelligence

Tauri v2 plugin that exposes Apple's on-device [Foundation Models](https://developer.apple.com/documentation/foundationmodels) framework to your macOS app. Write prompts, stream responses, manage multi-turn sessions, and wire up tool calling — all from Rust or the frontend.

## Requirements

- macOS 26+
- Apple Intelligence enabled on the device
- Xcode 26 command line tools (`xcode-select --install`)
- Tauri v2

## Installation

### Rust

Add to `src-tauri/Cargo.toml`:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
tauri-plugin-apple-intelligence = { path = "../path/to/tauri-plugin-apple-intelligence" }
```

Register the plugin in `src-tauri/src/lib.rs`:

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_apple_intelligence::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### JavaScript

```sh
npm install tauri-plugin-apple-intelligence-api
```

Or with a local path:

```json
{
  "dependencies": {
    "tauri-plugin-apple-intelligence-api": "file:../path/to/tauri-plugin-apple-intelligence"
  }
}
```

### Permissions

Add to your app's capability file (e.g. `src-tauri/capabilities/default.json`):

```json
{
  "permissions": [
    "apple-intelligence:default"
  ]
}
```

Or grant only specific commands:

```json
{
  "permissions": [
    "apple-intelligence:allow-availability",
    "apple-intelligence:allow-generate",
    "apple-intelligence:allow-generate-stream",
    "apple-intelligence:allow-create-session",
    "apple-intelligence:allow-respond",
    "apple-intelligence:allow-respond-stream",
    "apple-intelligence:allow-close-session",
    "apple-intelligence:allow-resolve-tool-call"
  ]
}
```

## Usage

### Check availability

Always check before using any generation features. The on-device model requires specific hardware and Apple Intelligence to be enabled.

```typescript
import { availability } from 'tauri-plugin-apple-intelligence-api'

const status = await availability()

if (status.available) {
  // proceed
} else {
  switch (status.reason) {
    case 'appleIntelligenceNotEnabled':
      // prompt the user to enable Apple Intelligence in System Settings
      break
    case 'deviceNotEligible':
      // device does not support Apple Intelligence
      break
    case 'modelNotReady':
      // model is downloading or initialising — try again later
      break
  }
}
```

### One-shot generation

Simple stateless generation. A session is created and destroyed internally.

```typescript
import { generate } from 'tauri-plugin-apple-intelligence-api'

const summary = await generate('Summarise this in two sentences: ' + articleText)
```

With options:

```typescript
const reply = await generate('Write a haiku about rain.', {
  temperature: 0.9,
  maximumResponseTokens: 100,
})
```

### Streaming generation

Receive text incrementally as the model produces it.

```typescript
import { generateStream } from 'tauri-plugin-apple-intelligence-api'

let fullText = ''

await generateStream(
  'Explain how black holes form.',
  (chunk) => {
    fullText += chunk
    console.log(chunk) // incremental delta
  }
)
```

### Stateful sessions

Sessions maintain conversation history across multiple turns. Use them when context from earlier messages should influence later responses.

```typescript
import { createSession } from 'tauri-plugin-apple-intelligence-api'

const session = await createSession({
  instructions: 'You are a concise technical assistant. Reply in plain text only.',
})

const first  = await session.respond('What is a monad?')
const second = await session.respond('Give me a practical example in TypeScript.')

await session.close()
```

Streaming within a session:

```typescript
let output = ''

await session.respondStream(
  'Now show me the same example in Rust.',
  (chunk) => { output += chunk }
)
```

> **Context window**: The on-device model has a finite context window. If a session grows too long, calls will reject with `"context window exceeded"`. Start a new session and summarise prior context into the instructions if needed.

### Tool calling

Tools let the model invoke your code at runtime — useful for fetching live data, querying local state, or any side effect the model should be able to trigger.

#### 1. Declare tools on the session

```typescript
const session = await createSession({
  instructions: 'You are a helpful assistant with access to weather data.',
  tools: [
    {
      name: 'get_weather',
      description: 'Get the current weather for a city. Pass the city name as a JSON string, e.g. {"city":"London"}.',
      parametersSchema: {},
    },
  ],
})
```

#### 2. Register handlers

```typescript
import { registerToolHandlers } from 'tauri-plugin-apple-intelligence-api'

const unlisten = await registerToolHandlers({
  async get_weather(args) {
    const { city } = JSON.parse(args as string)
    const data = await fetchWeatherAPI(city)
    return `${data.temp}°C, ${data.description}`
  },
})

// When done, remove the listener
unlisten()
```

#### 3. Prompt normally

```typescript
const response = await session.respond('What is the weather like in Tokyo right now?')
// The model calls get_weather("Tokyo") automatically, then incorporates
// the result into its reply.
```

> **v1 limitation**: Tool arguments are passed as a single JSON-encoded string in `input`. Instruct the model in your tool description to pass structured data as a JSON object string. Strongly-typed argument schemas are planned for v2.

## API Reference

### `availability(): Promise<AvailabilityStatus>`

Returns the current availability of Apple Intelligence on this device.

### `generate(prompt, options?): Promise<string>`

One-shot stateless generation. Creates an internal session, responds, and cleans up.

### `generateStream(prompt, onChunk, options?): Promise<string>`

Stateless streaming generation. `onChunk` is called with each incremental text delta. Returns the full accumulated response.

### `createSession(config?): Promise<Session>`

Creates a stateful session. `config.instructions` sets the system prompt. `config.tools` registers tools the model may call.

### `Session`

| Method | Description |
|--------|-------------|
| `respond(prompt, options?)` | Send a prompt, receive the full response. |
| `respondStream(prompt, onChunk, options?)` | Send a prompt, receive incremental deltas via `onChunk`. Returns the full response. |
| `close()` | Destroy the session and free resources. |

### `registerToolHandlers(handlers): Promise<UnlistenFn>`

Registers a map of tool name → async handler. Handlers receive the raw arguments string and should return any JSON-serialisable value. Returns a cleanup function that removes the event listener.

### `GenerationOptions`

| Field | Type | Description |
|-------|------|-------------|
| `temperature` | `number` | Sampling temperature. Higher = more varied output. |
| `maximumResponseTokens` | `number` | Hard cap on response length. Use sparingly — can produce truncated output. |

## Error handling

Errors surface as rejected promises with a string message. Common values:

| Message | Cause |
|---------|-------|
| `"context window exceeded"` | Session history is too long. Start a new session. |
| `"Apple Intelligence not available: ..."` | Model unavailable at call time. |
| `"session X not found"` | Session was already closed or never created. |

## Building from source

```sh
# Build the Swift static library and Rust crate
cargo build

# Run the smoke test (requires macOS 26 + Apple Intelligence enabled)
cargo run --example smoke

# Build the JS bindings
npm install && npm run build
```

## Platform support

macOS 26+ only. Apple Intelligence must be enabled in System Settings → Apple Intelligence & Siri.

This plugin has no effect on other platforms — add it behind a `#[cfg(target_os = "macos")]` guard if your app targets multiple platforms.

## License

MIT OR Apache-2.0
