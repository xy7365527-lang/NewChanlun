/* #1394：正式Client与独立片段证明oracle。分配账是具名模拟，不是Browser物理分配验收。 */
"use strict";
const assert = require("node:assert/strict"),
  test = require("node:test"),
  fs = require("node:fs"),
  crypto = require("node:crypto");
const {
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
  makeS16Base,
  makeClient,
  discovery,
  setup,
} = require("./q3_complete_fixture.cjs");
// v3 validate-once 锁在 test_q3_v3_validate_once.cjs（CI s-observer 执行）。
test("真实Rust Q3 1bar七族引用页：完整Content遍历与普通对象ref域区分后原子安装", async () => {
  const { client, server } = await setup();
  assert.equal(client.committed.cursor.after_generation, "1");
  assert.equal(client.committed.families.objects.length, 4);
  assert(server.requests.some((r) => r.url === "/api/v3/content"));
  assert(BigInt(client.lastJob.snapshot().requests) > 10n);
  let exported = "";
  for await (const c of V.canonicalChunks(client.committed.projection))
    exported += c;
  const encoded = await C.encode(C.parseJSON(exported));
  assert.equal(encoded.ref.root, client.committed.projection.ref.root);
});
test("Ready原四字段和六身份同operation/Job读取，缺资源/测量拒绝", async () => {
  const server = await makeServer(),
    client = makeClient(server);
  await client.loadReady("AsKnown", "1");
  assert.equal(server.requests[0].url, "/api/ready");
  assert.throws(
    () =>
      new API.Client({
        protocol: "s-session/3",
        fetch: server.fetch,
        limits: CONFIG,
      }),
    /测量/,
  );
  assert.throws(
    () =>
      new API.Client({
        protocol: "s-session/3",
        fetch: server.fetch,
        allocation: measurement(),
      }),
    /字段/,
  );
});
test("同cut空Watch也全字段对拍，失败保committed/cursor和累计账", async () => {
  const { client, server } = await setup(),
    before = client.committed;
  await client.watch();
  assert.notEqual(client.committed, before);
  const current = client.committed;
  server.mutate = async (response, p, url) => {
    if (url === "/api/v3/content" && response.nodes.length > 1)
      response.nodes.shift();
  };
  await assert.rejects(client.watch(), /missing node|节点|path/);
  assert.equal(client.committed, current);
  assert(BigInt(client.lastJob.snapshot().wire_bytes) > 0n);
});
test("晚到cancel/换会话片段与单Job累计cap拒绝，保已提交态", async () => {
  const { client, server } = await setup(),
    before = client.committed;
  let resolve, arrived;
  const wait = new Promise((r) => (resolve = r)),
    entry = new Promise((r) => (arrived = r));
  server.before = async (url) => {
    if (url === "/api/v3/snapshot") {
      arrived();
      await wait;
    }
  };
  const loading = client.load(discovery(server), "AsKnown", "1");
  await entry;
  client.cancel();
  resolve();
  await assert.rejects(loading, /取消|旧操作/);
  assert.equal(client.committed, before);
  server.before = null;
  server.mutate = async (response, p, url) => {
    if (url === "/api/v3/content")
      response.anchor.fixed_cut.session_generation = "2";
  };
  await assert.rejects(
    client.load(discovery(server), "AsKnown", "1"),
    /echo|anchor|request/,
  );
  assert.equal(client.committed, before);
  const cap = makeClient(server, {
    ...CONFIG,
    job_limits: { ...CONFIG.job_limits, max_total_wire_bytes: "100" },
  });
  server.mutate = null;
  await assert.rejects(cap.load(discovery(server), "AsKnown", "1"), /wire/);
  assert(BigInt(cap.lastJob.snapshot().wire_bytes) > 100n);
});
module.exports = {
  makeS16Base,
  makeServer,
  makeClient,
  setup,
  rebuild,
  CONFIG,
  measurement,
  proofNodes,
};

