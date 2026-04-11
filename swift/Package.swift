// swift-tools-version:5.10
import PackageDescription

let package = Package(
    name: "AppleIntelligenceFFI",
    platforms: [.macOS("26.0")],
    products: [
        .library(
            name: "AppleIntelligenceFFI",
            type: .static,
            targets: ["AppleIntelligenceFFI"]
        )
    ],
    targets: [
        .target(
            name: "AppleIntelligenceFFI",
            path: "Sources/AppleIntelligenceFFI"
        )
    ]
)
