"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const J = require("../browser/s-content-job-v3.js");
const FIELDS = ["max_total_wire_bytes", "max_total_node_bytes", "max_total_nodes",
  "max_total_records", "max_total_requests", "max_elapsed_ms", "max_staged_bytes", "max_store_bytes"];
const limits = (overrides = {}) => ({...Object.fromEntries(FIELDS.map(k => [k, "100"])), ...overrides});
const code = expected => error => error.code === expected;

test("八字段必须 exact，拒绝数字、缺项与额外项", () => {
  const missing = limits(); delete missing.max_elapsed_ms;
  for (const input of [missing, {...limits(), totalBytes: "10"}, limits({max_total_nodes: 10}), null])
    assert.throws(() => J.parseLimits(input), code("job_limit_field"));
});
test("八字段规范正十进制和 i64 边界", () => {
  for (const value of ["0", "-1", "+1", "01", " 1", "1.0", "", "9223372036854775808"])
    assert.throws(() => J.parseLimits(limits({max_total_nodes: value})), code("job_limit_domain"));
  const parsed = J.parseLimits(limits({max_total_nodes: "9223372036854775807"}));
  assert.equal(parsed.max_total_nodes, "9223372036854775807");
  assert.ok(Object.isFrozen(parsed));
});
const config = () => ({schema_revision: "s-query-client-resources/3", pageSize: 31,
  watchBatches: 4, pollMs: 500, timeoutMs: 30000, replyBytes: 100, job_limits: limits()});
test("浏览器配置没有旧默认或旧累计字段，控制项显式规范正整数", () => {
  for (const invalid of [{...config(), totalBytes: 4}, {...config(), maxRows: 10},
    {...config(), schema_revision: "s-query-client-resources/2"}, {...config(), pollMs: 0},
    {...config(), pollMs: "500"}, {...config(), pageSize: 1.5}, {...config(), replyBytes: Infinity}])
    assert.throws(() => J.validateConfig(invalid));
  for (const key of ["pageSize", "watchBatches", "pollMs", "timeoutMs", "replyBytes", "job_limits"]) {
    const c = config(); delete c[key]; assert.throws(() => J.validateConfig(c));
  }
  const input = config(), frozen = J.validateConfig(input);
  input.job_limits.max_total_nodes = "1";
  assert.equal(frozen.job_limits.max_total_nodes, "100");
  assert.ok(Object.isFrozen(frozen) && Object.isFrozen(frozen.job_limits));
});
// 全部空间例子均为 TestOnly 合成账，不能证明任何平台的物理分配。
const existing = (store = "10", staged = "2") => ({store_bytes: store, staged_bytes: staged});
function job(overrides = {}, baseline = existing()) {
  let now = 0n;
  return {j: new J.Job(limits(overrides), baseline, () => now), at: n => {now = BigInt(n);}};
}
test("缺少测量基线、子集关系和既存超限均在构造时拒绝", () => {
  for (const e of [undefined, {}, existing("1", "2"), existing("-1", "0"), existing(0, 0)])
    assert.throws(() => new J.Job(limits(), e, () => 0n));
  assert.throws(() => job({}, existing("101", "0")), code("job_limit_exceeded"));
  assert.throws(() => job({max_staged_bytes: "1"}), code("job_limit_exceeded"));
});
test("实际失败读、重试与祖先重传永久全额计入，超额也不丢尾量", () => {
  const {j} = job({max_total_wire_bytes: "12"});
  j.chargeRequests("1"); j.chargeWireBytes("5");
  j.chargeRequests("1"); j.chargeWireBytes("5");
  assert.throws(() => j.chargeWireBytes("5"), code("job_limit_exceeded"));
  assert.equal(j.snapshot().wire_bytes, "15");
  assert.equal(j.snapshot().requests, "2");
  assert.throws(() => j.chargeWireBytes("1"), code("job_limit_exceeded"));
  assert.equal(j.snapshot().wire_bytes, "15");
});
test("记录重访和请求尝试已发生量先入账再核 cap", () => {
  for (const [method, field, cap] of [["chargeRecords", "records", "max_total_records"],
    ["chargeRequests", "requests", "max_total_requests"]]) {
    const {j} = job({[cap]: "3"}); j[method]("2");
    assert.throws(() => j[method]("2"), code("job_limit_exceeded"));
    assert.equal(j.snapshot()[field], "4");
  }
});
test("首次读取幂等，排队不续期，期限边界为半开，实际尾量仍入账", () => {
  const {j, at} = job({max_elapsed_ms: "10"});
  at(20); j.beginFirstRead(); assert.equal(j.firstReadDeadline(), "30");
  at(29); j.beginFirstRead(); assert.equal(j.firstReadDeadline(), "30");
  at(30); assert.throws(() => j.chargeWireBytes("7"), code("job_deadline"));
  assert.equal(j.snapshot().wire_bytes, "7");
  assert.equal(j.snapshot().elapsed_ms, "10");
  assert.equal(j.snapshot().stopped, "job_deadline");
});
test("首次请求之前起钟；finish/install 门核期限；取消原因永久", () => {
  const {j, at} = job({max_elapsed_ms: "10"});
  j.chargeRequests("1"); at(10);
  assert.throws(() => j.finishCheck(), code("job_deadline"));
  const {j: cancelled} = job(); cancelled.cancel();
  assert.equal(cancelled.snapshot().stopped, "job_cancelled");
  assert.throws(() => cancelled.beginFirstRead(), code("job_cancelled"));
  cancelled.cancel(); assert.equal(cancelled.snapshot().stopped, "job_cancelled");
});
test("实际累计越过 u64 不溢出，快照深冻结且不暴露内部可变账", () => {
  const {j} = job({max_total_wire_bytes: "9223372036854775807"});
  assert.throws(() => j.chargeWireBytes("18446744073709551620"), code("job_limit_exceeded"));
  const s = j.snapshot(); assert.equal(s.wire_bytes, "18446744073709551620");
  assert.ok(Object.isFrozen(s) && Object.isFrozen(s.limits) && Object.isFrozen(s.store));
  assert.throws(() => {s.store.baseline = "0";});
  assert.equal(j.snapshot().store.baseline, "10");
});
const fs = require("node:fs"), path = require("node:path"), vm = require("node:vm"), crypto = require("node:crypto");
const vectors = JSON.parse(fs.readFileSync(path.join(__dirname,
  "../../.chanlun/review-results/issue1394-content-v3-20260916/inputs/content-format-vectors.json")));