test("S16封存cut3 source oracle：真实obj3 typed/CC006/witness/raw全过程", async () => {
  const server = await makeServer(await makeS16Base("3")),
    client = makeClient(server);
  await client.load(discovery(server), "AsKnown", "3");
  assert(client.committed.families.witnesses.length >= 3);
  assert(client.committed.families.objects.length > 4);
});

async function orderedServer() {
  const bases = new Map();
  for (const generation of ["3", "4", "5"]) {
    const b = await makeS16Base(generation);
    b.fixed.history_mode = "RecomputedWithRevision";
    b.fixed.as_of_generation = null;
    await rebuild(b);
    bases.set(generation, b);
  }
  const pool = new Map();
  for (const b of bases.values())
    for (const n of b.nodes.values()) pool.set(n.root, n);
  for (const b of bases.values()) {
    b.nodes = pool;
    b.graph = await C.GraphView.create([...pool.values()]);
  }
  const server = await makeServer(bases.get("3"));
  server.history = bases;
  server.head = bases.get("5");
  server.deltas = [];
  for (const generation of ["4", "5"]) {
    const b = bases.get(generation),
      root = b.graph.header(b.publication.delta_root),
      wire = {};
    for (const key of [
      "generation",
      "session_id",
      "catalog_revision",
      "base_cut",
      "next_cut",
      "seq_range",
      "index_frontier",
      "input_frontier",
      "catalog_run_status",
      "catalog_evidence",
    ]) {
      const r = b.graph.field(root, key);
      if (key === "catalog_evidence") wire[key] = r;
      else {
        let rendered = "";
        for await (const c of b.graph.renderChunks(r)) rendered += c;
        wire[key] = JSON.parse(rendered);
      }
    }
    wire.delta = root;
    server.deltas.push(wire);
  }
  return server;
}
test("S16真实稀疏Delta3→4→5七集合与同代Snapshot完整对拍只安装末代一次", async () => {
  const server = await orderedServer(),
    client = makeClient(server);
  await client.load(discovery(server), "RecomputedWithRevision", "3");
  const before = client.committed;
  await client.watch();
  assert.equal(client.committed.cursor.after_generation, "5");
  assert.notEqual(client.committed, before);
});
test("S16合法图但witness merged_high错且witID合法、raw八键不变均拒绝并保旧态", async () => {
  const server = await makeServer(await makeS16Base("3")),
    client = makeClient(server);
  await client.load(discovery(server), "AsKnown", "3");
  const before = client.committed;
  for (const mode of ["merged", "raw"]) {
    const base = await makeS16Base("3"),
      row = base.rows.find((r) => r.family === "witnesses");
    let rendered = "";
    for await (const c of base.graph.renderChunks(row.record)) rendered += c;
    const w = JSON.parse(rendered),
      oldId = w.witness_id;
    if (mode === "merged") w.merged_high = "999999";
    else w.raw_bars[0].high = "999999";
    assert.equal(w.witness_id, oldId);
    const built = await C.encode(C.fromPlain(w));
    for (const n of built.nodes) base.nodes.set(n.root, n);
    row.record = built.ref;
    await rebuild(base);
    server.history = new Map([["3", base]]);
    await assert.rejects(
      client.load(discovery(server), "AsKnown", "3"),
      mode === "merged" ? /merged_high/ : /raw原件/,
    );
    assert.equal(client.committed, before);
  }
});
test("S16中间代遗漏replaces/撤回不存在、同cut未显示字段变化均拒绝保旧cursor", async () => {
  const server = await orderedServer(),
    client = makeClient(server);
  await client.load(discovery(server), "RecomputedWithRevision", "3");
  const before = client.committed;
  const saved = server.deltas[0];
  let rendered = "";
  for await (const c of server.head.graph.renderChunks(saved.delta))
    rendered += c;
  for (const kind of ["replaces", "withdrawal"]) {
    const d = JSON.parse(rendered);
    if (kind === "replaces") {
      assert(d.replaces.length > 0);
      d.replaces.pop();
    } else {
      assert(d.withdrawals.length > 0);
      d.withdrawals[0].object_id = "obj3-" + "0".repeat(64);
    }
    const e = await C.encode(C.fromPlain(d));
    for (const n of e.nodes) server.head.nodes.set(n.root, n);
    for (const b of server.history.values())
      b.graph = await C.GraphView.create([...server.head.nodes.values()]);
    server.deltas[0] = { ...saved, delta: e.ref };
    await assert.rejects(
      client.watch(),
      kind === "replaces" ? /replaces/ : /replaces|撤回不存在/,
    );
    assert.equal(client.committed, before);
  }
  server.deltas[0] = saved;
  server.mutate = (response, p, url) => {
    if (url === "/api/v3/watch") response.next_cursor.last_seq = "999999";
  };
  await assert.rejects(client.watch(), /next_cursor/);
  assert.equal(client.committed, before);
});
test("S16 Gap重建全部片段成功才替换，坏尾证明与取消保旧committed", async () => {
  const server = await orderedServer(),
    client = makeClient(server);
  await client.load(discovery(server), "RecomputedWithRevision", "3");
  const before = client.committed;
  server.gap = true;
  server.mutate = (response, p, url) => {
    if (url === "/api/v3/content" && response.path.length > 3)
      response.nodes.pop();
  };
  await assert.rejects(client.watch(), /missing node/);
  assert.equal(client.committed, before);
  server.mutate = null;
  await client.watch();
  assert.equal(client.committed.cursor.after_generation, "5");
});
async function replaceWireRecord(base, row, mutate) {
  let text = "";
  for await (const c of base.graph.renderChunks(row.record)) text += c;
  const originalRecord = JSON.parse(text);
  await mutate(originalRecord);
  const encoded = await C.encode(C.fromPlain(originalRecord));
  for (const node of encoded.nodes) base.nodes.set(node.root, node);
  row.record = encoded.ref;
  await rebuild(base);
}
test("合法内容图且末CC054重复request、末CC007第24关系错误由完整业务遍历拒绝", async () => {
  const server = await makeServer(),
    client = makeClient(server);
  await client.load(discovery(server), "AsKnown", "1");
  const before = client.committed;
  const duplicate = await makeBase(),
    knowledge = duplicate.rows.find(
      (row) =>
        duplicate.graph.text(duplicate.graph.field(row.record, "kind")) ===
        "CC-054.knowledge_state",
    );
  await replaceWireRecord(duplicate, knowledge, (record) => {
    const requests = record.payload.requests;
    assert(requests.length > 0);
    const last = copy(requests.at(-1));
    for (let j = 0; j < 64; j++)
      requests.push({ ...last, request_id: last.request_id + ":" + j });
    requests.push(copy(last));
  });
  server.history = new Map([["1", duplicate]]);
  await assert.rejects(
    client.load(discovery(server), "AsKnown", "1"),
    /请求身份重复/,
  );
  assert.equal(client.committed, before);
  const malformed = await makeBase(),
    row = malformed.rows.find(
      (r) =>
        malformed.graph.text(malformed.graph.field(r.record, "kind")) ===
        "CC-056.version",
    );
  await replaceWireRecord(malformed, row, (record) => {
    const window = ["0", "1", "2"],
      names = [
        "left.high",
        "left.low",
        "mid.high",
        "mid.low",
        "right.high",
        "right.low",
      ],
      references = names.map((name, j) => ({
        name,
        axis: name.split(".")[1],
        source_coord: window[Math.floor(j / 2)],
        value: j % 2 ? "0" : "10",
      })),
      bar = { source_coord: "3", open: "3", high: "7", low: "2", close: "5" },
      relations = [];
    for (const axis of ["open", "high", "low", "close"])
      for (const ref of references)
        relations.push({
          axis,
          value: bar[axis],
          reference: ref.name,
          reference_value: ref.value,
          order: "LT",
        });
    record.kind = "CC-007.fractal_description";
    record.fact_key = API.canonical([record.kind, window]);
    record.payload = {
      shape_object_id: "obj3-" + "3".repeat(64),
      branch: "TOP",
      window,
      description: "confirmed top source oracle",
      reference_prices: references,
      descriptive_labels: {
        status: "not_determined",
        reason: "no_settled_numeric_rule",
        labels: [],
        source: "fenxing.md:64-74",
        raw_ohlc: [],
      },
      subsequent_development: {
        status: "observed",
        reason: null,
        bars: Array.from({ length: 64 }, () => ({
          bar,
          relations: copy(relations),
        })),
      },
    };
    record.payload.subsequent_development.bars.at(
      -1,
    ).relations[23].reference_value = "999";
  });
  // 新kind排在CC005之后、CC054之前；排序正确才能到末关系判据。
  const index = malformed.rows.indexOf(row);
  malformed.rows.splice(index, 1);
  const target = malformed.rows.findIndex(
    (r) =>
      malformed.graph.text(malformed.graph.field(r.record, "kind")) ===
      "CC-054.knowledge_state",
  );
  malformed.rows.splice(target, 0, row);
  await rebuild(malformed);
  server.history = new Map([["1", malformed]]);
  await assert.rejects(
    client.load(discovery(server), "AsKnown", "1"),
    /后续关系来源不同/,
  );
  assert.equal(client.committed, before);
});
test("原件namespace/修订链篡改与空Delta未显示观察字段变化均保旧状态", async () => {
  const { server, client } = await setup(),
    before = client.committed;
  for (const kind of ["namespace", "revision"]) {
    const base = await makeBase(),
      row = base.rows.find((r) => r.family === "raw_history");
    await replaceWireRecord(base, row, (r) => {
      if (kind === "namespace") r.source_namespace = "another-sealed-source";
      else r.supersedes_revision = "1";
    });
    server.history = new Map([["1", base]]);
    await assert.rejects(
      client.load(discovery(server), "AsKnown", "1"),
      kind === "namespace" ? /namespace/ : /修订链/,
    );
    assert.equal(client.committed, before);
  }
  const changed = await makeBase(),
    observation = changed.rows.find((r) => r.family === "observations");
  await replaceWireRecord(changed, observation, (r) => {
    r.detail.unshown_source_annotation = "changed";
  });
  server.history = new Map([["1", changed]]);
  server.base = changed;
  await assert.rejects(client.watch(), /空Delta完整projection改变/);
  assert.equal(client.committed, before);
});
test("既有单script静态入口的Browser bundle自包含，缺具名资源拒绝且不发Python发现", async () => {
  const vm = require("node:vm"),
    elements = new Map(),
    requests = [];
  function element() {
    return {
      innerHTML: "",
      textContent: "",
      value: "3",
      checked: true,
      disabled: false,
      dataset: {},
      classList: { toggle() {} },
      addEventListener() {},
      focus() {},
      scrollIntoView() {},
      replaceChildren() {},
      append() {},
    };
  }
  const document = {
    getElementById(id) {
      if (!elements.has(id)) elements.set(id, element());
      return elements.get(id);
    },
    querySelector(id) {
      return this.getElementById(id);
    },
    createElement() {
      const e = element();
      Object.defineProperty(e, "innerHTML", {
        get() {
          return this.textContent;
        },
      });
      return e;
    },
  };
  const context = vm.createContext({
    document,
    TextEncoder,
    TextDecoder,
    crypto: crypto.webcrypto,
    performance,
    setTimeout,
    clearTimeout,
    console,
    location: { search: "?protocol=s-session/3" },
    URLSearchParams,
    fetch: async (url) => {
      requests.push(url);
      throw Error("should not fetch");
    },
  });
  vm.runInContext(
    fs.readFileSync(__dirname + "/../browser/tb01c-client.js", "utf8"),
    context,
  );
  assert(context.TB01C.Q3 && context.Q3Typed && context.SContentJobV3);
  const html = fs.readFileSync(__dirname + "/../browser/index.html", "utf8");
  assert.deepEqual(
    [...html.matchAll(/<script src="([^"]+)"/g)].map((m) => m[1]),
    ["/tb01c-client.js"],
  );
  const source = html
    .match(/<script>([\s\S]*?)<\/script>/)[1]
    .replace(/load\("current"\);\s*$/, "");
  vm.runInContext(source, context);
  await vm.runInContext('load("current")', context);
  assert.equal(requests.length, 0);
  assert.match(
    elements.get("stream-result").innerHTML,
    /s-query-client-resources\/3/,
  );
});

