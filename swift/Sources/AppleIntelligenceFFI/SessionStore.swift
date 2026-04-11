import Foundation
import FoundationModels

/// Thread-safe registry of `LanguageModelSession` instances keyed by a numeric
/// id handed back to the Rust side.
final class SessionStore {
    static let shared = SessionStore()

    private let lock = NSLock()
    private var sessions: [UInt64: LanguageModelSession] = [:]
    private var nextId: UInt64 = 1

    func create(instructions: String?, toolSpecs: [[String: Any]]) throws -> UInt64 {
        let model = SystemLanguageModel.default
        let tools: [any Tool] = toolSpecs.map { spec in
            DynamicTool(
                name: spec["name"] as? String ?? "",
                description: spec["description"] as? String ?? ""
            )
        }

        let session: LanguageModelSession
        if let instructions = instructions {
            session = LanguageModelSession(
                model: model,
                tools: tools,
                instructions: Instructions(instructions)
            )
        } else {
            session = LanguageModelSession(model: model, tools: tools)
        }

        lock.lock(); defer { lock.unlock() }
        let id = nextId
        nextId += 1
        sessions[id] = session
        return id
    }

    func get(id: UInt64) -> LanguageModelSession? {
        lock.lock(); defer { lock.unlock() }
        return sessions[id]
    }

    @discardableResult
    func remove(id: UInt64) -> Bool {
        lock.lock(); defer { lock.unlock() }
        return sessions.removeValue(forKey: id) != nil
    }
}
