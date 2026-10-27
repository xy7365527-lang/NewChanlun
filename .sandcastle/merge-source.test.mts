import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { deepAgentsMergeInput } from "./merge-source.mts";
import { validateMergeManifest, type MergeManifest } from "./managed-merge.mts";

const entry = fileURLToPath(new URL("./main-swarm.mts", import.meta.url));
const loader = import.meta.resolve("tsx");
const write = (path: string, value: unknown) => { mkdirSync(dirname(path), { recursive: true }); writeFileSync(path, JSON.stringify(value, null, 2)); };

/** 按执行器实际 run 目录布局写出交付记录：claim、awaiting_merge、deepagents/review 状态和 review 回执。 */
function deliver(dir: string, ticket: number, sha: string, verdict = "VERDICT: PASS") {
  const runId = `t${ticket}-1`;
  write(join(dir, "run.json"), { run_id: runId, ticket, author_model: "gpt-6.1-sol", reviewer_model: "claude-opus-5-5" });
  write(join(dir, "claim.json"), { ticket, run_id: runId, generation: 1 });
  const pr = { number: 9000 + ticket, url: `https://example.invalid/pull/${9000 + ticket}`, head: sha };
  write(join(dir, "delivery", "state.json"), { phase: "awaiting_merge", head: sha, branch: `codex/deepagents/t${ticket}`, pr });
  write(join(dir, "delivery", `status-${sha}.json`), { state: "done", result: { context: "deepagents/review", sha }, what: { kind: "status", sha, context: "deepagents/review" } });
  write(join(dir, "workers", `review-${sha}`, "result.json"), { status: "completed", mode: "review", model: "claude-opus-5-5", expected_head: sha,
    checkout_after: { head: sha, status: "" }, result: { status: "completed", summary: `${verdict}\nfixture` } });
}

