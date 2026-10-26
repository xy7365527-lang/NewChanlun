import ApprovalCore
import CryptoKit
import Foundation
import LocalAuthentication
import Security

let repositoryID = 1_150_335_160
let usage = """
用法:
  newchanlun-approve enroll [--key-dir DIR] [--label TEXT]   （信任配置写到标准输出）
  newchanlun-approve show --request FILE --decision approve|reject [--previous RECEIPT] [--raw]
  newchanlun-approve sign --request FILE --decision approve|reject [--previous RECEIPT] --out FILE [--key-dir DIR]

enroll 在本机 Secure Enclave 生成不可导出的签名钥，每次使用都要求本人认证。
sign 先显示待签内容，再弹出系统认证；取消或失败不写回执。
"""

struct Failure: Error, CustomStringConvertible {
    let description: String
}

let keyHandleName = "signing-key.se-handle"

func defaultKeyDir() -> URL {
    FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Application Support/NewChanlunApproval", isDirectory: true)
}

func options(_ args: ArraySlice<String>, allowed: Set<String>, flags: Set<String> = []) throws -> [String: String] {
    var result: [String: String] = [:]
    var rest = args
    while let name = rest.popFirst() {
        if flags.contains(name) {
            result[name] = ""
        } else if allowed.contains(name), let value = rest.popFirst(), result[name] == nil {
            result[name] = value
        } else {
            throw Failure(description: "无法识别的参数 \(name)\n\(usage)")
        }
    }
    return result
}

/// Create a file that must not exist yet, readable only by the owner.
func writeNew(_ data: Data, to url: URL) throws {
    let fd = open(url.path, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0o600)
    guard fd >= 0 else { throw Failure(description: "\(url.path) 已存在或无法创建: \(String(cString: strerror(errno)))") }
    defer { close(fd) }
    let written = data.withUnsafeBytes { write(fd, $0.baseAddress, $0.count) }
    guard written == data.count, fsync(fd) == 0 else { throw Failure(description: "写入 \(url.path) 失败") }
}

func userPresenceAccessControl() throws -> SecAccessControl {
    var error: Unmanaged<CFError>?
    guard let control = SecAccessControlCreateWithFlags(
        kCFAllocatorDefault, kSecAttrAccessibleWhenUnlockedThisDeviceOnly, [.privateKeyUsage, .userPresence], &error
    ) else {
        throw Failure(description: "无法创建访问控制: \(error!.takeRetainedValue())")
    }
    return control
}

/// Signs with the Secure Enclave key. Every call evaluates user presence on a new LAContext and invalidates it.
struct SecureEnclaveSigner: PresenceSigner {
    let handle: Data
    let publicKeySPKI: Data

    init(keyDir: URL) throws {
        guard SecureEnclave.isAvailable else { throw Failure(description: "本机 Secure Enclave 不可用") }
        handle = try Data(contentsOf: keyDir.appendingPathComponent(keyHandleName))
        publicKeySPKI = try SecureEnclave.P256.Signing.PrivateKey(dataRepresentation: handle).publicKey.derRepresentation
    }

    func sign(_ payload: Data, reason: String) throws -> Data {
        let context = LAContext()
        context.touchIDAuthenticationAllowableReuseDuration = 0
        context.localizedReason = reason
        defer { context.invalidate() }

        let done = DispatchSemaphore(value: 0)
        final class Outcome: @unchecked Sendable { var error: Error? = Failure(description: "认证未完成") }
        let outcome = Outcome()
        context.evaluateAccessControl(try userPresenceAccessControl(), operation: .useKeySign,
                                      localizedReason: reason) { success, error in
            outcome.error = success ? nil : (error ?? Failure(description: "认证失败"))
            done.signal()
        }
        done.wait()
        if let error = outcome.error { throw Failure(description: "未签名: \(error.localizedDescription)") }

        let key = try SecureEnclave.P256.Signing.PrivateKey(dataRepresentation: handle, authenticationContext: context)
        return try key.signature(for: payload).derRepresentation
    }
}

func readRequest(_ opts: [String: String]) throws -> (ApprovalRequest, Decision, Data?) {
    guard let requestPath = opts["--request"], let decisionText = opts["--decision"] else {
        throw Failure(description: usage)
    }
    guard let decision = Decision(rawValue: decisionText) else { throw Failure(description: "--decision 只能是 approve 或 reject") }
    let request = try ApprovalRequest.parse(Data(contentsOf: URL(fileURLWithPath: requestPath)))
    let previous = try opts["--previous"].map { try Data(contentsOf: URL(fileURLWithPath: $0)) }
    return (request, decision, previous)
}

func run(_ args: [String]) throws {
    guard let command = args.first else { throw Failure(description: usage) }
    let rest = args.dropFirst()
    switch command {
    case "enroll":
        let opts = try options(rest, allowed: ["--key-dir", "--label"])
        let dir = opts["--key-dir"].map { URL(fileURLWithPath: $0, isDirectory: true) } ?? defaultKeyDir()
        guard SecureEnclave.isAvailable else { throw Failure(description: "本机 Secure Enclave 不可用") }
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true,
                                                attributes: [.posixPermissions: 0o700])
        let key = try SecureEnclave.P256.Signing.PrivateKey(compactRepresentable: false,
                                                            accessControl: userPresenceAccessControl())
        try writeNew(key.dataRepresentation, to: dir.appendingPathComponent(keyHandleName))
        let spki = key.publicKey.derRepresentation
        let label = opts["--label"] ?? "\(Host.current().localizedName ?? "this Mac") Secure Enclave"
        // The trust config comes from the key just created in this process, never from a file on disk.
        FileHandle.standardOutput.write(try trustConfig(publicKeySPKI: spki, label: label, repositoryID: repositoryID))
        FileHandle.standardError.write(Data("已在 Secure Enclave 生成签名钥。key_id \(hexDigest(spki))\n".utf8))
    case "show":
        let opts = try options(rest, allowed: ["--request", "--decision", "--previous"], flags: ["--raw"])
        let (request, decision, previous) = try readRequest(opts)
        let bytes = try composePayload(request: request, decision: decision, previousReceipt: previous)
        if opts["--raw"] != nil {
            FileHandle.standardOutput.write(bytes)
        } else {
            print(try ApprovalPayload.parse(bytes).rendered(raw: bytes))
        }
    case "sign":
        let opts = try options(rest, allowed: ["--request", "--decision", "--previous", "--out", "--key-dir"])
        guard let out = opts["--out"] else { throw Failure(description: usage) }
        let (request, decision, previous) = try readRequest(opts)
        let dir = opts["--key-dir"].map { URL(fileURLWithPath: $0, isDirectory: true) } ?? defaultKeyDir()
        let receipt = try signDecision(request: request, decision: decision, previousReceipt: previous,
                                       signer: SecureEnclaveSigner(keyDir: dir)) { print($0) }
        try writeNew(receipt.json, to: URL(fileURLWithPath: out))
        print("已签名。回执 \(out)，receipt_id \(receipt.receiptID)")
    default:
        throw Failure(description: usage)
    }
}

do {
    try run(Array(CommandLine.arguments.dropFirst()))
} catch {
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(1)
}
