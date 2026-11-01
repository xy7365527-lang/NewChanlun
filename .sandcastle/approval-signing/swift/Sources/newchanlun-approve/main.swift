import ApprovalCore
import CryptoKit
import Foundation
import LocalAuthentication
import Security

let repositoryID = 1_150_335_160
let usage = """
用法:
  newchanlun-approve enroll [--key-dir DIR] [--label TEXT]   （信任配置写到标准输出）
  newchanlun-approve status --request FILE
  newchanlun-approve show --request FILE --decision approve|reject [--raw]
  newchanlun-approve sign --request FILE --decision approve|reject [--key-dir DIR] [--enrollment-cancel-publish-check]
  newchanlun-approve publish-pending --receipt FILE

sign 先显示待签内容，再弹出系统认证签名，随后弹出 macOS 管理员认证窗口，把回执发布到 root 所有的权威记录。
只有发布成功的决定才生效。签名认证取消则不签名；管理员认证取消或失败则回执停在“待发布”，未生效。
publish-pending 用同样的管理员认证窗口补发布一份待发布回执。
status 显示权威记录里这一请求当前生效的决定。
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

func readRequest(_ opts: [String: String]) throws -> ApprovalRequest {
    guard let requestPath = opts["--request"] else { throw Failure(description: usage) }
    return try ApprovalRequest.parse(Data(contentsOf: URL(fileURLWithPath: requestPath)))
}

func readDecision(_ opts: [String: String]) throws -> Decision {
    guard let text = opts["--decision"], let decision = Decision(rawValue: text) else {
        throw Failure(description: "--decision 只能是 approve 或 reject")
    }
    return decision
}

func verdictLine(_ link: VerifiedReceipt) -> String {
    let verb = link.payload.decision == .approve ? "批准" : "拒绝"
    return "第 \(link.payload.sequence) 条 \(verb)  receipt_id \(link.receiptID)"
}

/// Publish through macOS's administrator dialog with the root-installed helper. The verified receipt bytes
/// travel in the command line, never through a file. True only once the record reads back with this receipt.
func publishWithAdminDialog(_ receipt: VerifiedReceipt, trust: TrustRoot, cancellationNotice: String = "") -> Bool {
    print(receipt.payload.rendered(raw: receipt.payloadBytes))
    print("现在发布到权威记录：macOS 会弹出管理员认证窗口。")
    let outcome = runAdminDialog([installedHelperPath, "publish", "--receipt-b64", receipt.canonicalJSON.base64EncodedString()],
                                 prompt: publishPrompt(receipt.payload, cancellationNotice: cancellationNotice))
    if !outcome.output.isEmpty { print(outcome.output, terminator: "") }
    if !outcome.succeeded {
        FileHandle.standardError.write(Data((outcome.cancelled ? "管理员认证已取消。\n" : outcome.error).utf8))
        return false
    }
    let sequence = receipt.payload.sequence
    guard let chain = try? AuthorityStore().chain(nonce: receipt.payload.request.requestNonce, trust: trust),
          chain.count >= sequence, chain[sequence - 1].receiptID == receipt.receiptID
    else {
        FileHandle.standardError.write(Data("权威记录读回时没有这份回执。\n".utf8))
        return false
    }
    print("已发布到权威记录：\(verdictLine(chain[sequence - 1]))。现在生效。")
    return true
}

func notPublished(_ decision: Decision, pending: String?) -> Never {
    let retry = pending.map { "回执存于 \($0)。确认后重试：\"\(installedHelperPath)\" publish-pending --receipt \"\($0)\"" }
        ?? "回执未能存盘，需重新 sign。"
    print("""

    !!!!!!!! 未发布：这个\(decision == .approve ? "批准" : "拒绝")还没有生效，执行器看不到它。 !!!!!!!!
    \(retry)
    """)
    exit(3)
}

func savePending(_ receipt: Receipt, keyDir: URL) -> String? {
    let dir = keyDir.appendingPathComponent("pending", isDirectory: true)
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true,
                                             attributes: [.posixPermissions: 0o700])
    let url = dir.appendingPathComponent("\(receipt.receiptID).json")
    if (try? writeNew(receipt.json, to: url)) != nil || FileManager.default.fileExists(atPath: url.path) {
        return url.path
    }
    return nil
}

func confirmOnTerminal() throws {
    guard let tty = FileHandle(forUpdatingAtPath: "/dev/tty") else { throw Failure(description: "需要在终端里确认，未发布") }
    tty.write(Data("核对上面的内容。输入 publish 并回车才会发布：".utf8))
    var line = Data()
    while true {
        let byte = tty.readData(ofLength: 1)
        if byte.isEmpty || byte == Data([0x0A]) { break }
        line.append(byte)
    }
    guard String(decoding: line, as: UTF8.self).trimmingCharacters(in: .whitespaces) == "publish" else {
        throw Failure(description: "未确认，未发布，决定没有生效")
    }
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
    case "status":
        let opts = try options(rest, allowed: ["--request"])
        let request = try readRequest(opts)
        let chain = try AuthorityStore().chain(nonce: request.requestNonce, trust: loadInstalledTrust())
        print("请求 #\(request.ticket) \(request.scope.rawValue) \(request.target)，nonce \(request.requestNonce)，过期 \(request.expiresAt)")
        chain.forEach { print("  " + verdictLine($0)) }
        if let head = chain.last {
            print("当前生效：\(head.payload.decision == .approve ? "批准" : "拒绝")（第 \(head.payload.sequence) 条）")
        } else {
            print("权威记录里还没有这一请求的已发布决定。")
        }
    case "show":
        let opts = try options(rest, allowed: ["--request", "--decision"], flags: ["--raw"])
        let request = try readRequest(opts)
        let head = try AuthorityStore().chain(nonce: request.requestNonce, trust: loadInstalledTrust()).last
        let bytes = try composePayload(request: request, decision: readDecision(opts), after: head)
        if opts["--raw"] != nil {
            FileHandle.standardOutput.write(bytes)
        } else {
            print(try ApprovalPayload.parse(bytes).rendered(raw: bytes))
        }
    case "sign":
        let opts = try options(rest, allowed: ["--request", "--decision", "--key-dir"],
                               flags: ["--enrollment-cancel-publish-check"])
        let request = try readRequest(opts)
        let decision = try readDecision(opts)
        let cancellationNotice = opts["--enrollment-cancel-publish-check"] == nil ? ""
            : try enrollmentCancellationNotice(request, decision: decision)
        let dir = opts["--key-dir"].map { URL(fileURLWithPath: $0, isDirectory: true) } ?? defaultKeyDir()
        let trust = try loadInstalledTrust()
        let signer = try SecureEnclaveSigner(keyDir: dir)
        guard trust.keys[hexDigest(signer.publicKeySPKI)] != nil else {
            throw Failure(description: "本机签名钥不在已安装的信任配置里")
        }
        let head = try AuthorityStore().chain(nonce: request.requestNonce, trust: trust).last
        let receipt = try signDecision(request: request, decision: decision, after: head, signer: signer) { print($0) }
        let pending = savePending(receipt, keyDir: dir)
        print("已签名，尚未生效。")
        guard let verified = try? verifyReceipt(receipt.json, trust: trust),
              publishWithAdminDialog(verified, trust: trust, cancellationNotice: cancellationNotice)
        else { notPublished(decision, pending: pending) }
    case "publish-pending":
        let opts = try options(rest, allowed: ["--receipt"])
        guard let path = opts["--receipt"] else { throw Failure(description: usage) }
        let trust = try loadInstalledTrust()
        let receipt = try verifyReceipt(Data(contentsOf: URL(fileURLWithPath: path)), trust: trust)
        print("待发布回执 \(path)：")
        guard publishWithAdminDialog(receipt, trust: trust) else { notPublished(receipt.payload.decision, pending: path) }
    case "publish":
        // Root side, reached through the administrator dialog (--receipt-b64) or by hand as `sudo -k ... --receipt FILE`.
        let opts = try options(rest, allowed: ["--receipt", "--receipt-b64"])
        guard (opts["--receipt"] == nil) != (opts["--receipt-b64"] == nil) else { throw Failure(description: usage) }
        guard geteuid() == 0 else { throw Failure(description: "publish 只能以 root 运行；请用 sign 或 publish-pending") }
        let trust = try loadInstalledTrust()
        let raw: Data
        if let path = opts["--receipt"] {
            raw = try Data(contentsOf: URL(fileURLWithPath: path))
        } else {
            guard let text = opts["--receipt-b64"], text.utf8.count <= 87_384, let data = Data(base64Encoded: text) else {
                throw Failure(description: "--receipt-b64 不是 base64 或过大")
            }
            raw = data
        }
        guard raw.count <= 65_536 else { throw Failure(description: "回执过大") }
        let receipt = try verifyReceipt(raw, trust: trust)
        print(receipt.payload.rendered(raw: receipt.payloadBytes))
        if opts["--receipt"] != nil { try confirmOnTerminal() }
        switch try AuthorityStore().publish(receipt, trust: trust) {
        case .published(let link):
            print("已发布到权威记录：\(verdictLine(link))。现在生效。")
        case .alreadyPublished(let link):
            print("这一决定早已发布：\(verdictLine(link))。")
        }
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
