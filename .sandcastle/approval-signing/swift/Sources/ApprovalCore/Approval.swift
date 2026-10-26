import CryptoKit
import Foundation

// Byte format shared with approval_signing/protocol.py. Both sides accept bytes
// only if re-encoding the parsed value reproduces them exactly.

public enum ApprovalError: Error, CustomStringConvertible {
    case malformed(String)
    case previousMismatch(String)

    public var description: String {
        switch self {
        case .malformed(let why): return "格式不符: \(why)"
        case .previousMismatch(let why): return "前序回执不属于本请求: \(why)"
        }
    }
}

public enum Scope: String {
    case candidateHead = "candidate_head"
    case specAuthorization = "spec_authorization"

    var targetPattern: String {
        switch self {
        case .candidateHead: return "commit:[0-9a-f]{40}"
        case .specAuthorization: return "spec:[1-9][0-9]{0,17}:sha256:[0-9a-f]{64}"
        }
    }
}

public enum Decision: String {
    case approve
    case reject
}

let requestHeader = "newchanlun-approval-request/v1"
let payloadHeader = "newchanlun-approval/v1"
let receiptFormat = "newchanlun-approval-receipt/v1"
let algorithm = "ecdsa-p256-sha256"
let requestKeys = ["repository_id", "ticket", "run_id", "claim_generation", "scope", "target",
                   "request_nonce", "expires_at"]
let payloadKeys = ["repository_id", "ticket", "run_id", "claim_generation", "scope", "target",
                   "decision", "request_nonce", "sequence", "previous_receipt", "expires_at"]

func matches(_ pattern: String, _ text: String) -> Bool {
    let regex = try! Regex(pattern)
    return text.wholeMatch(of: regex) != nil
}

let decimal = "0|[1-9][0-9]{0,17}"

public struct ApprovalRequest: Equatable {
    public let repositoryID: String
    public let ticket: String
    public let runID: String
    public let claimGeneration: String
    public let scope: Scope
    public let target: String
    public let requestNonce: String
    public let expiresAt: String

    init(fields v: [String: String]) throws {
        guard let scope = Scope(rawValue: v["scope"]!) else { throw ApprovalError.malformed("scope") }
        for key in ["repository_id", "ticket", "claim_generation"] where !matches(decimal, v[key]!) {
            throw ApprovalError.malformed(key)
        }
        guard matches("[A-Za-z0-9._-]{1,64}", v["run_id"]!) else { throw ApprovalError.malformed("run_id") }
        guard matches(scope.targetPattern, v["target"]!) else { throw ApprovalError.malformed("target") }
        guard matches("[0-9a-f]{32}", v["request_nonce"]!) else { throw ApprovalError.malformed("request_nonce") }
        try Self.checkTime(v["expires_at"]!)
        repositoryID = v["repository_id"]!
        ticket = v["ticket"]!
        runID = v["run_id"]!
        claimGeneration = v["claim_generation"]!
        self.scope = scope
        target = v["target"]!
        requestNonce = v["request_nonce"]!
        expiresAt = v["expires_at"]!
    }

    static func checkTime(_ text: String) throws {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        formatter.timeZone = TimeZone(identifier: "UTC")
        guard matches("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z", text),
              let date = formatter.date(from: text), formatter.string(from: date) == text
        else { throw ApprovalError.malformed("expires_at") }
    }

    var leadingFields: [(String, String)] {
        [("repository_id", repositoryID), ("ticket", ticket), ("run_id", runID),
         ("claim_generation", claimGeneration), ("scope", scope.rawValue), ("target", target)]
    }

    public func encode() -> Data {
        encodeLines(requestHeader, leadingFields + [("request_nonce", requestNonce), ("expires_at", expiresAt)])
    }

    public static func parse(_ data: Data) throws -> ApprovalRequest {
        let request = try ApprovalRequest(fields: parseLines(data, header: requestHeader, keys: requestKeys))
        guard request.encode() == data else { throw ApprovalError.malformed("request bytes are not canonical") }
        return request
    }
}

public struct ApprovalPayload: Equatable {
    public let request: ApprovalRequest
    public let decision: Decision
    public let sequence: Int
    public let previousReceipt: String?

    public init(request: ApprovalRequest, decision: Decision, sequence: Int, previousReceipt: String?) throws {
        guard sequence >= 1, (sequence == 1) == (previousReceipt == nil) else {
            throw ApprovalError.malformed("sequence/previous_receipt")
        }
        if let previous = previousReceipt, !matches("[0-9a-f]{64}", previous) {
            throw ApprovalError.malformed("previous_receipt")
        }
        self.request = request
        self.decision = decision
        self.sequence = sequence
        self.previousReceipt = previousReceipt
    }

    public func encode() -> Data {
        encodeLines(payloadHeader, request.leadingFields + [
            ("decision", decision.rawValue),
            ("request_nonce", request.requestNonce),
            ("sequence", String(sequence)),
            ("previous_receipt", previousReceipt ?? "none"),
            ("expires_at", request.expiresAt),
        ])
    }

    public static func parse(_ data: Data) throws -> ApprovalPayload {
        let v = try parseLines(data, header: payloadHeader, keys: payloadKeys)
        guard let decision = Decision(rawValue: v["decision"]!) else { throw ApprovalError.malformed("decision") }
        guard matches(decimal, v["sequence"]!), let sequence = Int(v["sequence"]!) else {
            throw ApprovalError.malformed("sequence")
        }
        let previous = v["previous_receipt"]! == "none" ? nil : v["previous_receipt"]!
        let payload = try ApprovalPayload(request: ApprovalRequest(fields: v), decision: decision,
                                          sequence: sequence, previousReceipt: previous)
        guard payload.encode() == data else { throw ApprovalError.malformed("payload bytes are not canonical") }
        return payload
    }