const nullNode = vectors.nodes.find(n => n.tag === "00");
const boolNode = vectors.nodes.find(n => n.tag === "01");
// Rust DOMAIN_SEPARATOR = b"NewChanlun/SContent/1\0"（22 字节），空体 frame 的独立真值为 31。
test("固定向量空节点的 frame 为 31 字节；本地复用每 job 计一次，坏节点重访始终拒绝", async () => {
  const {j} = job();
  assert.equal(await j.acceptNode(nullNode), "first_accept");
  assert.equal(await j.acceptNode({...nullNode}), "already_accepted");
  assert.equal(j.snapshot().nodes, "1"); assert.equal(j.snapshot().node_bytes, "31");
  for (let i = 0; i < 2; i++)
    await assert.rejects(j.acceptNode({...nullNode, body_hex: "00"}), code("node_digest"));
  assert.equal(j.snapshot().nodes, "1");
  const {j: localReuse} = job(); await localReuse.acceptNode(nullNode);
  assert.equal(localReuse.snapshot().node_bytes, "31");
});
test("节点额度是接纳前的双门，被拒根不计 node 账，真实 wire 独立保留", async () => {
  for (const overrides of [{max_total_nodes: "1"}, {max_total_node_bytes: "61"}]) {
    const {j} = job(overrides); j.chargeWireBytes("80"); await j.acceptNode(nullNode);
    await assert.rejects(j.acceptNode(boolNode), code("job_limit_exceeded"));
    assert.equal(j.snapshot().node_bytes, "31"); assert.equal(j.snapshot().nodes, "1");
    assert.equal(j.snapshot().wire_bytes, "80");
  }
});
function browserAPI(subtle = crypto.webcrypto.subtle) {
  const context = vm.createContext({TextEncoder, TextDecoder, Uint8Array, crypto: {subtle}});
  for (const name of ["s-content-v3.js", "s-content-job-v3.js"])
    vm.runInContext(fs.readFileSync(path.join(__dirname, "../browser", name), "utf8"), context);
  assert.equal(vm.runInContext("typeof require + '/' + typeof module + '/' + typeof Buffer", context),
    "undefined/undefined/undefined");
  return context.SContentJobV3;
}
test("Browser WebCrypto 无 Node globals，与 CommonJS 相同基线、节点与累计真值", async () => {
  const B = browserAPI(), bj = new B.Job(limits(), existing(), () => 0n), {j} = job();
  for (const x of [j, bj]) {x.chargeRecords("3"); x.chargeRequests("2");
    x.chargeWireBytes("90"); await x.acceptNode(nullNode);}
  assert.equal(JSON.stringify(bj.snapshot()), JSON.stringify(j.snapshot()));
});
test("异步 hash 期间原件变化不改变接纳；取消/过期不被晚到结果覆盖", async () => {
  let resume;
  const B = browserAPI({digest: (_algo, bytes) => new Promise(resolve => {
    resume = async () => resolve(await crypto.webcrypto.subtle.digest("SHA-256", bytes));
  })});
  const b = new B.Job(limits(), existing(), () => 0n), input = {...nullNode};
  const accepted = b.acceptNode(input); input.root = boolNode.root; input.tag = "01";
  await resume(); assert.equal(await accepted, "first_accept");
  assert.equal(await (async () => {const p = b.acceptNode(nullNode); await resume(); return p;})(), "already_accepted");
  for (const reason of ["cancel", "deadline"]) {
    let now = 0n; const x = new B.Job(limits({max_elapsed_ms: "10"}), existing(), () => now);
    const pending = x.acceptNode(nullNode);
    if (reason === "cancel") x.cancel(); else now = 10n;
    await resume(); await assert.rejects(pending, code(reason === "cancel" ? "job_cancelled" : "job_deadline"));
    assert.equal(x.snapshot().nodes, "0");
  }
});
test("暂存预留任一本 cap 不足即停止且双账原子，无部分 pending 或 peak", () => {
  for (const overrides of [{max_staged_bytes: "5"}, {max_store_bytes: "13"}]) {
    const {j} = job(overrides);
    assert.throws(() => j.reserveStaged("4"), code("job_limit_exceeded"));
    const s = j.snapshot();
    assert.equal(s.staged.pending, "0"); assert.equal(s.store.pending, "0");
    assert.equal(s.staged.peak, "2"); assert.equal(s.store.peak, "10");
    assert.equal(s.open_reservations, "0");
  }
});
test("暂存子集只在总账计一次，写后对账和实际删除不回退 peak 或累计账", () => {
  const {j} = job(); j.chargeWireBytes("20"); j.chargeRecords("3"); j.chargeRequests("2");
  const g = j.reserveStaged("6");
  assert.ok(Object.isFrozen(g)); assert.equal(g.kind, "staged"); assert.equal(g.requested, "6");
  assert.equal(j.snapshot().staged.exposure, "8"); assert.equal(j.snapshot().store.exposure, "16");
  j.reconcile(g, "4"); j.reclaimStaged("3");
  const s = j.snapshot();
  assert.deepEqual(s.staged, {baseline: "2", committed: "1", pending: "0", exposure: "3", peak: "8"});
  assert.deepEqual(s.store, {baseline: "10", committed: "0", pending: "0", exposure: "11", peak: "16"});
  assert.equal(s.wire_bytes, "20"); assert.equal(s.records, "3"); assert.equal(s.requests, "2");
});
test("保留的 index/log 只占总账，不提供任意历史回收或 baseline 改写", () => {
  const {j} = job(); const g = j.reserveStore("6"); j.reconcile(g, "4");
  assert.equal(j.snapshot().staged.exposure, "2"); assert.equal(j.snapshot().store.exposure, "14");
  assert.throws(() => j.reclaimStaged("1"), code("job_reclaim"));
  assert.equal(j.snapshot().store.committed, "4");
  assert.equal(j.reclaimStore, undefined); assert.equal(j.reset, undefined);
});
test("实际量超预留照实进账/peak 后停；停止后旧 grant 对账、释放与自身暂存回收仍可", () => {
  const {j} = job();
  const a = j.reserveStaged("4"), b = j.reserveStaged("3"), c = j.reserveStore("2");
  assert.throws(() => j.reconcile(a, "18446744073709551620"), code("job_reconcile"));
  assert.equal(j.snapshot().staged.exposure, "18446744073709551625");
  assert.equal(j.snapshot().store.exposure, "18446744073709551635");
  j.reconcile(b, "2"); j.release(c); j.reclaimStaged("18446744073709551622");
  const s = j.snapshot();
  assert.equal(s.staged.exposure, "2"); assert.equal(s.store.exposure, "10");
  assert.equal(s.staged.peak, "18446744073709551625");
  assert.equal(s.store.peak, "18446744073709551635");
  assert.equal(s.stopped, "job_reconcile");
  assert.throws(() => j.reserveStaged("1"), code("job_reconcile"));
});
test("伪造、跨 job、已释放/已对账句柄不能关闭合法 owner 的预留", () => {
  const {j} = job(), {j: other} = job(), grant = j.reserveStaged("4");
  for (const candidate of [{...grant}, {}, Object.create(grant)])
    assert.throws(() => j.release(candidate), code("job_reservation"));
  assert.throws(() => other.reconcile(grant, "3"), code("job_reservation"));
  assert.equal(j.snapshot().open_reservations, "1");
  assert.throws(() => j.reconcile(grant, "-1"));
  assert.equal(j.snapshot().open_reservations, "1");
  j.reconcile(grant, "3");
  assert.throws(() => j.reconcile(grant, "2"), code("job_reservation"));
  assert.throws(() => j.release(grant), code("job_reservation"));
  const released = j.reserveStore("2"); j.release(released);
  assert.throws(() => j.reconcile(released, "1"), code("job_reservation"));
});
test("finish 不能在 open grant 时安装；取消后 release 不减永久累计", () => {
  const {j} = job(); j.chargeWireBytes("7"); const g = j.reserveStaged("4");
  assert.throws(() => j.finishCheck(), code("job_reservation"));
  assert.equal(j.snapshot().stopped, null);
  j.cancel(); j.release(g);
  assert.equal(j.snapshot().wire_bytes, "7"); assert.equal(j.snapshot().store.exposure, "10");
  assert.equal(j.snapshot().store.peak, "14");
  assert.throws(() => j.finishCheck(), code("job_cancelled"));
});

