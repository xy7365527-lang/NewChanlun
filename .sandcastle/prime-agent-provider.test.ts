import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { BindMountSandboxHandle, ExecResult } from "@ai-hero/sandcastle";
import {
  parsePrimeSessionHeader,
  primeAgent,
  transferPrimeSession,
} from "./prime-agent-provider.ts";

// ── 纯函数 ────────────────────────────────────────────────────────────────

test("parsePrimeSessionHeader 从 session header 提取 id/cwd，非 header 行返回 undefined", () => {
  assert.deepEqual(
    parsePrimeSessionHeader(JSON.stringify({ type: "session", version: 3, id: "abc", cwd: "/x" })),
    { id: "abc", cwd: "/x" },
  );
  assert.equal(parsePrimeSessionHeader("not json"), undefined);
  assert.equal(parsePrimeSessionHeader(JSON.stringify({ type: "message", id: "m" })), undefined);
  assert.equal(
    parsePrimeSessionHeader(JSON.stringify({ type: "session", id: "abc" }))?.cwd,
    "",
  );
});

test("transferPrimeSession 只改 session header 的 cwd，消息正文里的路径串不动", () => {
  const jsonl = [
    JSON.stringify({ type: "session", version: 3, id: "s1", cwd: "/sandbox/repo" }),
    JSON.stringify({ type: "message", id: "m1", message: { role: "user", content: [{ type: "text", text: "see /sandbox/repo keep" }] } }),
    "",
  ].join("\n");
  const out = transferPrimeSession(jsonl, "/sandbox/repo", "/host/repo");
  const lines = out.split("\n");
  assert.equal(JSON.parse(lines[0]).cwd, "/host/repo");
  // 消息行原样保留（含正文里的 /sandbox/repo）
  assert.equal(lines[1], jsonl.split("\n")[1]);
  assert.ok(lines[1].includes("/sandbox/repo keep"));
});

test("transferPrimeSession 空串与同 cwd 是 no-op", () => {
  assert.equal(transferPrimeSession("", "/a", "/b"), "");
  const jsonl = JSON.stringify({ type: "session", id: "s", cwd: "/x" });
  assert.equal(transferPrimeSession(jsonl, "/x", "/x"), jsonl);
});

// ── buildPrintCommand ─────────────────────────────────────────────────────

test("buildPrintCommand 新会话无续跑旗标；续跑用 -r 绝对路径；fork 用 --fork", () => {
  const provider = primeAgent("deepseek-v4-pro", { provider: "deepseek" });
  const fresh = provider.buildPrintCommand({ prompt: "hi", dangerouslySkipPermissions: true });
  assert.ok(fresh.command.startsWith("prime-agent -p --mode json"));
  assert.ok(!fresh.command.includes(" -r "));
  assert.ok(!fresh.command.includes(" --fork "));
  assert.equal(fresh.stdin, "hi");

  const resumed = provider.buildPrintCommand({ prompt: "hi", dangerouslySkipPermissions: true, resumeSession: "sid-1" });
  assert.ok(resumed.command.includes(" -r '/home/agent/.prime/agent/sessions/sid-1.jsonl'"));

  const forked = provider.buildPrintCommand({ prompt: "hi", dangerouslySkipPermissions: true, resumeSession: "sid-1", forkSession: true });
  assert.ok(forked.command.includes(" --fork '/home/agent/.prime/agent/sessions/sid-1.jsonl'"));
});

test("provider 暴露 captureSessions 与完整 sessionStorage 契约", () => {
  const provider = primeAgent("m");
  assert.equal(provider.captureSessions, true);
  assert.ok(provider.sessionStorage);
  for (const m of ["captureToHost", "resumeIntoSandbox", "readHostSession", "existsOnHost", "hostSessionFilePath", "findByIdOnHost"] as const) {
    assert.equal(typeof provider.sessionStorage[m], "function", m);
  }
});

// ── capture/resume 往返（内存 mock handle） ────────────────────────────────

type MockHandle = BindMountSandboxHandle & { files: Map<string, string> };

