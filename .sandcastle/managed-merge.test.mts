import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import { runManagedMerge, type MergeOptions, type MergeManifest, type MergeCheck } from "./managed-merge.mts";
const report = (files: string[], status = "ready") => `<sandcastle-result>${JSON.stringify({ status, files, verification: [] })}</sandcastle-result>`;
function fixture(t: { after: (fn: () => void) => void }, conflict = false) {
  const cwd = mkdtempSync(join(tmpdir(), "nc-merge-"));
  const evidence = mkdtempSync(join(tmpdir(), "nc-merge-evidence-"));
  t.after(() => { rmSync(cwd, { recursive: true, force: true }); rmSync(evidence, { recursive: true, force: true }); });
  const git = (...args: string[]) => execFileSync("git", args, { cwd, encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-b", "main"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  writeFileSync(join(cwd, "a.txt"), "base\n"); writeFileSync(join(cwd, "b.txt"), "base\n");
  git("add", "."); git("commit", "-m", "base"); const base = git("rev-parse", "HEAD");
  const shas: string[] = [];
  for (const issue of [1, 2]) {
    git("checkout", "-b", `sandcastle/issue-${issue}`, base);
    writeFileSync(join(cwd, issue === 1 || conflict ? "a.txt" : "b.txt"), `issue${issue}\n`);
    git("add", "."); git("commit", "-m", `issue${issue}`); shas.push(git("rev-parse", "HEAD"));
  }
  const manifest: MergeManifest = { runId: "fixture", branch: "sandcastle/merge-fixture", base,
    inputs: shas.map((sha, n) => ({ issue: n + 1, branch: `sandcastle/issue-${n + 1}`, sha,
      acceptance: { head: sha, status: "accepted", reviewEvidence: `review-${sha}`, verificationEvidence: `verification-${sha}` } })),
  };
  git("checkout", "-b", manifest.branch, base);
  const verified: string[] = [];
  const pass = async (head: string): Promise<MergeCheck> => { verified.push(head); return { head, checkId: "fixture-v1", status: "passed", evidence: "fixture check" }; };
  const opts: MergeOptions = { cwd, manifest, checkpoint: join(evidence, "checkpoint.json"), checkId: "fixture-v1", verify: pass,
    resolveConflict: async () => { throw new Error("unexpected conflict"); },
  };
  return { cwd, git, base, shas, opts, verified, pass };
}

test("无冲突固定 SHA 双父归并逐步验证，main/来源票引用不改；重入不重复", async t => {
  const f = fixture(t); const r = await runManagedMerge(f.opts);
  assert.equal(r.phase, "done"); assert.equal(r.entries.length, 2); assert.equal(f.verified.length, 2);
  for (let n = 0; n < 2; n++) {
    const e = r.entries[n]!; assert.equal(f.git("show", "-s", "--format=%P", e.head), `${n ? r.entries[n - 1]!.head : f.base} ${f.shas[n]}`);
    assert.equal(e.check.head, e.head); assert.equal(f.git("rev-parse", `sandcastle/issue-${n + 1}`), f.shas[n]);
  }
  assert.equal(f.git("rev-parse", "main"), f.base);
  await runManagedMerge(f.opts); assert.equal(f.verified.length, 2);
});

test("冲突只允许冻结路径，由宿主暂存/提交；第二组合精确验证", async t => {
  const f = fixture(t, true); let seen = false;
  const r = await runManagedMerge({ ...f.opts, resolveConflict: async input => {
    seen = true; assert.deepEqual(input.files, ["a.txt"]); assert.equal(input.incoming, f.shas[1]);
    writeFileSync(join(f.cwd, "a.txt"), "both issues\n"); return report(["a.txt"]);
  } });
  assert.equal(seen, true); assert.equal(r.phase, "done"); assert.equal(f.git("show", "HEAD:a.txt"), "both issues");
});

for (const fault of ["outside", "staged", "blocked", "missing-report"]) test(`冲突 ${fault} 拒绝且保留现场`, async t => {
  const f = fixture(t, true);
  await assert.rejects(runManagedMerge({ ...f.opts, resolveConflict: async () => {
    writeFileSync(join(f.cwd, "a.txt"), "resolved\n");
    if (fault === "outside") writeFileSync(join(f.cwd, "b.txt"), "unexpected\n");
    if (fault === "staged") f.git("add", "a.txt");
    return report(fault === "missing-report" ? [] : ["a.txt"], fault === "blocked" ? "blocked" : "ready");
  } }));
  assert.equal(f.git("rev-parse", "MERGE_HEAD"), f.shas[1]);
  assert.equal(readFileSync(join(f.cwd, "a.txt"), "utf8"), "resolved\n");
  await assert.rejects(runManagedMerge(f.opts), /未知 WIP/);
});

test("组合验证失败不合后续输入，同 SHA 重试不造新 commit", async t => {
  const f = fixture(t);
  await assert.rejects(runManagedMerge({ ...f.opts, verify: async head => ({ head, checkId: "fixture-v1", status: "failed", evidence: "compiler failed" }) }), /组合验证失败/);
  const first = f.git("rev-parse", "HEAD"); assert.equal(f.git("show", "-s", "--format=%P", first), `${f.base} ${f.shas[0]}`);
  const state = JSON.parse(readFileSync(f.opts.checkpoint, "utf8")); assert.equal(state.position, 0); assert.equal(state.phase, "failed");
  const r = await runManagedMerge(f.opts); assert.equal(r.entries[0]!.head, first); assert.equal(r.entries.length, 2);
});

for (const phase of ["conflict", "prepared", "committed", "ready"]) test(`检查点 ${phase} 可重入，不重复提交/有效验证`, async t => {
  const f = fixture(t, true); let stopped = false;
  const options: MergeOptions = { ...f.opts, resolveConflict: async () => { writeFileSync(join(f.cwd, "a.txt"), "both\n"); return report(["a.txt"]); },
    onCheckpoint: s => { if (!stopped && s.phase === phase) { stopped = true; throw new Error("interrupt"); } },
  };
  await assert.rejects(runManagedMerge(options), /interrupt/);
  const r = await runManagedMerge(options); assert.equal(r.phase, "done"); assert.equal(r.entries.length, 2);
  assert.equal(f.git("rev-list", "--count", "--merges", `${f.base}..HEAD`), "2"); assert.equal(f.verified.length, 2);
});

test("未知 WIP、错误恢复配置不改现场", async t => {
  const f = fixture(t);
  await assert.rejects(runManagedMerge({ ...f.opts, onCheckpoint: s => { if (s.phase === "committed") throw new Error("stop"); } }));
  writeFileSync(join(f.cwd, "unknown.txt"), "owner data");
  const head = f.git("rev-parse", "HEAD");
  await assert.rejects(runManagedMerge(f.opts), /未知 WIP/);
  assert.equal(f.git("rev-parse", "HEAD"), head); assert.equal(readFileSync(join(f.cwd, "unknown.txt"), "utf8"), "owner data");
  await assert.rejects(runManagedMerge({ ...f.opts, checkId: "other" }), /配置改变/);
});

for (const branch of ["main", "sandcastle/issue-1", "sandcastle/merge-other"]) test(`拒绝目标 ${branch}，无写入`, async t => {
  const f = fixture(t); const head = f.git("rev-parse", "HEAD");
  await assert.rejects(runManagedMerge({ ...f.opts, manifest: { ...f.opts.manifest, branch } }), /功能归并分支/);
  assert.equal(f.git("rev-parse", "HEAD"), head);
});

test("缺少或错误 SHA 的单票验收不归并", async t => {
  const f = fixture(t); f.opts.manifest.inputs[0]!.acceptance.head = f.base;
  await assert.rejects(runManagedMerge(f.opts), /单票验收/);
  assert.equal(f.git("rev-parse", "HEAD"), f.base);
});

test("已包含/重复 SHA 不造空提交；非祖先同树保留双父记录", async t => {
  const f = fixture(t);
  f.git("checkout", "-b", "sandcastle/issue-3", f.base); f.git("commit", "--allow-empty", "-m", "topology only");
  const empty = f.git("rev-parse", "HEAD"); f.git("checkout", f.opts.manifest.branch);
  const input = (sha: string, issue: number) => ({ issue, branch: `sandcastle/issue-${issue}`, sha,
    acceptance: { head: sha, status: "accepted" as const, reviewEvidence: "review", verificationEvidence: "validation" } });
  f.opts.manifest.inputs = [input(f.base, 1), input(f.base, 2), input(empty, 3)];
  const r = await runManagedMerge(f.opts);
  assert.equal(r.entries.length, 2); assert.equal(r.manifest.inputs.length, 3);
  assert.equal(r.entries[0]!.kind, "already_integrated"); assert.equal(r.entries[0]!.head, f.base);
  assert.equal(f.git("rev-parse", "HEAD^{tree}"), f.git("rev-parse", `${f.base}^{tree}`));
  assert.equal(f.git("show", "-s", "--format=%P", "HEAD"), `${f.base} ${empty}`);
});

test("验证器写工位或返回错误 SHA 不记成功", async t => {
  const f = fixture(t);
  await assert.rejects(runManagedMerge({ ...f.opts, verify: async () => ({ head: f.base, checkId: "fixture-v1", status: "passed", evidence: "wrong SHA" }) }), /不匹配/);
});

test("commit 成功而检查点未更新：按准确父集/树/标记恢复，不重复", async t => {
  const f = fixture(t); let halted = false;
  await assert.rejects(runManagedMerge({ ...f.opts, onCheckpoint: s => {
    if (!halted && s.phase === "prepared" && s.tree) { halted = true; throw new Error("precommit crash"); }
  } }), /precommit crash/);
  f.git("commit", "-m", "recovered exact host commit", "-m", `Sandcastle-Merge: fixture/${f.shas[0]}`);
  const committed = f.git("rev-parse", "HEAD");
  const r = await runManagedMerge(f.opts);
  assert.equal(r.entries[0]!.head, committed); assert.equal(f.git("rev-list", "--count", "--merges", `${f.base}..HEAD`), "2");
});

test("错误树或标记的新提交不能冒充 checkpoint 后的宿主提交", async t => {
  const f = fixture(t);
  await assert.rejects(runManagedMerge({ ...f.opts, onCheckpoint: s => { if (s.phase === "prepared" && s.tree) throw new Error("stop"); } }));
  f.git("commit", "-m", "unknown commit without marker");
  const head = f.git("rev-parse", "HEAD"); await assert.rejects(runManagedMerge(f.opts), /未知 WIP/);
  assert.equal(f.git("rev-parse", "HEAD"), head);
});

test("冻结 SHA 不追随来源分支移动；归并模块无 tracker 关票或 main 合入接口", async t => {
  const f = fixture(t);
  f.git("update-ref", "refs/heads/sandcastle/issue-1", f.base, f.shas[0]!);
  const r = await runManagedMerge(f.opts);
  assert.equal(r.entries[0]!.sha, f.shas[0]);
  assert.equal(f.git("rev-parse", "sandcastle/issue-1"), f.base);
  const source = readFileSync(new URL("./managed-merge.mts", import.meta.url), "utf8");
  assert.ok(!source.includes('execFileSync("gh"'));
  assert.ok(!source.includes('"push"')); assert.ok(!source.includes('"checkout"'));
});

test("checkpoint 不能放进工蜂 worktree（包括以 .. 开头的普通目录名）", async t => {
  const f = fixture(t);
  await assert.rejects(runManagedMerge({ ...f.opts, checkpoint: join(f.cwd, "..host", "checkpoint.json") }), /不得放进/);
  assert.equal(f.git("rev-parse", "HEAD"), f.base);
});
