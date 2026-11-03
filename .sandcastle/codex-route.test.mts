import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { readSchedulerConfig } from "./scheduler-config.mts";
import { codexWorker } from "./codex-route.mts";

test("实装和独审默认 Codex/Astra，独立 home，同阶段 resume 保留", () => {
  const author = codexWorker();
  const review = codexWorker();
  const options = { prompt: "检查本票", dangerouslySkipPermissions: true };
  const first = author.buildPrintCommand(options);
  const other = review.buildPrintCommand(options);
  assert.equal(author.name, "codex");
  assert.match(first.command, /codex -a never -s workspace-write exec --ignore-user-config/);
  assert.match(first.command, /gpt-6.1-sol/);
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
  assert.equal(readSchedulerConfig(undefined, {}).values.mainMaxTickets, 1);
  assert.match(main, /MAX_TICKETS_PER_RUN = scheduler\.mainMaxTickets/);
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

test("显式模型与调度 effort 独立生效，显式 effort 优先且非法模型拒绝", () => {
  const code = `
    import assert from "node:assert/strict";
    import {codexWorker} from "./.sandcastle/codex-route.mts";
    const options = {prompt: "fixture", dangerouslySkipPermissions: true};
    const model = "claude-opus-5-5";
    const configured = codexWorker({docker: true, model}).buildPrintCommand(options);
    const explicit = codexWorker({docker: true, model, effort: "low"}).buildPrintCommand(options);
    assert.throws(() => codexWorker({docker: true, model: "unapproved-model"}), /模型只能是/);
    console.log(JSON.stringify({configured, explicit}));
  `;
  const result = spawnSync(process.execPath, ["--import", "tsx", "--input-type=module", "-e", code], {
    cwd: new URL("../", import.meta.url),
    encoding: "utf8",
    env: {...process.env, SANDCASTLE_WORKER_EFFORT: "high", SANDCASTLE_OPUS_MODEL: "claude-opus-5-5"},
  });
  assert.equal(result.status, 0, result.stderr);
  const {configured, explicit} = JSON.parse(result.stdout);
  for (const built of [configured, explicit]) {
    assert.match(built.command, /-m 'claude-opus-5-5'/);
    assert.match(built.command, /model_provider="cliproxy-local"/);
    assert.match(built.command, /codex -a never -s danger-full-access exec/);
    assert.equal(built.stdin, "fixture");
  }
  assert.match(configured.command, /model_reasoning_effort="high"/);
  assert.match(explicit.command, /model_reasoning_effort="low"/);
});
