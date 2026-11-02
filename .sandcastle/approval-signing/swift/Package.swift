// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "NewChanlunApprove",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "newchanlun-approve", targets: ["newchanlun-approve"]),
    ],
    targets: [
        .target(name: "ApprovalCore"),
        .executableTarget(name: "newchanlun-approve", dependencies: ["ApprovalCore"]),
        // Development check only. It holds the software test key; the shipped helper never links it.
        .executableTarget(name: "approval-core-check", dependencies: ["ApprovalCore"]),
    ]
)
