import Darwin
import Foundation

// Run one command as root after the owner authenticates in macOS's own administrator dialog.
// AppleScript's `do shell script ... with administrator privileges` shows the system dialog; its
// authorization stays inside that one osascript process, so every call here asks again.

/// osascript arguments that run `command` through `do shell script`, each element shell-quoted by
/// AppleScript's `quoted form of`, with `prompt` shown in the system dialog. `administrator: false`
/// runs the same script as the caller without a dialog; only the offline check uses it.
public func adminDialogArguments(_ command: [String], prompt: String, administrator: Bool = true) -> [String] {
    let privileges = administrator ? " with administrator privileges" : ""
    let script = [
        "on run argv",
        "set cmd to quoted form of (item 1 of argv)",
        "repeat with i from 2 to (count of argv) - 1",
        "set cmd to cmd & \" \" & quoted form of (item i of argv)",
        "end repeat",
        "do shell script cmd with prompt (item -1 of argv)\(privileges) without altering line endings",
        "end run",
    ]
    return script.flatMap { ["-e", $0] } + command + [prompt]
}

public struct DialogOutcome {
    public let status: Int32
    public let output: String
    public let error: String

    public init(status: Int32, output: String, error: String) {
        self.status = status
        self.output = output
        self.error = error
    }

    public var succeeded: Bool { status == 0 }
    /// The owner pressed Cancel in the dialog (AppleScript error -128).
    public var cancelled: Bool { error.contains("(-128)") }
}

public func runAdminDialog(_ command: [String], prompt: String, administrator: Bool = true) -> DialogOutcome {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
    process.arguments = adminDialogArguments(command, prompt: prompt, administrator: administrator)
    let out = Pipe()
    let err = Pipe()
    process.standardOutput = out
    process.standardError = err
    process.standardInput = FileHandle.nullDevice
    // Ctrl-C in the terminal while the dialog is up must not kill the caller before it reports the outcome.
    signal(SIGINT, SIG_IGN)
    defer { signal(SIGINT, SIG_DFL) }
    do {
        try process.run()
    } catch {
        return DialogOutcome(status: -1, output: "", error: "无法运行 osascript: \(error)")
    }
    let output = out.fileHandleForReading.readDataToEndOfFile()
    let error = err.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    let status = process.terminationReason == .exit ? process.terminationStatus : -1
    return DialogOutcome(status: status, output: String(decoding: output, as: UTF8.self),
                         error: String(decoding: error, as: UTF8.self))
}

public func enrollmentCancellationNotice(_ request: ApprovalRequest, decision: Decision) throws -> String {
    guard request.repositoryID == "1150335160", request.ticket == "1742",
          request.runID == "enrollment-check", request.claimGeneration == "0",
          request.scope == .candidateHead, decision == .approve else {
        throw ApprovalError.malformed("取消发布验收仅用于 #1742 enrollment-check、代次 0、candidate_head 的批准")
    }
    return "取消发布测试：请在此窗口点【取消】。不要输入密码，不要完成认证。\n"
}

/// Text for the system dialog when publishing `payload`. Names the action so an unexpected dialog stands out.
public func publishPrompt(_ payload: ApprovalPayload, cancellationNotice: String = "") -> String {
    let verb = payload.decision == .approve ? "批准" : "拒绝"
    let instruction = cancellationNotice.isEmpty ? "只在你刚发起签批时认证。" : ""
    return cancellationNotice + "NewChanlun 签批：把\(verb)发布到权威记录（#\(payload.request.ticket) \(payload.request.scope.rawValue) "
        + "\(payload.request.target)，第 \(payload.sequence) 条）。" + instruction
}
