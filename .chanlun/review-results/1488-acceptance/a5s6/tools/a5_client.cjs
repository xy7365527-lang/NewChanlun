#!/usr/bin/env node
/* #1488 a5s6-0076 A⑤（D3）：用现有测试客户端对正式会话入口的公共查询逐代核对。
 *
 * 客户端 = s_session/browser/tb01c-client.js 的 Client（protocol s-session/3，内部为 Q3Consumer
 * + s-content-v3 + s-content-job-v3），与 index.html 的 Q3 路径同一构造；资源档与分配测量取
 * tests/q3_complete_fixture.cjs 的 CONFIG/measurement() 原样（TestOnly 模拟分配，不是 Browser 物理分配）。
 * 每代：loadReady("AsKnown", g) → 原子提交的完整候选；再用 s-content-v3 独立重编码导出的投影，
 * 核根与 qcut3 摘要；与 oracle（main 6badaa6049 v3 臂 min12d-main1 RUN.json queries[]）逐代比 digest/counts。
 * 用法：a5_client.cjs --repo R --base-url http://127.0.0.1:P --gens 1-52|1,16,52 --out OUT.jsonl
 *        [--oracle RUN.json] [--limits fixture|relaxed] [--current]
 */
"use strict";
const fs = require("node:fs"), path = require("node:path");
const args = Object.fromEntries(process.argv.slice(2).reduce((acc, a, i, all) => {
  if (a.startsWith("--")) acc.push([a.slice(2), all[i + 1] && !all[i + 1].startsWith("--") ? all[i + 1] : "1"]);
  return acc;
}, []));
const repo = path.resolve(args.repo);
const API = require(path.join(repo, "s_session/browser/tb01c-client.js"));
const C = require(path.join(repo, "s_session/browser/s-content-v3.js"));
const V = API.Q3.V;
const LIMIT_KEYS = ["max_total_wire_bytes", "max_total_node_bytes", "max_total_nodes", "max_total_records",
  "max_total_requests", "max_elapsed_ms", "max_staged_bytes", "max_store_bytes"];