for (const [method, field] of [["chargeWireBytes", "wire_bytes"],
  ["chargeRecords", "records"], ["chargeRequests", "requests"]]) {
  for (const clockFault of ["rollback", "invalid"]) {
    test(`${field}：时钟 ${clockFault} 停止前永久记录已发生实际量`, () => {
      let now = 10n;
      const j = new J.Job(limits(), existing(), () => now);
      j[method]("3");
      now = clockFault === "rollback" ? 9n : "not-decimal";
      assert.throws(() => j[method]("7"), code("job_clock"));
      now = 10n;
      assert.equal(j.snapshot()[field], "10");
      assert.equal(j.snapshot().stopped, "job_clock");
      assert.throws(() => j[method]("1"), code("job_clock"));
      assert.equal(j.snapshot()[field], "10");
      assert.throws(() => j.preIoCheck(), code("job_clock"));
    });
  }
}

test("时钟回调取消后所有准入拒绝，已发生 actual 留账但 node/space 不得接纳", async () => {
  for (const method of ["beginFirstRead", "firstReadDeadline", "checkDeadline", "preIoCheck", "finishCheck",
    "reserveStaged", "reserveStore", "chargeWireBytes", "chargeRecords", "chargeRequests", "acceptNode"]) {
    let j, calls = 0;
    j = new J.Job(limits(), existing(), () => {
      if (++calls === (method === "acceptNode" ? 2 : 1)) j.cancel();
      return 0n;
    });
    await assert.rejects(async () => j[method](method === "acceptNode" ? nullNode : "7"), code("job_cancelled"));
    const s = j.snapshot();
    assert.equal(s.stopped, "job_cancelled"); assert.equal(s.nodes, "0");
    assert.equal(s.open_reservations, "0"); assert.equal(s.store.pending, "0"); assert.equal(s.staged.pending, "0");
    for (const [charge, field] of [["chargeWireBytes", "wire_bytes"], ["chargeRecords", "records"], ["chargeRequests", "requests"]])
      assert.equal(s[field], method === charge ? "7" : "0");
  }
});
test("clock 回调中的重入预留即使被回调捕获也永久拒绝，不让 finish 安装", () => {
  let j, enabled = false, nestedError;
  j = new J.Job(limits(), existing(), () => {
    if (enabled) {
      enabled = false;
      try { j.reserveStore("7"); } catch (e) {nestedError = e.code;}
    }
    return 0n;
  });
  j.beginFirstRead(); enabled = true;
  assert.throws(() => j.finishCheck(), code("job_clock"));
  assert.equal(nestedError, "job_clock"); assert.equal(j.snapshot().open_reservations, "0");
});
test("clock throw 永久停止三个实际入口，保留超 cap 实量并允许回调结束后清理", () => {
  for (const [method, field, cap] of [["chargeWireBytes", "wire_bytes", "max_total_wire_bytes"],
    ["chargeRecords", "records", "max_total_records"], ["chargeRequests", "requests", "max_total_requests"]]) {
    let broken = false;
    const j = new J.Job(limits({[cap]: "9"}), existing(), () => {
      if (broken) throw new Error("backend failed"); return 10n;
    });
    const g = j.reserveStaged("4"); j[method]("3"); broken = true;
    assert.throws(() => j[method]("7"), code("job_clock")); broken = false;
    assert.equal(j.snapshot()[field], "10"); assert.equal(j.snapshot().stopped, "job_clock");
    assert.throws(() => j.preIoCheck(), code("job_clock")); assert.throws(() => j.finishCheck(), code("job_clock"));
    j.reconcile(g, "3"); j.reclaimStaged("3");
    assert.equal(j.snapshot().store.exposure, "10"); assert.equal(j.snapshot().stopped, "job_clock");
  }
});
test("clock 回调中的快照/清理重入也被统一阻断，原句柄仍可在停止后正常关闭", () => {
  for (const action of ["snapshot", "release", "reconcile", "reclaimStaged"]) {
    let j, enabled = false, grant;
    j = new J.Job(limits(), existing(), () => {
      if (enabled) {enabled = false;
        if (action === "snapshot") j.snapshot();
        else if (action === "reclaimStaged") j.reclaimStaged("0");
        else j[action](grant, "1");
      }
      return 0n;
    });
    grant = j.reserveStaged("3"); enabled = true;
    assert.throws(() => j.preIoCheck(), code("job_clock"));
    assert.equal(j.snapshot().open_reservations, "1"); j.release(grant);
    assert.equal(j.snapshot().open_reservations, "0");
  }
});
test("config/schema/control/limits/existing 各字段只捕获一次，冻结只含已验证值", () => {
  const input = config(), foreign = {mutable: true}; let schemaReads = 0, controlReads = 0;
  Object.defineProperty(input, "schema_revision", {enumerable: true, get() {
    return ++schemaReads === 1 ? "s-query-client-resources/3" : foreign;
  }});
  Object.defineProperty(input, "pageSize", {enumerable: true, get() {return ++controlReads === 1 ? 31 : foreign;}});
  const result = J.validateConfig(input); foreign.mutable = false;
  assert.equal(result.schema_revision, "s-query-client-resources/3"); assert.equal(result.pageSize, 31);
  assert.equal(schemaReads, 1); assert.equal(controlReads, 1);
  let limitReads = 0, baselineReads = 0; const l = limits(), e = existing();
  Object.defineProperty(l, "max_total_nodes", {enumerable: true, get() {return ++limitReads === 1 ? "100" : "bad";}});
  Object.defineProperty(e, "store_bytes", {enumerable: true, get() {return ++baselineReads === 1 ? "10" : "bad";}});
  const j = new J.Job(l, e, () => 0n);
  assert.equal(j.snapshot().limits.max_total_nodes, "100"); assert.equal(j.snapshot().store.baseline, "10");
  assert.equal(limitReads, 1); assert.equal(baselineReads, 1);
});
