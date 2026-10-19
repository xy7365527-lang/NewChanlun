/**
 * health-monitor 阈值判定纯函数单测（#1547）。
 * 跑法：npx tsx --test .sandcastle/health-monitor.test.ts
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  extractAccessKey,
  findStaleClaims,
  judgeClaimLoop,
  judgeDisk,
  judgeFreshness,
  overall,
  parseDfAvailGiB,
} from "./health-monitor.mts";

const NOW = Date.parse("2026-10-19T12:00:00Z");
const MIN = 60 * 1000;
const HOUR = 60 * MIN;

test("judgeFreshness：10 分钟内 healthy，超过即 red，缺日志 red", () => {
  assert.equal(judgeFreshness(NOW - 9 * MIN, NOW), "healthy");
  assert.equal(judgeFreshness(NOW - 10 * MIN, NOW), "healthy");
  assert.equal(judgeFreshness(NOW - 10 * MIN - 1, NOW), "red");
  assert.equal(judgeFreshness(null, NOW), "red");
});

test("judgeClaimLoop：进程不在 → stopped", () => {
  assert.equal(judgeClaimLoop({ running: false, stderrTail: "", stderrMtimeMs: 0, stdoutMtimeMs: 0 }), "stopped");
});

test("judgeClaimLoop：stderr 更新且末行 fail-closed → fail_closed", () => {
  const tail = "x\n[claim-loop] automation host 非干净 main（branch=main, dirty=true）→ fail-closed\n";
  assert.equal(judgeClaimLoop({ running: true, stderrTail: tail, stderrMtimeMs: NOW, stdoutMtimeMs: NOW - HOUR }), "fail_closed");
});

test("judgeClaimLoop：fail-closed 已被后续 stdout 轮次覆盖 → ok", () => {
  const tail = "[claim-loop] ... → fail-closed\n";
  assert.equal(judgeClaimLoop({ running: true, stderrTail: tail, stderrMtimeMs: NOW - HOUR, stdoutMtimeMs: NOW }), "ok");
  assert.equal(judgeClaimLoop({ running: true, stderrTail: "其他错误\n", stderrMtimeMs: NOW, stdoutMtimeMs: 0 }), "ok");
});

test("findStaleClaims：非终态且 >48h 才算僵尸", () => {
  const ev = [
    { ts: new Date(NOW - 49 * HOUR).toISOString(), ticket: 1, branch: "b1", phase: "claim", status: "claimed" },
    { ts: new Date(NOW - 49 * HOUR).toISOString(), ticket: 1, phase: "implementer", status: "started" },
    { ts: new Date(NOW - 47 * HOUR).toISOString(), ticket: 2, phase: "claim", status: "claimed" },
    { ts: new Date(NOW - 100 * HOUR).toISOString(), ticket: 3, phase: "claim", status: "claimed" },
    { ts: new Date(NOW - 99 * HOUR).toISOString(), ticket: 3, phase: "sandbox", status: "closed" },
    { ts: new Date(NOW - 100 * HOUR).toISOString(), ticket: 4, phase: "claim", status: "rolled_back" },
  ];
  const s = findStaleClaims(ev, { now: NOW });
  assert.deepEqual(s.map((x) => x.ticket), [1]);
  assert.equal(s[0]!.branch, "b1");
  assert.equal(s[0]!.lastPhase, "implementer/started");
  assert.equal(s[0]!.ageHours, 49);
});

test("findStaleClaims：main.mts 在跑时不判僵尸；缺 branch 用确定性分支名", () => {
  const ev = [{ ts: new Date(NOW - 72 * HOUR).toISOString(), ticket: 9, phase: "implementer", status: "started" }];
  assert.deepEqual(findStaleClaims(ev, { now: NOW, mainRunning: true }), []);
  assert.equal(findStaleClaims(ev, { now: NOW })[0]!.branch, "sandcastle/issue-9");
});

test("parseDfAvailGiB / judgeDisk：100 GiB 门", () => {
  const df = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk5s1 976490568 877000000 98566144 90% /Volumes/AgentStorage\n";
  assert.equal(parseDfAvailGiB(df), 94);
  assert.equal(parseDfAvailGiB("garbage"), null);
  assert.equal(judgeDisk(94), "red");
  assert.equal(judgeDisk(100), "healthy");
  assert.equal(judgeDisk(null), "red");
});

test("extractAccessKey：只取 access.api-keys 首项，不误取 upstream api-keys", () => {
  const yaml = "server:\n    port: 8318\naccess:\n    api-keys:\n        - sk-local-abc\n        - sk-two\napi-keys:\n    claude:\n        - api-key: upstream\n";
  assert.equal(extractAccessKey(yaml), "sk-local-abc");
  assert.equal(extractAccessKey("api-keys:\n    claude: []\n"), null);
});

test("overall：任一 red 即 red", () => {
  assert.equal(overall([{ name: "a", status: "healthy", message: "" }]), "healthy");
  assert.equal(overall([{ name: "a", status: "healthy", message: "" }, { name: "b", status: "red", message: "" }]), "red");
});
