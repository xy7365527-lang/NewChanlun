/**
 * claim-loop 纯函数测试（#1182 验收第一项）。
 * frontier 判定（isRunnable / hasRunnableIssue）逐项断言。
 * 跑法：npx tsx --test .sandcastle/claim-loop.test.mts
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { hasRunnableIssue, isRunnable } from "./claim-loop.mts";
import type { FrontierCandidate } from "./claim-loop.mts";

function cand(partial: Partial<FrontierCandidate> & { number: number }): FrontierCandidate {
  return { assignees: [], blockedBy: 0, ...partial };
}

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
