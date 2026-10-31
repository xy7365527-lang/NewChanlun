import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { randomUUID } from "node:crypto";
import { codexWorker } from "./codex-route.mts";
import { readLaunch, reviewMounts, runExternalReview, verifyReviewAdmission, type ReviewLaunch, type WorkerDependencies } from "./external-review-worker.mts";
import type { BindMountSandboxHandle } from "@ai-hero/sandcastle";

const image = "sha256:" + "1".repeat(64);
const container = "a".repeat(64);
function fixture() {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "external-review-test-")));
  const checkout = join(root, "source"), folder = join(root, "receipt");
  mkdirSync(checkout); mkdirSync(folder);
  const git = (...args: string[]) => execFileSync("git", ["-C", checkout, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  git("init", "-q"); git("-c", "user.name=Fixture", "-c", "user.email=f@example.invalid", "commit", "--allow-empty", "-qm", "base");
  git("checkout", "--detach", "-q");
  const head = git("rev-parse", "HEAD");
  const launch: ReviewLaunch = { run_id: randomUUID(), launch_id: randomUUID(), pr_number: 1810, model: "gpt-6.1-sol",
    head_sha: head, base_sha: head, merge_base_sha: head, controller_code_head: "b".repeat(40), checkout, prompt: "完整 PR 独审", mode: "review" };
  writeFileSync(join(folder, "launch.json"), JSON.stringify(launch), { mode: 0o600 });
  return { root, checkout, folder, launch };
}

test("review 挂载只接受独立 clone，源码与 Git 全部只读", () => {
  const f = fixture();
  try {
    const mounts = reviewMounts([{ hostPath: f.checkout, sandboxPath: "/review" }], f.checkout);
    assert(mounts.every((m) => m.readonly));
    assert(mounts.some((m) => m.sandboxPath === "/review/.git"));
    assert.throws(() => reviewMounts([{ hostPath: f.checkout, sandboxPath: "/review" }, { hostPath: f.root, sandboxPath: "/host" }], f.checkout));
    assert.equal(readLaunch(f.folder).model, "gpt-6.1-sol");
  } finally { rmSync(f.root, { recursive: true, force: true }); }
});

test("显式两家族模型保留旧默认并拒绝路由外模型", () => {
  for (const model of ["gpt-6.1-sol", "claude-opus-5-5"] as const) {
    const a = codexWorker({ docker: true, model });
    const b = codexWorker({ docker: true, model });
    const options = { prompt: "只审不修", dangerouslySkipPermissions: false };
    const command = a.buildPrintCommand(options);
    assert.match(command.command, new RegExp(model));
    assert.match(command.command, /host.docker.internal:8318/);
    assert.doesNotMatch(command.command, /--skip-git-repo-check/);
    assert.notEqual(command.command, b.buildPrintCommand(options).command);
    assert.equal(command.stdin, options.prompt);
  }
  assert.throws(() => codexWorker({ docker: true, model: "other" as never }));
  assert.throws(() => verifyReviewAdmission({ imageId: image, consume() {} }));
});

for (const mode of ["PASS", "BLOCK", "failed", "missing-log", "missing-completed", "write", "bad-mount", "secret-env", "bad-filesystem"] as const) {
  test(`宿主监督 ${mode} 不把缺证据转为通过`, async () => {
    const f = fixture(); const session = randomUUID();
    let closed = false, creates = 0;
    let modelCommand = "";
    const output = { status: mode === "BLOCK" ? "blocked" : "completed", summary: `VERDICT: ${mode}`, findings: [], validation: [] };
    const handle: BindMountSandboxHandle = {
      worktreePath: "/review",
      async exec(command, options) {
        if (command === "cat /etc/hostname") return { stdout: container.slice(0, 12), stderr: "", exitCode: 0 };
        if (command.startsWith("node -e ")) return { stdout: JSON.stringify({ source_write: mode === "bad-filesystem" ? "ALLOWED" : "EROFS", git_write: "EROFS", host_read: "ENOENT", state_write: "ENOENT" }), stderr: "", exitCode: 0 };
        if (command.includes("find ")) {
          const sessionsDir = JSON.parse(command.match(/^find (".*?") /)![1]!);
          return { stdout: `${sessionsDir}/2026/10/29/rollout-2026-10-29-${session}.jsonl\n`, stderr: "", exitCode: 0 };
        }
        modelCommand = command;
        assert(command.includes("--output-schema /tmp/external-review-schema.json"));
        assert(command.includes("--disable multi_agent"));
        assert.equal(options?.stdin, f.launch.prompt);
        const events = [{ type: "thread.started", thread_id: session },
          { type: "item.completed", item: { type: "agent_message", text: JSON.stringify(output) } },
          ...(mode === "missing-completed" ? [] : [{ type: "turn.completed" }])];
        for (const e of events) options?.onLine?.(JSON.stringify(e));
        if (mode === "write") writeFileSync(join(f.checkout, "changed"), "write simulated");
        return { stdout: "", stderr: "", exitCode: mode === "failed" ? 1 : 0 };
      },
      async copyFileIn() {},
      async copyFileOut(_src, dest) {
        if (mode === "missing-log") throw new Error("missing session");
        mkdirSync(dirname(dest), { recursive: true });
        writeFileSync(dest, JSON.stringify({ type: "session_meta", payload: { id: session, cwd: "/review" } }) + "\n");
      },
      async close() { closed = true; },
    };
    const deps: WorkerDependencies = {
      identity: () => ({ head: f.launch.controller_code_head, dirty: "" }),
      ready: () => image,
      sandbox: () => ({ name: "fixture", env: {}, create: async (options) => {
        creates++;
        assert(options.mounts.every((m) => m.readonly));
        assert(options.mounts.every((m) => m.hostPath.startsWith(f.checkout)));
        assert.deepEqual(options.env, {});
        return handle;
      } }),
      inspect: () => ({ Id: container, Image: image,
        Mounts: reviewMounts([{ hostPath: f.checkout, sandboxPath: "/review" }], f.checkout)
          .map((m) => ({ Source: m.hostPath, Destination: m.sandboxPath, RW: mode === "bad-mount" })),
        Config: { Env: mode === "secret-env" ? ["GH_TOKEN=fixture"] : [] } }),
      imageEnv: () => [], timeoutMs: 10000,
    };
    try {
      await runExternalReview(f.folder, deps);
      const receipt = JSON.parse(readFileSync(join(f.folder, "receipt.json"), "utf8"));
      assert.equal(creates, 1); assert(closed);
      assert.equal(receipt.status, ["PASS", "BLOCK"].includes(mode) ? "completed" : "failed");
      if (receipt.status === "completed") {
        assert.match(modelCommand, /(?:^|\s)--skip-git-repo-check(?:\s|$)/);
        assert.equal(receipt.result.summary, `VERDICT: ${mode}`);
        assert.equal(receipt.session_id, session);
        assert(receipt.turn_completed);
        assert.equal(receipt.container_id, container);
      }
      await assert.rejects(runExternalReview(f.folder, deps));
      assert.equal(creates, 1);
    } finally { rmSync(f.root, { recursive: true, force: true }); }
  });
}


test("损坏 launch 在读取阶段也保存失败终态", async () => {
  const f = fixture();
  try {
    writeFileSync(join(f.folder, "launch.json"), "{invalid");
    await assert.rejects(runExternalReview(f.folder));
    const receipt = JSON.parse(readFileSync(join(f.folder, "receipt.json"), "utf8"));
    assert.equal(receipt.status, "failed");
    assert.equal(receipt.phase, "launch");
    assert.equal(receipt.container_closed, true);
  } finally { rmSync(f.root, { recursive: true, force: true }); }
});

test("已有 supervisor 锁时保留原回执并保存本进程启动终态", async () => {
  const f = fixture();
  try {
    writeFileSync(join(f.folder, "supervisor.lock"), "");
    writeFileSync(join(f.folder, "receipt.json"), "original evidence");
    await assert.rejects(runExternalReview(f.folder));
    assert.equal(readFileSync(join(f.folder, "receipt.json"), "utf8"), "original evidence");
    const receipt = JSON.parse(readFileSync(join(f.folder, `startup-failure-${process.pid}.json`), "utf8"));
    assert.equal(receipt.status, "failed");
    assert.equal(receipt.phase, "lock");
    assert.equal(receipt.launch_id, f.launch.launch_id);
  } finally { rmSync(f.root, { recursive: true, force: true }); }
});


test("创建容器结果不明时不能声明已关闭", async () => {
  const f = fixture();
  const deps: WorkerDependencies = {
    identity: () => ({ head: f.launch.controller_code_head, dirty: "" }),
    ready: () => image,
    sandbox: () => ({ name: "unknown-create", env: {}, create: async () => { throw new Error("unknown create"); } }),
    inspect: () => { throw new Error("must not inspect"); },
    imageEnv: () => [], timeoutMs: 10000,
  };
  try {
    await runExternalReview(f.folder, deps);
    const receipt = JSON.parse(readFileSync(join(f.folder, "receipt.json"), "utf8"));
    assert.equal(receipt.status, "failed");
    assert.equal(receipt.phase, "container");
    assert.equal(receipt.container_closed, false);
  } finally { rmSync(f.root, { recursive: true, force: true }); }
});