function makeMockHandle(seed: Record<string, string> = {}): MockHandle {
  const files = new Map(Object.entries(seed));
  const handle: MockHandle = {
    worktreePath: "/sandbox/repo",
    files,
    async exec(command: string): Promise<ExecResult> {
      const find = command.match(/^find '([^']*)' -maxdepth 1 -type f -name '\*\.jsonl' -print 2>\/dev\/null$/);
      if (find) {
        const dir = find[1];
        const out = [...files.keys()]
          .filter((p) => p.startsWith(dir + "/") && p.endsWith(".jsonl") && !p.slice(dir.length + 1).includes("/"))
          .sort()
          .join("\n");
        return { stdout: out ? out + "\n" : "", stderr: "", exitCode: 0 };
      }
      const head = command.match(/^head -n 1 '([^']*)'$/);
      if (head) {
        const content = files.get(head[1]);
        if (content === undefined) return { stdout: "", stderr: "No such file", exitCode: 1 };
        return { stdout: content.split("\n")[0] + "\n", stderr: "", exitCode: 0 };
      }
      const mkdir = command.match(/^mkdir -p '([^']*)'$/);
      if (mkdir) return { stdout: "", stderr: "", exitCode: 0 };
      throw new Error(`unexpected exec: ${command}`);
    },
    async copyFileIn(hostPath: string, sandboxPath: string): Promise<void> {
      files.set(sandboxPath, await readFile(hostPath, "utf8"));
    },
    async copyFileOut(sandboxPath: string, hostPath: string): Promise<void> {
      const content = files.get(sandboxPath);
      if (content === undefined) throw new Error(`copyFileOut: not found ${sandboxPath}`);
      await writeFile(hostPath, content);
    },
    async close(): Promise<void> {},
  };
  return handle;
}

test("captureToHost→resumeIntoSandbox 往返：按 header.id 定位、cwd 来回改写", async () => {
  const root = await mkdtemp(join(tmpdir(), "prime-provider-test-"));
  try {
    const hostSessionsDir = join(root, "host-sessions");
    const sandboxSessionsDir = "/sandbox/.prime/agent/sessions";
    const provider = primeAgent("m", { sessionStorage: { hostSessionsDir, sandboxSessionsDir } });
    const storage = provider.sessionStorage!;

    // 首轮：daemon 命名的文件（文件名 ≠ header.id），模拟 #1066 评论查实的 0.7.2 行为
    const header = JSON.stringify({ type: "session", version: 3, id: "sid-1", cwd: "/sandbox/repo", rlmDepth: 0 });
    const message = JSON.stringify({ type: "message", id: "m1", message: { role: "user", content: [{ type: "text", text: "see /sandbox/repo keep" }] } });
    const sandboxFile = `${sandboxSessionsDir}/daemon-file.jsonl`;
    const sandbox = makeMockHandle({ [sandboxFile]: `${header}\n${message}\n` });

    await storage.captureToHost({ hostCwd: "/host/repo", sandboxCwd: "/sandbox/repo", sessionId: "sid-1", handle: sandbox });

    const capturedPath = join(hostSessionsDir, "sid-1.jsonl");
    const captured = await readFile(capturedPath, "utf8");
    const capturedLines = captured.split("\n");
    assert.equal(JSON.parse(capturedLines[0]).cwd, "/host/repo");
    assert.ok(captured.includes("/sandbox/repo keep"), "消息正文路径串不该被改写");

    // 宿主定位面
    assert.equal(storage.hostSessionFilePath("/host/repo", "sid-1"), capturedPath);
    assert.equal(await storage.existsOnHost("/host/repo", "sid-1"), true);
    assert.equal((await storage.findByIdOnHost("sid-1")).path, capturedPath);
    assert.ok((await storage.readHostSession("/host/repo", "sid-1"))!.includes('"id":"sid-1"'));

    // 沙盒销毁→新沙盒：resumeIntoSandbox 写回确定性路径，cwd 改回沙盒
    const freshSandbox = makeMockHandle();
    await storage.resumeIntoSandbox({ hostCwd: "/host/repo", sandboxCwd: "/sandbox/repo", sessionId: "sid-1", handle: freshSandbox });

    const resumedPath = `${sandboxSessionsDir}/sid-1.jsonl`;
    const resumed = freshSandbox.files.get(resumedPath);
    assert.ok(resumed, "恢复文件应写回沙盒确定性路径");
    assert.equal(JSON.parse(resumed!.split("\n")[0]).cwd, "/sandbox/repo");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("captureToHost 找不到会话时报错（不静默冒充多轮）", async () => {
  const root = await mkdtemp(join(tmpdir(), "prime-provider-test-"));
  try {
    const provider = primeAgent("m", { sessionStorage: { hostSessionsDir: join(root, "host"), sandboxSessionsDir: "/sandbox/.prime/agent/sessions" } });
    const sandbox = makeMockHandle(); // 空沙盒
    await assert.rejects(
      provider.sessionStorage!.captureToHost({
        hostCwd: "/host/repo", sandboxCwd: "/sandbox/repo", sessionId: "missing", handle: sandbox,
      }),
      /not found/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
