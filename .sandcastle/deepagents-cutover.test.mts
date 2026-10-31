import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { QUEUE_LABEL, releaseLabel, withQueueAdmission } from "./deepagents-cutover.mts";
import { classifyTicket, implLabels, releaseTicket, recheckBeforeRelease, withEngineAdmission } from "../scripts/wayfinder_engine.mts";

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
  assert.deepEqual(implLabels(QUEUE_LABEL), ["ready-for-agent"]);
  assert.deepEqual(implLabels("sandcastle"), ["ready-for-agent", "sandcastle"]);
  const writes: string[] = [];
  assert.equal(releaseTicket(1, { read: () => JSON.stringify({ labels: [], assignees: [], state: "OPEN" }),
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
