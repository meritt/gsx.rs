// swift-tools-version: 6.2

import Foundation
import PackageDescription

let release = (
    url: "https://github.com/meritt/gsx.rs/releases/download/v0.3.0/gsx-v0.3.0-xcframework.zip",
    checksum: "e84edf34c51e1843d593db31b59681985401a8b0db68e53085490a53884287ee"
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
