/* #1419/#1424：Q3 沿原公开 Client 入口消费真实 S3 W 写者多代导出（27AZ P1）。
 * 夹具 fixtures/q3-real-s3-w-recover-revise.json 由 Rust
 * v3::financial_session::tests::export_real_s3_w_fixture_for_q3 真实 init_v3 + 接纳/Begin/提交逐代导出，
 * provenance 记 head、命令与逐里程碑的 Rust 侧当前对象计数；本套以该计数为对拍 oracle，不另建读链。
 * 覆盖：cut0 首根、W 等待期两 kind、自等待期跨真实写者恢复（纪元 1→2）至方向确立的逐代 Watch、首中枢、
 * 改价修订撤回与改回恢复两代 Watch、尾段中枢对。
 *
 * 夹具载体（主控裁定，#1419 28U）：普通 blob 入仓（91,895,732 B < 100 MB，不走 LFS/外置），
 * sha256 2a83c1808b21fdc6647d37a602d782622b2d74c5bfb4342e16cb69f7d0490a2a；取自 f11ff5847d 远端双跑导出 A，与 B 逐字节相同。
 *
 * 验收分层（主控裁定，#1419 28AO/28U）：
 * - 第 1、2 条（规则串、cut0 与等待期）进 CI 真跑：ci.yml `Q3 S3 real cut0 consumption (#1419)`，
 *   `node --max-old-space-size=4096 --test --test-name-pattern="规则串|cut0" s_session/tests/test_q3_s3_real_cut.cjs`（约 30 s，峰值 RSS 1.4 GB）。
 * - 第 3–5 条不进 CI，属重型验证窗口：单条峰值 RSS 12–15 GB、单条耗时 12–50 分钟，超出 GitHub runner（7 GB）。
 *   每条单独一个进程跑（整文件一个进程峰值 17.6 GB），本机或远端均为：
 *     Q3_WIRE_PROBE=1 node --max-old-space-size=16000 --test --test-name-pattern="跨真实写者恢复" s_session/tests/test_q3_s3_real_cut.cjs
 *     Q3_WIRE_PROBE=1 node --max-old-space-size=16000 --test --test-name-pattern="首中枢" s_session/tests/test_q3_s3_real_cut.cjs
 *     Q3_WIRE_PROBE=1 node --max-old-space-size=16000 --test --test-name-pattern="尾段" s_session/tests/test_q3_s3_real_cut.cjs
 *   收据（head 3d7e5b0f1e，5/5 ok）：/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1419-q3-export/receipts-28u/t1..t5.log，
 *   导出双跑日志同目录 20261028-120621-f11ff5847d-28u-a.log、20261028-150416-f11ff5847d-28u-b.log。
 *   重跑触发面：q3-consumer.js、q3-center.js、q3-typed.js、s-content-v3.js、q3_complete_fixture.cjs 或本夹具任一改动，
 *   合入前须重跑第 3–5 条并更新收据；改动面之外的 PR 不要求。 */
"use strict";
const assert = require("node:assert/strict"),
  test = require("node:test");
const { API, C, V, CONFIG, rebuild, makeServer, fixtureOf, makeS16Base, makeClient: makeClientWith, discovery } = require("./q3_complete_fixture.cjs");
// 仅本套放宽测试 Client 的时限（共享 CONFIG、生产 cap 与 Client 默认值不动）：真实大代单次 AsKnown 读取
// 远超共享 CONFIG 的 30 s/60 s（耗时剖析见 #1419 28U 报告）。
// 同理放宽本套 Client 作业的 wire 计量上限：实测单个作业最大 wire 为第 106 代 AsKnown 读取
// 1,995,584,735 B（#1419 28U，Q3_WIRE_PROBE=1 读数）；上限 = ⌈1,995,584,735 × 1.5⌉ = 2,993,377,103 B。
const REAL_CONFIG = {
  ...CONFIG,
  timeoutMs: 4 * 3600 * 1000,
  job_limits: { ...CONFIG.job_limits, max_elapsed_ms: String(4 * 3600 * 1000), max_total_wire_bytes: "2993377103" },
};
// Q3_WIRE_PROBE=1：逐作业（load/watch）打印响应体字节数、耗时与 RSS，即上面上限的读数来源。
const makeClient = (server) => {
  let wire = 0;
  if (process.env.Q3_WIRE_PROBE) {
    const fetch = server.fetch;
    server.fetch = async (url, options) => {
      const response = await fetch(url, options);
      const body = Buffer.from(await response.arrayBuffer());
      wire += body.length;
      // 保留 status/headers，探针路径与非探针路径对 Client 判定一致（28AO LOW-3）。
      const init = { status: response.status, statusText: response.statusText, headers: response.headers };
      return new Response([101, 103, 204, 205, 304].includes(response.status) ? null : body, init);
    };
  }
  const client = makeClientWith(server, REAL_CONFIG);
  if (process.env.Q3_WIRE_PROBE)
    for (const op of ["load", "watch"]) {
      const run = client[op].bind(client);
      client[op] = async (...args) => {
        wire = 0;
        const started = Date.now();
        try {
          return await run(...args);
        } finally {
          console.error("Q3-WIRE", op, server.base.fixed.cut_generation, wire, Date.now() - started, process.memoryUsage().rss);
        }
      };
    }
  return client;
};

// 夹具只含消费所需各代 snapshot/delta 的可达节点（provenance.node_closure），整文件只解析一次。
const FILE = "q3-real-s3-w-recover-revise.json";
const { data } = fixtureOf(FILE);
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
  const graph = await C.GraphView.create([...pool.values()]);
  for (const b of bases.values()) {
    b.nodes = pool;
    b.graph = graph;
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
  assert.equal(steps.recovered_epoch2_direction.recovered_writer, true);
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
  const from = gen("w_contained_waiting"),
    to = gen("recovered_epoch2_direction"),
    server = await watchServer(range(from, to)),
    client = makeClient(server);
  await client.load(discovery(server), "RecomputedWithRevision", from);
  // Client 合同：一次 Watch 至多 watchBatches 个 delta；逐段推进到恢复后方向确立代。
  const step = BigInt(REAL_CONFIG.watchBatches);
  for (let g = BigInt(from); g < BigInt(to); ) {
    g = g + step < BigInt(to) ? g + step : BigInt(to);
    await server.advance(String(g));
    await client.watch();
    assert.equal(client.committed.cursor.after_generation, String(g));
  }
  assert.deepEqual(await kindsOf(client), rustKinds("recovered_epoch2_direction"));
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
