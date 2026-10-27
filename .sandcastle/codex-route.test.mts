import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { codexWorker } from "./codex-route.mts";

test("实装和独审默认 Codex/Astra，独立 home，同阶段 resume 保留", () => {
  const author = codexWorker();
  const review = codexWorker();
  const options = { prompt: "检查本票", dangerouslySkipPermissions: true };
  const first = author.buildPrintCommand(options);
  const other = review.buildPrintCommand(options);
  assert.equal(author.name, "codex");
  assert.match(first.command, /codex -a never -s workspace-write exec --ignore-user-config/);
  assert.match(first.command, /gpt-6-astra/);
  assert.match(first.command, /model_provider="cliproxy-local"/);
  assert.doesNotMatch(first.command, /dangerously|claude|kimi/);
  assert.notEqual(first.command, other.command);
  assert.equal(first.stdin, options.prompt);
  const resumed = author.buildPrintCommand({ ...options, resumeSession: "fixture-session" });
  assert.match(resumed.command, /resume 'fixture-session'/);
  assert.equal(first.command.split(" CODEX_HOME=")[1].split(" codex")[0], resumed.command.split(" CODEX_HOME=")[1].split(" codex")[0]);
});

test("Docker Codex 使用容器可达网关，main 保留一票两阶段", () => {
  const command = codexWorker({ docker: true }).buildPrintCommand({ prompt: "hi", dangerouslySkipPermissions: true }).command;
  assert.match(command, /host.docker.internal:8318\/v1/);
  assert.doesNotMatch(command, /127.0.0.1:8318/);
  // #1742 方案B：Docker 内由容器和只读挂载隔离，不起内层 bwrap；宿主路径仍是 workspace-write。
  assert.match(command, /codex -a never -s danger-full-access exec/);
  assert.doesNotMatch(command, /workspace-write/);
  const main = readFileSync(new URL("./main.mts", import.meta.url), "utf8");
  assert.equal(main.match(/codexWorker\(\{ effort: EFFORT \}\)/g)?.length, 2);
  assert.doesNotMatch(main, /claudeWorker\(/);
  assert.match(main, /MAX_TICKETS_PER_RUN = 1/);
});

test("main 受管提交使用专属模板，不改变既有 swarm 模板合同", () => {
  const main = readFileSync(new URL("./main.mts", import.meta.url), "utf8");
  assert.match(main, /beginManagedStage\(sandbox.worktreePath, branch, issue, "implementer"\)/);
  assert.match(main, /beginManagedStage\(sandbox.worktreePath, branch, issue, "reviewer"\)/);
  assert.match(main, /codex-implement-prompt.md/); assert.match(main, /codex-review-prompt.md/);
  assert.doesNotMatch(main, /if \(!implement.commits.length\)/);
  for (const phase of ["implement", "review"]) {
    const managed = readFileSync(new URL(`./codex-${phase}-prompt.md`, import.meta.url), "utf8");
    assert.match(managed, /sandcastle-result/); assert.match(managed, /不要 git add\/commit/);
    const original = readFileSync(new URL(`./${phase}-prompt.md`, import.meta.url), "utf8");
    assert.doesNotMatch(original, /sandcastle-result/);
  }
});
