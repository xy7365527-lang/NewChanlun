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
for (when, accepted) in [("2000-02-29T00:00:00Z", true), ("2096-02-29T23:59:59Z", true),
                         ("2001-02-29T00:00:00Z", false), ("1500-02-29T00:00:00Z", false),
                         ("1582-10-10T00:00:00Z", false), ("2100-01-01T00:00:00Z", false),
                         ("2099-12-31T24:00:00Z", false)] {
    check(rejects(requestText.replacingOccurrences(of: "2099-01-01T00:00:00Z", with: when)) != accepted,
          "expiry \(when) \(accepted ? "accepted" : "rejected") as in protocol.py")
}
check(rejects(String(requestText.dropLast())), "missing final LF rejected")

var shown: [String] = []
let signer = TestKeySigner()
let trust = try parseTrust(trustConfig(publicKeySPKI: signer.publicKeySPKI, label: "test", repositoryID: 1_150_335_160))
let first = try signDecision(request: request, decision: .approve, after: nil, signer: signer) { shown.append($0) }
let firstVerified = try verifyReceipt(first.json, trust: trust)
let firstPayload = try ApprovalPayload.parse(first.payload)
check(firstPayload.sequence == 1 && firstPayload.previousReceipt == nil, "first decision is sequence 1")
check(shown.count == 1 && shown[0].contains(String(decoding: first.payload, as: UTF8.self)),
      "display contains the exact signed bytes")
check(shown[0].contains("#1742") && shown[0].contains("commit:120421fe338a9fcf8467b63aa7d6a28e3f871b8b"),
      "display shows ticket and full head")
check(firstPayload.systemReason.contains("#1742") && firstPayload.systemReason.contains("120421fe338a9fcf8467b63aa7d6a28e3f871b8b"),
      "system prompt reason names ticket and full head")

let second = try signDecision(request: request, decision: .reject, after: firstVerified, signer: signer) { _ in }
let secondVerified = try verifyReceipt(second.json, trust: trust)
let secondPayload = try ApprovalPayload.parse(second.payload)
check(secondPayload.sequence == 2 && secondPayload.previousReceipt == first.receiptID, "reject chains after approve")

var cancelledShown = false
do {
    _ = try signDecision(request: request, decision: .approve, after: nil, signer: CancelledSigner()) { _ in
        cancelledShown = true
    }
    check(false, "cancelled authentication yields no receipt")
} catch {
    check(cancelledShown, "cancelled authentication yields no receipt")
}

let otherRequest = try ApprovalRequest.parse(Data(requestText.replacingOccurrences(
    of: "120421fe338a9fcf8467b63aa7d6a28e3f871b8b", with: "f".padding(toLength: 40, withPad: "f", startingAt: 0)).utf8))
check((try? composePayload(request: otherRequest, decision: .approve, after: firstVerified)) == nil,
      "previous receipt for another head is refused")

func refused(_ what: String, _ body: () throws -> Void) {
    do {
        try body()
        check(false, what)
    } catch {
        check(true, what)
    }
}

refused("receipt from an untrusted key is refused") {
    _ = try verifyReceipt(try makeReceipt(first.payload, signer: TestKeySigner(), reason: "").json, trust: trust)
}
refused("receipt with a changed payload byte is refused") {
    let tampered = Data(String(decoding: first.json, as: UTF8.self)
        .replacingOccurrences(of: first.payload.base64EncodedString(),
                              with: Data(String(decoding: first.payload, as: UTF8.self)
                                .replacingOccurrences(of: "approve", with: "reject").utf8).base64EncodedString()).utf8)
    _ = try verifyReceipt(tampered, trust: trust)
}

let canonical = String(decoding: first.json, as: UTF8.self)
refused("a receipt with a duplicate payload key is refused") {
    let otherPayload = second.payload.base64EncodedString()
    _ = try verifyReceipt(Data(canonical.replacingOccurrences(of: "{", with: "{\"payload\":\"\(otherPayload)\",").utf8),
                          trust: trust)
}
refused("a receipt spelled with extra whitespace is refused") {
    _ = try verifyReceipt(Data(canonical.replacingOccurrences(of: "\",\"", with: "\", \"").utf8), trust: trust)
}

let scratch = FileManager.default.temporaryDirectory.appendingPathComponent("approval-core-check-\(getpid())").path
try FileManager.default.createDirectory(atPath: scratch, withIntermediateDirectories: true)
defer { try? FileManager.default.removeItem(atPath: scratch) }
check(rootControlledProblem(scratch) != nil, "a user-owned directory is not a protected record")
check(rootControlledProblem("/bin/ls") == nil, "a root-owned path is protected")
refused("the real store refuses a user-owned record directory") {
    _ = try AuthorityStore(root: scratch).chain(nonce: request.requestNonce, trust: trust)
}

