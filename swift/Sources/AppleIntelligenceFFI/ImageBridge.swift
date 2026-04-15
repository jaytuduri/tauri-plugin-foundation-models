// C ABI bridge to ImagePlayground for programmatic image generation.
//
// Two exported functions:
//   img_availability  — async; initializes ImageCreator and returns available styles
//   img_generate      — async; generates up to 4 images, fires ImageCallback per image
//
// All strings are NUL-terminated UTF-8. Callback payloads are stack-owned and
// must NOT be freed by the caller.

import Foundation
import ImagePlayground
import CoreGraphics
import ImageIO

// MARK: - img_availability

/// Async. Initializes an `ImageCreator` and fires `completion` with a JSON string:
/// - `{"available":true,"styles":[{"id":"..."},...]}` on success
/// - `{"available":false,"reason":"<reason>"}` when not supported
///
/// Status 0 = completed (check `available` in payload). Nonzero = unexpected error.
@_cdecl("img_availability")
public func img_availability(
    _ ctx: UnsafeMutableRawPointer?,
    _ completion: CompletionCallback
) -> Int32 {
    guard #available(macOS 15.4, *) else {
        let json = #"{"available":false,"reason":"osVersionTooOld"}"#
        json.withCString { completion(ctx, 0, $0) }
        return 0
    }
    Task.detached {
        do {
            let creator = try await ImageCreator()
            let styleList = creator.availableStyles.map { ["id": $0.id] }
            let payload: [String: Any] = ["available": true, "styles": styleList]
            let data = (try? JSONSerialization.data(withJSONObject: payload)) ?? Data()
            let json = String(data: data, encoding: .utf8) ?? #"{"available":true,"styles":[]}"#
            json.withCString { completion(ctx, 0, $0) }
        } catch ImageCreator.Error.notSupported {
            let json = #"{"available":false,"reason":"notSupported"}"#
            json.withCString { completion(ctx, 0, $0) }
        } catch {
            let msg = "imageCreatorInitFailed:\(error)"
            msg.withCString { completion(ctx, 1, $0) }
        }
    }
    return 0
}

// MARK: - img_generate

/// Async. Generates images and fires `imageCallback` for each one produced.
///
/// - Parameter conceptsJson: JSON array `[{"type":"text","value":"..."}]`
/// - Parameter styleId:      Style ID from `img_availability`, or "" for first available
/// - Parameter limit:        Number of images to generate (clamped to 1–4)
/// - Parameter optionsJson:  JSON `{"creationVariety":"high","personalization":"enabled"}`
///                           (fields optional; both require macOS 26.4+)
/// - Parameter imageCallback: Fires per image: `{"index":<Int>,"dataBase64":"<base64-png>"}`
/// - Parameter completion:   Fires at end: `{"count":<Int>}` on success, error string on failure
@_cdecl("img_generate")
public func img_generate(
    _ conceptsJson: UnsafePointer<CChar>?,
    _ styleId: UnsafePointer<CChar>?,
    _ limit: Int32,
    _ optionsJson: UnsafePointer<CChar>?,
    _ ctx: UnsafeMutableRawPointer?,
    _ imageCallback: ImageCallback,
    _ completion: CompletionCallback
) -> Int32 {
    guard #available(macOS 15.4, *) else {
        "osVersionTooOld".withCString { completion(ctx, 1, $0) }
        return 0
    }

    let conceptsStr  = stringFromC(conceptsJson)
    let styleIdStr   = stringFromC(styleId)
    let optsStr      = stringFromC(optionsJson)
    let clampedLimit = max(1, min(Int(limit), 4))

    Task.detached {
        do {
            let creator = try await ImageCreator()

            // Parse concepts
            let conceptsData = conceptsStr.data(using: .utf8) ?? Data()
            let conceptsArr  = (try? JSONSerialization.jsonObject(with: conceptsData))
                as? [[String: String]] ?? []
            let concepts: [ImagePlaygroundConcept] = conceptsArr.compactMap { c in
                guard let type = c["type"], let value = c["value"] else { return nil }
                switch type {
                case "text": return .text(value)
                default:     return nil
                }
            }
            guard !concepts.isEmpty else {
                "noConceptsProvided".withCString { completion(ctx, 1, $0) }
                return
            }

            // Resolve style
            let style: ImagePlaygroundStyle
            if styleIdStr.isEmpty {
                guard let first = creator.availableStyles.first else {
                    "noStylesAvailable".withCString { completion(ctx, 1, $0) }
                    return
                }
                style = first
            } else if let found = creator.availableStyles.first(where: { $0.id == styleIdStr }) {
                style = found
            } else {
                "styleNotFound".withCString { completion(ctx, 1, $0) }
                return
            }

            // Generate images — options API requires macOS 26.4+; fall back otherwise.
            var count = 0
            if #available(macOS 26.4, *) {
                var options = ImagePlaygroundOptions()
                let optsData = optsStr.data(using: .utf8) ?? Data()
                let optsObj  = (try? JSONSerialization.jsonObject(with: optsData))
                    as? [String: String] ?? [:]
                if let variety = optsObj["creationVariety"] {
                    switch variety {
                    case "high": options.creationVariety = .high
                    default:     break
                    }
                }
                if let person = optsObj["personalization"] {
                    switch person {
                    case "enabled":  options.personalization = .enabled
                    case "disabled": options.personalization = .disabled
                    default:         break
                    }
                }
                let stream = creator.images(
                    for: concepts, style: style, options: options, limit: clampedLimit
                )
                for try await created in stream {
                    deliverImage(created.cgImage, index: count, ctx: ctx, cb: imageCallback)
                    count += 1
                }
            } else {
                let stream = creator.images(for: concepts, style: style, limit: clampedLimit)
                for try await created in stream {
                    deliverImage(created.cgImage, index: count, ctx: ctx, cb: imageCallback)
                    count += 1
                }
            }

            let done = #"{"count":\#(count)}"#
            done.withCString { completion(ctx, 0, $0) }

        } catch ImageCreator.Error.notSupported {
            "notSupported".withCString { completion(ctx, 1, $0) }
        } catch ImageCreator.Error.creationFailed {
            "creationFailed".withCString { completion(ctx, 1, $0) }
        } catch ImageCreator.Error.faceInImageTooSmall {
            "faceInImageTooSmall".withCString { completion(ctx, 1, $0) }
        } catch {
            "\(error)".withCString { completion(ctx, 1, $0) }
        }
    }
    return 0
}

// MARK: - Helpers

@available(macOS 15.4, *)
private func deliverImage(
    _ cgImage: CGImage,
    index: Int,
    ctx: UnsafeMutableRawPointer?,
    cb: ImageCallback
) {
    guard let png = pngData(from: cgImage) else { return }
    let b64     = png.base64EncodedString()
    let payload = #"{"index":\#(index),"dataBase64":"\#(b64)"}"#
    payload.withCString { cb(ctx, $0) }
}

@available(macOS 15.4, *)
private func pngData(from cgImage: CGImage) -> Data? {
    let buf = NSMutableData()
    guard let dest = CGImageDestinationCreateWithData(
        buf as CFMutableData, "public.png" as CFString, 1, nil
    ) else { return nil }
    CGImageDestinationAddImage(dest, cgImage, nil)
    return CGImageDestinationFinalize(dest) ? (buf as Data) : nil
}
