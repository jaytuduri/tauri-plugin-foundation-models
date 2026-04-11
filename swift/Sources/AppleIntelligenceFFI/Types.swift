import Foundation

/// Shared C callback typealiases. Declared in their own file so they're
/// unambiguously visible from every source file in the target.

public typealias TokenCallback = @convention(c) (
    UnsafeMutableRawPointer?, UnsafePointer<CChar>?
) -> Void

public typealias CompletionCallback = @convention(c) (
    UnsafeMutableRawPointer?, Int32, UnsafePointer<CChar>?
) -> Void

public typealias ToolCallCallback = @convention(c) (
    UnsafeMutableRawPointer?,
    UInt64,
    UInt64,
    UnsafePointer<CChar>?,
    UnsafePointer<CChar>?
) -> Void