// q3_complete_fixture.cjs CONFIG 原样；relaxed 只放长两项时间（诊断档，不是验收档）。
function config(kind) {
  const relaxed = kind === "relaxed";
  return {
    schema_revision: "s-query-client-resources/3", pageSize: 31, watchBatches: 4, pollMs: 500,
    timeoutMs: relaxed ? 600000 : 30000, replyBytes: 1000000,
    job_limits: Object.fromEntries(LIMIT_KEYS.map(k => [k, k === "max_elapsed_ms" ? (relaxed ? "3600000" : "60000") : "1000000000"])),
  };
}
// q3_complete_fixture.cjs measurement() 原样（TestOnly 模拟 exact 分配单位）。
function measurement() {
  let store = 128n, staged = 16n;
  const perOperation = new Map();
  return {
    contract: "s-query-private-allocation/1", testOnly: true,
    measurementSource: "TestOnly simulated exact allocation units; not Browser physical storage",
    async measure() { return {exact: true, source: this.measurementSource, store_bytes: store.toString(), staged_bytes: staged.toString()}; },
    async reserveBound(node) { return BigInt(node.body_hex.length / 2 + node.root.length + 128); },
    async write(node, {operation}) {
      const allocated = BigInt(node.body_hex.length / 2 + node.root.length + 128);
      store += allocated; staged += allocated;
      perOperation.set(operation, (perOperation.get(operation) || 0n) + allocated);
      return {exact: true, source: this.measurementSource, allocated_bytes: allocated.toString()};
    },
    async release({operation}) {
      const deleted = perOperation.get(operation) || 0n;
      store -= deleted; staged -= deleted; perOperation.delete(operation);
      return {exact: true, source: this.measurementSource, deleted_bytes: deleted.toString()};
    },
  };
}
function gens(spec) {
  const out = [];
  for (const part of spec.split(",")) {
    const [a, b] = part.split("-").map(Number);
    for (let g = a; g <= (b || a); g++) out.push(g);
  }
  return out;
}
async function main() {
  const base = args["base-url"];
  const oracle = args.oracle ? Object.fromEntries(JSON.parse(fs.readFileSync(args.oracle)).queries.map(q => [q.generation, q])) : {};
  const out = fs.openSync(args.out, "a");
  const http = [];
  const fetcher = async (url, init) => {
    const t0 = performance.now();
    const res = await fetch(base + url, init);
    const entry = {url, status: res.status, ms: null, bytes: null, digest: null, error_kind: null};
    http.push(entry);
    try {
      const text = await res.clone().text();
      entry.bytes = Buffer.byteLength(text);
      if (url === "/api/v3/snapshot" || res.status !== 200) {
        const body = JSON.parse(text);
        const p = body.payload || body;
        entry.digest = p.cut_projection_digest || null;
        if (res.status !== 200 || p.ok === false) entry.error_kind = JSON.stringify(p.error || p.kind || p).slice(0, 400);
      }
    } catch (e) { entry.parse_error = String(e).slice(0, 200); }
    entry.ms = Math.round(performance.now() - t0);
    return res;
  };
  const client = new API.Client({protocol: "s-session/3", fetch: fetcher, limits: config(args.limits || "fixture"), allocation: measurement()});
  const plan = gens(args.gens).map(g => ({mode: "AsKnown", g}));
  if (args.current) plan.push({mode: "RecomputedWithRevision", g: null});
  let pass = 0, fail = 0;
  for (const {mode, g} of plan) {
    http.length = 0;
    const t0 = performance.now();
    const rec = {mode, generation: g, limits: args.limits || "fixture", started_utc: new Date().toISOString()};
    try {
      const cand = await client.loadReady(mode, g === null ? null : String(g));
      rec.client_ms = Math.round(performance.now() - t0);
      rec.committed = true;
      rec.fixed_cut = {cut_generation: cand.fixed.cut_generation, structure_cut: cand.fixed.structure_cut,
        rule_revision: cand.fixed.rule_revision, history_mode: cand.fixed.history_mode};
      rec.family_counts = Object.fromEntries(Object.entries(cand.families).map(([k, v]) => [k, v.length]));
      rec.pages = cand.pages;
      const digests = [...new Set(http.filter(h => h.digest).map(h => h.digest))];
      rec.server_digests = digests;
      // s-content-v3 独立重编码：导出的完整投影 → encode → 根与 qcut3。
      const t1 = performance.now();
      let exported = "";
      for await (const c of V.canonicalChunks(cand.projection)) exported += c;
      const encoded = await C.encode(C.parseJSON(exported));
      rec.export_bytes = Buffer.byteLength(exported);
      rec.reencode_root_equal = encoded.ref.root === cand.projection.ref.root;
      rec.qcut3 = "qcut3-" + V.F(0x22, V.unhex(encoded.ref.root.slice(4)));
      rec.qcut3_equals_server = digests.length === 1 && digests[0] === rec.qcut3;
      rec.reencode_ms = Math.round(performance.now() - t1);
      const o = g !== null ? oracle[g] : null;
      if (o) {
        rec.oracle_digest_equal = o.digest === rec.qcut3;
        const counts = o.counts;
        rec.oracle_counts_equal = ["objects", "withdrawn_objects", "relations", "witnesses", "observations", "raw_history"]
          .every(k => String(rec.family_counts[k]) === counts[k]);
        rec.oracle_structure_cut_equal = o.structure_cut === rec.fixed_cut.structure_cut;
        rec.oracle_rule_revision_equal = o.rule_revision === rec.fixed_cut.rule_revision;
      }
      rec.ok = rec.reencode_root_equal && rec.qcut3_equals_server &&
        (!o || (rec.oracle_digest_equal && rec.oracle_counts_equal && rec.oracle_structure_cut_equal && rec.oracle_rule_revision_equal));
    } catch (e) {
      rec.client_ms = Math.round(performance.now() - t0);
      rec.committed = false; rec.ok = false;
      rec.error = String(e).slice(0, 1000); rec.error_code = e.code || null;
      rec.last_failure = client.lastFailure || null;
    }
    const job = client.lastJob ? client.lastJob.snapshot() : null;
    rec.job = job && Object.fromEntries(Object.entries(job).filter(([k]) => k !== "limits"));
    rec.http = {requests: http.length, by_url: http.reduce((m, h) => (m[h.url] = (m[h.url] || 0) + 1, m), {}),
      non200: http.filter(h => h.status !== 200).slice(0, 5), bytes: http.reduce((s, h) => s + (h.bytes || 0), 0),
      max_ms: Math.max(0, ...http.map(h => h.ms || 0))};
    rec.ok ? pass++ : fail++;
    fs.writeSync(out, JSON.stringify(rec) + "\n");
    console.log(`${mode} g=${g} ok=${rec.ok} ${rec.client_ms}ms req=${rec.http.requests}${rec.error ? " ERR " + rec.error.slice(0, 160) : ""}`);
  }
  console.log(JSON.stringify({pass, fail}));
  process.exitCode = fail ? 1 : 0;
}
main().catch(e => { console.error(e); process.exitCode = 2; });
