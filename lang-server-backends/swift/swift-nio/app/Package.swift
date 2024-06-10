// swift-tools-version:5.1
import PackageDescription

let package = Package(
    name: "app",
    platforms: [
        .macOS(.v10_15)
    ],
    products: [
        .executable(name: "app", targets: ["App"])
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-nio.git", from: "2.66.0"),
        .package(path: "../FfeadCpp"),
    ],
    targets: [
        .target(name: "App", dependencies: [
            "NIO",
            "NIOHTTP1",
            "FfeadCpp",
        ], path: "Sources")
    ]
)
