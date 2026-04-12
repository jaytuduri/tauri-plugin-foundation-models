# API Reference

Complete reference for the `tauri-plugin-foundation-models-api` JavaScript/TypeScript package.

## Functions

### `availability()`

```typescript
function availability(): Promise<AvailabilityStatus>
```

Check whether Apple Intelligence is available on this device. Always call this before using any generation features.

**Returns** an `AvailabilityStatus`:

```typescript
type AvailabilityStatus =
  | { available: true }
  | { available: false; reason: UnavailabilityReason }

type UnavailabilityReason =
  | 'deviceNotEligible'         // hardware doesn't support Apple Intelligence
  | 'appleIntelligenceNotEnabled' // user hasn't enabled it in System Settings
  | 'modelNotReady'             // model is downloading or initialising
  | 'other'
```

---

### `generate(prompt, options?)`

```typescript
function generate(prompt: string, options?: GenerationOptions): Promise<string>
```

One-shot, stateless text generation. Creates an internal session, generates a response, and cleans up. Use this for simple prompts that don't need conversation history.

**Parameters**

| Name | Type | Description |
|------|------|-------------|
| `prompt` | `string` | The text prompt to send to the model. |
| `options` | `GenerationOptions` | Optional generation parameters. |

**Returns** the model's full response text.

---

### `generateStream(prompt, onChunk, options?)`

```typescript
function generateStream(
  prompt: string,
  onChunk: (chunk: string) => void,
  options?: GenerationOptions
): Promise<string>
```

Stateless streaming generation. Like `generate`, but `onChunk` is called with each incremental text delta as the model produces it.

**Parameters**

| Name | Type | Description |
|------|------|-------------|
| `prompt` | `string` | The text prompt. |
| `onChunk` | `(chunk: string) => void` | Called with each incremental text delta. |
| `options` | `GenerationOptions` | Optional generation parameters. |

**Returns** the full accumulated response text.

---

### `createSession(config?)`

```typescript
function createSession(config?: SessionConfig): Promise<Session>
```

Create a stateful session that maintains conversation history across multiple turns.

**Parameters**

| Name | Type | Description |
|------|------|-------------|
| `config` | `SessionConfig` | Optional session configuration. |

**Returns** a `Session` instance.

---

### `registerToolHandlers(handlers)`

```typescript
function registerToolHandlers(
  handlers: Record<string, ToolHandler>
): Promise<UnlistenFn>
```

Register named tool handlers. When the model invokes a tool during generation, the matching handler is called and its return value is sent back as the tool result.

Tools must also be declared on the session via `createSession({ tools: [...] })` — this function only wires up execution.

**Parameters**

| Name | Type | Description |
|------|------|-------------|
| `handlers` | `Record<string, ToolHandler>` | Map of tool name to async handler function. |

**Returns** an unlisten function that removes the event listener when called.

**Handler signature**:

```typescript
type ToolHandler = (args: unknown) => unknown | Promise<unknown>
```

Handlers receive the raw arguments from the model and should return any JSON-serializable value.

---

## `Session` class

Returned by `createSession()`. Maintains conversation context across turns.

### `session.respond(prompt, options?)`

```typescript
respond(prompt: string, options?: GenerationOptions): Promise<string>
```

Send a prompt and receive the full response. The session retains this exchange in its history.

### `session.respondStream(prompt, onChunk, options?)`

```typescript
respondStream(
  prompt: string,
  onChunk: (chunk: string) => void,
  options?: GenerationOptions
): Promise<string>
```

Send a prompt and receive incremental text deltas via `onChunk`. Returns the full response.

### `session.close()`

```typescript
close(): Promise<void>
```

Destroy the session and free resources. The session cannot be used after this call.

### `session.id`

```typescript
readonly id: number
```

The numeric session identifier. Used internally for IPC.

---

## Types

### `GenerationOptions`

```typescript
interface GenerationOptions {
  temperature?: number          // Sampling temperature. Higher = more varied.
  maximumResponseTokens?: number // Hard cap on response length.
}
```

### `SessionConfig`

```typescript
interface SessionConfig {
  instructions?: string   // System prompt for the session.
  tools?: ToolSpec[]       // Tools the model may call.
}
```

### `ToolSpec`

```typescript
interface ToolSpec {
  name: string
  description: string
  parametersSchema: Record<string, unknown>
}
```

### `ToolCallEvent`

Emitted internally when the model invokes a tool. Handled automatically by `registerToolHandlers`.

```typescript
interface ToolCallEvent {
  sessionId: number
  callId: number
  name: string
  arguments: unknown
}
```

---

## Errors

All errors surface as rejected promises with a string message.

| Message | Cause |
|---------|-------|
| `"context window exceeded"` | Session history is too long. Start a new session. |
| `"unsupported language or locale"` | The current system language is not supported. |
| `"Apple Intelligence not available: ..."` | Model unavailable at call time. |
| `"session X not found"` | Session was already closed or never created. |
| `"invalid input: ..."` | A string passed to the native layer contained a NUL byte. |

---

## Permissions

Add permissions to your app's capability file (`src-tauri/capabilities/default.json`).

**Grant all** (recommended for most apps):

```json
{ "permissions": ["foundation-models:default"] }
```

**Granular permissions**:

| Permission | Commands |
|------------|----------|
| `foundation-models:allow-availability` | `availability` |
| `foundation-models:allow-generate` | `generate` |
| `foundation-models:allow-generate-stream` | `generate_stream` |
| `foundation-models:allow-create-session` | `create_session` |
| `foundation-models:allow-respond` | `respond` |
| `foundation-models:allow-respond-stream` | `respond_stream` |
| `foundation-models:allow-close-session` | `close_session` |
| `foundation-models:allow-resolve-tool-call` | `resolve_tool_call` |
