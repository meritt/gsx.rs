// swift-tools-version: 6.2

import Foundation
import PackageDescription

let release = (
    url: "https://github.com/meritt/gsx.rs/releases/download/v0.1.1/gsx-v0.1.1-xcframework.zip",
    checksum: "6e5dd428016b70e794118c509f6d7d61c5f0fb7467c6cc54ff1c94f768e78e55"
)

let local = "packages/swift/GSXFFI.xcframework"

let ffi: Target =
    FileManager.default.fileExists(atPath: Context.packageDirectory + "/" + local)
    ? .binaryTarget(name: "GSXFFI", path: local)
    : .binaryTarget(name: "GSXFFI", url: release.url, checksum: release.checksum)

let package = Package(
    name: "GSX",
    platforms: [.macOS("27.0")],
    products: [
        .library(name: "GSX", targets: ["GSX"])
    ],
    targets: [
        ffi,
        .target(name: "GSX", dependencies: ["GSXFFI"], path: "packages/swift/Sources/GSX"),
        .testTarget(name: "GSXTests", dependencies: ["GSX"], path: "packages/swift/Tests/GSXTests"),
    ]
)
