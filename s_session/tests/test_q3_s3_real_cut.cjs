/* #1419/#1424：Q3 沿原公开 Client 入口消费真实 S3 W 写者多代导出（27AZ P1）。
 * 夹具 fixtures/q3-real-s3-w-recover-revise.json 由 Rust
 * v3::financial_session::tests::export_real_s3_w_fixture_for_q3 真实 init_v3 + 接纳/Begin/提交逐代导出，
 * provenance 记 head、命令与逐里程碑的 Rust 侧当前对象计数；本套以该计数为对拍 oracle，不另建读链。
 * 覆盖：cut0 首根、W 等待期两 kind、方向确立、真实写者恢复（纪元 1→2）跨代 Watch、首中枢、
 * 改价修订撤回与改回恢复两代 Watch、尾段中枢对。 */
"use strict";
const assert = require("node:assert/strict"),
  test = require("node:test"),
  fs = require("node:fs");
const { API, C, V, rebuild, makeServer, makeS16Base, makeClient, discovery } = require("./q3_complete_fixture.cjs");

const FILE = "q3-real-s3-w-recover-revise.json";
const data = JSON.parse(fs.readFileSync(__dirname + "/fixtures/" + FILE));
const steps = Object.fromEntries(data.provenance.steps.map((s) => [s.step, s]));
const gen = (label) => steps[label].generation;

async function kindsOf(client) {
  const kinds = {};
  for (const r of client.committed.families.objects) {
    const k = await V.S(await r.field("kind"));
    kinds[k] = String(BigInt(kinds[k] || 0) + 1n);
  }
  return kinds;
}
// Rust 侧同代 s_content_objects 当前对象逐 kind 计数（数值转十进制串以便深比较）。
const rustKinds = (label) =>
  Object.fromEntries(Object.entries(steps[label].current_by_kind).map(([k, v]) => [k, String(v)]));

async function loadAsKnown(label) {
  const server = await makeServer(await makeS16Base(gen(label), FILE)),
    client = makeClient(server);
  await client.load(discovery(server), "AsKnown", gen(label));
  assert.equal(client.committed.cursor.after_generation, gen(label));
  return client;
}

// 与 test_q3_complete_client.cjs orderedServer 同构：首代 Snapshot，其后逐代稀疏 Delta。
async function watchServer(generations) {
  const bases = new Map();
  for (const g of generations) {
    const b = await makeS16Base(g, FILE);
    b.fixed.history_mode = "RecomputedWithRevision";
    b.fixed.as_of_generation = null;
    await rebuild(b);
    bases.set(g, b);
  }
  const pool = new Map();
  for (const b of bases.values()) for (const n of b.nodes.values()) pool.set(n.root, n);
  for (const b of bases.values()) {
    b.nodes = pool;
    b.graph = await C.GraphView.create([...pool.values()]);
  }
  const server = await makeServer(bases.get(generations[0]));
  server.history = bases;
  server.deltaFor = async (g) => {
    const b = bases.get(g),
      root = b.graph.header(b.publication.delta_root),
      wire = {};
    for (const key of ["generation", "session_id", "catalog_revision", "base_cut", "next_cut", "seq_range", "index_frontier", "input_frontier", "catalog_run_status", "catalog_evidence"]) {
      const r = b.graph.field(root, key);
      if (key === "catalog_evidence") wire[key] = r;
      else {
        let rendered = "";
        for await (const c of b.graph.renderChunks(r)) rendered += c;
        wire[key] = JSON.parse(rendered);
      }
    }
    wire.delta = root;
    return wire;
  };
  server.advance = async (to) => {
    const from = BigInt(server.head ? server.head.fixed.cut_generation : generations[0]);
    server.deltas = [];
    for (let g = from + 1n; g <= BigInt(to); g++) server.deltas.push(await server.deltaFor(String(g)));
    server.head = bases.get(to);
  };
  return server;
}
const range = (a, b) => {
  const out = [];
  for (let g = BigInt(a); g <= BigInt(b); g++) out.push(String(g));
  return out;
};

test("真实S3 W：导出出自 W 唯一写者规则串且恢复后写者纪元为2", () => {
  assert.equal(data.meta.rule_revision, API.Q3.typed.W_STRUCTURE_RULE);
  assert.equal(data.provenance.rule_revision, data.meta.rule_revision);
  assert.equal(data.meta.writer_epoch, "2");
  assert.equal(steps.recovered_epoch2_append.recovered_writer, true);
});

test("真实S3 W cut0 首根与等待期：AsKnown 对象族逐 kind 与 Rust 同代一致，等待期有 W 两 kind", async () => {
  for (const label of ["w_first_bar", "w_contained_waiting"]) {
    const client = await loadAsKnown(label);
    assert.deepEqual(await kindsOf(client), rustKinds(label), label);
  }
  const waiting = rustKinds("w_contained_waiting");
  assert(BigInt(waiting["CC-004.initialization"] || 0) > 0n);
  assert(BigInt(waiting["CC-004.undetermined_raw"] || 0) > 0n);
});

test("真实S3 W 跨真实写者恢复（纪元1→2）逐代 Watch 与同代 AsKnown 一致", async () => {
  const from = gen("direction_established"),
    to = gen("recovered_epoch2_append"),
    server = await watchServer(range(from, to)),
    client = makeClient(server);
  await client.load(discovery(server), "RecomputedWithRevision", from);
  await server.advance(to);
  await client.watch();
  assert.equal(client.committed.cursor.after_generation, to);
  assert.deepEqual(await kindsOf(client), rustKinds("recovered_epoch2_append"));
});

test("真实S3 W 首中枢非空，改价修订撤回与改回恢复两代 Watch 各自与 Rust 同代一致", async () => {
  const first = gen("chain_first_center"),
    withdraw = gen("revision_withdraw"),
    restore = gen("revision_restore");
  assert(BigInt(rustKinds("chain_first_center")["CC-016.center"] || 0) > 0n);
  assert(Object.keys(steps.revision_withdraw.withdrawn_by_kind).length > 0);
  const server = await watchServer(range(first, restore)),
    client = makeClient(server);
  await client.load(discovery(server), "RecomputedWithRevision", first);
  assert.deepEqual(await kindsOf(client), rustKinds("chain_first_center"));
  const withdrawnBefore = client.committed.families.withdrawn_objects.length;
  await server.advance(withdraw);
  await client.watch();
  assert.equal(client.committed.cursor.after_generation, withdraw);
  assert.deepEqual(await kindsOf(client), rustKinds("revision_withdraw"));
  assert(client.committed.families.withdrawn_objects.length > withdrawnBefore, "撤回代未进入撤回族");
  await server.advance(restore);
  await client.watch();
  assert.equal(client.committed.cursor.after_generation, restore);
  assert.deepEqual(await kindsOf(client), rustKinds("revision_restore"));
});

test("真实S3 W 尾段：中枢对非空，末代 AsKnown 与 Rust 一致", async () => {
  const client = await loadAsKnown("chain_tail_pair");
  const want = rustKinds("chain_tail_pair");
  assert(BigInt(want["CC-020.center_pair"] || 0) > 0n);
  assert.deepEqual(await kindsOf(client), want);
});
