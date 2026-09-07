// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Supertype",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "Supertype", targets: ["Supertype"]),
        .executable(name: "SupertypeFFITest", targets: ["SupertypeFFITest"])
    ],
    targets: [
        .target(
            name: "CSupertypeCore",
            path: "Sources/CSupertypeCore",
            publicHeadersPath: "include"
        ),
        .executableTarget(
            name: "Supertype",
            dependencies: ["CSupertypeCore"],
            path: "Sources/Supertype",
            linkerSettings: [
                .linkedFramework("AppKit"),
                .linkedFramework("AVFoundation"),
                .linkedFramework("ApplicationServices"),
                .unsafeFlags(["-L", "../core/target/debug", "-lsupertype_core"], .when(configuration: .debug)),
                .unsafeFlags(["-L", "../core/target/release", "-lsupertype_core"], .when(configuration: .release))
            ]
        ),
        .executableTarget(
            name: "SupertypeFFITest",
            dependencies: ["CSupertypeCore"],
            path: "Sources/SupertypeFFITest",
            linkerSettings: [
                .unsafeFlags(["-L", "../core/target/debug", "-lsupertype_core"], .when(configuration: .debug)),
                .unsafeFlags(["-L", "../core/target/release", "-lsupertype_core"], .when(configuration: .release))
            ]
        )
    ]
)
