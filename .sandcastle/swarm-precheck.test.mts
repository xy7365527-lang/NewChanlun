import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { codeIdentity, issueReceipt } from "./docker-acceptance.mts";

const entry = fileURLToPath(new URL("./main-swarm.mts", import.meta.url));
const loader = import.meta.resolve("tsx");
const identity = codeIdentity();
/** 子进程里 receipt 核对用的是真实 checkout 身份；.sandcastle 有未提交改动时精确 head 不成立，该类用例只在干净提交上跑。 */
const dirtySkip = identity.dirty ? ".sandcastle 有未提交改动，receipt 不精确（提交后重跑）" : false;
function fixture(t: { after: (fn: () => void) => void }, conflict = false) {
  const root = mkdtempSync(join(tmpdir(), "nc-precheck-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repo = join(root, "repo"); const bin = join(root, "bin");
  mkdirSync(repo); mkdirSync(bin);
  const calls = join(root, "calls");
  for (const name of ["docker", "gh", "codex"]) {
    const path = join(bin, name);
    writeFileSync(path, `#!/bin/sh\nprintf '%s\\n' '${name}' >> "$PROBE_CALLS"\nif [ '${name}' = docker ] && [ "$PROBE_NAMESPACE_OK" = 1 ]; then\n  case "$*" in *'--entrypoint sh'*) printf '%s\\n' 'apparmor=docker-default (enforce)' 'seccomp=2' 'capEff=0000000000000000' 'capBnd=00000000a80425fb' 'roWrite=touch: Read-only file system' 'rwWrite=ok' 'roChildInRw=Read-only file system' 'roFileAppend=Read-only file system' 'renameMountpoint=Device or resource busy' 'hardlinkAcross=Invalid cross-device link' 'symlinkIntoRo=Read-only file system'; exit 0 ;; esac\nfi\nexit 42\n`);
    chmodSync(path, 0o755);
  }
  // #1742 旧入口准入替身：记录 drain-control 请求，不触碰本机生产冻结根；PROBE_FROZEN=1 模拟已冻结。
  const drainCalls = join(root, "drain-calls");
  writeFileSync(join(bin, "python3"), `#!/bin/sh\nprintf '%s\\n' "$*" >> "$PROBE_DRAIN"\nif [ "$PROBE_FROZEN" = 1 ]; then echo frozen; exit 75; fi\necho admitted\ncat >/dev/null\n`);
  chmodSync(join(bin, "python3"), 0o755);
  const git = (...args: string[]) => execFileSync("git", args, { cwd: repo, encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-b", "main"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  writeFileSync(join(repo, "doc.md"), "base\n"); git("add", "."); git("commit", "-m", "base");
  const base = git("rev-parse", "HEAD");
  const inputs = [1, 2].map(issue => {
    git("checkout", "-b", `sandcastle/issue-${issue}`, base);
    writeFileSync(join(repo, conflict ? "doc.md" : `doc${issue}.md`), `issue ${issue}\n`);
    git("add", "."); git("commit", "-m", `issue ${issue}`); const sha = git("rev-parse", "HEAD");
    return { issue, branch: `sandcastle/issue-${issue}`, sha,
      acceptance: { head: sha, status: "accepted", reviewEvidence: "fixture-review", verificationEvidence: "fixture-validation" } };
  });
  git("checkout", "main");
  const manifest = { runId: "precheck", branch: "sandcastle/merge-precheck", base, inputs };
  const path = join(root, "accepted.json"); writeFileSync(path, JSON.stringify(manifest));
  const worktree = join(repo, ".sandcastle/worktrees/sandcastle-merge-precheck");
  const checkpoint = join(repo, ".sandcastle/logs/merges/precheck/checkpoint.json");
  const acceptance = join(root, ".sandcastle-host", "docker-acceptance-1742"); // 子进程 HOME=root，生产固定目录
  const run = (accepted = true, namespaceReady = false, frozen = false, row?: "13" | "14" | "15") => spawnSync(process.execPath, ["--import", loader, entry], {
    cwd: repo, encoding: "utf8", timeout: 30_000,
    env: { PATH: `${bin}:${dirname(process.execPath)}:/usr/bin:/bin`, HOME: root, PROBE_CALLS: calls,
      PROBE_DRAIN: drainCalls, PROBE_FROZEN: frozen ? "1" : "0",
      PROBE_NAMESPACE_OK: namespaceReady ? "1" : "0", CLIPROXY_LOCAL_KEY: "fixture-no-network",
      ...(row ? { SANDCASTLE_DOCKER_ACCEPTANCE_ROW: row } : {}),
      ...(accepted ? { SANDCASTLE_SWARM_MERGE_MANIFEST: path } : {}) },
  });
  const called = () => existsSync(calls) ? readFileSync(calls, "utf8").trim().split("\n") : [];
  const drained = () => existsSync(drainCalls) ? readFileSync(drainCalls, "utf8").trim().split("\n") : [];
  const issue = () => issueReceipt({ dir: acceptance, image: "sandcastle:newchanlun", approval: "fixture-user-approval", swarmTicket: 4242 });
  return { repo, git, base, inputs, manifest, run, called, drained, checkpoint, worktree, issue, acceptance };
}

test("公共 CLI：Docker 不可用时无冲突 accepted 清单完成双父归并及幂等重入", t => {
  const f = fixture(t);
  const first = f.run();
  assert.equal(first.status, 0, first.stderr);
  const state = JSON.parse(readFileSync(f.checkpoint, "utf8"));
  assert.equal(state.phase, "done"); assert.equal(state.position, 2);
  assert.equal(state.entries.length, 2); assert.equal(state.checks.length, 2);
  assert.equal(f.git("rev-parse", "main"), f.base);
  let parent = f.base;
  for (const [n, e] of state.entries.entries()) {
    assert.equal(f.git("show", "-s", "--format=%P", e.head), `${parent} ${f.inputs[n]!.sha}`);
    parent = e.head;
  }
  const before = readFileSync(f.checkpoint, "utf8");
  const again = f.run(); assert.equal(again.status, 0, again.stderr);
  assert.equal(readFileSync(f.checkpoint, "utf8"), before);
  assert.deepEqual(f.called(), []);
  // 新归并在固定生产冻结根取一次准入；已存在的归并工位属已准入 run，重入不再申请。
  assert.equal(f.drained().length, 1);
  assert.match(f.drained()[0]!, / hold --root \/Volumes\/AgentStorage\/newchanlun-legacy-drain$/);
});

test("公共 CLI：旧入口已冻结时新 accepted 清单退出 75，不建归并工位、检查点或分支", t => {
  const f = fixture(t);
  const result = f.run(true, false, true);
  assert.equal(result.status, 75, result.stderr); assert.match(result.stderr, /旧入口已冻结/);
  assert.equal(existsSync(f.checkpoint), false); assert.equal(existsSync(f.worktree), false);
  assert.equal(f.git("branch", "--list", "sandcastle/merge-precheck"), "");
  assert.equal(f.git("rev-parse", "main"), f.base);
  assert.deepEqual(f.called(), []);
});

test("公共 CLI：冲突时 Docker 预检失败，保留 MERGE_HEAD/checkpoint 且不启动工蜂", t => {
  const f = fixture(t, true);
  const first = f.run();
  assert.equal(first.status, 1); assert.match(first.stderr, /尚无精确验收 receipt.*未声明受控验收行/);
  assert.ok(existsSync(f.checkpoint), "须在冲突现场检查准入，而不是入口直接拒绝");
  const before = readFileSync(f.checkpoint, "utf8"); const state = JSON.parse(before);
  assert.equal(state.phase, "conflict"); assert.equal(state.position, 1);
  assert.deepEqual(state.allowed, ["doc.md"]);
  assert.equal(state.snapshot.mergeHead, f.inputs[1]!.sha);
  assert.equal(execFileSync("git", ["rev-parse", "MERGE_HEAD"], { cwd: f.worktree, encoding: "utf8" }).trim(), f.inputs[1]!.sha);
  const conflict = readFileSync(join(f.worktree, "doc.md"), "utf8"); assert.match(conflict, /<<<<<<< HEAD/);
  assert.equal(f.git("rev-parse", "main"), f.base);
  assert.deepEqual(f.called(), [], "无 receipt 时不得为观测而启动任何容器");
  const again = f.run(); assert.equal(again.status, 1); assert.match(again.stderr, /尚无精确验收 receipt/);
  assert.equal(readFileSync(f.checkpoint, "utf8"), before);
  assert.equal(readFileSync(join(f.worktree, "doc.md"), "utf8"), conflict);
  assert.deepEqual(f.called(), []);
});

test("公共 CLI：普通 swarm 在 planner/claim 前无 receipt 即拒绝，不启动容器", t => {
  const f = fixture(t); const result = f.run(false);
  assert.equal(result.status, 1); assert.match(result.stderr, /尚无精确验收 receipt/);
  assert.deepEqual(f.called(), []);
  assert.equal(existsSync(f.checkpoint), false);
  assert.equal(f.git("worktree", "list", "--porcelain").match(/^worktree /gm)?.length, 1);
  assert.equal(f.git("rev-parse", "main"), f.base);
});

test("公共 CLI：观测全部成立但无 receipt 或未声明验收行，冲突入口仍 FAIL-loud 且不启动容器", { skip: dirtySkip }, t => {
  const f = fixture(t, true);
  const noReceipt = f.run(true, true, false, "14");
  assert.equal(noReceipt.status, 1); assert.match(noReceipt.stderr, /验收目录不存在|receipt 缺失/);
  f.issue();
  const noRow = f.run(true, true);
  assert.equal(noRow.status, 1); assert.match(noRow.stderr, /未声明受控验收行/);
  const admitted = f.run(true, true, false, "14");
  assert.equal(admitted.status, 1); assert.match(admitted.stderr, /主控模型 .* 预检失败/, "声明了 14 且 receipt 有效时才进入下一道门");
  // 模型预检失败发生在占用之后：同一 run+检查点重入是同一绑定，只记恢复，不重复占用也不被误拒。
  const reentered = f.run(true, true, false, "14");
  assert.equal(reentered.status, 1); assert.match(reentered.stderr, /主控模型 .* 预检失败/);
  assert.equal(readdirSync(f.acceptance).filter(n => n.startsWith("resume-")).length, 1);
  assert.equal(f.git("rev-parse", "main"), f.base);
});

test("公共 CLI：冲突 receipt 有效、观测成立但模型路由拒绝时仍不启动冲突工蜂", { skip: dirtySkip }, t => {
  const f = fixture(t, true); f.issue(); const result = f.run(true, true, false, "14");
  assert.equal(result.status, 1); assert.match(result.stderr, /主控模型 .* 预检失败/);
  const state = JSON.parse(readFileSync(f.checkpoint, "utf8"));
  assert.equal(state.phase, "conflict"); assert.equal(state.position, 1);
  assert.equal(state.snapshot.mergeHead, f.inputs[1]!.sha);
  assert.equal(execFileSync("git", ["rev-parse", "MERGE_HEAD"], { cwd: f.worktree, encoding: "utf8" }).trim(), f.inputs[1]!.sha);
  assert.deepEqual(f.called(), ["docker", "docker"]);
  assert.equal(f.git("rev-parse", "main"), f.base);
});
