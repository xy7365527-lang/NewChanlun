import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, readFileSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import { runManagedSwarmTicket, type SwarmTicketOptions } from "./swarm-managed.mts";
import { verifyManagedRust } from "./managed-commit.mts";
import { hostGit, ReviewPromptTooLargeError } from "./review-context.mts";
const report = (files: string[], status = "ready") => `<sandcastle-result>${JSON.stringify({ status, files, verification: [] })}</sandcastle-result>`;
function fixture(t: { after: (fn: () => void) => void }) {
  const cwd = mkdtempSync(join(tmpdir(), "nc-swarm-managed-"));
  t.after(() => rmSync(cwd, { recursive: true, force: true }));
  const git = (...args: string[]) => execFileSync("git", args, { cwd, encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-b", "main"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  git("commit", "--allow-empty", "-m", "base"); git("checkout", "-b", "sandcastle/issue-1617");
  const logged: Record<string, unknown>[] = []; const notified: string[] = [];
  const opts: SwarmTicketOptions = {
    cwd, branch: "sandcastle/issue-1617", issue: 1617,
    implement: async () => { writeFileSync(join(cwd, "a.txt"), "author"); return report(["a.txt"]); },
    review: async prompt => { assert.ok(prompt.includes(git("rev-parse", "HEAD"))); return report([]); },
    reviewOptions: { template: "{{COMMITS}}\n{{FILES}}", target: "main", git: hostGit(cwd), fetchIssue: () => "fixture" },
    channels: { logWorker: e => logged.push(e), notify: (_issue, body) => { notified.push(body); } },
  };
  return { cwd, git, opts, logged, notified };
}

test("swarm 作者不 git commit；宿主封包后独审精确 head，评审修正也由宿主提交", async t => {
  const f = fixture(t); let authorHead = "";
  const result = await runManagedSwarmTicket({ ...f.opts, review: async prompt => {
    authorHead = f.git("rev-parse", "HEAD"); assert.ok(prompt.includes(authorHead));
    assert.match(f.git("show", "-s", "--format=%B"), /implementer 受管封包/);
    writeFileSync(join(f.cwd, "a.txt"), "reviewed"); return report(["a.txt"]);
  } });
  assert.deepEqual(result.commits, [authorHead, f.git("rev-parse", "HEAD")]);
  assert.equal(f.git("rev-parse", "HEAD^"), authorHead);
  assert.equal(f.git("show", "HEAD:a.txt"), "reviewed");
  assert.equal(f.logged.at(-1)?.status, "completed");
});

test("作者无变更不触发独审、不算完成分支", async t => {
  const f = fixture(t); let reviewed = false;
  const result = await runManagedSwarmTicket({ ...f.opts, implement: async () => report([]), review: async () => { reviewed = true; return report([]); } });
  assert.deepEqual(result.commits, []); assert.equal(reviewed, false);
});

test("作者 blocked 或清单漏项不提交、不启动独审，保留产物", async t => {
  for (const output of [report(["a.txt"], "blocked"), report([])]) {
    const f = fixture(t); const before = f.git("rev-parse", "HEAD"); let reviewed = false;
    await assert.rejects(runManagedSwarmTicket({ ...f.opts,
      implement: async () => { writeFileSync(join(f.cwd, "a.txt"), "pending"); return output; },
      review: async () => { reviewed = true; return report([]); },
    }));
    assert.equal(f.git("rev-parse", "HEAD"), before); assert.equal(reviewed, false);
    assert.equal(readFileSync(join(f.cwd, "a.txt"), "utf8"), "pending");
  }
});

test("远端失败交独审修复并重验新 head；无改动不能吞掉失败", async t => {
  for (const fixes of [true, false]) {
    const f = fixture(t); let checked = 0;
    const promise = runManagedSwarmTicket({ ...f.opts,
      implement: async () => { mkdirSync(join(f.cwd, "rust")); writeFileSync(join(f.cwd, "rust/a.rs"), "broken"); return report(["rust/a.rs"]); },
      verify: (stage, sealed, _run, previous) => verifyManagedRust(stage, sealed, () => {
        checked++; if (checked === 1) throw new Error("E0308 actual compiler diagnostic"); return "passed";
      }, previous),
      review: async prompt => {
        assert.match(prompt, /E0308 actual compiler diagnostic/);
        if (fixes) { writeFileSync(join(f.cwd, "rust/a.rs"), "fixed"); return report(["rust/a.rs"]); }
        return report([]);
      },
    });
    if (fixes) { const r = await promise; assert.equal(r.verification?.status, "passed"); assert.equal(checked, 2); }
    else { await assert.rejects(promise, /独审验证失败/); assert.equal(checked, 1); assert.ok(!f.logged.some(e => e.status === "completed")); }
  }
});

test("完整提示追加诊断后超限：作者封包保留，评审工蜂不启动，失败只通知一次", async t => {
  const f = fixture(t); let reviewed = false;
  await assert.rejects(runManagedSwarmTicket({ ...f.opts,
    reviewOptions: { ...f.opts.reviewOptions, template: "x".repeat(59_900 * 4) },
    verify: (_stage, sealed) => ({ head: sealed.head, status: "failed", stdout: "x".repeat(12_000), stderr: "e".repeat(12_000) }),
    review: async () => { reviewed = true; return report([]); },
  }), ReviewPromptTooLargeError);
  assert.equal(reviewed, false); assert.match(f.git("show", "-s", "--format=%B"), /implementer 受管封包/);
  assert.deepEqual(f.logged.map(e => e.reason), ["prompt_too_large"]); assert.equal(f.notified.length, 1);
});

test("归并入口与自动领票分离，候选不自动变成 accepted", () => {
  const source = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.ok(source.includes("SANDCASTLE_SWARM_MERGE_MANIFEST"));
  assert.ok(source.includes('status: "pending"'));
  assert.ok(!source.includes("merge-prompt.md"));
  assert.ok(source.includes("runManagedSwarmTicket"));
});
