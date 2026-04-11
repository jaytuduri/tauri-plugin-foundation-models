import { invoke, Channel } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/** Must match `TOOL_CALL_EVENT` in src/commands.rs */
const TOOL_CALL_EVENT = 'apple-intelligence://tool-call'

export type UnavailabilityReason =
  | 'deviceNotEligible'
  | 'appleIntelligenceNotEnabled'
  | 'modelNotReady'
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
  sessionId: number
  callId: number
  name: string
  arguments: unknown
}

export type ToolHandler = (args: unknown) => unknown | Promise<unknown>

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
  const channel = new Channel<string>()
  channel.onmessage = onChunk
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
    const channel = new Channel<string>()
    channel.onmessage = onChunk
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