    /// Text shown to the human before the system prompt. Built from the parsed bytes, not from inputs.
    public func rendered(raw: Data) -> String {
        let verb = decision == .approve ? "批准 (approve)" : "拒绝 (reject)"
        let chain = previousReceipt.map { "第 \(sequence) 条，接在 \($0) 之后" } ?? "第 1 条"
        return """
        ===== 待签内容（由将要签名的字节解析）=====
          仓库 ID      \(request.repositoryID)
          票号         #\(request.ticket)
          运行         \(request.runID)，claim generation \(request.claimGeneration)
          范围         \(request.scope.rawValue)
          目标         \(request.target)
          决定         \(verb)
          请求 nonce   \(request.requestNonce)
          决定链       \(chain)
          过期         \(request.expiresAt)
        ----- 将被签名的确切字节 -----
        \(String(decoding: raw, as: UTF8.self))-----
        """
    }

    /// Reason string the system authentication dialog shows.
    public var systemReason: String {
        let verb = decision == .approve ? "批准" : "拒绝"
        return "签署\(verb) #\(request.ticket) \(request.scope.rawValue) \(request.target)"
    }
}

func encodeLines(_ header: String, _ fields: [(String, String)]) -> Data {
    Data(([header] + fields.map { "\($0.0)=\($0.1)" }).joined(separator: "\n").appending("\n").utf8)
}

func parseLines(_ data: Data, header: String, keys: [String]) throws -> [String: String] {
    guard data.allSatisfy({ $0 < 0x80 }), let text = String(data: data, encoding: .ascii) else {
        throw ApprovalError.malformed("bytes must be ASCII")
    }
    guard text.hasSuffix("\n") else { throw ApprovalError.malformed("must end with LF") }
    let lines = text.dropLast().split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
    guard lines.first == header else { throw ApprovalError.malformed("expected header \(header)") }
    guard lines.count - 1 == keys.count else { throw ApprovalError.malformed("wrong number of fields") }
    var values: [String: String] = [:]
    for (key, line) in zip(keys, lines.dropFirst()) {
        guard let eq = line.firstIndex(of: "="), line[..<eq] == key, line.index(after: eq) < line.endIndex else {
            throw ApprovalError.malformed("expected field \(key)")
        }
        values[key] = String(line[line.index(after: eq)...])
    }
    return values
}

public func hexDigest(_ data: Data) -> String {
    SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
}

/// Something that signs one payload only after the user proves presence for that call.
public protocol PresenceSigner {
    var publicKeySPKI: Data { get }
    /// Throws when the user cancels or authentication fails. Must not reuse an earlier authentication.
    func sign(_ payload: Data, reason: String) throws -> Data
}

public struct Receipt {
    public let payload: Data
    public let receiptID: String
    public let json: Data
}

/// Payload bytes for `decision` on `request`, chained after `previousReceipt` when given.
public func composePayload(request: ApprovalRequest, decision: Decision, previousReceipt: Data?) throws -> Data {
    guard let previousReceipt else {
        return try ApprovalPayload(request: request, decision: decision, sequence: 1, previousReceipt: nil).encode()
    }
    guard let object = try? JSONSerialization.jsonObject(with: previousReceipt) as? [String: Any],
          let encoded = object["payload"] as? String, let previousBytes = Data(base64Encoded: encoded)
    else { throw ApprovalError.previousMismatch("previous receipt is not a receipt") }
    let previous = try ApprovalPayload.parse(previousBytes)
    guard previous.request == request else {
        throw ApprovalError.previousMismatch("ticket, run, scope, target, nonce or expiry differ")
    }
    return try ApprovalPayload(request: request, decision: decision, sequence: previous.sequence + 1,
                               previousReceipt: hexDigest(previousBytes)).encode()
}

/// Show the exact payload, get one presence-gated signature, return the receipt. Throws without a receipt on refusal.
public func signDecision(
    request: ApprovalRequest, decision: Decision, previousReceipt: Data?,
    signer: PresenceSigner, present: (String) -> Void
) throws -> Receipt {
    let bytes = try composePayload(request: request, decision: decision, previousReceipt: previousReceipt)
    let shown = try ApprovalPayload.parse(bytes)
    present(shown.rendered(raw: bytes))
    let signature = try signer.sign(bytes, reason: shown.systemReason)
    let object: [String: String] = [
        "format": receiptFormat,
        "algorithm": algorithm,
        "key_id": hexDigest(signer.publicKeySPKI),
        "payload": bytes.base64EncodedString(),
        "signature": signature.base64EncodedString(),
    ]
    let json = try JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])
    return Receipt(payload: bytes, receiptID: hexDigest(bytes), json: json + Data("\n".utf8))
}

/// Trust-config entry for a public key, for the operator to install as root.
public func trustConfig(publicKeySPKI: Data, label: String, repositoryID: Int) throws -> Data {
    let object: [String: Any] = [
        "format": "newchanlun-approval-trust/v1",
        "repository_id": repositoryID,
        "allowed_scopes": [Scope.candidateHead.rawValue, Scope.specAuthorization.rawValue],
        "keys": [[
            "key_id": hexDigest(publicKeySPKI),
            "public_key_spki_der_b64": publicKeySPKI.base64EncodedString(),
            "label": label,
        ]],
    ]
    return try JSONSerialization.data(withJSONObject: object, options: [.sortedKeys, .prettyPrinted])
        + Data("\n".utf8)
}
