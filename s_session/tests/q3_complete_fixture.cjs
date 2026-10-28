/* #1394：Q3 完整客户端与 v3 validate-once 两个套件共用的夹具（真实 Rust Q3 夹具、独立片段证明 oracle、
 * 具名模拟分配账）。分配账是具名模拟，不是 Browser 物理分配验收。从 test_q3_complete_client.cjs 原样抽出。 */
"use strict";
const assert = require("node:assert/strict"),
  test = require("node:test"),
  fs = require("node:fs"),
  crypto = require("node:crypto");
const API = require("../browser/tb01c-client.js"),
  C = require("../browser/s-content-v3.js"),
  V = API.Q3.V;
const raw = fs.readFileSync(__dirname + "/fixtures/q3-complete-onebar.json"),
  original = JSON.parse(raw),
  copy = (v) => JSON.parse(JSON.stringify(v));
const hex = (b) => Buffer.from(b).toString("hex"),
  rootAt = (node, at = 0) =>
    "sc1-" + node.body_hex.slice(at * 2, (at + 32) * 2),
  extent = (node) => BigInt("0x" + node.body_hex.slice(0, 16));
const CONFIG = {
  schema_revision: "s-query-client-resources/3",
  pageSize: 31,
  watchBatches: 4,
  pollMs: 500,
  timeoutMs: 30000,
  replyBytes: 1000000,
  job_limits: Object.fromEntries(
    [
      "max_total_wire_bytes",
      "max_total_node_bytes",
      "max_total_nodes",
      "max_total_records",
      "max_total_requests",
      "max_elapsed_ms",
      "max_staged_bytes",
      "max_store_bytes",
    ].map((k) => [k, k === "max_elapsed_ms" ? "60000" : "1000000000"]),
  ),
};
function measurement() {
  let store = 128n,
    staged = 16n;
  const perOperation = new Map();
  return {
    contract: "s-query-private-allocation/1",
    testOnly: true,
    measurementSource:
      "TestOnly simulated exact allocation units; not Browser physical storage",
    async measure() {
      return {
        exact: true,
        source: this.measurementSource,
        store_bytes: store.toString(),
        staged_bytes: staged.toString(),
      };
    },
    async reserveBound(node) {
      return BigInt(node.body_hex.length / 2 + node.root.length + 128);
    },
    async write(node, { operation }) {
      const allocated = BigInt(
        node.body_hex.length / 2 + node.root.length + 128,
      );
      store += allocated;
      staged += allocated;
      perOperation.set(
        operation,
        (perOperation.get(operation) || 0n) + allocated,
      );
      return {
        exact: true,
        source: this.measurementSource,
        allocated_bytes: allocated.toString(),
      };
    },
    async release({ operation }) {
      const deleted = perOperation.get(operation) || 0n;
      store -= deleted;
      staged -= deleted;
      perOperation.delete(operation);
      return {
        exact: true,
        source: this.measurementSource,
        deleted_bytes: deleted.toString(),
      };
    },
  };
}
async function makeBase(input = original) {
  const original = input;
  const rows = [];
  for (const p of original.pages) rows.push(...copy(p.rows));
  const fixed = copy(original.pages[0].fixed_cut),
    catalog = copy(original.pages[0].catalog),
    counts = copy(original.pages[0].counts),
    nodes = new Map(original.nodes.map((n) => [n.root, n]));
  const graph = await C.GraphView.create(original.nodes),
    axes = graph.field(fixed.catalog_evidence, "axes"),
    staticCounts = { implemented: 0, not_implemented: 0, run: 0, not_run: 0 },
    items = [];
  function str(ref) {
    return graph.text(ref);
  }
  function wireIntegers(v) {
    if (typeof v === "number") {
      assert(Number.isSafeInteger(v));
      return String(v);
    }
    if (Array.isArray(v)) return v.map(wireIntegers);
    if (v && typeof v === "object")
      return Object.fromEntries(
        Object.entries(v).map(([k, x]) => [k, wireIntegers(x)]),
      );
    return v;
  }
  for (const row of original.catalog_rows) {
    let axis = null;
    for (const e of graph.entries(axes))
      if (e.key === row.id) axis = e.value_ref;
    const item = {
      ...row,
      branches: wireIntegers(row.branches),
      implementation_status: axis
        ? str(graph.field(axis, "impl_status"))
        : "not_implemented",
      proof_status: axis
        ? str(graph.field(axis, "proof_status"))
        : "not_proved",
      run_status: axis ? str(graph.field(axis, "run_status")) : "not_run",
      evidence: axis ? C.refEdge(graph.field(axis, "evidence")) : {},
    };
    staticCounts[item.implementation_status]++;
    if (Object.hasOwn(staticCounts, item.run_status))
      staticCounts[item.run_status]++;
    items.push(item);
  }
  const logicalCatalog = {};
  for (const key of [
    "session_id",
    "structure_cut",
    "catalog_revision",
    "index_frontier",
    "input_frontier",
    "seq_range",
    "catalog_run_status",
    "scope",
  ])
    logicalCatalog[key] = fixed[key];
  logicalCatalog.catalog_evidence = C.refEdge(fixed.catalog_evidence);
  logicalCatalog.generation = fixed.cut_generation;
  logicalCatalog.items = items;
  logicalCatalog.counts = Object.fromEntries(
    Object.entries(staticCounts).map(([k, v]) => [k, String(v)]),
  );
  const built = await C.encode(C.fromPlain(logicalCatalog));
  assert.deepEqual(
    built.ref,
    catalog,
    "production build_catalog oracle equals frozen HTTP catalog root",
  );
  for (const node of built.nodes) nodes.set(node.root, node);
  return { rows, fixed, catalog, counts, nodes, graph };
}
function proofNodes(base, anchor, path, target, offset, end) {
  const selected = new Map(),
    get = (root) => {
      const n = base.nodes.get(root);
      assert(n, "oracle missing node " + root);
      selected.set(root, n);
      return n;
    };
  function all(root) {
    const n = get(root),
      tag = parseInt(n.tag, 16);
    if ([3, 4, 5, 6].includes(tag)) all(rootAt(n, 8));
    else if ([7, 0x12].includes(tag)) all(rootAt(n));
    else if (tag === 0x13) {
      all(rootAt(n, 8));
      all(rootAt(n, 40));
    } else if (tag === 0x14) {
      all(rootAt(n));
      all(rootAt(n, 32));
    }
  }
  function leaf(root, n, index) {
    const node = get(root);
    if (n === 0n) return null;
    if (n === 1n) {
      assert.equal(node.tag, "12");
      return rootAt(node);
    }
    assert.equal(node.tag, "13");
    let k = 1n;
    while (k * 2n < n) k *= 2n;
    return index < k
      ? leaf(rootAt(node, 8), k, index)
      : leaf(rootAt(node, 40), n - k, index - k);
  }
  function header(ref) {
    const n = get(ref.root);
    if (ref.kind === "json_text") get(rootAt(n));
  }
  function item(ref, index) {
    const h = get(ref.root),
      r = leaf(rootAt(h, 8), BigInt(ref.extent), index);
    if (ref.kind === "object") {
      const field = get(r);
      all(rootAt(field));
      const child = base.graph.header(rootAt(field, 32));
      header(child);
      return child;
    }
    const child = base.graph.header(r);
    header(child);
    return child;
  }
  let current =
    anchor.kind === "projection" ? anchor.projection_ref : anchor.delta_ref;
  header(current);
  for (const step of path) {
    if (Object.hasOwn(step, "field")) {
      let index = 0n,
        found = false;
      for (const e of base.graph.entries(current)) {
        if (e.key === step.field) {
          current = item(current, index);
          found = true;
          break;
        }
        index++;
      }
      assert(found, "oracle field absent");
    } else if (Object.hasOwn(step, "index"))
      current = item(current, BigInt(step.index));
    else {
      const n = get(current.root);
      current = base.graph.header(rootAt(n));
      header(current);
    }
  }
  assert.equal(current.root, target.root);
  header(target);
  if (["array", "object"].includes(target.kind)) {
    if (offset === end && BigInt(target.extent) === 0n)
      leaf(rootAt(get(target.root), 8), 0n, 0n);
    for (let i = offset; i < end; i++) item(target, i);
  } else if (["string", "number"].includes(target.kind)) {
    const h = get(target.root),
      n = (BigInt(target.extent) + 4095n) / 4096n;
    if (n === 0n) leaf(rootAt(h, 8), 0n, 0n);
    else
      for (let i = offset / 4096n; i < (end + 4095n) / 4096n; i++)
        get(leaf(rootAt(h, 8), n, i));
  }
  return [...selected.values()].sort((a, b) => (a.root < b.root ? -1 : 1));
}
async function rebuild(base) {
  const rows = base.rows.map((row) => ({
    ...row,
    record: C.refEdge(row.record),
  }));
  const projection = {
      fixed_cut: {
        ...base.fixed,
        catalog_evidence: C.refEdge(base.fixed.catalog_evidence),
      },
      catalog: C.refEdge(base.catalog),
      rows,
      counts: base.counts,
      scope: base.fixed.scope,
      omissions: [],
      order_version: "s-record-order/2",
    },
    e = await C.encode(C.fromPlain(projection));
  for (const n of e.nodes) base.nodes.set(n.root, n);
  base.ref = e.ref;
  base.digest = "qcut3-" + V.F(0x22, V.unhex(e.ref.root.slice(4)));
  base.graph = await C.GraphView.create([...base.nodes.values()]);
  return base;
}
async function makeServer(prepared = null) {
  let base = await rebuild(prepared || (await makeBase()));
  const server = { base, mutate: null, requests: [], before: null };
  async function token(offset, mode, issued) {
    const fixed = {
      ...base.fixed,
      history_mode: mode,
      as_of_generation: mode === "AsKnown" ? base.fixed.cut_generation : null,
    };
    const t = {};
    for (const k of [
      "session_id",
      "session_generation",
      "cut_generation",
      "scope",
      "history_mode",
      "structure_cut",
      "catalog_revision",
      "index_frontier",
      "profile_hash",
      "rule_revision",
    ])
      t[k] = fixed[k];
    Object.assign(t, {
      token_schema: "s-snapshot-token/2",
      order_version: "s-record-order/2",
      page_size: "31",
      offset: String(offset),
      cut_projection_digest: base.digest,
      issued_capture_digest: issued,
    });
    t.token_checksum = await API.sha256(t);
    return t;
  }
  server.fetch = async (url, options) => {
    server.requests.push({ url, body: options.body });
    if (server.before) await server.before(url, options);
    const req = options.body ? API.parse(options.body) : null,
      p = req?.payload;
    let response;
    if (url === "/api/v3/snapshot") {
      const requested = p.snapshot_token?.cut_generation || p.as_of_generation;
      if (server.history?.has(requested)) {
        base = server.history.get(requested);
        server.base = base;
      }
      const t = p.snapshot_token,
        mode = t ? t.history_mode : p.history_mode;
      if (base.fixed.history_mode !== mode) {
        base.fixed.history_mode = mode;
        base.fixed.as_of_generation =
          mode === "AsKnown" ? base.fixed.cut_generation : null;
        await rebuild(base);
      }
      const offset = t ? Number(t.offset) : 0,
        end = Math.min(base.rows.length, offset + 31);
      response = {
        fixed_cut: base.fixed,
        catalog: base.catalog,
        rows: base.rows.slice(offset, end),
        counts: base.counts,
        scope: base.fixed.scope,
        omissions: [],
        order_version: "s-record-order/2",
        offset: String(offset),
        done: end === base.rows.length,
        next_token:
          end === base.rows.length
            ? null
            : await token(end, mode, "a".repeat(64)),
        cut_projection_digest: base.digest,
        validated_capture_digest: "a".repeat(64),
        ok: true,
      };
    } else if (url === "/api/v3/content") {
      const q = p.content_token || p,
        anchor = q.anchor,
        path = q.path,
        target = base.graph.resolve(
          anchor.kind === "projection"
            ? anchor.projection_ref
            : anchor.delta_ref,
          path,
        ),
        offset = BigInt(q.offset),
        limit = BigInt(q.limit),
        total = ["array", "object", "string", "number"].includes(target.kind)
          ? BigInt(target.extent)
          : 1n,
        end = offset + limit < total ? offset + limit : total;
      let items;
      if (["array", "object"].includes(target.kind))
        items = [...base.graph.entries(target, offset, end)].map((e) =>
          target.kind === "array"
            ? e.value_ref
            : { key: e.key, value_ref: e.value_ref },
        );
      else if (["string", "number"].includes(target.kind)) {
        items = "";
        for (const bytes of base.graph.byteChunks(target, offset, end))
          items += hex(bytes);
      } else items = offset === 0n ? [target] : [];
      let next = null;
      if (end !== total) {
        next = {
          token_schema: "s-content-token/1",
          anchor,
          path,
          target_ref: target,
          unit: q.unit,
          offset: end.toString(),
          limit: q.limit,
          issued_capture_digest: q.issued_capture_digest || "a".repeat(64),
        };
        next.token_checksum = await C.tokenChecksum(next);
      }
      response = {
        anchor,
        path,
        target_ref: target,
        unit: q.unit,
        offset: q.offset,
        end: end.toString(),
        total: total.toString(),
        items,
        nodes: proofNodes(base, anchor, path, target, offset, end),
        done: end === total,
        next_token: next,
        validated_capture_digest: "a".repeat(64),
        ok: true,
      };
    } else if (url === "/api/v3/watch") {
      if (server.head) {
        base = server.head;
        server.base = base;
      }
      const f = {
        ...base.fixed,
        history_mode: "RecomputedWithRevision",
        as_of_generation: null,
      };
      response = {
        deltas: server.deltas || [],
        next_cursor: server.deltas?.length
          ? {
              ...p.cursor,
              after_generation: f.cut_generation,
              base_cut: f.structure_cut,
              index_frontier: f.index_frontier,
              last_seq: f.input_frontier,
            }
          : p.cursor,
        observed_head: f,
        delivery_frontier: {
          policy_revision: "s-delivery/1",
          retain_generations: "8",
          first_available_generation: "1",
          head_generation: f.cut_generation,
        },
        has_more: false,
        gap: null,
        validated_capture_digest: "a".repeat(64),
        ok: true,
      };
    } else if (url === "/api/ready")
      return new Response(
        API.canonical({
          ok: true,
          producer_epoch: "1",
          control_instance_id: "test-controller",
          cut: {
            session_id: base.fixed.session_id,
            session_generation: "1",
            generation: base.fixed.cut_generation,
            structure_cut: base.fixed.structure_cut,
            catalog_revision: base.fixed.catalog_revision,
            index_frontier: base.fixed.index_frontier,
          },
        }),
      );
    else throw new Error(url);
    response = copy(response);
    if (server.gap && url === "/api/v3/watch") {
      response.deltas = [];
      response.next_cursor = p.cursor;
      response.gap = {
        requested_identity: p.cursor,
        reason: "client_backlog_overflow",
        missing_range: {
          from_generation: (BigInt(p.cursor.after_generation) + 1n).toString(),
          to_generation: base.fixed.cut_generation,
        },
        rebuild: response.observed_head,
        snapshot_token: await token(
          0,
          "RecomputedWithRevision",
          "a".repeat(64),
        ),
      };
    }
    if (server.mutate) await server.mutate(response, p, url);
    const payloadHash = await API.sha256(response),
      e = {
        schema_revision: "s-session/3",
        session_id: base.fixed.session_id,
        session_generation: base.fixed.session_generation,
        source_namespace: "s-observe/replies",
        source_epoch: "1",
        producer_id: "Q:" + base.fixed.session_id,
        producer_epoch: "1",
        message_id:
          "reply-" +
          (await API.sha256([
            req.source_namespace,
            req.source_epoch,
            req.message_id,
            payloadHash,
          ])),
        payload_hash: payloadHash,
        causal_refs: [
          Object.fromEntries(
            [
              "source_namespace",
              "source_epoch",
              "message_id",
              "payload_hash",
            ].map((k) => [k, req[k]]),
          ),
        ],
        payload: response,
      };
    return new Response(API.canonical(e));
  };
  return server;
}
// 每个夹具文件只解析一次，节点池跨代复用（#1419 28U：真实 S3 导出有数十万节点，逐代重解析会 OOM）。
const FIXTURE_CACHE = new Map();
function fixtureOf(file) {
  if (!FIXTURE_CACHE.has(file)) {
    const data = JSON.parse(fs.readFileSync(__dirname + "/fixtures/" + file));
    FIXTURE_CACHE.set(file, {
      data,
      pool: new Map(data.nodes.map((n) => [n.root, n])),
    });
  }
  return FIXTURE_CACHE.get(file);
}
// 自给定根出发按 s-content/1 子引用布局取可达节点（与 proofNodes 的 all() 同一布局）；缺节点即夹具不完整。
function closureOf(pool, roots) {
  const seen = new Map(),
    stack = [...roots];
  while (stack.length) {
    const root = stack.pop();
    if (seen.has(root)) continue;
    const n = pool.get(root);
    assert(n, "fixture missing node " + root);
    seen.set(root, n);
    const tag = parseInt(n.tag, 16);
    if ([3, 4, 5, 6].includes(tag)) stack.push(rootAt(n, 8));
    else if ([7, 0x12].includes(tag)) stack.push(rootAt(n));
    else if (tag === 0x13) stack.push(rootAt(n, 8), rootAt(n, 40));
    else if (tag === 0x14) stack.push(rootAt(n), rootAt(n, 32));
  }
  return [...seen.values()];
}
// file：fixtures 下的真实 Rust 导出（同一导出格式）；默认 S16 封存 cut3/4/5。
// 每代只认证该代 snapshot 与 delta 可达的节点，不对全池建图。
async function makeS16Base(generation = "3", file = "q3-complete-s16-cuts345.json") {
  const { data, pool } = fixtureOf(file),
    publication = data.publications.find((p) => p.generation === generation);
  assert(publication, "fixture has no publication " + generation);
  const own = closureOf(pool, [publication.snapshot_root, publication.delta_root]),
    nodes = new Map(own.map((n) => [n.root, n])),
    graph = await C.GraphView.create(own);
  assert.equal(
    publication.batch_id,
    "batch3-" + V.F(0x21, V.unhex(publication.manifest_root.slice(4))),
    "real source manifest identity",
  );
  const snapshot = graph.header(publication.snapshot_root);
  await graph.verifyComplete(snapshot);
  const text = (ref, k) => graph.text(graph.field(ref, k));
  const fixed = {
    session_generation: data.session_generation,
    cut_generation: generation,
    rule_revision: data.meta.rule_revision,
  };
  for (const key of [
    "session_id",
    "structure_cut",
    "catalog_revision",
    "index_frontier",
    "profile_id",
    "profile_hash",
    "input_frontier",
    "catalog_run_status",
    "history_mode",
  ])
    fixed[key] = text(snapshot, key);
  for (const key of ["scope", "seq_range", "as_of_generation"]) {
    let rendered = "";
    for await (const c of graph.renderChunks(graph.field(snapshot, key)))
      rendered += c;
    fixed[key] = JSON.parse(rendered);
  }
  fixed.catalog_evidence = graph.field(snapshot, "catalog_evidence");
  const rows = [],
    counts = {};
  for (const family of API.FAMILIES) {
    const arr = graph.field(snapshot, family),
      ordered = [];
    for (const e of graph.entries(arr)) {
      const r = e.value_ref,
        key =
          family === "objects" || family === "withdrawn_objects"
            ? [text(r, "object_id")]
            : family === "relations"
              ? ["subject", "relation_type", "object"].map((k) => text(r, k))
              : family === "raw_history"
                ? ["identity_key", "revision"].map((k) => text(r, k))
                : [
                    text(
                      r,
                      family === "witnesses" ? "witness_id" : "observation_id",
                    ),
                  ];
      let record = {};
      const all = [
        "object_id",
        "object_revision",
        "kind",
        "fact_key",
        "window_start",
        "window_mid",
        "window_end",
        "withdrawn_generation",
        "slot",
        "witness_id",
        "subject",
        "relation_type",
        "object",
        "observation_id",
        "seq",
        "identity_key",
        "revision",
      ];
      for (const name of all)
        for (const f of graph.entries(r))
          if (f.key === name)
            record[name] =
              f.value_ref.kind === "null" ? null : graph.text(f.value_ref);
      ordered.push({
        family,
        key,
        record: r,
        sort: API.rowsOf({
          objects: [],
          withdrawn_objects: [],
          witnesses: [],
          relations: [],
          observations: [],
          raw_history: [],
          [family]: [record],
        })[0].record,
      });
    }
    const tuple = (r) => {
      if (family === "objects")
        return Object.hasOwn(r, "fact_key")
          ? [1n, r.kind, r.fact_key, r.object_id, BigInt(r.object_revision)]
          : [
              0n,
              BigInt(r.window_start),
              BigInt(r.window_mid),
              BigInt(r.window_end),
              r.object_id,
              BigInt(r.object_revision),
            ];
      if (family === "withdrawn_objects")
        return [
          BigInt(r.withdrawn_generation),
          r.object_id,
          BigInt(r.object_revision),
        ];
      if (family === "witnesses")
        return [r.object_id, BigInt(r.slot), r.witness_id];
      if (family === "relations") return [r.subject, r.relation_type, r.object];
      if (family === "observations")
        return [
          r.kind,
          r.window_start === null ? -1n : BigInt(r.window_start),
          r.observation_id,
        ];
      return [BigInt(r.seq), r.identity_key, BigInt(r.revision)];
    };
    ordered.sort((a, b) => {
      const aa = tuple(a.sort),
        bb = tuple(b.sort);
      for (let i = 0; i < aa.length; i++) {
        const c =
          typeof aa[i] === "bigint"
            ? aa[i] < bb[i]
              ? -1
              : aa[i] > bb[i]
                ? 1
                : 0
            : Buffer.compare(Buffer.from(aa[i]), Buffer.from(bb[i]));
        if (c) return c;
      }
      return 0;
    });
    for (const row of ordered) {
      delete row.sort;
      rows.push(row);
    }
    counts[family] = String(ordered.length);
  }
  counts.total = String(rows.length);
  // 与Rust build_catalog相同的只读source配方；source batch3仍来自封存manifest。
  const axes = graph.field(fixed.catalog_evidence, "axes"),
    items = [],
    status = { implemented: 0, not_implemented: 0, run: 0, not_run: 0 };
  function wire(v) {
    if (typeof v === "number") {
      assert(Number.isSafeInteger(v));
      return String(v);
    }
    if (Array.isArray(v)) return v.map(wire);
    if (v && typeof v === "object")
      return Object.fromEntries(
        Object.entries(v).map(([k, x]) => [k, wire(x)]),
      );
    return v;
  }
  for (const row of data.catalog_rows) {
    let axis = null;
    for (const e of graph.entries(axes))
      if (e.key === row.id) axis = e.value_ref;
    const item = {
      ...row,
      branches: wire(row.branches),
      implementation_status: axis
        ? text(axis, "impl_status")
        : "not_implemented",
      proof_status: axis ? text(axis, "proof_status") : "not_proved",
      run_status: axis ? text(axis, "run_status") : "not_run",
      evidence: axis ? C.refEdge(graph.field(axis, "evidence")) : {},
    };
    status[item.implementation_status]++;
    if (Object.hasOwn(status, item.run_status)) status[item.run_status]++;
    items.push(item);
  }
  const cat = {};
  for (const key of [
    "session_id",
    "structure_cut",
    "catalog_revision",
    "index_frontier",
    "input_frontier",
    "seq_range",
    "catalog_run_status",
    "scope",
  ])
    cat[key] = fixed[key];
  Object.assign(cat, {
    catalog_evidence: C.refEdge(fixed.catalog_evidence),
    generation,
    items,
    counts: Object.fromEntries(
      Object.entries(status).map(([k, v]) => [k, String(v)]),
    ),
  });
  const catalog = await C.encode(C.fromPlain(cat));
  for (const n of catalog.nodes) nodes.set(n.root, n);
  return {
    rows,
    fixed,
    catalog: catalog.ref,
    counts,
    nodes,
    graph,
    data,
    publication,
  };
}
function makeClient(server, config = CONFIG) {
  return new API.Client({
    protocol: "s-session/3",
    fetch: server.fetch,
    limits: config,
    allocation: measurement(),
  });
}
const discovery = (s) => ({
  schema_revision: "s-session/3",
  cut: {
    session_id: s.base.fixed.session_id,
    session_generation: s.base.fixed.session_generation,
  },
});
async function setup() {
  const server = await makeServer(),
    client = makeClient(server);
  await client.load(discovery(server), "AsKnown", "1");
  return { server, client };
}

module.exports = {
  API,
  C,
  V,
  raw,
  original,
  copy,
  hex,
  rootAt,
  extent,
  CONFIG,
  measurement,
  makeBase,
  proofNodes,
  rebuild,
  makeServer,
  fixtureOf,
  makeS16Base,
  makeClient,
  discovery,
  setup,
};
