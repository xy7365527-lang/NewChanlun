/* #1491 ADR 0037 §一：Q3 完整消费者只读 Q 投影行（s-record-order/2），行序键与快照布局无关。
 * Rust `projection.rs` 的 `record_order_key` 在投影时按此键重排快照数组；本锁防止消费者改按出生序
 * 读行而与 Rust Q 投影分叉。布局只在规则串（rule_revision）里，不是行序键的输入。 */
"use strict";
const assert = require("node:assert/strict"),
  test = require("node:test");
const Q3 = require("../browser/q3-consumer.js")({});
const { cmp, of } = Q3.V;
const tuple = (family, r) => Q3.tuple(family, of(r));
function compare(a, b) {
  for (let j = 0; j < a.length; j++) {
    const c =
      typeof a[j] === "bigint"
        ? a[j] < b[j]
          ? -1
          : a[j] > b[j]
            ? 1
            : 0
        : cmp(a[j], b[j]);
    if (c) return c;
  }
  return 0;
}
// 出生代与窗口/内容键次序相反：layout/1 与 bounded-layout/1 的快照数组次序在此必然不同。
const ACTIVE = [
  { object_id: "o-late", window_start: "0", window_mid: "1", window_end: "2", object_revision: "2", first_known_generation: "4" },
  { object_id: "o-early", window_start: "5", window_mid: "6", window_end: "7", object_revision: "1", first_known_generation: "1" },
  { object_id: "t-late", kind: "CC-004.inclusion_step", fact_key: "f-0", object_revision: "1", first_known_generation: "3" },
  { object_id: "t-early", kind: "CC-004.inclusion_step", fact_key: "f-1", object_revision: "1", first_known_generation: "1" },
];
const WITHDRAWN = [
  { object_id: "w-b", object_revision: "1", first_known_generation: "1", withdrawn_generation: "4" },
  { object_id: "w-a", object_revision: "1", first_known_generation: "2", withdrawn_generation: "3" },
];
async function recordOrder(family, rows) {
  const keyed = [];
  for (const r of rows) keyed.push({ id: r.object_id, key: await tuple(family, r) });
  keyed.sort((a, b) => compare(a.key, b.key));
  return keyed.map((x) => x.id);
}
function birthOrder(rows, generation) {
  return [...rows]
    .sort((a, b) => Number(a[generation]) - Number(b[generation]))
    .map((r) => r.object_id);
}
test("Q3 objects 行序键＝Rust record_order_key 形状，不含出生代", async () => {
  assert.deepEqual(await tuple("objects", ACTIVE[0]), [0n, 0n, 1n, 2n, "o-late", 2n]);
  assert.deepEqual(await tuple("objects", ACTIVE[2]), [1n, "CC-004.inclusion_step", "f-0", "t-late", 1n]);
  assert.deepEqual(await recordOrder("objects", ACTIVE), ["o-late", "o-early", "t-late", "t-early"]);
});
test("Q3 withdrawn_objects 行序键＝(撤回代, object_id, revision)", async () => {
  assert.deepEqual(await tuple("withdrawn_objects", WITHDRAWN[0]), [4n, "w-b", 1n]);
  assert.deepEqual(await recordOrder("withdrawn_objects", WITHDRAWN), ["w-a", "w-b"]);
});
test("负控：出生序不是 Q 行序（bounded 快照数组次序不得直接当 Q 行序）", async () => {
  const objects = await recordOrder("objects", ACTIVE);
  assert.deepEqual(birthOrder(ACTIVE, "first_known_generation"), ["o-early", "t-early", "t-late", "o-late"]);
  assert.notDeepEqual(objects, birthOrder(ACTIVE, "first_known_generation"));
  // 撤回族的 Q 行序键本就以撤回代为首；出生代序（first_known_generation）不是它。
  assert.notDeepEqual(
    await recordOrder("withdrawn_objects", WITHDRAWN),
    birthOrder(WITHDRAWN, "first_known_generation"),
  );
});