func isNew(_ outcome: PublishOutcome) -> Bool {
    if case .published = outcome { return true }
    return false
}
let store = AuthorityStore(root: scratch, problem: { _ in nil })
let nonceDir = scratch + "/" + request.requestNonce
check(try store.chain(nonce: request.requestNonce, trust: trust).isEmpty, "nothing published yet")
let third = try signDecision(request: request, decision: .approve, after: secondVerified, signer: signer) { _ in }
refused("a gap is refused") { _ = try store.publish(try verifyReceipt(third.json, trust: trust), trust: trust) }
check(isNew(try store.publish(firstVerified, trust: trust)), "first decision published")
check(!isNew(try store.publish(firstVerified, trust: trust)), "republishing is idempotent")
let fork = try signDecision(request: request, decision: .approve, after: firstVerified, signer: signer) { _ in }
check(isNew(try store.publish(secondVerified, trust: trust)), "reject published after approve")
refused("a fork at sequence 2 is refused") { _ = try store.publish(try verifyReceipt(fork.json, trust: trust), trust: trust) }
let foreignBytes = try ApprovalPayload(request: otherRequest, decision: .approve, sequence: 3,
                                       previousReceipt: secondVerified.receiptID).encode()
refused("a link for another target with the same nonce is refused") {
    _ = try store.publish(try verifyReceipt(try makeReceipt(foreignBytes, signer: signer, reason: "").json, trust: trust),
                          trust: trust)
}
let published = try store.chain(nonce: request.requestNonce, trust: trust)
check(published.map(\.receiptID) == [firstVerified.receiptID, secondVerified.receiptID]
      && published.last?.payload.decision == .reject, "chain reads back approve then reject")
check(try FileManager.default.contentsOfDirectory(atPath: nonceDir).sorted() == ["1.json", "2.json"],
      "no pending files left after publishing")

func withChange(_ what: String, _ change: () throws -> Void, undo: () throws -> Void) throws {
    try change()
    refused(what) { _ = try store.chain(nonce: request.requestNonce, trust: trust) }
    try undo()
}
let secondStored = try Data(contentsOf: URL(fileURLWithPath: nonceDir + "/2.json"))
try withChange("a tampered record entry fails closed", {
    try Data(String(decoding: secondStored, as: UTF8.self).replacingOccurrences(of: "\"signature\":\"M", with: "\"signature\":\"N").utf8)
        .write(to: URL(fileURLWithPath: nonceDir + "/2.json"))
}, undo: { try secondStored.write(to: URL(fileURLWithPath: nonceDir + "/2.json")) })
let firstStored = try Data(contentsOf: URL(fileURLWithPath: nonceDir + "/1.json"))
try withChange("a deleted middle entry fails closed", {
    try FileManager.default.removeItem(atPath: nonceDir + "/1.json")
}, undo: { try firstStored.write(to: URL(fileURLWithPath: nonceDir + "/1.json")) })
try withChange("a foreign entry fails closed", {
    try Data().write(to: URL(fileURLWithPath: nonceDir + "/notes.txt"))
}, undo: { try FileManager.default.removeItem(atPath: nonceDir + "/notes.txt") })
try withChange("an entry stored under the wrong sequence fails closed", {
    try FileManager.default.moveItem(atPath: nonceDir + "/2.json", toPath: nonceDir + "/3.json")
}, undo: { try FileManager.default.moveItem(atPath: nonceDir + "/3.json", toPath: nonceDir + "/2.json") })
try withChange("a stray dot-file in a chain directory fails closed", {
    try Data().write(to: URL(fileURLWithPath: nonceDir + "/.pending-9-1"))
}, undo: { try FileManager.default.removeItem(atPath: nonceDir + "/.pending-9-1") })
try Data().write(to: URL(fileURLWithPath: scratch + "/.staging/leftover"))
check((try? store.chain(nonce: request.requestNonce, trust: trust).count) == 2, "staging leftovers are outside every chain")

if CommandLine.arguments.count == 2 {
    let dir = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    try requestBytes.write(to: dir.appendingPathComponent("request.txt"))
    try first.json.write(to: dir.appendingPathComponent("receipt-1-approve.json"))
    try second.json.write(to: dir.appendingPathComponent("receipt-2-reject.json"))
    try trustConfig(publicKeySPKI: signer.publicKeySPKI, label: "approval-core-check software test key",
                    repositoryID: 1_150_335_160).write(to: dir.appendingPathComponent("trust.json"))
    let authority = dir.appendingPathComponent("authority").path
    try? FileManager.default.removeItem(atPath: authority)
    try FileManager.default.createDirectory(atPath: authority, withIntermediateDirectories: true)
    let fixtureStore = AuthorityStore(root: authority, problem: { _ in nil })
    _ = try fixtureStore.publish(firstVerified, trust: trust)
    _ = try fixtureStore.publish(secondVerified, trust: trust)
    print("fixtures written to \(dir.path)")
}

exit(failures == 0 ? 0 : 1)
