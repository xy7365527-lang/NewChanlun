/* #1394 整合（根裁定 (4)，C-2 裁定 1）：v3 validate-once 锁单独成套，CI s-observer 执行；完整客户端套件
 * test_q3_complete_client.cjs 整体约 10.5 分钟不进 CI。夹具与该套件共用 q3_complete_fixture.cjs。 */
"use strict";
const assert = require("node:assert/strict"),
  test = require("node:test");
const {
  V,
  CONFIG,
  measurement,
  makeServer,
  makeS16Base,
  discovery,
} = require("./q3_complete_fixture.cjs");
// #1394 整合（根裁定 (4)）：v3 与 v2 共用同一 validate-once 机制。新载两份客户端：A 跑真实 Client 采集，
// oracle 是空缓存实例，在同一 state 上做一次完整验证，取首验的 check/charge 次数；都不受本文件其他用例的缓存影响。
test("v3同一snapshot只完整验证一次：Client内部与页面钩子合计一次，重载命中重放记录计费，内容变化必重验", async () => {
  const freshClient = () => {
    const key = require.resolve("../browser/tb01c-client.js"),
      saved = require.cache[key];
    delete require.cache[key];
    try {
      return require("../browser/tb01c-client.js");
    } finally {
      require.cache[key] = saved;
    }
  };
  const A = freshClient(),
    oracle = freshClient();
  const server = await makeServer(await makeS16Base("3"));
  const hooked = [];
  const make = () =>
    new A.Client({
      protocol: "s-session/3",
      fetch: server.fetch,
      limits: CONFIG,
      allocation: measurement(),
      hooks: {
        validateState: async (state, protocol) => {
          let checks = 0;
          await A.Q3.validateState(state, protocol, () => checks++);
          const full = { checks: 0, charges: 0 };
          if (hooked.length === 0)
            await oracle.Q3.validateState(
              state,
              protocol,
              () => full.checks++,
              () => full.charges++,
            );
          hooked.push({ state, checks, full });
        },
      },
    });
  const first = make();
  await first.load(discovery(server), "AsKnown", "3");
  assert.equal(hooked.length, 1);
  const full = hooked[0].full;
  assert(full.checks > 1 && full.charges > 0, "空缓存实例必须逐条走完 state");
  assert.equal(hooked[0].checks, 1, "Client 内部已完整验证，页面钩子不得再验");
  const second = make();
  await second.load(discovery(server), "AsKnown", "3");
  assert.equal(hooked[1].checks, 1, "同一 cut 重载按内容命中");
  assert.equal(
    second.lastJob.snapshot().records,
    first.lastJob.snapshot().records,
    "命中按首验次数重放记录计费",
  );
  const state = hooked[0].state;
  let checks = 0,
    charges = 0;
  await A.Q3.validateState(
    state,
    "s-session/3",
    () => checks++,
    () => charges++,
  );
  assert.deepEqual({ checks, charges }, { checks: 1, charges: full.charges });
  let expired = 0;
  await assert.rejects(
    () =>
      A.Q3.validateState(state, "s-session/3", () => {
        expired++;
        throw new Error("读取期限已过");
      }),
    /读取期限已过/,
  );
  assert.equal(expired, 1, "命中仍守读取期限");
  // 内容变化：一条对象换成另一条同 kind、同 extent 的对象——普通字段与容器形状一字不变，只有 Reader
  // 内容根不同；键若不含 root 就会误命中。
  const plain = state.plain,
    snap = plain.snapshot.plain,
    objects = snap.objects;
  let pair = null;
  for (let i = 0; i < objects.length && !pair; i++)
    for (let j = i + 1; j < objects.length && !pair; j++)
      if (
        objects[i].ref.kind === objects[j].ref.kind &&
        objects[i].ref.extent === objects[j].ref.extent &&
        objects[i].ref.root !== objects[j].ref.root
      )
        pair = [i, j];
  assert(pair, "夹具须有同 kind、同 extent、内容不同的两条对象");
  const replaced = objects.slice();
  replaced[pair[0]] = objects[pair[1]];
  const changed = V.of({
    ...plain,
    snapshot: V.of({ ...snap, objects: replaced }),
  });
  for (let round = 0; round < 2; round++) {
    let n = 0;
    await assert.rejects(
      () => A.Q3.validateState(changed, "s-session/3", () => n++),
      Error,
      "内容变化必重验并拒绝，失败不缓存（第" + (round + 1) + "次）",
    );
    assert(n > 1, "内容变化必须重新逐条验证（实际 check " + n + " 次）");
  }
});
