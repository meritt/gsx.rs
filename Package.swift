// swift-tools-version: 6.2

import Foundation
import PackageDescription

let release = (
    url: "https://github.com/meritt/gsx.rs/releases/download/v0.2.0/gsx-v0.2.0-xcframework.zip",
    checksum: "60bb1885fdcac54ae3e2cf341cf89a8a37e26e7f493d28525fc1d95b1445b426"
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
