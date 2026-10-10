// swift-tools-version:6.0
import Foundation
import PackageDescription

let cargoTargetDir = URL(fileURLWithPath: #filePath)
    .deletingLastPathComponent()
    .deletingLastPathComponent()
    .appendingPathComponent("target/debug")
    .path

let package = Package(
    name: "MacstatBar",
    platforms: [.macOS(.v14)],
    targets: [
        .systemLibrary(name: "CMacstat"),
        .executableTarget(
            name: "MacstatBar",
            dependencies: ["CMacstat"],
            linkerSettings: [.unsafeFlags(["-L", cargoTargetDir])]
        ),
    ]
)
