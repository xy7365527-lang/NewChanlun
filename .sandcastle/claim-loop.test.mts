/**
 * claim-loop 纯函数测试（#1182 验收第一项 + #1259 回滚判定）。
 * frontier 判定（isRunnable / hasRunnableIssue）与认领回滚判定（needsRollback / findStaleClaims）逐项断言。
 * 跑法：npx tsx --test .sandcastle/claim-loop.test.mts
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { findStaleClaims, hasRunnableIssue, isRunnable, needsRollback } from "./claim-loop.mts";
import type { FrontierCandidate, WorkerEvent } from "./claim-loop.mts";

function cand(partial: Partial<FrontierCandidate> & { number: number }): FrontierCandidate {
  return { assignees: [], blockedBy: 0, ...partial };
}

function ev(partial: WorkerEvent): WorkerEvent {
  return partial;
}

// 固定「现在」，使年龄门槛断言不受真实时钟影响。
const NOW = Date.parse("2026-08-25T12:00:00Z");
const FIFTEEN_MIN = 15 * 60 * 1000;

test("isRunnable：未 assign 且无 open blocker 才可拾取", () => {
  assert.equal(isRunnable(cand({ number: 1 })), true);
  assert.equal(isRunnable(cand({ number: 2, assignees: ["xy7365527-lang"] })), false);
  assert.equal(isRunnable(cand({ number: 3, blockedBy: 1 })), false);
  assert.equal(isRunnable(cand({ number: 4, assignees: ["x"], blockedBy: 1 })), false);
});

test("hasRunnableIssue：空队列 / 全 assign / 全阻塞 均不可拾取", () => {
  assert.equal(hasRunnableIssue([]), false);
  assert.equal(hasRunnableIssue([cand({ number: 1, assignees: ["x"] })]), false);
  assert.equal(hasRunnableIssue([cand({ number: 1, blockedBy: 1 })]), false);
  assert.equal(
    hasRunnableIssue([cand({ number: 1, assignees: ["x"] }), cand({ number: 2, blockedBy: 2 })]),
    false,
  );
});

test("hasRunnableIssue：至少一张合资格即 true（与顺序无关）", () => {
  assert.equal(
    hasRunnableIssue([cand({ number: 1, assignees: ["x"] }), cand({ number: 2, blockedBy: 1 }), cand({ number: 3 })]),
    true,
  );
  assert.equal(
    hasRunnableIssue([cand({ number: 3 }), cand({ number: 1, assignees: ["x"] })]),
    true,
  );
});


test("needsRollback：最后事件=claim/claimed 且无 implementer started 才需回滚", () => {
  // 卡死形态：claim 后无任何后续事件 → 回滚
  assert.equal(needsRollback([ev({ ticket: 1, phase: "claim", status: "claimed" })], 1, { now: NOW }), true);
  // 反向判据：有 implementer started（含 orphan 工蜂残局）→ 不回滚
  assert.equal(
    needsRollback(
      [
        ev({ ticket: 1, phase: "claim", status: "claimed" }),
        ev({ ticket: 1, phase: "implementer", status: "started" }),
      ],
      1,
      { now: NOW },
    ),
    false,
  );
  // 已回滚过（最后事件 = rolled_back）→ 不重复回滚
  assert.equal(
    needsRollback(
      [
        ev({ ticket: 1, phase: "claim", status: "claimed" }),
        ev({ ticket: 1, phase: "claim", status: "rolled_back" }),
      ],
      1,
      { now: NOW },
    ),
    false,
  );
  // 无该票事件 → 不回滚
  assert.equal(needsRollback([], 1, { now: NOW }), false);
});

test("needsRollback：minAgeMs 门槛区分主路径（0）与看门狗（15min）", () => {
  const fresh = ev({ ts: "2026-08-25T11:59:00Z", ticket: 2, phase: "claim", status: "claimed" });
  const stale = ev({ ts: "2026-08-25T11:40:00Z", ticket: 2, phase: "claim", status: "claimed" });
  // 看门狗：未满 15min 不回滚，超时回滚
  assert.equal(needsRollback([fresh], 2, { minAgeMs: FIFTEEN_MIN, now: NOW }), false);
  assert.equal(needsRollback([stale], 2, { minAgeMs: FIFTEEN_MIN, now: NOW }), true);
  // 主路径（minAgeMs=0）：立即回滚，不看年龄
  assert.equal(needsRollback([fresh], 2, { minAgeMs: 0, now: NOW }), true);
});

test("findStaleClaims：跨票聚合、跳过已开工票、分支名取 claim 事件记录", () => {
  const events = [
    ev({ ts: "2026-08-25T11:00:00Z", ticket: 1, branch: "sandcastle/issue-1", phase: "claim", status: "claimed" }),
    ev({ ts: "2026-08-25T11:05:00Z", ticket: 1, phase: "implementer", status: "started" }),
    ev({ ts: "2026-08-25T11:10:00Z", ticket: 2, branch: "sandcastle/issue-2", phase: "claim", status: "claimed" }),
  ];
  assert.deepEqual(findStaleClaims(events, { now: NOW }), [{ ticket: 2, branch: "sandcastle/issue-2" }]);
});

test("findStaleClaims：claim 事件缺 branch 时按确定性分支名补齐", () => {
  const events = [ev({ ts: "2026-08-25T11:00:00Z", ticket: 3, phase: "claim", status: "claimed" })];
  assert.deepEqual(findStaleClaims(events, { now: NOW }), [{ ticket: 3, branch: "sandcastle/issue-3" }]);
});