test("Browser bundle固定输入与既有runtime字节漂移锁", () => {
  const result = require("./build_tb01c_browser.cjs").run();
  assert.equal(result.status, "passed");
  assert.equal(
    result.inputs["s-content-v3.js"],
    "fb21d3467a189ac1d09cd585b07ca927083bee550fdd2c50419eac82b20ba926",
  );
});
test("正式Controller双fresh页31原件同源根与当前完整消费者一致", async () => {
  const server = await makeServer(),
    client = makeClient(server);
  await client.load(discovery(server), "RecomputedWithRevision", null);
  for (const p of original.controller_snapshots) {
    assert.deepEqual(p.fixed_cut, client.committed.fixed);
    assert.equal(p.cut_projection_digest, server.base.digest);
    assert.deepEqual(p.rows, server.base.rows);
    assert.equal(p.done, true);
    assert.equal(p.rows.length, 7);
  }
  assert.equal(client.committed.allocation_test_only, true);
  assert.match(client.committed.allocation_measurement_source, /not Browser/);
  assert.match(client.committed.resources_sha256, /^[a-f0-9]{64}$/);
});
test("原页面Q3实际调用和原七/八列布局由完整图视图驱动，模拟测量明确标识", async () => {
  const vm = require("node:vm"),
    server = await makeServer(),
    elements = new Map();
  function element(tag = "div") {
    return {
      tag,
      children: [],
      innerHTML: "",
      textContent: "",
      value: "1",
      checked: true,
      disabled: false,
      dataset: {},
      classList: { toggle() {} },
      addEventListener() {},
      focus() {},
      scrollIntoView() {},
      replaceChildren(...nodes) {
        this.children = nodes;
      },
      append(...nodes) {
        this.children.push(...nodes);
      },
    };
  }
  const document = {
    getElementById(id) {
      if (!elements.has(id)) elements.set(id, element());
      return elements.get(id);
    },
    querySelector(id) {
      return this.getElementById(id);
    },
    createElement(tag) {
      const e = element(tag);
      Object.defineProperty(e, "innerHTML", {
        get() {
          return this.textContent;
        },
      });
      return e;
    },
  };
  const context = vm.createContext({
    document,
    TextEncoder,
    TextDecoder,
    crypto: crypto.webcrypto,
    performance,
    setTimeout,
    clearTimeout,
    console,
    AbortController,
    URLSearchParams,
    fetch: server.fetch,
    SQueryClientContext: {
      protocol: "s-session/3",
      resources: CONFIG,
      allocation: measurement(),
    },
  });
  vm.runInContext(
    fs.readFileSync(__dirname + "/../browser/tb01c-client.js", "utf8"),
    context,
  );
  const html = fs.readFileSync(__dirname + "/../browser/index.html", "utf8"),
    source = html
      .split("<script>")[1]
      .split("</script>")[0]
      .replace('load("current");', "");
  vm.runInContext(source, context);
  await vm.runInContext('load("current")', context);
  const committed = vm.runInContext("v2Client.committed", context);
  assert(committed, "页面完整候选实际提交");
  await vm.runInContext("renderQ3(v2Client.committed,pageEpoch)", context);
  assert.equal(
    elements.get("#catalog-table tbody").children[0].children.length,
    7,
  );
  const rawTable = elements
    .get("raw-history")
    .children.find((e) => e.tag === "table");
  assert.equal(rawTable.children[0].children[0].children.length, 8);
  assert.equal(rawTable.children[1].children[0].children.length, 8);
  assert.match(elements.get("stream-result").textContent, /模拟分配测量/);
  assert(
    server.requests.every(
      (r) => r.url === "/api/ready" || r.url.startsWith("/api/v3/"),
    ),
  );
});
test("基线测量属于operation绝对期限，迟到测量与取消不触发读取/安装", async () => {
  const server = await makeServer(),
    allocation = measurement();
  let resolve;
  allocation.measure = () => new Promise((r) => (resolve = r));
  const config = {
      ...CONFIG,
      timeoutMs: 20,
      job_limits: { ...CONFIG.job_limits, max_elapsed_ms: "20" },
    },
    client = new API.Client({
      protocol: "s-session/3",
      fetch: server.fetch,
      limits: config,
      allocation,
    });
  await assert.rejects(
    client.load(discovery(server), "AsKnown", "1"),
    /基线测量.*期限/,
  );
  resolve({
    exact: true,
    source: allocation.measurementSource,
    store_bytes: "128",
    staged_bytes: "16",
  });
  await Promise.resolve();
  assert.equal(client.committed, null);
  assert.equal(server.requests.length, 0);
});
test("取消发生于read await内仍先永久记录实际尾wire，旧cut/cursor不安装", async () => {
  const { server, client } = await setup(),
    before = client.committed;
  let entered, release, tailLength;
  const blocked = new Promise((r) => (entered = r));
  client.fetch = async (url, options) => {
    const response = await server.fetch(url, options),
      bytes = new Uint8Array(await response.arrayBuffer());
    tailLength = bytes.length;
    return {
      ok: true,
      status: 200,
      body: {
        getReader() {
          return {
            read() {
              entered();
              return new Promise(
                (r) => (release = () => r({ done: false, value: bytes })),
              );
            },
            cancel: async () => {},
            releaseLock() {},
          };
        },
      },
    };
  };
  const loading = client.load(discovery(server), "AsKnown", "1");
  await blocked;
  const job = client.lastJob,
    beforeWire = BigInt(job.snapshot().wire_bytes);
  client.cancel();
  release();
  await assert.rejects(loading, /旧操作|取消/);
  const after = job.snapshot();
  assert.equal(BigInt(after.wire_bytes), beforeWire + BigInt(tailLength));
  assert.equal(after.stopped, "job_cancelled");
  assert.equal(client.committed, before);
  assert.deepEqual(client.committed.cursor, before.cursor);
});
test("witness逆投影完整wire前件拒绝任何price/未知键/缺字段，不能吞成独立原件", async () => {
  const server = await makeServer(await makeS16Base("3")),
    client = makeClient(server);
  await client.load(discovery(server), "AsKnown", "3");
  const before = client.committed;
  for (const mutate of [
    (bar) => (bar.price = bar.close),
    (bar) => (bar.unknown_original_field = "extra"),
    (bar) => delete bar.input_revision,
  ]) {
    const base = await makeS16Base("3"),
      row = base.rows.find((r) => r.family === "witnesses");
    await replaceWireRecord(base, row, (w) => mutate(w.raw_bars[0]));
    await base.graph.verifyComplete(base.ref);
    server.history = new Map([["3", base]]);
    await assert.rejects(
      client.load(discovery(server), "AsKnown", "3"),
      /OHLC原始历史.*完整字段/,
    );
    assert.equal(client.committed, before);
  }
});
test("七Delta顶层逐visit先永久计费：删replaces拒绝前已读集合不能账为零", async () => {
  const server = await orderedServer(),
    client = makeClient(server);
  await client.load(discovery(server), "RecomputedWithRevision", "3");
  const before = client.committed,
    saved = server.deltas[0];
  let text = "";
  for await (const c of server.head.graph.renderChunks(saved.delta)) text += c;
  const d = JSON.parse(text);
  d.replaces.pop();
  const changed = await C.encode(C.fromPlain(d));
  for (const node of changed.nodes) server.head.nodes.set(node.root, node);
  for (const base of server.history.values())
    base.graph = await C.GraphView.create([...server.head.nodes.values()]);
  server.deltas[0] = { ...saved, delta: changed.ref };
  await assert.rejects(client.watch(), /replaces/);
  const expected =
    [
      "upserts",
      "withdrawals",
      "replaces",
      "witnesses",
      "relations",
      "observations",
      "raw_history_added",
    ].reduce((n, key) => n + BigInt(d[key].length), 0n) +
    BigInt(d.withdrawals.length + d.replaces.length);
  assert.equal(BigInt(client.lastJob.snapshot().records), expected);
  assert.equal(client.committed, before);
});
for (const stop of ["cap", "clock"])
  test(
    stop + "永久停止后read真实尾量保同operation账，不能复活/安装",
    async () => {
      const server = await makeServer(),
        allocation = measurement();
      let entered,
        release,
        tailLength,
        clockBad = false;
      const reached = new Promise((r) => (entered = r)),
        clock = () => {
          if (clockBad) throw Error("clock unavailable");
          return Math.floor(performance.now());
        };
      const config =
        stop === "cap"
          ? {
              ...CONFIG,
              job_limits: {
                ...CONFIG.job_limits,
                max_total_wire_bytes: "100000",
              },
            }
          : CONFIG;
      const client = new API.Client({
        protocol: "s-session/3",
        fetch: async (url, options) => {
          const response = await server.fetch(url, options),
            bytes = new Uint8Array(await response.arrayBuffer());
          tailLength = bytes.length;
          return {
            ok: true,
            status: 200,
            body: {
              getReader() {
                return {
                  read() {
                    entered();
                    return new Promise(
                      (r) => (release = () => r({ done: false, value: bytes })),
                    );
                  },
                  cancel: async () => {},
                  releaseLock() {},
                };
              },
            },
          };
        },
        limits: config,
        allocation,
        clock,
      });
      const loading = client.load(discovery(server), "AsKnown", "1");
      await reached;
      const job = client.lastJob,
        beforeWire = BigInt(job.snapshot().wire_bytes);
      let locallyRead = 0n;
      if (stop === "cap") {
        const bytes = fs.readFileSync(
          __dirname + "/fixtures/q3-complete-s16-cuts345.json",
        );
        locallyRead = BigInt(bytes.length);
        assert.throws(
          () => job.chargeWireBytes(locallyRead),
          /max_total_wire_bytes/,
        );
      } else {
        clockBad = true;
        assert.throws(() => job.checkDeadline(), /时钟/);
      }
      release();
      await assert.rejects(
        loading,
        stop === "cap" ? /max_total_wire_bytes/ : /时钟/,
      );
      const receipt = job.snapshot();
      assert.equal(
        BigInt(receipt.wire_bytes),
        beforeWire + locallyRead + BigInt(tailLength),
      );
      assert.equal(receipt.requests, "1");
      assert.equal(receipt.records, "0");
      assert.equal(
        receipt.stopped,
        stop === "cap" ? "job_limit_exceeded" : "job_clock",
      );
      assert.match(receipt.accounting_operation, /^[0-9]+$/);
      if (stop === "clock") {
        assert.equal(receipt.core_metrics_verified, false);
        assert(receipt.core_snapshot_error);
      }
      clockBad = false;
      assert.throws(() => job.preIoCheck());
      await assert.rejects(job.acceptNode(original.nodes[0]));
      assert.equal(client.committed, null);
      assert.equal(server.requests.length, 1);
    },
  );
