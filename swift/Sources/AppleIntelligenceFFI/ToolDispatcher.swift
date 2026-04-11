import Foundation
import FoundationModels

/// Bridges FoundationModels `Tool` invocations over to the Rust side via a C
/// callback, then suspends until the frontend resolves the call.
final class ToolDispatcher {
    static let shared = ToolDispatcher()

    private let lock = NSLock()
    private var ctx: UnsafeMutableRawPointer?
    private var callback: ToolCallCallback?
    private var nextCallId: UInt64 = 1
    private var pending: [UInt64: CheckedContinuation<ToolResolution, Never>] = [:]

    struct ToolResolution {
        let result: String
        let isError: Bool
    }

    func install(ctx: UnsafeMutableRawPointer?, callback: ToolCallCallback) {
        lock.lock(); defer { lock.unlock() }
        self.ctx = ctx
        self.callback = callback
    }

    /// Called by DynamicTool when the model invokes the tool. Awaits the
    /// frontend's resolution and returns the JSON result string.
    func dispatch(sessionId: UInt64, name: String, argumentsJson: String) async -> ToolResolution {
        let callId: UInt64 = {
            lock.lock(); defer { lock.unlock() }
            let id = nextCallId
            nextCallId += 1
            return id
        }()

        return await withCheckedContinuation { cont in
            lock.lock()
            pending[callId] = cont
            let cb = callback
            let cbCtx = self.ctx
            lock.unlock()

            if let cb = cb {
                name.withCString { namePtr in
                    argumentsJson.withCString { argsPtr in
                        cb(cbCtx, sessionId, callId, namePtr, argsPtr)
                    }
                }
            } else {
                resolve(callId: callId, result: "{\"error\":\"no tool dispatcher\"}", isError: true)
            }
        }
    }

    func resolve(callId: UInt64, result: String, isError: Bool) {
        lock.lock()
        let cont = pending.removeValue(forKey: callId)
        lock.unlock()
        cont?.resume(returning: ToolResolution(result: result, isError: isError))
    }
}

// MARK: - DynamicTool
//
// v1 LIMITATION: FoundationModels' `Tool` protocol wants an associated
// `Arguments: Generable` type so the model knows the argument schema at
// compile time. Building a fully dynamic schema from a JSON Schema string
// requires private macro internals that aren't part of the stable API.
//
// For v1 we expose tools that accept a single free-form `input` string and
// rely on the tool description to instruct the model on how to format it.
// Tool handlers on the JS side receive the raw string and are responsible
// for parsing it. Richer structured arguments will land in v2.

@Generable
struct DynamicToolArguments {
    @Guide(description: "JSON-encoded arguments for the tool")
    var input: String
}

enum ToolError: Error, LocalizedError {
    case failed(String)
    var errorDescription: String? {
        if case .failed(let msg) = self { return msg }
        return nil
    }
}

struct DynamicTool: Tool {
    typealias Arguments = DynamicToolArguments
    typealias Output = String

    let name: String
    let description: String

    func call(arguments: DynamicToolArguments) async throws -> String {
        let resolution = await ToolDispatcher.shared.dispatch(
            sessionId: 0,
            name: name,
            argumentsJson: arguments.input
        )
        if resolution.isError {
            throw ToolError.failed(resolution.result)
        }
        return resolution.result
    }
}
