import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import { verifyCombination, MERGE_CHECK_ID } from "./swarm-merge.mts";

test("组合验证真实 diff-check 绑定精确 SHA，文本票不触发本机 Rust", async t => {
  const cwd = mkdtempSync(join(tmpdir(), "nc-merge-check-"));
  t.after(() => rmSync(cwd, { recursive: true, force: true }));
  const git = (...args: string[]) => execFileSync("git", args, { cwd, encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-b", "main"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  git("commit", "--allow-empty", "-m", "base"); const base = git("rev-parse", "HEAD");
  writeFileSync(join(cwd, "a.md"), "valid\n"); git("add", "."); git("commit", "-m", "doc"); const head = git("rev-parse", "HEAD");
  const manifest = { runId: "x", branch: "sandcastle/merge-x", base, inputs: [{ issue: 1, branch: "sandcastle/issue-1", sha: head, acceptance: { head, status: "accepted" as const, reviewEvidence: "review", verificationEvidence: "verification" } }] };
  const result = await verifyCombination(cwd, manifest, head, ["a.md"]);
  assert.equal(result.status, "passed"); assert.equal(result.head, head); assert.equal(result.checkId, MERGE_CHECK_ID);
  writeFileSync(join(cwd, "a.md"), "bad trailing space   \n"); git("add", "."); git("commit", "-m", "bad diff"); const failed = git("rev-parse", "HEAD");
  const r = await verifyCombination(cwd, manifest, failed, ["a.md"]); assert.equal(r.status, "failed"); assert.equal(r.head, failed);
});

test("真实宿主归并入口读取 accepted 清单、保留功能 worktree，可原命令重入（不代替 Docker）", async t => {
  const { runAcceptedSwarmMerge } = await import("./swarm-merge.mts");
  const { withLegacyAdmission } = await import("./drain-control.mts");
  const cwd = mkdtempSync(join(tmpdir(), "nc-merge-entry-"));
  // #1742 旧来源新归并须取冻结准入；测试用临时根，不触碰本机生产冻结根。
  const drainRoot = mkdtempSync(join(tmpdir(), "nc-merge-entry-drain-"));
  const admission = <T>(action: () => T | Promise<T>) => withLegacyAdmission(action, drainRoot);
  const outer = mkdtempSync(join(tmpdir(), "nc-merge-manifest-"));
  const original = process.cwd();
  t.after(() => { process.chdir(original); for (const d of [cwd, outer, drainRoot]) rmSync(d, { recursive: true, force: true }); });
  const git = (...args: string[]) => execFileSync("git", args, { cwd, encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-b", "main"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  git("commit", "--allow-empty", "-m", "base"); const base = git("rev-parse", "HEAD");
  git("checkout", "-b", "sandcastle/issue-1"); writeFileSync(join(cwd, "doc.md"), "accepted\n"); git("add", "."); git("commit", "-m", "doc"); const sha = git("rev-parse", "HEAD"); git("checkout", "main");
  const manifest = { runId: "entry", branch: "sandcastle/merge-entry", base, inputs: [{ issue: 1, branch: "sandcastle/issue-1", sha,
    acceptance: { head: sha, status: "accepted", reviewEvidence: "review", verificationEvidence: "validation" } }] };
  const path = join(outer, "accepted.json"); writeFileSync(path, JSON.stringify(manifest));
  process.chdir(cwd);
  const events: Record<string, unknown>[] = [];
  const first = await runAcceptedSwarmMerge(path, "unused-no-conflict", e => events.push(e), { admission });
  assert.equal(first.phase, "done"); assert.equal(git("rev-parse", "main"), base);
  const second = await runAcceptedSwarmMerge(path, "unused-no-conflict", e => events.push(e), { admission });
  assert.equal(second.snapshot.head, first.snapshot.head); assert.equal(second.checks.length, 1);
  assert.ok(git("worktree", "list", "--porcelain").includes("refs/heads/sandcastle/merge-entry"));
  assert.equal(events.filter(e => e.status === "done").length, 1);
});
