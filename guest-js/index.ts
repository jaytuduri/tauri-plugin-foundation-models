import { invoke, Channel } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/** Must match `TOOL_CALL_EVENT` in src/commands.rs */
const TOOL_CALL_EVENT = 'apple-intelligence://tool-call'

export type UnavailabilityReason =
  | 'deviceNotEligible'
  | 'appleIntelligenceNotEnabled'
  | 'modelNotReady'
  | 'osVersionTooOld'
  | 'other'

export type AvailabilityStatus =
  | { available: true; reason?: undefined }
  | { available: false; reason: UnavailabilityReason }

export interface GenerationOptions {
  temperature?: number
  maximumResponseTokens?: number
}

export interface ToolSpec {
  name: string
  description: string
  parametersSchema: Record<string, unknown>
}

export interface SessionConfig {
  instructions?: string
  tools?: ToolSpec[]
}

export interface ToolCallEvent {
  /** Always 0 in v1 — the FoundationModels Tool protocol does not expose session context. */
  sessionId: number
  callId: number
  name: string
  arguments: unknown
}

export type ToolHandler = (args: unknown) => unknown | Promise<unknown>

function makeChannel(onChunk: (chunk: string) => void): Channel<string> {
  const channel = new Channel<string>()
  channel.onmessage = onChunk
  return channel
}

/** Check whether Apple Intelligence is available on this device. */
export async function availability(): Promise<AvailabilityStatus> {
  return invoke<AvailabilityStatus>('plugin:apple-intelligence|availability')
}

/** One-shot, stateless text generation. */
export async function generate(
  prompt: string,
  options?: GenerationOptions
): Promise<string> {
  return invoke<string>('plugin:apple-intelligence|generate', { prompt, options })
}

/** Stateless streaming generation. `onChunk` receives incremental text deltas. */
export async function generateStream(
  prompt: string,
  onChunk: (chunk: string) => void,
  options?: GenerationOptions
): Promise<string> {
  const channel = makeChannel(onChunk)
  return invoke<string>('plugin:apple-intelligence|generate_stream', {
    prompt,
    options,
    onToken: channel,
  })
}

/** Create a stateful chat session. Returns a handle with respond/stream/close. */
export async function createSession(config: SessionConfig = {}): Promise<Session> {
  const id = await invoke<number>('plugin:apple-intelligence|create_session', { config })
  return new Session(id)
}

export class Session {
  constructor(public readonly id: number) {}

  respond(prompt: string, options?: GenerationOptions): Promise<string> {
    return invoke<string>('plugin:apple-intelligence|respond', {
      sessionId: this.id,
      prompt,
      options,
    })
  }

  respondStream(
    prompt: string,
    onChunk: (chunk: string) => void,
    options?: GenerationOptions
  ): Promise<string> {
    const channel = makeChannel(onChunk)
    return invoke<string>('plugin:apple-intelligence|respond_stream', {
      sessionId: this.id,
      prompt,
      options,
      onToken: channel,
    })
  }

  close(): Promise<void> {
    return invoke<void>('plugin:apple-intelligence|close_session', { sessionId: this.id })
  }
}

// ── Image generation (ImagePlayground) ───────────────────────────────────

export interface ImageConcept {
  type: 'text'
  value: string
}

export interface ImageStyle {
  id: string
}

export type ImageAvailabilityStatus =
  | { available: true; styles: ImageStyle[] }
  | { available: false; reason: string; styles?: undefined }

export interface GeneratedImage {
  /** Zero-based index of this image in the batch. */
  index: number
  /** Base64-encoded PNG data. */
  dataBase64: string
}

export interface ImageGenerationOptions {
  /** Style identifier from `imgAvailability()`. Defaults to the first available style. */
  styleId?: string
  /** Number of images to generate (1–4). Defaults to 1. */
  limit?: number
  /**
   * `"high"` requests more visual variety when generating multiple images.
   * Requires macOS 26.4+; ignored on earlier releases.
   */
  creationVariety?: 'high'
  /**
   * Enable or disable personalization in generated images.
   * Requires macOS 26.4+; ignored on earlier releases.
   */
  personalization?: 'enabled' | 'disabled'
}

/**
 * Check whether on-device image generation is available and list the styles
 * that can be passed to `generateImages()`.
 */
export async function imgAvailability(): Promise<ImageAvailabilityStatus> {
  return invoke<ImageAvailabilityStatus>('plugin:apple-intelligence|img_availability')
}

/**
 * Generate up to four images from text concepts.
 *
 * `onImage` is called once for each image as it is produced.
 * Returns the total number of images generated.
 *
 * @example
 * const count = await generateImages(
 *   [{ type: 'text', value: 'A cat wearing mittens' }],
 *   (img) => { showImage(`data:image/png;base64,${img.dataBase64}`) },
 *   { limit: 2 }
 * )
 */
export async function generateImages(
  concepts: ImageConcept[],
  onImage: (img: GeneratedImage) => void,
  options?: ImageGenerationOptions
): Promise<number> {
  const channel = new Channel<string>()
  channel.onmessage = (json: string) => {
    try { onImage(JSON.parse(json)) } catch { /* ignore malformed frames */ }
  }
  return invoke<number>('plugin:apple-intelligence|generate_image', {
    concepts,
    options,
    onImage: channel,
  })
}

/**
 * Register a set of named tool handlers. When the model calls any of them,
 * the handler is invoked and its return value (JSON-serializable) is sent
 * back as the tool result. Returns an unlisten function.
 *
 * You must still declare the tools' schemas on the session via
 * `createSession({ tools: [...] })` — this only wires up execution.
 */
export async function registerToolHandlers(
  handlers: Record<string, ToolHandler>
): Promise<UnlistenFn> {
  return listen<ToolCallEvent>(TOOL_CALL_EVENT, async (event) => {
    const { callId, name, arguments: args } = event.payload
    const handler = handlers[name]
    if (!handler) {
      await invoke('plugin:apple-intelligence|resolve_tool_call', {
        payload: { callId, result: { error: `unknown tool: ${name}` }, isError: true },
      })
      return
    }
    try {
      const result = await handler(args)
      await invoke('plugin:apple-intelligence|resolve_tool_call', {
        payload: { callId, result, isError: false },
      })
    } catch (err) {
      await invoke('plugin:apple-intelligence|resolve_tool_call', {
        payload: { callId, result: { error: String(err) }, isError: true },
      })
    }
  })
}
