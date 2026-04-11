// C ABI bridge to FoundationModels for the Tauri plugin.
//
// NOTE: FoundationModels is Swift-first and uses macros (@Generable) plus
// async/await. We expose a minimal C surface that marshals JSON strings
// across the boundary. All returned strings are heap-allocated with strdup()
// and must be released by the caller via ai_free_string().

import Foundation
import FoundationModels

// MARK: - String helpers

@inline(__always)
private func cstrdup(_ s: String) -> UnsafeMutablePointer<CChar>? {
    return s.withCString { strdup($0) }
}

@inline(__always)
private func stringFromC(_ p: UnsafePointer<CChar>?) -> String {
    guard let p = p else { return "" }
    return String(cString: p)
}

@_cdecl("ai_free_string")
public func ai_free_string(_ ptr: UnsafeMutablePointer<CChar>?) {
    guard let ptr = ptr else { return }
    free(ptr)
}

// MARK: - Availability

@_cdecl("ai_availability")
public func ai_availability() -> UnsafeMutablePointer<CChar>? {
    let model = SystemLanguageModel.default
    let payload: [String: Any]
    switch model.availability {
    case .available:
        payload = ["available": true]
    case .unavailable(let reason):
        let reasonStr: String
        switch reason {
        case .deviceNotEligible: reasonStr = "deviceNotEligible"
        case .appleIntelligenceNotEnabled: reasonStr = "appleIntelligenceNotEnabled"
        case .modelNotReady: reasonStr = "modelNotReady"
        @unknown default: reasonStr = "other"
        }
        payload = ["available": false, "reason": reasonStr]
    }
    let data = (try? JSONSerialization.data(withJSONObject: payload)) ?? Data()
    return cstrdup(String(data: data, encoding: .utf8) ?? "{}")
}

// MARK: - Sessions

@_cdecl("ai_create_session")
public func ai_create_session(
    _ instructionsJson: UnsafePointer<CChar>?,
    _ outSessionId: UnsafeMutablePointer<UInt64>?,
    _ outError: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let json = stringFromC(instructionsJson)
    let cfg = (try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [String: Any] ?? [:]
    let instructions = cfg["instructions"] as? String
    let toolSpecs = cfg["tools"] as? [[String: Any]] ?? []
    do {
        let id = try SessionStore.shared.create(instructions: instructions, toolSpecs: toolSpecs)
        outSessionId?.pointee = id
        return 0
    } catch {
        outError?.pointee = cstrdup("\(error)")
        return 1
    }
}

@_cdecl("ai_close_session")
public func ai_close_session(_ sessionId: UInt64) -> Int32 {
    return SessionStore.shared.remove(id: sessionId) ? 0 : 1
}

// MARK: - Respond (one-shot)

@_cdecl("ai_respond")
public func ai_respond(
    _ sessionId: UInt64,
    _ prompt: UnsafePointer<CChar>?,
    _ optionsJson: UnsafePointer<CChar>?,
    _ ctx: UnsafeMutableRawPointer?,
    _ completion: CompletionCallback
) -> Int32 {
    guard let session = SessionStore.shared.get(id: sessionId) else { return 1 }
    let promptStr = stringFromC(prompt)
    let options = parseOptions(stringFromC(optionsJson))

    Task.detached {
        do {
            let response = try await session.respond(to: promptStr, options: options)
            let text = response.content
            text.withCString { completion(ctx, 0, $0) }
        } catch {
            let msg = "\(error)"
            msg.withCString { completion(ctx, 1, $0) }
        }
    }
    return 0
}

@_cdecl("ai_respond_stream")
public func ai_respond_stream(
    _ sessionId: UInt64,
    _ prompt: UnsafePointer<CChar>?,
    _ optionsJson: UnsafePointer<CChar>?,
    _ ctx: UnsafeMutableRawPointer?,
    _ token: TokenCallback,
    _ completion: CompletionCallback
) -> Int32 {
    guard let session = SessionStore.shared.get(id: sessionId) else { return 1 }
    let promptStr = stringFromC(prompt)
    let options = parseOptions(stringFromC(optionsJson))

    Task.detached {
        do {
            var previousCount = 0
            var lastFull = ""
            let stream = session.streamResponse(to: promptStr, options: options)
            for try await partial in stream {
                // streamResponse emits progressively fuller cumulative strings.
                // Use index arithmetic to slice only the new suffix — avoids
                // a full String copy on each partial (O(n) per token → O(delta)).
                let full = partial.content
                if full.count > previousCount {
                    let startIdx = full.index(full.startIndex, offsetBy: previousCount)
                    let delta = full[startIdx...]
                    String(delta).withCString { token(ctx, $0) }
                    previousCount = full.count
                    lastFull = full
                }
            }
            lastFull.withCString { completion(ctx, 0, $0) }
        } catch {
            let msg = "\(error)"
            msg.withCString { completion(ctx, 1, $0) }
        }
    }
    return 0
}

// MARK: - Tool dispatcher

@_cdecl("ai_set_tool_dispatcher")
public func ai_set_tool_dispatcher(
    _ ctx: UnsafeMutableRawPointer?,
    _ cb: ToolCallCallback
) {
    ToolDispatcher.shared.install(ctx: ctx, callback: cb)
}

@_cdecl("ai_resolve_tool_call")
public func ai_resolve_tool_call(
    _ callId: UInt64,
    _ resultJson: UnsafePointer<CChar>?,
    _ isError: Int32
) -> Int32 {
    let text = stringFromC(resultJson)
    ToolDispatcher.shared.resolve(callId: callId, result: text, isError: isError != 0)
    return 0
}

// MARK: - Helpers

private func parseOptions(_ json: String) -> GenerationOptions {
    guard !json.isEmpty,
          let data = json.data(using: .utf8),
          let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
    else {
        return GenerationOptions()
    }
    var opts = GenerationOptions()
    if let t = obj["temperature"] as? Double { opts.temperature = t }
    if let m = obj["maximumResponseTokens"] as? Int { opts.maximumResponseTokens = m }
    return opts
}
