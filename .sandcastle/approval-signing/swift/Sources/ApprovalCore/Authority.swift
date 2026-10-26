import CryptoKit
import Darwin
import Foundation

// The root-owned record of published decisions. A signed receipt takes effect only
// once `publish` (run by the owner under sudo) has written it here. Only root can add
// or remove entries, so the record cannot be rolled back by the owner's other processes.

public let installedDirectory = "/Library/Application Support/NewChanlun"
public let installedHelperPath = installedDirectory + "/newchanlun-approve"
public let trustConfigPath = installedDirectory + "/approval-trust.json"
public let authorityRootPath = installedDirectory + "/approval-receipts"
let pendingPrefix = ".pending-"

public struct AuthorityError: Error, CustomStringConvertible {
    public let description: String
    init(_ description: String) { self.description = description }
}

/// Why a non-root account could change `path`, or nil if root controls it and every ancestor.
public func rootControlledProblem(_ path: String) -> String? {
    var current = path
    while true {
        var st = stat()
        if lstat(current, &st) != 0 { return "\(current): \(String(cString: strerror(errno)))" }
        if st.st_mode & 0o170000 == 0o120000 { return "\(current) 是符号链接" }
        if st.st_uid != 0 { return "\(current) 不归 root 所有" }
        if st.st_mode & 0o022 != 0 { return "\(current) 组或其他人可写" }
        if current == "/" { return nil }
        let parent = (current as NSString).deletingLastPathComponent
        current = parent.isEmpty ? "/" : parent
    }
}

public struct TrustRoot {
    public let repositoryID: String
    public let allowedScopes: Set<Scope>
    public let keys: [String: P256.Signing.PublicKey]
}

func canonicalBase64(_ value: Any?, _ field: String) throws -> Data {
    guard let text = value as? String, let data = Data(base64Encoded: text), data.base64EncodedString() == text else {
        throw AuthorityError("\(field) 不是规范 base64")
    }
    return data
}

public func parseTrust(_ data: Data) throws -> TrustRoot {
    guard let raw = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
          Set(raw.keys) == ["format", "repository_id", "allowed_scopes", "keys"],
          raw["format"] as? String == trustFormat,
          let repository = raw["repository_id"] as? NSNumber, CFGetTypeID(repository) != CFBooleanGetTypeID(),
          matches(decimal, repository.stringValue),
          let scopeNames = raw["allowed_scopes"] as? [String], !scopeNames.isEmpty,
          let entries = raw["keys"] as? [[String: Any]], !entries.isEmpty
    else { throw AuthorityError("信任配置格式不符") }
    let scopes = scopeNames.compactMap(Scope.init(rawValue:))
    guard scopes.count == scopeNames.count else { throw AuthorityError("信任配置含未知 scope") }
    var keys: [String: P256.Signing.PublicKey] = [:]
    for entry in entries {
        guard Set(entry.keys) == ["key_id", "public_key_spki_der_b64", "label"], entry["label"] is String,
              let keyID = entry["key_id"] as? String
        else { throw AuthorityError("信任配置的公钥条目格式不符") }
        let spki = try canonicalBase64(entry["public_key_spki_der_b64"], "public_key_spki_der_b64")
        guard keyID == hexDigest(spki), let key = try? P256.Signing.PublicKey(derRepresentation: spki) else {
            throw AuthorityError("信任配置的 key_id 与公钥不符或不是 P-256")
        }
        keys[keyID] = key
    }
    return TrustRoot(repositoryID: repository.stringValue, allowedScopes: Set(scopes), keys: keys)
}

/// The trust config the helper uses. Refuses any config a non-root account can edit.
public func loadInstalledTrust() throws -> TrustRoot {
    if let problem = rootControlledProblem(trustConfigPath) {
        throw AuthorityError("信任配置不受 root 控制: \(problem)")
    }
    return try parseTrust(Data(contentsOf: URL(fileURLWithPath: trustConfigPath)))
}

public struct VerifiedReceipt {
    public let receiptID: String
    public let keyID: String
    public let payloadBytes: Data
    public let signature: Data
    public let payload: ApprovalPayload

    /// Receipt JSON rebuilt from the verified fields; this, not the submitted text, is what gets stored.
    public var canonicalJSON: Data {
        // Encoding a [String: String] dictionary cannot fail.
        (try? receiptJSON(keyID: keyID, payload: payloadBytes, signature: signature)) ?? Data()
    }
}

public func verifyReceipt(_ data: Data, trust: TrustRoot) throws -> VerifiedReceipt {
    guard let raw = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
          Set(raw.keys) == ["format", "algorithm", "key_id", "payload", "signature"],
          raw["format"] as? String == receiptFormat, raw["algorithm"] as? String == algorithm
    else { throw AuthorityError("回执格式不符") }
    guard let keyID = raw["key_id"] as? String, let key = trust.keys[keyID] else {
        throw AuthorityError("回执的公钥不在信任配置里")
    }
    let payloadBytes = try canonicalBase64(raw["payload"], "payload")
    let signature = try canonicalBase64(raw["signature"], "signature")
    guard let ecdsa = try? P256.Signing.ECDSASignature(derRepresentation: signature),
          key.isValidSignature(ecdsa, for: payloadBytes)
    else { throw AuthorityError("回执签名无效") }
    let payload = try ApprovalPayload.parse(payloadBytes)
    guard payload.request.repositoryID == trust.repositoryID else { throw AuthorityError("回执属于别的仓库") }
    guard trust.allowedScopes.contains(payload.request.scope) else { throw AuthorityError("回执 scope 未获许可") }
    return VerifiedReceipt(receiptID: hexDigest(payloadBytes), keyID: keyID, payloadBytes: payloadBytes,
                           signature: signature, payload: payload)
}