test("目录选择拥有独立epoch：A的kind await晚到不能追加B视图", async () => {
  const vm = require("node:vm"),
    elements = new Map();
  function element() {
    return {
      children: [],
      textContent: "",
      value: "",
      checked: true,
      dataset: {},
      classList: { toggle() {} },
      addEventListener() {},
      append(...nodes) {
        this.children.push(...nodes);
      },
      replaceChildren(...nodes) {
        this.children = nodes;
      },
    };
  }
  const document = {
    getElementById(id) {
      if (!elements.has(id)) elements.set(id, element());
      return elements.get(id);
    },
    createElement: element,
    querySelector(id) {
      return this.getElementById(id);
    },
  };
  const context = vm.createContext({
    document,
    TextEncoder,
    TextDecoder,
    setTimeout,
    clearTimeout,
    performance,
    console,
    crypto: crypto.webcrypto,
  });
  vm.runInContext(
    fs.readFileSync(__dirname + "/../browser/tb01c-client.js", "utf8"),
    context,
  );
  const html = fs.readFileSync(__dirname + "/../browser/index.html", "utf8");
  vm.runInContext(
    html
      .split("<script>")[1]
      .split("</script>")[0]
      .replace('load("current");', ""),
    context,
  );
  await vm.runInContext(
    `(async()=>{const V=TB01C.Q3.V;let release,reached;globalThis.oldReadGate=new Promise(r=>release=r);globalThis.oldReadReached=new Promise(r=>reached=r);globalThis.releaseOld=release;let held=false;const a={async field(k){if(k==='kind'&&!held){held=true;reached();await oldReadGate;}return V.of(({kind:'A.kind',object_id:'A.id'})[k]);}},b={async field(k){return V.of(({kind:'B.kind',object_id:'B.id'})[k]);}},view={state:V.of({catalog:{items:[{id:'CC-A',evidence:{object_ids:['A.id']}},{id:'CC-B',evidence:{object_ids:['B.id']}}]}}),families:{objects:[a,b],withdrawn_objects:[],witnesses:[],relations:[]},fixed:{structure_cut:'cut-3',history_mode:'AsKnown'}};q3Candidate=view;v2Client={committed:view};selectedCatalogId='CC-A';globalThis.oldRender=renderQ3Objects(view,pageEpoch);await oldReadReached;selectedCatalogId='CC-B';await renderQ3Objects(view,pageEpoch);})()`,
    context,
  );
  vm.runInContext("releaseOld()", context);
  await vm.runInContext("oldRender", context);
  const labels = elements
    .get("objects")
    .children.map((e) => e.children[0]?.textContent);
  assert(labels.includes("B.kind · B.id"));
  assert(!labels.includes("A.kind · A.id"));
  assert.match(elements.get("view-selection").textContent, /CC-B/);
});
test("cancel正处于pure Job异步Hash时立即阻止节点准入，live正控仍准入", async () => {
  const server = await makeServer(),
    results = [];
  for (const cancelled of [false, true]) {
    const client = makeClient(server),
      base = server.base,
      anchor = {
        kind: "projection",
        fixed_cut: base.fixed,
        cut_projection_digest: base.digest,
        projection_ref: base.ref,
      },
      node = base.nodes.values().next().value;
    let error = null;
    try {
      await client.run(async (budget) => {
        const reader = client.reader(anchor, base.fixed, budget),
          pending = reader.accept(node);
        if (cancelled) client.cancel();
        await pending;
      });
    } catch (e) {
      error = String(e);
    }
    results.push({
      cancelled,
      error,
      snapshot: client.lastJob.snapshot(),
      committed: client.committed,
    });
  }
  assert.equal(results[0].snapshot.nodes, "1");
  assert.equal(results[1].snapshot.nodes, "0");
  assert.equal(results[1].snapshot.node_bytes, "0");
  assert.match(results[1].error, /取消/);
  assert.equal(results[1].committed, null);
});
