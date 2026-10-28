import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { admitMergeInput, appRemoteFacts, deepAgentsMergeInput } from "./merge-source.mts";
import { runManagedMerge, validateMergeManifest, type MergeManifest } from "./managed-merge.mts";

const entry = fileURLToPath(new URL("./main-swarm.mts", import.meta.url));
const loader = import.meta.resolve("tsx");
const write = (path: string, value: unknown) => { mkdirSync(dirname(path), { recursive: true }); writeFileSync(path, JSON.stringify(value, null, 2)); };
const readJson = (path: string) => JSON.parse(readFileSync(path, "utf8"));

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
/** 执行器 publish 捕获 Stale 时的实际写法：只写 stale.json，原 awaiting_merge 的 state.json 不动。 */
const markStale = (dir: string, head: string, observed: string) =>
  write(join(dir, "delivery", "stale.json"), { head, observed, at: "2026-10-27T12:00:00+00:00" });

function fixture(t: { after: (fn: () => void) => void }, conflict = false) {
  const root = mkdtempSync(join(tmpdir(), "nc-u6-source-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repo = join(root, "repo"); const bin = join(root, "bin"); const runs = join(root, "runs");
  const remote = join(root, "remote.git"); const pilot = join(root, "pilot");
  mkdirSync(repo); mkdirSync(bin); mkdirSync(pilot);
  const calls = join(root, "calls");
  // python3 替身记入同一调用表：仅含 DeepAgents 来源的清单不申请旧入口冻结准入。
  for (const name of ["docker", "gh", "codex", "python3"]) {
    const path = join(bin, name);
    writeFileSync(path, `#!/bin/sh\nprintf '%s\\n' '${name}' >> "$PROBE_CALLS"\nexit 42\n`);
    chmodSync(path, 0o755);
  }
  // 替身 python：与生产读取器同一命令合同，事实取自临时裸远端与 PR 表；收到用户 token 即失败。
  const facts = join(root, "fake-python");
  writeFileSync(facts, `#!${process.execPath}
const fs = require("node:fs"); const { execFileSync } = require("node:child_process");
const cfg = JSON.parse(fs.readFileSync(process.env.PYTHONPATH + "/facts.json", "utf8"));
const a = process.argv.slice(2); const branch = a[a.indexOf("--branch") + 1];
fs.appendFileSync(cfg.log, JSON.stringify({ branch, key: a[a.indexOf("--private-key") + 1], gh: process.env.GH_TOKEN ?? null }) + "\\n");
if (process.env.GH_TOKEN || cfg.fail) { process.stderr.write("facts unavailable"); process.exit(2); }
let ref = null; try { ref = execFileSync("git", ["--git-dir", cfg.remote, "rev-parse", "--verify", "-q", "refs/heads/" + branch], { encoding: "utf8" }).trim(); } catch {}
process.stdout.write(JSON.stringify({ branch, ref, pulls: cfg.pulls[branch] ?? [] }));
`);
  chmodSync(facts, 0o755);
  const git = (...args: string[]) => execFileSync("git", args, { cwd: repo, encoding: "utf8", stdio: "pipe" }).trim();
  execFileSync("git", ["init", "--bare", "-b", "main", remote], { stdio: "pipe" });
  git("init", "-b", "main"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  git("remote", "add", "origin", remote);
  writeFileSync(join(repo, "doc.md"), "base\n"); git("add", "."); git("commit", "-m", "base");
  const base = git("rev-parse", "HEAD");
  const pulls: Record<string, unknown[]> = {};
  const sources = [11, 12].map(ticket => {
    const branch = `codex/deepagents/t${ticket}`;
    git("checkout", "-b", branch, base);
    writeFileSync(join(repo, conflict ? "doc.md" : `doc${ticket}.md`), `ticket ${ticket}\n`);
    git("add", "."); git("commit", "-m", `ticket ${ticket}`);
    const sha = git("rev-parse", "HEAD"); const dir = join(runs, `t${ticket}-1`);
    git("push", "-q", "-u", "origin", branch);
    deliver(dir, ticket, sha);
    pulls[branch] = [{ number: 9000 + ticket, state: "open", merged: false, head: sha, head_ref: branch }];
    return { ticket, sha, dir, branch };
  });
  git("checkout", "main");
  const config = { remote, pulls, log: join(root, "facts.log"), fail: false };
  const saveConfig = () => write(join(pilot, "facts.json"), config); saveConfig();
  const manifest: MergeManifest = { runId: "u6", branch: "sandcastle/merge-u6", base,
    inputs: sources.map(s => deepAgentsMergeInput(s.dir, s.sha, `fixture-validation ${s.sha}`)) };
  const path = join(root, "accepted.json"); writeFileSync(path, JSON.stringify(manifest));
  const worktree = join(repo, ".sandcastle/worktrees/sandcastle-merge-u6");
  const checkpoint = join(repo, ".sandcastle/logs/merges/u6/checkpoint.json");
  const env = { DEEPAGENTS_PYTHON: facts, DEEPAGENTS_PILOT_PATH: pilot, PILOT1742_APP_PRIVATE_KEY: join(root, "app.pem"), HOME: root };
  const run = (extra: Record<string, string> = {}) => spawnSync(process.execPath, ["--import", loader, entry], {
    cwd: repo, encoding: "utf8", timeout: 60_000,
    env: { PATH: `${bin}:${dirname(process.execPath)}:/usr/bin:/bin`, PROBE_CALLS: calls, GH_TOKEN: "user-token-canary",
      CLIPROXY_LOCAL_KEY: "fixture-no-network", SANDCASTLE_SWARM_MERGE_MANIFEST: path, ...env, ...extra },
  });
  const called = () => existsSync(calls) ? readFileSync(calls, "utf8").trim().split("\n") : [];
  const factCalls = () => existsSync(config.log) ? readFileSync(config.log, "utf8").trim().split("\n").map(l => JSON.parse(l)) : [];
  const refs = () => git("for-each-ref", "--format=%(objectname) %(refname)");
  const reader = appRemoteFacts({ ...env, GH_TOKEN: "user-token-canary" });
  return { root, repo, remote, git, base, sources, manifest, run, called, factCalls, checkpoint, worktree, refs, config, saveConfig, reader };
}
type Fixture = ReturnType<typeof fixture>;

function assertNoWrites(f: Fixture) {
  assert.equal(existsSync(f.checkpoint), false); assert.equal(existsSync(f.worktree), false);
  assert.equal(f.git("branch", "--list", "sandcastle/merge-u6"), "");
  assert.equal(f.git("rev-parse", "main"), f.base); assert.deepEqual(f.called(), []);
}

test("公共 CLI：两份 DeepAgents 交付按固定 SHA 双父归并，经只读远端事实复核，重入不重复提交", t => {
  const f = fixture(t);
  const before = f.refs();
  const first = f.run(); assert.equal(first.status, 0, first.stderr);
  const state = readJson(f.checkpoint);
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
  for (const s of f.sources) assert.equal(f.git("rev-parse", s.branch), s.sha);
  assert.equal(f.git("rev-parse", "main"), f.base);
  assert.equal(f.git("branch", "--list", "sandcastle/issue-*"), "");
  const calls = f.factCalls();
  assert.ok(calls.length >= 4); assert.ok(calls.every(c => c.gh === null && c.key === join(f.root, "app.pem")), "读取器不得继承用户 token");
  assert.deepEqual([...new Set(calls.map(c => c.branch))].sort(), f.sources.map(s => s.branch));
  const head = f.git("rev-parse", "sandcastle/merge-u6"); const saved = readFileSync(f.checkpoint, "utf8");
  const again = f.run(); assert.equal(again.status, 0, again.stderr);
  assert.equal(f.git("rev-parse", "sandcastle/merge-u6"), head);
  assert.equal(readFileSync(f.checkpoint, "utf8"), saved);
  assert.equal(f.git("rev-list", "--count", `${f.base}..${head}`), "4");
  assert.deepEqual(f.called(), []);
  const after = f.refs().split("\n").filter(l => !l.endsWith("refs/heads/sandcastle/merge-u6")).join("\n");
  assert.equal(after, before);
});

test("公共 CLI：来源失效或远端事实不可得时，旧证据在任何 Git 写入前被拒绝", async t => {
  const cases: Array<[string, (f: Fixture) => Record<string, string> | void, RegExp]> = [
    ["执行器写 stale.json，state.json 仍为 awaiting_merge", f => markStale(f.sources[1]!.dir, f.sources[1]!.sha, f.base), /已被执行器判 stale/],
    ["执行器写效果仍 pending", f => write(join(f.sources[1]!.dir, "delivery", `push-${f.base}.json`), { state: "pending", what: { kind: "ref" } }), /未认回的写效果：push-/],
    ["远端被推进而本地未 fetch", f => {
      execFileSync("git", ["--git-dir", f.remote, "update-ref", `refs/heads/${f.sources[1]!.branch}`, f.base]);
      assert.equal(f.git("rev-parse", `refs/remotes/origin/${f.sources[1]!.branch}`), f.sources[1]!.sha); }, /远端分支已不在已审 SHA：codex\/deepagents\/t12=/],
    ["远端分支被删除", f => { execFileSync("git", ["--git-dir", f.remote, "update-ref", "-d", `refs/heads/${f.sources[1]!.branch}`]); }, /远端分支已不在已审 SHA：codex\/deepagents\/t12=absent/],
    ["交付 PR 已关闭", f => { (f.config.pulls[f.sources[1]!.branch] as any[])[0].state = "closed"; f.saveConfig(); }, /PR #9012 不是指向已审 SHA 的 open PR/],
    ["同分支另有指向他处的 open PR", f => { (f.config.pulls[f.sources[1]!.branch] as any[]).push({ number: 9999, state: "open", merged: false, head: f.base, head_ref: f.sources[1]!.branch }); f.saveConfig(); }, /另有 open PR #9999/],
    ["远端读取失败", f => { f.config.fail = true; f.saveConfig(); }, /远端事实读取失败，停止归并/],
    ["未配置只读读取器", () => ({ DEEPAGENTS_PYTHON: "" }), /需要 App 只读远端事实/],
    ["本地来源分支前进", f => { f.git("checkout", "-q", f.sources[1]!.branch); writeFileSync(join(f.repo, "late.md"), "late\n");
      f.git("add", "late.md"); f.git("commit", "-q", "-m", "late"); f.git("checkout", "-q", "main"); }, /来源分支已不在已审 SHA：refs\/heads\/codex\/deepagents\/t12/],
    ["review 回执为 BLOCK", f => { const s = f.sources[1]!; const p = join(s.dir, "workers", `review-${s.sha}`, "result.json");
      const r = readJson(p); r.result.summary = "VERDICT: BLOCK\nfixture"; writeFileSync(p, JSON.stringify(r)); }, /VERDICT: PASS/],
    ["缺 deepagents/review 状态", f => { const s = f.sources[1]!; rmSync(join(s.dir, "delivery", `status-${s.sha}.json`)); }, /记录不可读/],
  ];
  for (const [name, mutate, message] of cases) {
    await t.test(name, t2 => {
      const f = fixture(t2); const extra = mutate(f) ?? {};
      const result = f.run(extra);
      assert.equal(result.status, 1, name); assert.match(result.stderr, message);
      assertNoWrites(f);
    });
  }
});

test("公共 CLI：冲突仍先调原 Docker 检查并保留 MERGE_HEAD；之后执行器写 stale.json，重入在 Docker 前拒绝且现场不变", t => {
  const f = fixture(t, true);
  const first = f.run();
  assert.equal(first.status, 1); assert.match(first.stderr, /尚无精确验收 receipt/);
  const saved = readFileSync(f.checkpoint, "utf8"); const state = JSON.parse(saved);
  assert.equal(state.phase, "conflict"); assert.equal(state.position, 1); assert.deepEqual(state.allowed, ["doc.md"]);
  assert.equal(state.snapshot.mergeHead, f.sources[1]!.sha);
  assert.equal(execFileSync("git", ["rev-parse", "MERGE_HEAD"], { cwd: f.worktree, encoding: "utf8" }).trim(), f.sources[1]!.sha);
  const conflict = readFileSync(join(f.worktree, "doc.md"), "utf8"); assert.match(conflict, /<<<<<<< HEAD/);
  assert.deepEqual(f.called(), [], "无 receipt 时不得启动容器");
  markStale(f.sources[1]!.dir, f.sources[1]!.sha, f.base);
  const again = f.run();
  assert.equal(again.status, 1); assert.match(again.stderr, /已被执行器判 stale/);
  assert.equal(readFileSync(f.checkpoint, "utf8"), saved);
  assert.equal(readFileSync(join(f.worktree, "doc.md"), "utf8"), conflict);
  assert.equal(execFileSync("git", ["rev-parse", "MERGE_HEAD"], { cwd: f.worktree, encoding: "utf8" }).trim(), f.sources[1]!.sha);
  assert.deepEqual(f.called(), []);
  assert.equal(f.git("rev-parse", "main"), f.base);
});

const RESOLVED = '<sandcastle-result>{"status":"ready","files":["doc.md"],"verification":[]}</sandcastle-result>';

test("冲突等待期间来源失效：不提交、不记 done，保留 MERGE_HEAD 与 prepared 检查点；来源恢复后同检查点续完", async t => {
  const f = fixture(t, true);
  f.git("worktree", "add", "-b", f.manifest.branch, f.worktree, f.base);
  const s = f.sources[1]!;
  const options = { cwd: f.worktree, checkpoint: f.checkpoint, manifest: f.manifest, checkId: "u6-probe",
    admit: (input: any) => admitMergeInput(f.repo, input, f.reader),
    verify: async (head: string) => ({ head, checkId: "u6-probe", status: "passed" as const, evidence: "fixture" }) };
  let workerCalls = 0;
  await assert.rejects(runManagedMerge({ ...options, resolveConflict: async () => {
    workerCalls++; markStale(s.dir, s.sha, f.base);
    writeFileSync(join(f.worktree, "doc.md"), "resolved\n"); return RESOLVED;
  } }), /已被执行器判 stale/);
  const state = readJson(f.checkpoint);
  assert.equal(state.phase, "prepared"); assert.equal(state.position, 1); assert.equal(state.entries.length, 1);
  assert.equal(state.snapshot.mergeHead, s.sha);
  assert.equal(execFileSync("git", ["rev-parse", "MERGE_HEAD"], { cwd: f.worktree, encoding: "utf8" }).trim(), s.sha);
  assert.equal(execFileSync("git", ["show", ":doc.md"], { cwd: f.worktree, encoding: "utf8" }), "resolved\n");
  const head = f.git("rev-parse", f.manifest.branch);
  assert.equal(f.git("show", "-s", "--format=%P", head), `${f.base} ${f.sources[0]!.sha}`);
  // 执行器对同一 head 重新发布成功时会删掉 stale.json；之后从同一检查点续完，不再调冲突工蜂。
  rmSync(join(s.dir, "delivery", "stale.json"));
  const done = await runManagedMerge({ ...options, resolveConflict: async () => { workerCalls++; return RESOLVED; } });
  assert.equal(done.phase, "done"); assert.equal(done.entries.length, 2); assert.equal(workerCalls, 1);
  assert.equal(f.git("show", "-s", "--format=%P", done.snapshot.head), `${head} ${s.sha}`);
  assert.equal(f.git("rev-list", "--count", `${f.base}..${done.snapshot.head}`), "4");
});

test("组合验证期间来源失效：停在 committed，不记 done；重入在复核处继续拒绝", async t => {
  const f = fixture(t);
  f.git("worktree", "add", "-b", f.manifest.branch, f.worktree, f.base);
  const s = f.sources[1]!;
  const options = { cwd: f.worktree, checkpoint: f.checkpoint, manifest: f.manifest, checkId: "u6-probe",
    admit: (input: any) => admitMergeInput(f.repo, input, f.reader),
    resolveConflict: async (): Promise<string> => { throw new Error("不应出现冲突"); } };
  await assert.rejects(runManagedMerge({ ...options, verify: async head => {
    if (f.git("rev-parse", `${head}^2`) === s.sha) execFileSync("git", ["--git-dir", f.remote, "update-ref", `refs/heads/${s.branch}`, f.base]);
    return { head, checkId: "u6-probe", status: "passed", evidence: "fixture" };
  } }), /远端分支已不在已审 SHA/);
  const state = readJson(f.checkpoint);
  assert.equal(state.phase, "committed"); assert.equal(state.entries.length, 1);
  await assert.rejects(runManagedMerge({ ...options, verify: async head => ({ head, checkId: "u6-probe", status: "passed", evidence: "fixture" }) }), /远端分支已不在已审 SHA/);
  assert.equal(readJson(f.checkpoint).phase, "committed");
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
  assert.doesNotThrow(() => admitMergeInput(f.repo, legacy as any, () => { throw new Error("旧输入不得读远端"); }));
});