public enum PublishOutcome {
    case published(VerifiedReceipt)
    case alreadyPublished(VerifiedReceipt)
}

public struct AuthorityStore {
    public let root: String
    let problem: (String) -> String?

    /// `problem` decides whether a path is protected. Only tests pass anything other than the root check.
    public init(root: String = authorityRootPath, problem: @escaping (String) -> String? = rootControlledProblem) {
        self.root = root
        self.problem = problem
    }

    func directory(_ nonce: String) -> String { root + "/" + nonce }

    /// Every published decision for `nonce`, in order and checked link by link. Empty if none is published.
    /// Any foreign entry, unprotected file, bad signature, gap or broken link throws.
    public func chain(nonce: String, trust: TrustRoot) throws -> [VerifiedReceipt] {
        guard matches("[0-9a-f]{32}", nonce) else { throw AuthorityError("nonce 格式不符") }
        if let issue = problem(root) { throw AuthorityError("权威记录目录不受保护: \(issue)") }
        let dir = directory(nonce)
        var st = stat()
        if lstat(dir, &st) != 0 {
            if errno == ENOENT { return [] }
            throw AuthorityError("\(dir): \(String(cString: strerror(errno)))")
        }
        if let issue = problem(dir) { throw AuthorityError("权威记录不受保护: \(issue)") }
        guard st.st_mode & 0o170000 == 0o040000 else { throw AuthorityError("\(dir) 不是目录") }

        var bySequence: [Int: VerifiedReceipt] = [:]
        for name in try FileManager.default.contentsOfDirectory(atPath: dir) where !name.hasPrefix(pendingPrefix) {
            guard matches("[1-9][0-9]{0,17}\\.json", name), let sequence = Int(name.dropLast(5)) else {
                throw AuthorityError("权威记录里有陌生条目 \(name)")
            }
            let path = dir + "/" + name
            if let issue = problem(path) { throw AuthorityError("权威记录不受保护: \(issue)") }
            guard lstat(path, &st) == 0, st.st_mode & 0o170000 == 0o100000 else {
                throw AuthorityError("\(path) 不是普通文件")
            }
            let receipt = try verifyReceipt(Data(contentsOf: URL(fileURLWithPath: path)), trust: trust)
            guard receipt.payload.sequence == sequence, receipt.payload.request.requestNonce == nonce else {
                throw AuthorityError("\(name) 的内容与它的位置不符")
            }
            bySequence[sequence] = receipt
        }
        let chain = bySequence.keys.sorted().map { bySequence[$0]! }
        for (index, link) in chain.enumerated() {
            guard link.payload.sequence == index + 1,
                  link.payload.previousReceipt == (index == 0 ? nil : chain[index - 1].receiptID),
                  link.payload.request == chain[0].payload.request
            else { throw AuthorityError("权威记录在第 \(index + 1) 条断链") }
        }
        return chain
    }

    /// Append `receipt` as the next link of its request's chain. Refuses forks, gaps and foreign requests.
    public func publish(_ receipt: VerifiedReceipt, trust: TrustRoot) throws -> PublishOutcome {
        let nonce = receipt.payload.request.requestNonce
        let sequence = receipt.payload.sequence
        let existing = try chain(nonce: nonce, trust: trust)
        if sequence <= existing.count {
            if existing[sequence - 1].receiptID == receipt.receiptID { return .alreadyPublished(existing[sequence - 1]) }
            throw AuthorityError("第 \(sequence) 条已发布为另一个决定。用 status 看当前状态，再重新签。")
        }
        guard sequence == existing.count + 1 else {
            throw AuthorityError("权威记录只到第 \(existing.count) 条，不能发布第 \(sequence) 条")
        }
        if let head = existing.last {
            guard receipt.payload.previousReceipt == head.receiptID, receipt.payload.request == head.payload.request else {
                throw AuthorityError("回执没有接在当前已发布的第 \(head.payload.sequence) 条之后")
            }
        }

        let dir = directory(nonce)
        if mkdir(dir, 0o755) != 0 && errno != EEXIST {
            throw AuthorityError("无法创建 \(dir): \(String(cString: strerror(errno)))")
        }
        if let issue = problem(dir) { throw AuthorityError("权威记录不受保护: \(issue)") }
        let temp = dir + "/" + pendingPrefix + "\(sequence)-\(getpid())"
        let final = dir + "/\(sequence).json"
        try writeDurably(receipt.canonicalJSON, to: temp)
        if renamex_np(temp, final, UInt32(RENAME_EXCL)) != 0 {
            let failure = errno
            unlink(temp)
            if failure == EEXIST, let now = try? chain(nonce: nonce, trust: trust), now.count >= sequence,
               now[sequence - 1].receiptID == receipt.receiptID {
                return .alreadyPublished(now[sequence - 1])
            }
            throw AuthorityError("发布第 \(sequence) 条失败: \(String(cString: strerror(failure)))")
        }
        let dirFD = open(dir, O_RDONLY)
        if dirFD >= 0 {
            fsync(dirFD)
            close(dirFD)
        }
        return .published(receipt)
    }
}

func writeDurably(_ data: Data, to path: String) throws {
    let fd = open(path, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0o644)
    guard fd >= 0 else { throw AuthorityError("无法创建 \(path): \(String(cString: strerror(errno)))") }
    defer { close(fd) }
    let written = data.withUnsafeBytes { write(fd, $0.baseAddress, $0.count) }
    guard fchmod(fd, 0o644) == 0, written == data.count, fsync(fd) == 0 else {
        unlink(path)
        throw AuthorityError("写入 \(path) 失败")
    }
}
