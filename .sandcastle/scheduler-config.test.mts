import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { configPath, readSchedulerConfig, schedulerEnvName, type SchedulerKey } from "./scheduler-config.mts";

const expected = {
  childKillGraceMs: 1000,
  childExitPollMs: 25,
  childExitWaitMs: 3000,
  healthProcessWaitMs: 15000,
  healthProcessPollSeconds: 1,
  healthHttpTimeoutMs: 5000,
  adrLockMaxWaits: 600,
  adrLockPollMs: 100,
  dockerModelHttpTimeoutMs: 60000,
  dockerModelProcessTimeoutMs: 75000,
  frontierQueryLimit: 20,
  backlogQueryLimit: 1000,
  mainModel: "gpt-6.1-sol", opusModel: "claude-opus-5-5", workerEffort: "medium", childEffort: "xhigh",
  mainMaxIterations: 3, mainMaxTickets: 1, implementerIdleSeconds: 1800,
  worktreeMaxAttempts: 2, worktreeRetryDelayMs: 5000,
  swarmMaxIterations: 3, swarmMaxParallel: 4,
  childIdleSeconds: 600, childTimeoutSeconds: 3600, decisionMaps: [1323, 1340],
  decisionMaxConcurrent: 3, decisionLoopMs: 600000, decisionMaxRounds: 3,
  decisionMaxAttempts: 6, decisionTimeoutMs: 1800000,
  decisionBackoffBaseMs: 20000, decisionBackoffCapMs: 300000,
  claimPollSeconds: 60, claimRollbackMs: 900000,
  wayfinderStaleMs: 600000, claimStaleMs: 172800000, diskMinGiB: 100,
};
const original = JSON.parse(readFileSync(configPath, "utf8"));
function fixture(fn: (path: string) => void) {
  const dir = mkdtempSync(join(tmpdir(), "scheduler1606-"));
  try { fn(join(dir, "config.json")); } finally { rmSync(dir, { recursive: true, force: true }); }
}
test("版本1完整值保持集成基线 main@3a01bed557，来源完整且有效值冻结", () => {
  const config = readSchedulerConfig(configPath, {});
  assert.equal(config.revision, 1); assert.equal(config.ticket, 1606);
  assert.deepEqual(config.values, expected);
  assert.equal(Object.isFrozen(config.values), true);
  for (const key of Object.keys(expected)) assert.match(config.origins[key]!, /^record:1 /);
});
test("文件、JSON、逐字段、逐来源缺失均拒绝；不回退旧常量", () => fixture(path => {
  assert.throws(() => readSchedulerConfig(path, {}), /无法读取JSON/);
  writeFileSync(path, "{");
  assert.throws(() => readSchedulerConfig(path, {}), /无法读取JSON/);
  for (const section of ["values", "sources"]) for (const key of Object.keys(expected)) {
    const record = structuredClone(original); delete record[section][key];
    writeFileSync(path, JSON.stringify(record));
    assert.throws(() => readSchedulerConfig(path, {}), new RegExp(section + "\\." + key));
  }
}));
test("非法类型、边界、跨字段关系和未知字段均拒绝", () => fixture(path => {
  for (const [key, value] of [
    ["mainModel", "bad model"], ["opusModel", "gpt-6.1-sol"], ["workerEffort", "max"], ["childEffort", ""],
    ["swarmMaxParallel", 0], ["mainMaxTickets", 1.5], ["decisionMaps", []],
    ["decisionMaxAttempts", 4], ["childTimeoutSeconds", 43201],
    ["worktreeRetryDelayMs", 4999], ["claimPollSeconds", -1],
    ["decisionBackoffBaseMs", 300001], ["unknown", 1],
  ] as const) {
    const record = structuredClone(original); record.values[key] = value;
    writeFileSync(path, JSON.stringify(record));
    assert.throws(() => readSchedulerConfig(path, {}), /配置 #1606/);
  }
}));
test("记录结构、版本、来源和整数边界保持严格拒绝", () => fixture(path => {
  for (const raw of [null, [], {}, { ...original, schemaVersion: "sandcastle-scheduler/2" },
    { ...original, revision: 1.5 }, { ...original, ticket: 0 }, { ...original, extra: true },
    { ...original, values: null }, { ...original, sources: [] },
    { ...original, sources: { ...original.sources, mainModel: "" } },
    { ...original, sources: { ...original.sources, mainModel: 1 } }]) {
    writeFileSync(path, JSON.stringify(raw));
    assert.throws(() => readSchedulerConfig(path, {}), /配置 #1606/);
  }
  for (const value of [0, -1, 0.5, Number.MAX_SAFE_INTEGER + 1, "1", null]) {
    const record = structuredClone(original); record.values.childKillGraceMs = value;
    writeFileSync(path, JSON.stringify(record));
    assert.throws(() => readSchedulerConfig(path, {}), /values.childKillGraceMs/);
  }
  const record = structuredClone(original);
  record.values.claimPollSeconds = 0.5; record.values.diskMinGiB = 0.5;
  writeFileSync(path, JSON.stringify(record));
  assert.equal(readSchedulerConfig(path, {}).values.claimPollSeconds, 0.5);
}));
test("全部环境覆盖可追溯；旧env名称保留；非法覆盖拒绝且不回显值", () => {
  const env: NodeJS.ProcessEnv = {};
  for (const [key, value] of Object.entries(expected)) {
    env[schedulerEnvName(key as SchedulerKey)] = Array.isArray(value) ? JSON.stringify(value) : String(value);
  }
  const config = readSchedulerConfig(configPath, env);
  assert.deepEqual(config.values, expected);
  for (const key of Object.keys(expected) as SchedulerKey[]) assert.equal(config.origins[key], "env:" + schedulerEnvName(key));
  assert.equal(readSchedulerConfig(configPath, { CLAIM_LOOP_POLL_SECONDS: "120", HEALTH_DISK_MIN_GIB: "148" }).values.claimPollSeconds, 120);
  assert.equal(readSchedulerConfig(configPath, { HEALTH_DISK_MIN_GIB: "148" }).values.diskMinGiB, 148);
  for (const key of ["CLAIM_LOOP_POLL_SECONDS", "SANDCASTLE_MAIN_MODEL", "SANDCASTLE_DECISION_MAPS"]) {
    assert.throws(() => readSchedulerConfig(configPath, { [key]: "secret invalid sentinel" }),
      (error: Error) => !error.message.includes("secret invalid sentinel"));
  }
});
test("各入口绑定集中记录；启动日志含值和来源但不含无关secret", () => {
  const files = {
    "swarm-route.mts": ["dockerModelHttpTimeoutMs", "dockerModelProcessTimeoutMs"],

    "main.mts": ["frontierQueryLimit", "backlogQueryLimit", "workerEffort", "mainMaxIterations", "mainMaxTickets", "implementerIdleSeconds", "worktreeMaxAttempts", "worktreeRetryDelayMs"],
    "main-swarm.mts": ["swarmMaxIterations", "swarmMaxParallel"],
    "codex-child.ts": ["opusModel", "childKillGraceMs", "childExitPollMs", "childExitWaitMs", "childEffort", "childIdleSeconds", "childTimeoutSeconds"],
    "codex-route.mts": ["workerEffort"],
    "decision-claimer.mts": ["opusModel", "adrLockMaxWaits", "adrLockPollMs", "decisionMaps", "decisionMaxConcurrent", "decisionLoopMs", "decisionMaxRounds", "decisionMaxAttempts", "decisionTimeoutMs", "decisionBackoffBaseMs", "decisionBackoffCapMs"],
    "claim-loop.mts": ["claimPollSeconds", "claimRollbackMs"],
    "health-monitor.mts": ["healthProcessWaitMs", "healthProcessPollSeconds", "healthHttpTimeoutMs", "wayfinderStaleMs", "claimStaleMs", "diskMinGiB"],
  };
  assert.match(readFileSync(new URL("./main-model.mts", import.meta.url), "utf8"),
    /MAIN_MODEL = useScheduler\("main-model", \["mainModel"\]\)\.mainModel/);
  for (const [file, keys] of Object.entries(files)) {
    const source = readFileSync(new URL(file, import.meta.url), "utf8");
    for (const key of keys) assert.match(source, new RegExp("scheduler\\." + key));
  }
  const result = spawnSync(process.execPath, ["--import", "tsx", "--input-type=module", "-e",
    'import {useScheduler} from "./.sandcastle/scheduler-config.mts"; useScheduler("fixture",["mainModel","claimPollSeconds"]);'],
    { encoding: "utf8", env: { ...process.env, CLAIM_LOOP_POLL_SECONDS: "71", SECRET_TEST: "do-not-log-secret" } });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stderr, /mainModel="gpt-6.1-sol" source=record:1/);
  assert.match(result.stderr, /claimPollSeconds=71 source=env:CLAIM_LOOP_POLL_SECONDS/);
  assert.doesNotMatch(result.stdout + result.stderr, /do-not-log-secret/);
});
test("真实 claim-loop dry-run 与集成基线 main@3a01bed557 同值等价，不写票不启动工蜂", () => fixture(path => {
  const dir = dirname(path); const bin = join(dir, "bin"); mkdirSync(bin);
  const calls = join(dir, "calls");
  writeFileSync(join(bin, "pgrep"), "#!/bin/sh\nexit 1\n", { mode: 0o755 });
  writeFileSync(join(bin, "gh"), '#!/bin/sh\necho "$*" >> "$CALLS"\ncase "$1 $2" in\n"issue list") echo \'[{"number":1606,"assignees":[]}]\';;\n"api repos/"*) echo 0;;\n*) exit 98;;\nesac\n', { mode: 0o755 });
  const baseline = readFileSync(new URL("./fixtures/issue1606/claim-loop-before.mts", import.meta.url), "utf8");
  const before = join(dir, "before.mts"); writeFileSync(before, baseline);
  for (const helper of ["drain-control.mts", "drain-control.py"]) writeFileSync(join(dir, helper), readFileSync(new URL("./" + helper, import.meta.url)));
  const env = { PATH: bin + ":" + process.env.PATH, CALLS: calls, HOME: dir };
  const run = (script: string) => {
    writeFileSync(calls, "");
    const result = spawnSync("bun", [script, "--once", "--dry-run"], { cwd: dir, env, encoding: "utf8", timeout: 10000 });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /dry-run=true poll=60s/);
    assert.match(result.stdout, /dry-run/);
    return { stdout: result.stdout, calls: readFileSync(calls, "utf8") };
  };
  assert.deepEqual(run(fileURLToPath(new URL("./claim-loop.mts", import.meta.url))), run(before));
}));
