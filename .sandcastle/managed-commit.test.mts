import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, readFileSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import { beginManagedStage, finishManagedStage, parseStageReport, verifyManagedRust } from "./managed-commit.mts";
const report = (files: string[], status = "ready") => `<sandcastle-result>${JSON.stringify({ status, files, verification: ["fixture checked"] })}</sandcastle-result>`;
function fixture(t: { after: (fn: () => void) => void }) {
  const cwd = mkdtempSync(join(tmpdir(), "nc-managed-"));
  t.after(() => rmSync(cwd, { recursive: true, force: true }));
  const git = (...args: string[]) => execFileSync("git", args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  git("init", "-b", "sandcastle/issue-1617"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  git("commit", "--allow-empty", "-m", "base");
  return { cwd, git, begin: (phase: "implementer" | "reviewer" = "implementer") => beginManagedStage(cwd, "sandcastle/issue-1617", 1617, phase) };
}
test("作者封包→精确提交→独审修正→重入不重复提交", t => {
  const f = fixture(t); const author = f.begin();
  writeFileSync(join(f.cwd, "a.txt"), "author");
  const a = finishManagedStage(author, report(["a.txt"])); assert.equal(a.committed, true);
  assert.equal(f.git("show", `${a.head}:a.txt`), "author");
  assert.equal(finishManagedStage(author, report(["a.txt"])).head, a.head);
  const review = f.begin("reviewer"); assert.equal(review.head, a.head);
  writeFileSync(join(f.cwd, "a.txt"), "reviewed");
  const b = finishManagedStage(review, report(["a.txt"])); assert.notEqual(b.head, a.head);
  assert.equal(f.git("rev-parse", "HEAD^"), a.head); assert.equal(f.git("show", "HEAD:a.txt"), "reviewed");
  assert.equal(finishManagedStage(review, report(["a.txt"])).head, b.head);
});
test("无修正不造空提交；blocked 不提交", t => {
  const f = fixture(t); const s = f.begin("reviewer");
  assert.equal(finishManagedStage(s, report([])).committed, false);
  writeFileSync(join(f.cwd, "a.txt"), "pending");
  assert.throws(() => finishManagedStage(s, report(["a.txt"], "blocked")), /blocked/);
  assert.equal(f.git("rev-parse", "HEAD"), s.head);
});
test("清单外文件、脏工位、错误分支不触碰", t => {
  const f = fixture(t); const s = f.begin();
  writeFileSync(join(f.cwd, "other.txt"), "other owner");
  assert.throws(() => finishManagedStage(s, report([])), /清单不一致/);
  assert.equal(readFileSync(join(f.cwd, "other.txt"), "utf8"), "other owner");
  assert.throws(() => f.begin(), /不干净/);
  assert.throws(() => beginManagedStage(f.cwd, "main", 1617, "implementer"), /本票分支/);
});
test("提交失败保留暂存产物，可用原阶段重试", t => {
  const f = fixture(t); const s = f.begin(); writeFileSync(join(f.cwd, "a.txt"), "a");
  const hooks = join(f.cwd, ".git", "hooks");
  writeFileSync(join(hooks, "pre-commit"), "#!/bin/sh\nexit 1\n", { mode: 0o755 });
  assert.throws(() => finishManagedStage(s, report(["a.txt"])));
  assert.equal(f.git("diff", "--cached", "--name-only"), "a.txt");
  rmSync(join(hooks, "pre-commit")); assert.equal(finishManagedStage(s, report(["a.txt"])).committed, true);
});
test("基线改变不自动接管；字面路径支持特殊文件名", t => {
  const f = fixture(t); const s = f.begin(); f.git("commit", "--allow-empty", "-m", "other");
  assert.throws(() => finishManagedStage(s, report([])), /HEAD 改变/);
  const next = f.begin(); writeFileSync(join(f.cwd, ":(glob)*"), "literal");
  assert.equal(finishManagedStage(next, report([":(glob)*"])).committed, true);
});
test("阶段报告拒绝受保护路径、越界、重复、缺块与多块", () => {
  for (const file of ["../escape", "/tmp/a", ".git/config", ".agents/x", ".codex/x", ".aws/x"]) assert.throws(() => parseStageReport(report([file])));
  assert.throws(() => parseStageReport(report(["a", "a"])));
  assert.throws(() => parseStageReport("done")); assert.throws(() => parseStageReport(report([]) + report([])));
});
test("删除与重命名的完整文件清单可提交", t => {
  const f = fixture(t); writeFileSync(join(f.cwd, "old"), "x"); f.git("add", "old"); f.git("commit", "-m", "seed");
  const s = f.begin(); rmSync(join(f.cwd, "old")); writeFileSync(join(f.cwd, "new"), "x");
  assert.equal(finishManagedStage(s, report(["old", "new"])).committed, true);
});
test("远端只检查精确提交；失败不伪称通过；非Rust不启动", t => {
  const f = fixture(t); const s = f.begin(); mkdirSync(join(f.cwd, "rust")); writeFileSync(join(f.cwd, "rust/a.rs"), "// fixture");
  const sealed = finishManagedStage(s, report(["rust/a.rs"])); let seen = "";
  assert.match(verifyManagedRust(s, sealed, ref => { seen = ref; }), /通过/); assert.equal(seen, sealed.head);
  assert.match(verifyManagedRust(s, sealed, () => { throw new Error("offline"); }), /不能验收合入/);
  assert.match(verifyManagedRust(s, { ...sealed, report: { ...sealed.report, files: [] } }, () => assert.fail("unexpected")), /未改动/);
});
test("真实 nc-remote 启动合同使用可 bundle 的具名精确 ref 并清理", t => {
  const f = fixture(t); const stage = f.begin(); mkdirSync(join(f.cwd, "rust")); writeFileSync(join(f.cwd, "rust/a.rs"), "// fixture");
  const sealed = finishManagedStage(stage, report(["rust/a.rs"]));
  const home = mkdtempSync(join(tmpdir(), "nc-managed-home-")); t.after(() => rmSync(home, { recursive: true, force: true }));
  mkdirSync(join(home, ".local/bin"), { recursive: true });
  writeFileSync(join(home, ".local/bin/nc-remote"), '#!/bin/sh\nset -eu\ngit -C "$NC_REPO" bundle create "$HOME/check.bundle" "$1"\ngit -C "$NC_REPO" rev-parse "$1" > "$HOME/seen"\n', { mode: 0o755 });
  const old = process.env.HOME;
  try { process.env.HOME = home; assert.match(verifyManagedRust(stage, sealed), /通过/); }
  finally { if (old === undefined) delete process.env.HOME; else process.env.HOME = old; }
  assert.equal(readFileSync(join(home, "seen"), "utf8").trim(), sealed.head);
  assert.equal(f.git("for-each-ref", "--format=%(refname)", "refs/sandcastle-verification"), "");
});