function fixture(t: { after: (fn: () => void) => void }, conflict = false) {
  const root = mkdtempSync(join(tmpdir(), "nc-u6-source-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repo = join(root, "repo"); const bin = join(root, "bin"); const runs = join(root, "runs");
  mkdirSync(repo); mkdirSync(bin);
  const calls = join(root, "calls");
  for (const name of ["docker", "gh", "codex"]) {
    const path = join(bin, name);
    writeFileSync(path, `#!/bin/sh\nprintf '%s\\n' '${name}' >> "$PROBE_CALLS"\nexit 42\n`);
    chmodSync(path, 0o755);
  }
  const git = (...args: string[]) => execFileSync("git", args, { cwd: repo, encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-b", "main"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  writeFileSync(join(repo, "doc.md"), "base\n"); git("add", "."); git("commit", "-m", "base");
  const base = git("rev-parse", "HEAD");
  const sources = [11, 12].map(ticket => {
    git("checkout", "-b", `codex/deepagents/t${ticket}`, base);
    writeFileSync(join(repo, conflict ? "doc.md" : `doc${ticket}.md`), `ticket ${ticket}\n`);
    git("add", "."); git("commit", "-m", `ticket ${ticket}`);
    const sha = git("rev-parse", "HEAD"); const dir = join(runs, `t${ticket}-1`);
    deliver(dir, ticket, sha);
    return { ticket, sha, dir, branch: `codex/deepagents/t${ticket}` };
  });
  git("checkout", "main");
  const manifest: MergeManifest = { runId: "u6", branch: "sandcastle/merge-u6", base,
    inputs: sources.map(s => deepAgentsMergeInput(s.dir, s.sha, `fixture-validation ${s.sha}`)) };
  const path = join(root, "accepted.json"); writeFileSync(path, JSON.stringify(manifest));
  const worktree = join(repo, ".sandcastle/worktrees/sandcastle-merge-u6");
  const checkpoint = join(repo, ".sandcastle/logs/merges/u6/checkpoint.json");
  const run = () => spawnSync(process.execPath, ["--import", loader, entry], {
    cwd: repo, encoding: "utf8", timeout: 30_000,
    env: { PATH: `${bin}:${dirname(process.execPath)}:/usr/bin:/bin`, HOME: root, PROBE_CALLS: calls,
      CLIPROXY_LOCAL_KEY: "fixture-no-network", SANDCASTLE_SWARM_MERGE_MANIFEST: path },
  });
  const called = () => existsSync(calls) ? readFileSync(calls, "utf8").trim().split("\n") : [];
  const refs = () => git("for-each-ref", "--format=%(objectname) %(refname)");
  return { repo, git, base, sources, manifest, run, called, checkpoint, worktree, refs };
}

test("公共 CLI：两份 DeepAgents 交付按固定 SHA 双父归并，保留来源身份，重入不重复提交", t => {
  const f = fixture(t);
  const before = f.refs();
  const first = f.run(); assert.equal(first.status, 0, first.stderr);
  const state = JSON.parse(readFileSync(f.checkpoint, "utf8"));
  assert.equal(state.phase, "done"); assert.equal(state.entries.length, 2); assert.equal(state.checks.length, 2);
  let parent = f.base;
  for (const [n, e] of state.entries.entries()) {
    const s = f.sources[n]!;
    assert.equal(e.sha, s.sha); assert.equal(e.kind, "merged");
    assert.equal(f.git("show", "-s", "--format=%P", e.head), `${parent} ${s.sha}`);
    const body = f.git("show", "-s", "--format=%B", e.head);
    assert.ok(body.includes(`Sandcastle-Merge: u6/${s.sha}`));
    assert.ok(body.includes(`Merge-Source: deepagents ticket=${s.ticket} run=t${s.ticket}-1 gen=1 branch=${s.branch}`));
    assert.equal(e.check.head, e.head); assert.equal(e.check.status, "passed");
    parent = e.head;
  }
  assert.equal(state.manifest.inputs[1].branch, "codex/deepagents/t12");
  for (const s of f.sources) assert.equal(f.git("rev-parse", s.branch), s.sha);
  assert.equal(f.git("rev-parse", "main"), f.base);
  assert.equal(f.git("branch", "--list", "sandcastle/issue-*"), "");
  const head = f.git("rev-parse", "sandcastle/merge-u6"); const saved = readFileSync(f.checkpoint, "utf8");
  const again = f.run(); assert.equal(again.status, 0, again.stderr);
  assert.equal(f.git("rev-parse", "sandcastle/merge-u6"), head);
  assert.equal(readFileSync(f.checkpoint, "utf8"), saved);
  assert.equal(f.git("rev-list", "--count", `${f.base}..${head}`), "4");
  assert.deepEqual(f.called(), []);
  const after = f.refs().split("\n").filter(l => !l.endsWith("refs/heads/sandcastle/merge-u6")).join("\n");
  assert.equal(after, before);
});

test("公共 CLI：执行器 head 改变、来源分支前进或回执非 PASS 时，旧证据在任何 Git 写入前被拒绝", async t => {
  const cases: Array<[string, (f: ReturnType<typeof fixture>) => void, RegExp]> = [
    ["run 转 stale(h')", f => { const p = join(f.sources[1]!.dir, "delivery", "state.json"); const s = JSON.parse(readFileSync(p, "utf8"));
      writeFileSync(p, JSON.stringify({ ...s, phase: "stale", head: f.base })); }, /不在该 SHA 的 awaiting_merge/],
    ["本地来源分支前进", f => { f.git("checkout", "-q", f.sources[1]!.branch); writeFileSync(join(f.repo, "late.md"), "late\n");
      f.git("add", "late.md"); f.git("commit", "-q", "-m", "late"); f.git("checkout", "-q", "main"); }, /来源分支已不在已审 SHA：refs\/heads\/codex\/deepagents\/t12/],
    ["远端跟踪分支指向他处", f => { f.git("update-ref", "refs/remotes/origin/codex/deepagents/t12", f.base); }, /来源分支已不在已审 SHA：refs\/remotes\/origin\/codex\/deepagents\/t12/],
    ["review 回执为 BLOCK", f => { const s = f.sources[1]!; const p = join(s.dir, "workers", `review-${s.sha}`, "result.json");
      const r = JSON.parse(readFileSync(p, "utf8")); r.result.summary = "VERDICT: BLOCK\nfixture"; writeFileSync(p, JSON.stringify(r)); }, /VERDICT: PASS/],
    ["缺 deepagents/review 状态", f => { const s = f.sources[1]!; rmSync(join(s.dir, "delivery", `status-${s.sha}.json`)); }, /记录不可读/],
  ];
  for (const [name, mutate, message] of cases) {
    await t.test(name, t2 => {
      const f = fixture(t2); mutate(f);
      const result = f.run();
      assert.equal(result.status, 1, name); assert.match(result.stderr, message);
      assert.equal(existsSync(f.checkpoint), false); assert.equal(existsSync(f.worktree), false);
      assert.equal(f.git("branch", "--list", "sandcastle/merge-u6"), "");
      assert.equal(f.git("rev-parse", "main"), f.base); assert.deepEqual(f.called(), []);
    });
  }
});

test("公共 CLI：冲突仍先调原 Docker 检查并保留 MERGE_HEAD；之后来源变 stale，重入在 Docker 前拒绝且现场不变", t => {
  const f = fixture(t, true);
  const first = f.run();
  assert.equal(first.status, 1); assert.match(first.stderr, /默认 Docker namespace 不可用/);
  const saved = readFileSync(f.checkpoint, "utf8"); const state = JSON.parse(saved);
  assert.equal(state.phase, "conflict"); assert.equal(state.position, 1); assert.deepEqual(state.allowed, ["doc.md"]);
  assert.equal(state.snapshot.mergeHead, f.sources[1]!.sha);
  assert.equal(execFileSync("git", ["rev-parse", "MERGE_HEAD"], { cwd: f.worktree, encoding: "utf8" }).trim(), f.sources[1]!.sha);
  const conflict = readFileSync(join(f.worktree, "doc.md"), "utf8"); assert.match(conflict, /<<<<<<< HEAD/);
  assert.deepEqual(f.called(), ["docker"]);
  const p = join(f.sources[1]!.dir, "delivery", "state.json");
  writeFileSync(p, JSON.stringify({ ...JSON.parse(readFileSync(p, "utf8")), phase: "stale", head: f.base }));
  const again = f.run();
  assert.equal(again.status, 1); assert.match(again.stderr, /不在该 SHA 的 awaiting_merge/);
  assert.equal(readFileSync(f.checkpoint, "utf8"), saved);
  assert.equal(readFileSync(join(f.worktree, "doc.md"), "utf8"), conflict);
  assert.equal(execFileSync("git", ["rev-parse", "MERGE_HEAD"], { cwd: f.worktree, encoding: "utf8" }).trim(), f.sources[1]!.sha);
  assert.deepEqual(f.called(), ["docker"]);
  assert.equal(f.git("rev-parse", "main"), f.base);
});

test("清单形状：DeepAgents 不能冒充旧分支，旧输入合同不变", t => {
  const f = fixture(t);
  const [a] = f.manifest.inputs as any[];
  const check = (inputs: unknown[]) => () => validateMergeManifest({ ...f.manifest, inputs } as MergeManifest);
  assert.doesNotThrow(check([a]));
  assert.throws(check([{ ...a, branch: "sandcastle/issue-11" }]), /codex\/deepagents\/t<票号>/);
  assert.throws(check([{ ...a, acceptance: { ...a.acceptance, head: f.sources[1]!.sha } }]), /DeepAgents 独审证据|绑定精确 SHA/);
  assert.throws(check([{ ...a, acceptance: { ...a.acceptance, reviewEvidence: join(a.run.dir, "workers", `review-${f.sources[1]!.sha}`, "result.json") } }]), /review 回执/);
  assert.throws(check([{ ...a, run: { ...a.run, id: "t12-1", dir: join(dirname(a.run.dir), "t12-1") } }]), /run 身份与票号不符/);
  assert.throws(check([{ ...a, source: "other" }]), /未知归并输入来源/);
  const legacy = { issue: 11, branch: "sandcastle/issue-11", sha: a.sha, acceptance: { head: a.sha, status: "accepted", reviewEvidence: "review", verificationEvidence: "verification" } };
  assert.doesNotThrow(check([legacy]));
  assert.throws(check([{ ...legacy, branch: "codex/deepagents/t11" }]), /归并输入必须为固定单票 SHA/);
  assert.throws(check([{ ...legacy, acceptance: { ...legacy.acceptance, head: f.base } }]), /缺少绑定精确 SHA 的单票验收\/独审证据/);
  assert.throws(() => deepAgentsMergeInput(a.run.dir, f.sources[1]!.sha, "x"), /DeepAgents/);
});
