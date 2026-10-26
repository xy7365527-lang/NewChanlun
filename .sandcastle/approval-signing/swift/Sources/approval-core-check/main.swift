import ApprovalCore
import CryptoKit
import Foundation

// Offline checks of ApprovalCore plus cross-language fixtures for the Python verifier.
// Uses a software P-256 test key, which is why it is a separate executable from the helper.

struct TestKeySigner: PresenceSigner {
    let key = P256.Signing.PrivateKey()
    var publicKeySPKI: Data { key.publicKey.derRepresentation }
    func sign(_ payload: Data, reason: String) throws -> Data { try key.signature(for: payload).derRepresentation }
}

struct CancelledSigner: PresenceSigner {
    struct Cancelled: Error {}
    let publicKeySPKI = Data([0])
    func sign(_ payload: Data, reason: String) throws -> Data { throw Cancelled() }
}

var failures = 0
func check(_ ok: Bool, _ what: String) {
    print("\(ok ? "PASS" : "FAIL") \(what)")
    if !ok { failures += 1 }
}

func rejects(_ text: String) -> Bool { (try? ApprovalRequest.parse(Data(text.utf8))) == nil }

let requestText = """
newchanlun-approval-request/v1
repository_id=1150335160
ticket=1742
run_id=t1742-run.1
claim_generation=3
scope=candidate_head
target=commit:120421fe338a9fcf8467b63aa7d6a28e3f871b8b
request_nonce=0123456789abcdef0123456789abcdef
expires_at=2099-01-01T00:00:00Z

"""
let requestBytes = Data(requestText.utf8)
let request = try ApprovalRequest.parse(requestBytes)
check(request.encode() == requestBytes, "request round-trips byte for byte")
check(rejects(requestText.replacingOccurrences(of: "\n", with: "\r\n")), "CRLF rejected")
check(rejects(requestText.replacingOccurrences(of: "ticket=1742", with: "ticket=01742")), "leading zero rejected")
check(rejects(requestText.replacingOccurrences(of: "120421fe", with: "120421FE")), "uppercase hex rejected")
check(rejects(requestText.replacingOccurrences(of: "scope=candidate_head", with: "scope=main_merge")), "unknown scope rejected")
check(rejects(requestText.replacingOccurrences(of: "commit:", with: "spec:1340:sha256:")), "target must match scope")
check(rejects(requestText.replacingOccurrences(of: "2099-01-01", with: "2099-02-30")), "impossible date rejected")
check(rejects(requestText + "extra=1\n"), "extra field rejected")
check(rejects(String(requestText.dropLast())), "missing final LF rejected")

var shown: [String] = []
let signer = TestKeySigner()
let first = try signDecision(request: request, decision: .approve, previousReceipt: nil, signer: signer) { shown.append($0) }
let firstPayload = try ApprovalPayload.parse(first.payload)
check(firstPayload.sequence == 1 && firstPayload.previousReceipt == nil, "first decision is sequence 1")
check(shown.count == 1 && shown[0].contains(String(decoding: first.payload, as: UTF8.self)),
      "display contains the exact signed bytes")
check(shown[0].contains("#1742") && shown[0].contains("commit:120421fe338a9fcf8467b63aa7d6a28e3f871b8b"),
      "display shows ticket and full head")
check(firstPayload.systemReason.contains("#1742") && firstPayload.systemReason.contains("120421fe338a9fcf8467b63aa7d6a28e3f871b8b"),
      "system prompt reason names ticket and full head")

let second = try signDecision(request: request, decision: .reject, previousReceipt: first.json, signer: signer) { _ in }
let secondPayload = try ApprovalPayload.parse(second.payload)
check(secondPayload.sequence == 2 && secondPayload.previousReceipt == first.receiptID, "reject chains after approve")

var cancelledShown = false
do {
    _ = try signDecision(request: request, decision: .approve, previousReceipt: nil, signer: CancelledSigner()) { _ in
        cancelledShown = true
    }
    check(false, "cancelled authentication yields no receipt")
} catch {
    check(cancelledShown, "cancelled authentication yields no receipt")
}

let otherRequest = try ApprovalRequest.parse(Data(requestText.replacingOccurrences(
    of: "120421fe338a9fcf8467b63aa7d6a28e3f871b8b", with: "f".padding(toLength: 40, withPad: "f", startingAt: 0)).utf8))
check((try? composePayload(request: otherRequest, decision: .approve, previousReceipt: first.json)) == nil,
      "previous receipt for another head is refused")

if CommandLine.arguments.count == 2 {
    let dir = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    try requestBytes.write(to: dir.appendingPathComponent("request.txt"))
    try first.json.write(to: dir.appendingPathComponent("receipt-1-approve.json"))
    try second.json.write(to: dir.appendingPathComponent("receipt-2-reject.json"))
    try trustConfig(publicKeySPKI: signer.publicKeySPKI, label: "approval-core-check software test key",
                    repositoryID: 1_150_335_160).write(to: dir.appendingPathComponent("trust.json"))
    print("fixtures written to \(dir.path)")
}

exit(failures == 0 ? 0 : 1)
