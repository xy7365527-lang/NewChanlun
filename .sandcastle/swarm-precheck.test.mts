import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const entry = fileURLToPath(new URL("./main-swarm.mts", import.meta.url));
const loader = import.meta.resolve("tsx");
function fixture(t: { after: (fn: () => void) => void }, conflict = false) {
  const root = mkdtempSync(join(tmpdir(), "nc-precheck-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repo = join(root, "repo"); const bin = join(root, "bin");
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
  const run = (accepted = true) => spawnSync(process.execPath, ["--import", loader, entry], {
    cwd: repo, encoding: "utf8", timeout: 30_000,
    env: { PATH: `${bin}:${dirname(process.execPath)}:/usr/bin:/bin`, HOME: root, PROBE_CALLS: calls,
      ...(accepted ? { SANDCASTLE_SWARM_MERGE_MANIFEST: path } : {}) },
  });
  const called = () => existsSync(calls) ? readFileSync(calls, "utf8").trim().split("\n") : [];
  return { repo, git, base, inputs, manifest, run, called, checkpoint, worktree };
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
});

test("公共 CLI：冲突时 Docker 预检失败，保留 MERGE_HEAD/checkpoint 且不启动工蜂", t => {
  const f = fixture(t, true);
  const first = f.run();
  assert.equal(first.status, 1); assert.match(first.stderr, /默认 Docker namespace 不可用/);
  assert.ok(existsSync(f.checkpoint), "须在冲突现场检查 Docker，而不是入口直接拒绝");
  const before = readFileSync(f.checkpoint, "utf8"); const state = JSON.parse(before);
  assert.equal(state.phase, "conflict"); assert.equal(state.position, 1);
  assert.deepEqual(state.allowed, ["doc.md"]);
  assert.equal(state.snapshot.mergeHead, f.inputs[1]!.sha);
  assert.equal(execFileSync("git", ["rev-parse", "MERGE_HEAD"], { cwd: f.worktree, encoding: "utf8" }).trim(), f.inputs[1]!.sha);
  const conflict = readFileSync(join(f.worktree, "doc.md"), "utf8"); assert.match(conflict, /<<<<<<< HEAD/);
  assert.equal(f.git("rev-parse", "main"), f.base);
  assert.deepEqual(f.called(), ["docker"]);
  const again = f.run(); assert.equal(again.status, 1); assert.match(again.stderr, /默认 Docker namespace 不可用/);
  assert.equal(readFileSync(f.checkpoint, "utf8"), before);
  assert.equal(readFileSync(join(f.worktree, "doc.md"), "utf8"), conflict);
  assert.deepEqual(f.called(), ["docker", "docker"]);
});

test("公共 CLI：普通 swarm 在 planner/claim 前仍因 Docker 不可用拒绝", t => {
  const f = fixture(t); const result = f.run(false);
  assert.equal(result.status, 1); assert.match(result.stderr, /默认 Docker namespace 不可用/);
  assert.deepEqual(f.called(), ["docker"]);
  assert.equal(existsSync(f.checkpoint), false);
  assert.equal(f.git("worktree", "list", "--porcelain").match(/^worktree /gm)?.length, 1);
  assert.equal(f.git("rev-parse", "main"), f.base);
});
