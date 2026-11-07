import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { QUEUE_LABEL, releaseLabel, withQueueAdmission, graphAdmissionFrozen, withGraphAdmission } from "./deepagents-cutover.mts";
import { withFileLock, withLegacyAdmission, DrainFrozenError } from "./drain-control.mts";
import { specRulingBody, specBodyDigest, classifyTicket, releaseTicket, recheckBeforeRelease, withEngineAdmission } from "../scripts/wayfinder_engine.mts";

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "cutover-1742-"));
  const state = join(root, "exec"), drain = join(root, "drain");
  mkdirSync(state); mkdirSync(drain);
  const record = { v: 1, drain_generation: 1, frozen_at: "fixture", legacy_snapshot: [], workers_log_length: 0, workers_log_path: "/fixture/workers.jsonl",
    open_claims: [], remote_jobs_idle: true, remote_freeze_var: "1", variable_source: "user-token-readonly",
    legacy_services_unloaded: ["com.newchanlun.sandcastle-claimer", "com.newchanlun.sandcastle-harvest"],
    release_label: QUEUE_LABEL, engine_head: "a".repeat(40) };
  writeFileSync(join(state, "cutover.json"), JSON.stringify(record));
  writeFileSync(join(drain, "state.json"), JSON.stringify({ v: 1, mode: "draining", generation: 1 }));
  return { root, state, drain, record };
}

test("有效切流才选择新标签，解冻、代次变更与坏记录回到冻结的旧入口", () => {
  const f = fixture();
  try {
    assert.equal(releaseLabel(f.state, f.drain), QUEUE_LABEL);
    writeFileSync(join(f.drain, "state.json"), '{"v":1,"mode":"open","generation":1}');
    assert.equal(releaseLabel(f.state, f.drain), "sandcastle");
    writeFileSync(join(f.drain, "state.json"), '{"v":1,"mode":"draining","generation":2}');
    assert.equal(releaseLabel(f.state, f.drain), "sandcastle");
    writeFileSync(join(f.state, "cutover.json"), '{');
    assert.equal(releaseLabel(f.state, f.drain), "sandcastle");
  } finally { rmSync(f.root, { recursive: true, force: true }); }
});

test("新标签分类、复核与释放使用同一标签，旧标签不混领", () => {
  const t = { number: 1, title: "[impl] Q1", labels: ["ready-for-agent", QUEUE_LABEL], assignees: [], blockedBy: 0,
    specApproved: true, underSpec: true };
  assert.equal(classifyTicket(t, QUEUE_LABEL), "released");
  assert.equal(classifyTicket({ ...t, labels: ["sandcastle"] }, QUEUE_LABEL), "held");
  assert.match(recheckBeforeRelease({ labels: [{ name: "sandcastle" }] }, QUEUE_LABEL)!, /另一放行标签/);
  const writes: string[] = [];
  assert.equal(releaseTicket(1, { specs: [2], orchestrator: "fixture" }, { read: args => JSON.stringify(
    args.startsWith("issue view 1") ? { labels: [], assignees: [], state: "OPEN" }
      : args.startsWith("issue view 2") ? { number: 2, title: "SPEC: fixture", body: "", state: "OPEN" }
      : [{ user: { login: "fixture" }, created_at: "2026-11-07", body: specRulingBody({ specNumber: 2,
        verdict: "PASS", author: "draft", reviewer: "review", digest: specBodyDigest(""), evidence: "/fixture/review.json" }) }]),
    write: cmd => { writes.push(cmd); return ""; } }, QUEUE_LABEL), null);
  assert.deepEqual(writes, ["issue edit 1 --add-label deepagents:queue"]);
});

test("切流后在 draining 锁内能放行，resume 使新准入拒绝", async () => {
  const f = fixture();
  try {
    let writes = 0;
    await withEngineAdmission(() => { writes++; }, { stateRoot: f.state, drainRoot: f.drain });
    assert.equal(writes, 1);
    writeFileSync(join(f.drain, "state.json"), '{"v":1,"mode":"open","generation":1}');
    await assert.rejects(withQueueAdmission(() => { writes++; }, f.state, f.drain));
    assert.equal(writes, 1);
  } finally { rmSync(f.root, { recursive: true, force: true }); }
});


test("engine 与裁决面共享同一准入；切流只放行这两面、不重开旧工蜂", async () => {
  const f = fixture();
  const roots = { stateRoot: f.state, drainRoot: f.drain };
  try {
    assert.equal(withEngineAdmission, withGraphAdmission);
    assert.equal(graphAdmissionFrozen(roots), false);
    await assert.rejects(withLegacyAdmission(() => assert.fail("旧工蜂不能入"), f.drain), DrainFrozenError);
    rmSync(join(f.state, "cutover.json"));
    assert.equal(graphAdmissionFrozen(roots), true);
    await assert.rejects(withGraphAdmission(() => assert.fail("无切流不能入"), roots), DrainFrozenError);
    writeFileSync(join(f.drain, "state.json"), '{"v":1,"mode":"open","generation":1}');
    assert.equal(graphAdmissionFrozen(roots), false);
    assert.equal(await withGraphAdmission(() => "legacy-open", roots), "legacy-open");
  } finally { rmSync(f.root, { recursive: true, force: true }); }
});

test("选定切流到取得 admission 锁之间记录失效：锁内拒绝，不回退旧准入", async () => {
  const f = fixture();
  let pending!: Promise<unknown>;
  let writes = 0;
  try {
    await withFileLock(join(f.drain, "admission.lock"), async () => {
      pending = assert.rejects(withGraphAdmission(() => { writes++; },
        { stateRoot: f.state, drainRoot: f.drain }), DrainFrozenError);
      // withGraphAdmission 已同步选中 queue；子进程等锁时记录变为同一冻结下的旧代次。
      writeFileSync(join(f.state, "cutover.json"), JSON.stringify({ ...f.record, drain_generation: 2 }));
    });
    await pending;
    assert.equal(writes, 0);
  } finally { rmSync(f.root, { recursive: true, force: true }); }
});

test("orchestrator_login 不是非空字符串时 import 即失败，不静默 fail-closed", async () => {
  const { copyFileSync, readFileSync } = await import("node:fs");
  const { spawnSync } = await import("node:child_process");
  const here = new URL(".", import.meta.url).pathname;
  const policy = JSON.parse(readFileSync(join(here, "queue-policy.json"), "utf8"));
  const root = mkdtempSync(join(tmpdir(), "cutover-login-"));
  try {
    for (const name of ["deepagents-cutover.mts", "drain-control.mts", "drain-control.py"]) copyFileSync(join(here, name), join(root, name));
    const load = (login: unknown) => {
      writeFileSync(join(root, "queue-policy.json"), JSON.stringify({ ...policy, orchestrator_login: login }));
      return spawnSync(process.execPath, ["--experimental-strip-types", "--input-type=module", "--eval",
        "const m = await import('./deepagents-cutover.mts'); console.log(JSON.stringify(m.ORCHESTRATOR_LOGIN));"],
        { cwd: root, encoding: "utf8", timeout: 30_000 });
    };
    const control = load("fixture-orchestrator");
    assert.equal(control.status, 0, control.stderr);
    assert.equal(JSON.parse(control.stdout.trim()), "fixture-orchestrator");
    for (const login of [undefined, null, "", 123, ["xy7365527-lang"]]) {
      const result = load(login);
      assert.notEqual(result.status, 0, `login=${JSON.stringify(login)} 应启动即失败`);
      assert.match(result.stderr, /orchestrator_login/);
    }
  } finally { rmSync(root, { recursive: true, force: true }); }
});
