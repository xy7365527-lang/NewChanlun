import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { consumeSwarmPlan, prepareSwarmPlan, type FoldedPlan } from "./swarm-planning.mts";
import type { HostGh } from "./swarm-prompts.mts";

const rows: Array<{ number: number; title: string; body: string; labels: Array<{ name: string }>; assignees: Array<{ login: string }> }> = [
  { number: 11, title: "A", body: "改 docs/a.md，无前置。", labels: [{ name: "sandcastle" }], assignees: [] },
  { number: 12, title: "B", body: "改 docs/b.md，blocked by #11。", labels: [{ name: "sandcastle" }], assignees: [] },
];
const issue = (id: string, branch = `sandcastle/issue-${id}`) => ({ id, title: "t", branch });
function gateway(current = rows) {
  const writes: string[][] = [];
  const gh: HostGh = (args) => {
    if (args[1] === "list") return JSON.stringify(current);
    if (args[1] === "view") return JSON.stringify(current.find(r => String(r.number) === args[2]));
    writes.push(args); return "";
  };
  return { gh, writes };
}
const admit = async (claim: () => void) => claim();

test("主控拿到原 prompt 与完整正文，消费其选择而非宿主静态排序", async () => {
  const { gh, writes } = gateway();
  const prepared = prepareSwarmPlan(gh);
  assert.match(prepared.prompt, /blocked by #11/);
  assert.deepEqual(prepared.queue.map(q => q.body), rows.map(q => q.body));
  const chosen = [issue("11")];
  assert.deepEqual(await consumeSwarmPlan({ ...prepared, issues: chosen }, gh, admit), chosen);
  assert.deepEqual(writes, [["issue", "edit", "11", "--remove-label", "sandcastle", "--add-assignee", "@me"]]);
});


test("主控给定的合法顺序完整传给 claim，不静态选择第一票", async () => {
  const { gh, writes } = gateway(rows.map(r => ({ ...r, body: "无依赖且写集独立。" })));
  const prepared = prepareSwarmPlan(gh);
  const chosen = [issue("12"), issue("11")];
  assert.deepEqual(await consumeSwarmPlan({ ...prepared, issues: chosen }, gh, admit), chosen);
  assert.deepEqual(writes.map(w => w[2]), ["12", "11"]);
});

test("任一非法项拒绝整单，合法前缀不会部分认领", async () => {
  for (const bad of [issue("13"), issue("11"), issue("12", "main"), issue("11; id")]) {
    const { gh, writes } = gateway();
    const prepared = prepareSwarmPlan(gh);
    await assert.rejects(consumeSwarmPlan({ ...prepared, issues: [issue("11"), bad] }, gh, admit));
    assert.deepEqual(writes, []);
  }
});

test("fresh 正文依赖、标签或 assignee 变化在认领前拒绝", async () => {
  const prepared = prepareSwarmPlan(gateway().gh);
  for (const change of [
    { body: "现在 blocked by #99。" }, { labels: [{ name: "needs-approval" }] }, { assignees: [{ login: "owner" }] },
  ]) {
    const fresh = rows.map((r, i) => i ? r : { ...r, ...change });
    const { gh, writes } = gateway(fresh);
    await assert.rejects(consumeSwarmPlan({ ...prepared, issues: [issue("11")] }, gh, admit), /快照已变化/);
    assert.deepEqual(writes, []);
  }
});

test("等待认领门期间正文变化也在第一笔 claim 前拒绝", async () => {
  const current = rows.map(r => ({ ...r }));
  const { gh, writes } = gateway(current);
  const prepared = prepareSwarmPlan(gh);
  await assert.rejects(consumeSwarmPlan({ ...prepared, issues: [issue("11")] }, gh, async claim => {
    current[0]!.body = "blocked by #99";
    claim();
  }), /快照已变化/);
  assert.deepEqual(writes, []);
});

test("空队列和主控判定全受阻的空计划不进入认领门", async () => {
  for (const current of [[], rows]) {
    const { gh, writes } = gateway(current);
    const input = { ...prepareSwarmPlan(gh), issues: [] };
    assert.deepEqual(await consumeSwarmPlan(input, gh, async () => { throw new Error("不得进入认领门"); }), []);
    assert.deepEqual(writes, []);
  }
});

test("drain 门拒绝时零 claim，缺正文或非数组计划在边界拒绝", async () => {
  const { gh, writes } = gateway();
  const prepared = prepareSwarmPlan(gh);
  await assert.rejects(consumeSwarmPlan({ ...prepared, issues: [issue("11")] }, gh, async () => { throw new Error("frozen"); }), /frozen/);
  await assert.rejects(consumeSwarmPlan({ queue: [{ number: 11, title: "x", labels: [] }], issues: [] }, gh, admit));
  await assert.rejects(consumeSwarmPlan({ ...prepared, issues: {} }, gh, admit));
  assert.deepEqual(writes, []);
});

test("实际 main-swarm 空计划入口不启动 Docker 或 planner", () => {
  const dir = mkdtempSync(join(tmpdir(), "f3-empty-"));
  try {
    const calls = join(dir, "calls.jsonl");
    writeFileSync(join(dir, "gh"), `#!/usr/bin/env node\nconst fs = require('fs'); fs.appendFileSync(${JSON.stringify(calls)}, JSON.stringify(process.argv.slice(2))+'\\n'); console.log('[]');\n`, { mode: 0o755 });
    writeFileSync(join(dir, "docker"), `#!/bin/sh\necho docker >> ${JSON.stringify(calls)}\nexit 99\n`, { mode: 0o755 });
    const skill = readFileSync(resolve(".agents/skills/graph-run/SKILL.md"), "utf8");
    assert.match(skill, /node --import tsx \.sandcastle\/swarm-plan\.mts/);
    assert.match(skill, /SANDCASTLE_SWARM_PLAN=<仓外计划\.json> node --import tsx \.sandcastle\/main-swarm\.mts/);
    const prepared = spawnSync(process.execPath, ["--import", "tsx", resolve(".sandcastle/swarm-plan.mts")], {
      encoding: "utf8", timeout: 15000, env: { ...process.env, PATH: `${dir}:${process.env.PATH}` },
    });
    assert.equal(prepared.status, 0, prepared.stderr);
    const snapshot = JSON.parse(prepared.stdout);
    assert.deepEqual(snapshot.queue, []);
    assert.match(snapshot.prompt, /<issues-json>/);
    const plan = join(dir, "plan.json");
    writeFileSync(plan, JSON.stringify({ queue: snapshot.queue, issues: [] } satisfies FoldedPlan));
    const result = spawnSync(process.execPath, ["--import", "tsx", resolve(".sandcastle/main-swarm.mts")], {
      encoding: "utf8", timeout: 15000,
      env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, SANDCASTLE_SWARM_PLAN: plan,
        SANDCASTLE_SWARM_MERGE_MANIFEST: "", SANDCASTLE_DOCKER_ACCEPTANCE_ROW: "" },
    });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /无认领、无工蜂/);
    assert.deepEqual(readFileSync(calls, "utf8").trim().split("\n").map(s => JSON.parse(s)).map(a => a.slice(0, 2)), [["issue", "list"], ["issue", "list"]]);
    writeFileSync(plan, JSON.stringify({ queue: rows.map(r => ({ ...r, labels: ["sandcastle"] })), issues: [issue("11")] }));
    const stale = spawnSync(process.execPath, ["--import", "tsx", resolve(".sandcastle/main-swarm.mts")], {
      encoding: "utf8", timeout: 15000,
      env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, SANDCASTLE_SWARM_PLAN: plan,
        SANDCASTLE_SWARM_MERGE_MANIFEST: "", SANDCASTLE_DOCKER_ACCEPTANCE_ROW: "" },
    });
    assert.notEqual(stale.status, 0);
    assert.match(stale.stderr, /快照已变化/);
    assert.doesNotMatch(readFileSync(calls, "utf8"), /docker|"edit"/);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});
