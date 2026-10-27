import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { DrainFrozenError, withLegacyAdmission } from "./drain-control.mts";
import { runAcceptedSwarmMerge } from "./swarm-merge.mts";

const helper = fileURLToPath(new URL("./drain-control.py", import.meta.url));
const drain = (root: string, ...args: string[]) => execFileSync("python3", [helper, ...args, "--root", root], { encoding: "utf8" });

/** 真实临时 Git 仓与真实 drain-control 锁（临时根），无冲突旧 sandcastle 清单。 */
function fixture(t: { after: (fn: () => void) => void }) {
  const root = mkdtempSync(join(tmpdir(), "nc-merge-drain-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repo = join(root, "repo"); const drainRoot = join(root, "drain");
  mkdirSync(repo);
  const git = (...args: string[]) => execFileSync("git", args, { cwd: repo, encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-b", "main"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  writeFileSync(join(repo, "doc.md"), "base\n"); git("add", "."); git("commit", "-m", "base");
  const base = git("rev-parse", "HEAD");
  const inputs = [1, 2].map(issue => {
    git("checkout", "-b", `sandcastle/issue-${issue}`, base);
    writeFileSync(join(repo, `doc${issue}.md`), `issue ${issue}\n`);
    git("add", "."); git("commit", "-m", `issue ${issue}`); const sha = git("rev-parse", "HEAD");
    return { issue, branch: `sandcastle/issue-${issue}`, sha,
      acceptance: { head: sha, status: "accepted", reviewEvidence: "fixture-review", verificationEvidence: "fixture-validation" } };
  });
  git("checkout", "main");
  const path = join(root, "accepted.json");
  writeFileSync(path, JSON.stringify({ runId: "drain", branch: "sandcastle/merge-drain", base, inputs }));
  const worktree = join(repo, ".sandcastle/worktrees/sandcastle-merge-drain");
  const checkpoint = join(repo, ".sandcastle/logs/merges/drain/checkpoint.json");
  const merge = () => runAcceptedSwarmMerge(path, "unused-image", () => {}, {
    repo, admission: action => withLegacyAdmission(action, drainRoot) });
  return { repo, git, base, drainRoot, worktree, checkpoint, merge };
}

test("冻结后旧 sandcastle 清单的新归并在首个副作用前拒绝，解冻后同一清单正常完成", async t => {
  const f = fixture(t);
  assert.match(drain(f.drainRoot, "freeze"), /"draining"/);
  await assert.rejects(f.merge(), DrainFrozenError);
  assert.equal(existsSync(f.worktree), false); assert.equal(existsSync(f.checkpoint), false);
  assert.equal(existsSync(join(f.repo, ".sandcastle/logs/merges/drain")), false);
  assert.equal(f.git("branch", "--list", "sandcastle/merge-drain"), "");
  assert.equal(f.git("rev-parse", "main"), f.base);
  drain(f.drainRoot, "resume", "--generation", "1");
  const done = await f.merge();
  assert.equal(done.phase, "done"); assert.equal(done.entries.length, 2);
});

test("冻结前已准入的归并 run 在冻结后可续做收尾，不重复提交", async t => {
  const f = fixture(t);
  const first = await f.merge();
  assert.equal(first.phase, "done");
  const before = readFileSync(f.checkpoint, "utf8");
  const head = f.git("rev-parse", "sandcastle/merge-drain");
  drain(f.drainRoot, "freeze");
  const again = await f.merge();
  assert.equal(again.phase, "done");
  assert.equal(readFileSync(f.checkpoint, "utf8"), before);
  assert.equal(f.git("rev-parse", "sandcastle/merge-drain"), head);
});
