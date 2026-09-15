// #1372：正式 HTML 校验器 + 正式 v2 collector。HTTP 替身使用历史 B 字节构造 v2 帧，非真实 C/GUI 验收。
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '../..');
const clientPath = path.join(root, 's_session/browser/tb01c-client.js');
const htmlPath = path.join(root, 's_session/browser/index.html');
const API = require(clientPath);
const hashBytes = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
function canonical(value) {
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  if (Array.isArray(value)) return '[' + value.map(canonical).join(',') + ']';
  return '{' + Object.keys(value).sort((a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b)))
    .map(k => JSON.stringify(k) + ':' + canonical(value[k])).join(',') + '}';
}
const hash = value => hashBytes(canonical(value));
const copy = value => JSON.parse(JSON.stringify(value));
function htmlContext(extra = {}) {
  const elements = new Map();
  function element() { return {innerHTML: '', textContent: '', value: '3', checked: true, disabled: false,
    classList: {toggle() {}}, addEventListener() {}, focus() {}, scrollIntoView() {}}; }
  const document = {getElementById(id) { if (!elements.has(id)) elements.set(id, element()); return elements.get(id); },
    querySelector(q) { return this.getElementById(q); }, createElement() { const e = element();
      Object.defineProperty(e, 'innerHTML', {get() { return this.textContent; }}); return e; }};
  const context = vm.createContext({document, structuredClone, console, setTimeout, clearTimeout, performance, ...extra});
  const source = fs.readFileSync(htmlPath, 'utf8').match(/<script>([\s\S]*?)<\/script>/)[1].replace(/load\("current"\);\s*$/, '');
  vm.runInContext(source, context);
  return {context, elements};
}
// root 真实 HTTP driver 可复用这三份实际 HTML 函数；不存在测试放行实现。
function makeHooks() {
  // 与真实页面的静态脚本绑定一致；HTTP helper 不能缺少新域正式校验模块。
  const {context} = htmlContext({TB01C: API});
  return Object.fromEntries(['validateState', 'validateDelta', 'comparePublication'].map(k => [k, context[k]]));
}
// 原历史样本不足31行：测试页3迫使跨页；生产默认页31另有具名正控。
const TEST_PAGE_SIZE = 3;
const families = ['objects', 'withdrawn_objects', 'witnesses', 'relations', 'observations', 'raw_history'];
function primary(family, row) {
  return family === 'objects' || family === 'withdrawn_objects' ? [row.object_id] :
    family === 'relations' ? [row.subject, row.relation_type, row.object] :
    family === 'raw_history' ? [row.identity_key, row.revision] : [family === 'witnesses' ? row.witness_id : row.observation_id];
}
function order(family, r) {
  switch (family) {
    case 'objects': return Object.hasOwn(r, 'fact_key') ? [1n, r.kind, r.fact_key, r.object_id, BigInt(r.object_revision)] : [0n, BigInt(r.window_start), BigInt(r.window_mid), BigInt(r.window_end), r.object_id, BigInt(r.object_revision)];
    case 'withdrawn_objects': return [BigInt(r.withdrawn_generation), r.object_id, BigInt(r.object_revision)];
    case 'witnesses': return [r.object_id, BigInt(r.slot), r.witness_id];
    case 'relations': return [r.subject, r.relation_type, r.object];
    case 'observations': return [r.kind, r.window_start === null ? -1n : BigInt(r.window_start), r.observation_id];
    default: return [BigInt(r.seq), r.identity_key, BigInt(r.revision)];
  }
}
function projection(state, mode) {
  const s = state.snapshot;
  const fixed = Object.fromEntries(['session_id', 'structure_cut', 'catalog_revision', 'index_frontier', 'profile_id', 'profile_hash', 'input_frontier', 'seq_range', 'catalog_run_status', 'catalog_evidence', 'scope'].map(k => [k, copy(s[k])]));
  Object.assign(fixed, {session_generation: '1', cut_generation: s.generation, rule_revision: s.catalog_evidence.rule_revision,
    history_mode: mode, as_of_generation: mode === 'AsKnown' ? s.generation : null});
  const rows = [], counts = {};
  for (const family of families) {
    const records = copy(s[family]).sort((a, b) => {
      const aa = order(family, a), bb = order(family, b);
      for (let i = 0; i < aa.length; i++) {
        const c = typeof aa[i] === 'string' ? Buffer.compare(Buffer.from(aa[i]), Buffer.from(bb[i])) : aa[i] < bb[i] ? -1 : aa[i] > bb[i] ? 1 : 0;
        if (c) return c;
      }
      return 0;
    });
    for (const record of records) rows.push({family, key: primary(family, record), record});
    counts[family] = String(records.length);
  }
  counts.total = String(rows.length);
  return {fixed_cut: fixed, catalog: copy(state.catalog), rows, counts, scope: copy(s.scope), omissions: [], order_version: s.profile_id === 'ohlc_integer_tb02a_v1' ? 's-record-order/2' : 's-record-order/1'};
}
function token(p, offset, pageSize = TEST_PAGE_SIZE) {
  const t = Object.fromEntries(['session_id', 'session_generation', 'cut_generation', 'scope', 'history_mode', 'structure_cut', 'catalog_revision', 'index_frontier', 'profile_hash', 'rule_revision'].map(k => [k, copy(p.fixed_cut[k])]));
  Object.assign(t, {token_schema: 's-snapshot-token/1', order_version: p.order_version, page_size: String(pageSize), offset: String(offset),
    cut_projection_digest: hash(p), issued_capture_digest: 'a'.repeat(64)});
  t.token_checksum = hash(t); return t;
}
function page(p, offset, pageSize = TEST_PAGE_SIZE) {
  const end = Math.min(p.rows.length, offset + pageSize);
  return {...copy(p), rows: copy(p.rows.slice(offset, end)), offset: String(offset), done: end === p.rows.length,
    next_token: end === p.rows.length ? null : token(p, end, pageSize), cut_projection_digest: hash(p), validated_capture_digest: 'a'.repeat(64), ok: true};
}
function cursor(f) { return {session_id: f.session_id, session_generation: f.session_generation, catalog_revision: f.catalog_revision,
  scope: copy(f.scope), after_generation: f.cut_generation, base_cut: f.structure_cut, index_frontier: f.index_frontier,
  last_seq: f.input_frontier, order_version: 's-record-order/1'}; }
function seal(request, payload, fixed) {
  const digest = hash(payload);
  return {schema_revision: 's-session/2', session_id: fixed.session_id, session_generation: fixed.session_generation,
    source_namespace: 's-observe/replies', source_epoch: '2', message_id: 'reply-' + hash([request.source_namespace, request.source_epoch, request.message_id, digest]),
    producer_id: 'Q:' + fixed.session_id, producer_epoch: '2', payload_hash: digest,
    causal_refs: [Object.fromEntries(['source_namespace', 'source_epoch', 'message_id', 'payload_hash'].map(k => [k, request[k]]))], payload};
}
function fixtures() {
  const bytes = fs.readFileSync(path.join(__dirname, 'fixtures/r12_consumer.json')), records = JSON.parse(bytes).records;
  const get = name => { assert.equal(hashBytes(records[name].body), records[name].sha256); return JSON.parse(records[name].body); };
  const a3 = get('a3'), a4 = get('a4'), b1 = get('b1');
  return {a3, a4, b1, delta: get('watch3').deltas.find(d => d.generation === '4'), fixtureHash: hashBytes(bytes)};
}
// —— #1394：真实 S64 前缀的 changes-only/1 稀疏增量（浏览器 red 反例的入仓最小副本） ——
// 传输替身用真实 Q read_state/read_delta 导出字节构造 v2 帧，非真正 HTTP/GUI 验收；
// 整包来源与 SHA-256 记在 fixture.provenance，仓内测试不引用仓外路径。
const SPARSE_FIXTURE = 's_session/tests/fixtures/tb03b-changes-only-browser.json';
const SPARSE_SOURCE_SHA256 = 'b3bc365ff328689888ae477769d76bdd3272bf0b717bdd387ba779b3cab72a21';
function sparseFixture() {
  const bytes = fs.readFileSync(path.join(root, SPARSE_FIXTURE));
  const bundle = JSON.parse(new TextDecoder('utf-8', {fatal: true}).decode(bytes));
  assert.equal(bundle.provenance.source_sha256, SPARSE_SOURCE_SHA256);
  assert.equal(bundle.provenance.source_bytes, 587962);
  assert.deepEqual(Object.keys(bundle.states), ['3', '4', '5']);
  assert.deepEqual(bundle.deltas.map(d => d.generation), ['4', '5']);
  return {bundle, fixtureHash: hashBytes(bytes)};
}
function sparseState(sp, gen) { const state = sp.states[gen]; assert(state, '真实前缀缺状态 ' + gen); return state; }
function sparseDelta(sp, gen) { const delta = sp.deltas.find(d => d.generation === gen); assert(delta, '真实前缀缺增量 ' + gen); return delta; }
function sparseDiscovery(sp) {
  return {cut: {session_id: sparseState(sp, '3').snapshot.session_id, session_generation: '1'},
    snapshot: {profile_id: sparseState(sp, '3').snapshot.profile_id}};
}
// profile=ohlc_integer_tb02a_v1 ⇒ s-record-order/2；与旧档默认 order/1 分开写，不做一档兼容。
function sparseCursor(fixed) {
  return {session_id: fixed.session_id, session_generation: fixed.session_generation, catalog_revision: fixed.catalog_revision,
    scope: copy(fixed.scope), after_generation: fixed.cut_generation, base_cut: fixed.structure_cut,
    index_frontier: fixed.index_frontier, last_seq: fixed.input_frontier, order_version: 's-record-order/2'};
}
function sparseServer(sp) {
  const server = {mutate: null, requests: []};
  server.fetch = async (url, options) => {
    const request = JSON.parse(options.body), payload = request.payload;
    assert.equal(url, '/api/v2/' + payload.op); assert.equal(request.payload_hash, hash(payload));
    server.requests.push({url, request});
    let response, fixed;
    if (payload.op === 'snapshot') {
      const t = payload.snapshot_token, gen = t ? t.cut_generation : (payload.as_of_generation || '3');
      const p = projection(sparseState(sp, gen), t ? t.history_mode : payload.history_mode), offset = t ? Number(t.offset) : 0;
      const pageSize = Number(t ? t.page_size : payload.page_size);
      if (t) assert.deepEqual(t, token(p, offset, pageSize));
      response = page(p, offset, pageSize); fixed = p.fixed_cut;
    } else {
      assert.equal(payload.op, 'watch');
      const head = projection(sparseState(sp, '5'), 'RecomputedWithRevision').fixed_cut;
      response = {deltas: [copy(sparseDelta(sp, '4')), copy(sparseDelta(sp, '5'))], next_cursor: sparseCursor(head), observed_head: head,
        delivery_frontier: {policy_revision: 's-delivery/1', retain_generations: '8', first_available_generation: '3', head_generation: '5'},
        has_more: false, gap: null, validated_capture_digest: 'a'.repeat(64), ok: true};
      fixed = head;
    }
    if (server.mutate) server.mutate(response, request);
    return new Response(canonical(seal(request, response, fixed)), {status: 200, headers: {'Content-Type': 'application/json'}});
  };
  return server;
}
// 旧 Watch（非 v2 客户端）走 /api/state + /api/delta；Gap 分支先按 cut-3 重建缓存，再续接稀疏 4/5。
function sparseLegacy(sp, deltaHook = null) {
  const sessionId = sparseState(sp, '3').snapshot.session_id;
  const json = body => ({ok: true, status: 200, json: async () => copy(body)});
  const fetchImpl = async url => {
    if (url === '/api/state' || url === '/api/state?as_of=3') return json(sparseState(sp, '3'));
    if (url === '/api/state?as_of=5') return json(sparseState(sp, '5'));
    if (url === '/api/delta?after_generation=0')
      return json({session_id: sessionId, gap: {reason: 'delivery_retention_gap', rebuild_cut: 'cut-3'}});
    if (url === '/api/delta?after_generation=3') {
      const payload = {session_id: sessionId, deltas: [copy(sparseDelta(sp, '4')), copy(sparseDelta(sp, '4')), copy(sparseDelta(sp, '5'))]};
      if (deltaHook) deltaHook(payload);
      return json(payload);
    }
    throw new Error('旧 Watch 请求了未声明的地址：' + url);
  };
  return htmlContext({TB01C: API, fetch: fetchImpl});
}
function stubClient() {
  return new API.Client({fetch: async () => { throw new Error('本用例不应发起网络读取'); },
    hooks: {validateState() {}, validateDelta() {}, comparePublication() {}}});
}
function fakeServer(f) {
  const server = {mutate: null, wire: null, gap: false, requests: [], beforeReply: null};
  server.fetch = async (url, options) => {
    const request = JSON.parse(options.body), payload = request.payload;
    assert.equal(request.schema_revision, 's-session/2'); assert.equal(request.payload_hash, hash(payload));
    assert.equal(options.body, canonical(request)); assert.equal(url, '/api/v2/' + payload.op);
    assert.equal(options.method, 'POST'); assert.deepEqual(request.causal_refs, []);
    server.requests.push({url, request});
    if (server.beforeReply) await server.beforeReply(request);
    let response, fixed;
    if (payload.op === 'snapshot') {
      const t = payload.snapshot_token;
      if (t) assert.deepEqual(Object.keys(payload).sort(), ['op', 'snapshot_token']);
      const gen = t ? t.cut_generation : payload.as_of_generation || '3';
      const sid = t ? t.session_id : payload.session_id;
      const state = sid === f.b1.snapshot.session_id ? f.b1 : gen === '4' ? f.a4 : f.a3;
      const p = projection(state, t ? t.history_mode : payload.history_mode), offset = t ? Number(t.offset) : 0;
      assert.equal(request.session_id, sid); assert.equal(request.session_generation, '1');
      const pageSize = Number(t ? t.page_size : payload.page_size);
      if (t) assert.deepEqual(t, token(p, offset, pageSize));
      response = page(p, offset, pageSize); fixed = p.fixed_cut;
    } else {
      const p = projection(server.gap ? f.b1 : f.a4, 'RecomputedWithRevision'); fixed = p.fixed_cut;
      response = {deltas: [copy(f.delta)], next_cursor: cursor(fixed), observed_head: fixed,
        delivery_frontier: {policy_revision: 's-delivery/1', retain_generations: '8', first_available_generation: '1', head_generation: fixed.cut_generation},
        has_more: false, gap: null, validated_capture_digest: 'a'.repeat(64), ok: true};
      if (server.gap) Object.assign(response, {deltas: [], next_cursor: copy(payload.cursor), gap: {requested_identity: copy(payload.cursor),
        reason: 'session_identity_changed', missing_range: null, rebuild: fixed, snapshot_token: token(p, 0)}});
    }
    if (server.mutate) server.mutate(response, request);
    let raw = canonical(seal(request, response, fixed));
    if (server.wire) raw = server.wire(raw, request);
    return new Response(raw, {status: 200, headers: {'Content-Type': 'application/json'}});
  };
  return server;
}
// 实际 OHLC Q 状态仅重封装分页公共头；故意重签摘要，测完整候选语义门而非传输hash门。
function actualTB02Server(state) {
  return async (url, options) => {
    const request = JSON.parse(options.body), payload = request.payload, t = payload.snapshot_token;
    assert.equal(url, '/api/v2/snapshot');
    const p = projection(state, t ? t.history_mode : payload.history_mode);
    const response = page(p, t ? Number(t.offset) : 0, Number(t ? t.page_size : payload.page_size));
    return new Response(canonical(seal(request, response, p.fixed_cut)), {status:200});
  };
}
async function run() {
  const argv = process.argv.slice(2);
  assert(argv.length === 0 || (argv.length === 2 && argv[0] === '--tb02-states'), '用法：node test_tb01c_browser.cjs [--tb02-states <正式Q状态JSON>]');
  const tb02Bytes = argv.length ? fs.readFileSync(path.resolve(argv[1])) : null;
  const tb02Value = tb02Bytes ? API.parse(new TextDecoder('utf-8', {fatal:true}).decode(tb02Bytes)) : null;
  const tb02States = tb02Value === null ? null : Array.isArray(tb02Value) ? tb02Value : [tb02Value];
  const f = fixtures(), result = {scope: 'TestOnly：真实B原件构造v2响应替身；非真实C/HTTP/GUI验收',
    test_page_size: TEST_PAGE_SIZE, production_page_size: API.DEFAULTS.pageSize,
    html_sha256: hashBytes(fs.readFileSync(htmlPath)), client_sha256: hashBytes(fs.readFileSync(clientPath)), fixture_sha256: f.fixtureHash, cases: []};
  const sparse = sparseFixture(), sp = sparse.bundle;
  result.sparse_fixture = {path: SPARSE_FIXTURE, fixture_sha256: sparse.fixtureHash, source_sha256: sp.provenance.source_sha256,
    source_bytes: sp.provenance.source_bytes, states: Object.keys(sp.states), deltas: sp.deltas.map(d => d.generation)};
  const discovery = {cut: {session_id: f.a3.snapshot.session_id, session_generation: '1'}};
  async function test(name, fn) { try { await fn(); result.cases.push({name, passed: true}); }
    catch (error) { result.cases.push({name, passed: false, error: error.stack}); } }
  async function setup() {
    const server = fakeServer(f), commits = [];
    const client = new API.Client({fetch: server.fetch, hooks: makeHooks(), limits: {pageSize: TEST_PAGE_SIZE}, onCommit: (c, e) => commits.push({c, e})});
    await client.load(discovery); return {client, server, commits};
  }
  await test('新笔逻辑slot与见证整数slot分别核型且不放宽其他整数', async () => {
    const {context} = htmlContext({TB01C: API});
    context.validateExactRecords([{slot:'0'}, {payload:{schema_revision:'s-new-bi/1',slot:'TB-02-B'}}]);
    for (const slot of ['TB-02-B', '01', 0, -1])
      assert.throws(() => context.validateExactRecords([{slot}]), /精确整数/);
    assert.throws(() => context.validateExactRecords([{payload:{schema_revision:'s-new-bi/1',slot:'TB-02-B',data:{merged_index:0}}}]), /精确整数/);
  });
  await test('完整多页只原子安装一次，记录与精确cursor同提交', async () => {
    const {client, commits, server} = await setup(); assert(client.committed.pages > 1); assert.equal(commits.length, 1);
    assert.deepEqual(copy(client.committed.projection), projection(f.a3, 'RecomputedWithRevision'));
    assert.deepEqual(copy(client.committed.cursor), cursor(client.committed.projection.fixed_cut));
    assert(server.requests.every(r => r.url === '/api/v2/snapshot'));
  });
  await test('正式默认页31及Watch4资源保持，完整快照可安装', async () => {
    const server = fakeServer(f), client = new API.Client({fetch: server.fetch, hooks: makeHooks()});
    await client.load(discovery); assert.equal(client.limits.pageSize, 31); assert.equal(client.limits.watchBatches, 4);
    assert.equal(client.limits.pollMs, 500); assert(server.requests.every(r => r.request.payload.page_size === '31'));
    assert.deepEqual(copy(client.committed.projection), projection(f.a3, 'RecomputedWithRevision'));
  });
  await test('默认fetch以全局接收者完成分页与Watch校验提交', async () => {
    const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'fetch'), server = fakeServer(f);
    Object.defineProperty(globalThis, 'fetch', {configurable: true, writable: true, value: async function (url, options) {
      assert.equal(this, globalThis, '默认fetch必须保留浏览器要求的全局接收者');
      return server.fetch(url, options);
    }});
    try {
      const client = new API.Client({hooks: makeHooks(), limits: {pageSize: TEST_PAGE_SIZE}});
      await client.load(discovery); assert(client.committed.pages > 1);
      await client.watch();
      assert.deepEqual(copy(client.committed.projection), projection(f.a4, 'RecomputedWithRevision'));
      assert(server.requests.some(r => r.url === '/api/v2/watch'));
    } finally {
      if (descriptor) Object.defineProperty(globalThis, 'fetch', descriptor); else delete globalThis.fetch;
    }
  });
  await test('显式注入fetch保持原函数与调用接口', async () => {
    const server = fakeServer(f);
    async function injected(url, options) {
      assert.equal(this, client);
      return server.fetch(url, options);
    }
    const client = new API.Client({fetch: injected, hooks: makeHooks()});
    assert.equal(client.fetch, injected);
    await client.load(discovery); await client.watch();
    assert.deepEqual(copy(client.committed.projection), projection(f.a4, 'RecomputedWithRevision'));
  });
  const pageMutations = {
    '跨页记录缺项': p => p.rows.pop(),
    '跨页重复记录': p => { if (p.rows.length > 1) p.rows[1] = copy(p.rows[0]); },
    '跨页顺序倒退': p => p.rows.reverse(),
    '完整投影摘要不符': p => { if (p.rows.length) p.rows[0].record.review_corruption = 'unexpected'; },
    '固定切面身份漂移': p => { p.fixed_cut.session_generation = '2'; },
    '完整六族计数漏项': p => { delete p.counts.raw_history; }
  };
  for (const [name, mutate] of Object.entries(pageMutations)) await test(name + '保持旧cut并可同页重试', async () => {
    const {client, server} = await setup(), before = canonical(client.committed);
    server.mutate = (p, r) => { if (r.payload.op === 'snapshot' && p.offset === String(TEST_PAGE_SIZE)) mutate(p); };
    await assert.rejects(client.load(discovery, 'RecomputedWithRevision', '4')); assert.equal(canonical(client.committed), before);
    server.mutate = null; await client.load(discovery, 'RecomputedWithRevision', '4'); assert.equal(client.committed.cursor.after_generation, '4');
  });
  await test('续页token不允许跳过offset，即使checksum重算', async () => {
    const {client, server} = await setup(), before = canonical(client.committed);
    server.mutate = p => { if (p.next_token) { p.next_token.offset = String(TEST_PAGE_SIZE + 1); delete p.next_token.token_checksum; p.next_token.token_checksum = hash(p.next_token); } };
    await assert.rejects(client.load(discovery)); assert.equal(canonical(client.committed), before);
  });
  const wireMutations = {
    '公共头因果身份错误': raw => { const r = JSON.parse(raw); r.causal_refs[0].message_id = 'other'; return canonical(r); },
    'payload摘要错误': raw => { const r = JSON.parse(raw); r.payload_hash = '0'.repeat(64); return canonical(r); },
    '重复JSON键': raw => raw.replace('"schema_revision":"s-session/2"', '"schema_revision":"s-session/2","schema_revision":"s-session/2"'),
    '精度丢失JSON数字': raw => raw.replace('"session_generation":"1"', '"session_generation":9007199254740993'),
    '孤立Unicode代理项': raw => raw.replace('"schema_revision":"s-session/2"', '"schema_revision":"\\ud800"'),
    '响应截断': raw => raw.slice(0, -7),
    '额外第二JSON帧': raw => raw + '\n{}'
  };
  for (const [name, mutate] of Object.entries(wireMutations)) await test(name + '不提交', async () => {
    const {client, server} = await setup(), before = canonical(client.committed); server.wire = mutate;
    await assert.rejects(client.watch()); assert.equal(canonical(client.committed), before);
  });
  await test('健康更正批次逐字段对拍后提交第四代', async () => {
    const {client, commits} = await setup(); await client.watch(); assert.equal(commits.length, 2);
    assert.deepEqual(copy(client.committed.projection), projection(f.a4, 'RecomputedWithRevision'));
  });
  const batchMutations = {
    '替代集合漏项': p => { p.deltas[0].delta.replaces = []; },
    '撤回原因被改写': p => { p.deltas[0].delta.withdrawals[0].reason = 'wrong'; },
    '批次乱序': p => { p.deltas[0].generation = '5'; p.deltas[0].delta.generation = '5'; },
    '空批次伪装到达当前head': p => { p.deltas = []; p.has_more = false; },
    'cursor前沿不符': p => { p.next_cursor.index_frontier = 'wrong'; },
    'batch额外未声明字段': p => { p.deltas[0].delta.ignored_operation = []; }
  };
  for (const [name, mutate] of Object.entries(batchMutations)) await test(name + '保持旧cut且健康重试成功', async () => {
    const {client, server} = await setup(), before = canonical(client.committed);
    server.mutate = (p, r) => { if (r.payload.op === 'watch') mutate(p); };
    await assert.rejects(client.watch()); assert.equal(canonical(client.committed), before);
    server.mutate = null; await client.watch(); assert.equal(client.committed.cursor.after_generation, '4');
  });
  await test('Gap以新身份原token完整分页后重建', async () => {
    const {client, server, commits} = await setup(); server.gap = true; await client.watch();
    assert.equal(client.committed.cursor.session_id, f.b1.snapshot.session_id); assert.equal(commits.at(-1).e.kind, 'gap');
    assert(server.requests.some(({request}) => request.payload.snapshot_token?.offset === '0' && request.session_id === f.b1.snapshot.session_id));
  });
  await test('Gap错目标页保持原身份并允许健康重试', async () => {
    const {client, server} = await setup(), before = canonical(client.committed); server.gap = true;
    server.mutate = (p, r) => { if (r.payload.op === 'snapshot') p.fixed_cut.session_id = f.a3.snapshot.session_id; };
    await assert.rejects(client.watch()); assert.equal(canonical(client.committed), before);
    server.mutate = null; await client.watch(); assert.equal(client.committed.cursor.session_id, f.b1.snapshot.session_id);
  });
  await test('迟到分页不能覆盖较新完整提交', async () => {
    const {client, server} = await setup(); let release, entered;
    const gate = new Promise(resolve => { release = resolve; }), ready = new Promise(resolve => { entered = resolve; });
    let first = true; server.beforeReply = async () => { if (first) { first = false; entered(); await gate; } };
    const old = client.load(discovery); const rejected = assert.rejects(old); await ready;
    await client.load(discovery, 'RecomputedWithRevision', '4'); release(); await rejected;
    assert.equal(client.committed.cursor.after_generation, '4');
  });
  await test('UTF8规范键序摘要与独立实现一致', async () => {
    const value = {'😀': '跨域', '\ue000': '私用区', nested: {'中': true, a: null}};
    assert.equal(await API.sha256(value), hash(value));
  });
  await test('浏览器完整目录生成摘要与冻结正本逐值无漂移', async () => {
    const declared = JSON.parse(fs.readFileSync(path.join(root, 's_session/catalog/signed-catalog.json'), 'utf8'));
    function wire(v) {
      if (typeof v === 'number') { assert(Number.isSafeInteger(v)); return String(v); }
      if (Array.isArray(v)) return v.map(wire);
      if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k,x])=>[k,wire(x)]));
      return v;
    }
    const items = Object.fromEntries(declared.items.map(i => [i.id, wire(Object.fromEntries(['kind','title','domain','branches'].map(k=>[k,i[k] ?? (k==='branches'?[]:'')])))]));
    assert.equal(API.TB02_CATALOG.catalog_revision, declared.catalog_revision);
    assert.equal(API.TB02_CATALOG.source_sha256, declared.source_sha256);
    assert.deepEqual(API.TB02_CATALOG.ids, Object.keys(items).sort());
    assert.equal(API.TB02_CATALOG.ids.length, declared.item_count);
    assert.equal(API.TB02_CATALOG.public_static_sha256, hash(items));
  });
  await test('无效新请求使在途旧分页失效且保留已提交cut', async () => {
    const {client, server} = await setup(), before = canonical(client.committed); let release, entered;
    const gate = new Promise(resolve => { release = resolve; }), ready = new Promise(resolve => { entered = resolve; });
    let first = true; server.beforeReply = async () => { if (first) { first = false; entered(); await gate; } };
    const old = client.load(discovery, 'RecomputedWithRevision', '4'), rejected = assert.rejects(old); await ready;
    await assert.rejects(client.load(discovery, 'AsKnown', '01')); release(); await rejected;
    assert.equal(canonical(client.committed), before);
  });
  await test('G0来源按显式协议校验，v1空profile与v2初始化profile互不兜底', async () => {
    const {context} = htmlContext();
    const old = {generation: '0', profile_id: '', profile_hash: '', history_mode: 'RecomputedWithRevision', as_of_generation: null};
    const initialized = {...old, profile_id: 'testonly', profile_hash: 'a'.repeat(64)};
    context.validateSource(old); context.validateSource(initialized, 's-session/2');
    assert.throws(() => context.validateSource(initialized)); assert.throws(() => context.validateSource(old, 's-session/2'));
  });
  await test('正式HTML发现v2后从分页安装，无legacy状态混入', async () => {
    const server = fakeServer(f), discoveryState = copy(f.a3); discoveryState.cut.session_generation = '1';
    let legacyReads = 0;
    const wrappedAPI = {...API, Client: class extends API.Client { constructor(options) { super({...options, fetch: server.fetch, limits: {pageSize: TEST_PAGE_SIZE}}); } }};
    const {context} = htmlContext({TB01C: wrappedAPI, fetch: async url => {
      assert.equal(url, '/api/state'); legacyReads++; return {ok: true, status: 200, json: async () => discoveryState};
    }});
    await vm.runInContext('load("current")', context);
    assert.equal(vm.runInContext('lastAppliedGen', context), '3');
    assert.equal(vm.runInContext('v2Client.committed.pages > 1', context), true);
    await vm.runInContext('applyStream()', context); assert.equal(vm.runInContext('lastAppliedGen', context), '4');
    assert.equal(legacyReads, 1);
  });
  // —— #1394：浏览器 red 反例的修复回归（真实 S 前缀 + changes-only/1 稀疏增量） ——
  await test('真实稀疏反例：原版按本代upserts核轴引用必红，改按本代完整活动集成立', async () => {
    const {context} = htmlContext({TB01C: API});
    for (const gen of ['4', '5']) {
      const dd = sparseDelta(sp, gen).delta;
      const referenced = Object.values(dd.catalog_evidence.axes).flatMap(a => a.evidence.object_ids);
      const resent = new Set(dd.upserts.map(o => o.object_id));
      assert(referenced.some(id => !resent.has(id)), '真实批次应为稀疏：轴引用的历史对象不在本代 upserts');
      // 原版判据（把本代 upserts 当完整活动集）在真实稀疏批次上就是这几条拒绝。
      assert.throws(() => context.TB01C.validateTB02Axes(dd.catalog_evidence.axes, dd.upserts), /目录引用缺失\/重复/);
      // 修复后的引用集是本代完整活动对象；形状/重复判据未放宽。
      assert.doesNotThrow(() => API.validateTB02Axes(dd.catalog_evidence.axes, sparseState(sp, gen).snapshot.objects));
    }
  });
  await test('真实第4/5代稀疏增量经v2客户端逐代核引用后提交', async () => {
    const server = sparseServer(sp), commits = [];
    const client = new API.Client({fetch: server.fetch, hooks: makeHooks(), limits: {pageSize: TEST_PAGE_SIZE},
      onCommit: (c, e) => commits.push({c, e})});
    await client.load(sparseDiscovery(sp));
    assert.equal(client.committed.cursor.after_generation, '3');
    assert.equal(client.committed.state.snapshot.objects.length, 9);
    await client.watch();
    assert.equal(commits.at(-1).e.kind, 'watch'); assert.equal(commits.at(-1).e.batches, 2);
    assert.equal(client.committed.cursor.after_generation, '5'); assert.equal(client.committed.cursor.base_cut, 'cut-5');
    assert.equal(client.committed.state.snapshot.objects.length, 15);
    assert.deepEqual(copy(client.committed.projection), projection(sparseState(sp, '5'), 'RecomputedWithRevision'));
    assert(server.requests.some(({url}) => url === '/api/v2/watch'));
  });
  await test('合法稀疏批次：目录引用已提交对象A与本批新增B均放行，无变化批次同样通过', async () => {
    const client = stubClient();
    const base = copy(sparseState(sp, '4').snapshot), delta = copy(sparseDelta(sp, '5'));
    const referenced = [...new Set(Object.values(delta.delta.catalog_evidence.axes).flatMap(a => a.evidence.object_ids))];
    const resent = new Set(delta.delta.upserts.map(o => o.object_id));
    const inherited = referenced.filter(id => !resent.has(id));
    assert(inherited.length > 0, '真实稀疏批次应含未重发但被目录引用的历史对象A');
    assert(inherited.every(id => base.objects.some(o => o.object_id === id)), '未重发对象A应已在已提交活动集');
    assert(delta.delta.upserts.some(o => referenced.includes(o.object_id)), '本批新增B应被目录引用');
    // clone 走客户端 parse（原型为 null），对拍前按测试侧 copy 归一，语义门仍是六族全字段/全序。
    assert.deepEqual(API.rowsOf(copy(client.applyBatch(base, delta))), API.rowsOf(copy(sparseState(sp, '5').snapshot)));
    const idle = copy(sparseDelta(sp, '5'));
    for (const field of ['upserts', 'withdrawals', 'replaces', 'witnesses', 'relations', 'observations', 'raw_history_added']) idle.delta[field] = [];
    assert.deepEqual(API.rowsOf(copy(client.applyBatch(copy(sparseState(sp, '5').snapshot), idle))), API.rowsOf(copy(sparseState(sp, '5').snapshot)));
  });
  await test('真正缺失引用、撤回后仍引用旧对象、同轴重复引用均拒绝', async () => {
    const client = stubClient(), base = copy(sparseState(sp, '4').snapshot);
    const axisIds = delta => delta.delta.catalog_evidence.axes['CC-006'].evidence.object_ids;
    const absent = copy(sparseDelta(sp, '5'));
    axisIds(absent).push('obj-1394-not-present');
    assert.throws(() => client.applyBatch(base, absent), /目录引用缺失\/重复/);
    const gone = base.withdrawn_objects[0].object_id;
    assert(!base.objects.some(o => o.object_id === gone), '撤回对象不应仍在活动集');
    const withdrawn = copy(sparseDelta(sp, '5'));
    axisIds(withdrawn).push(gone);
    assert.throws(() => client.applyBatch(base, withdrawn), /目录引用缺失\/重复/);
    const duplicated = copy(sparseDelta(sp, '5'));
    axisIds(duplicated).push(axisIds(duplicated)[0]);
    assert.throws(() => client.applyBatch(base, duplicated), /目录引用缺失\/重复/);
  });
  await test('传统全量批次（legacy profile）不触发TB02轴判据且六族结果不变', async () => {
    const client = stubClient();
    const applied = client.applyBatch(copy(f.a3.snapshot), copy(f.delta));
    assert.deepEqual(API.rowsOf(copy(applied)), API.rowsOf(copy(f.a4.snapshot)));
    assert.equal(f.delta.delta.catalog_evidence.profile_id, f.a3.snapshot.profile_id);
    assert(!Object.hasOwn(f.delta.delta.catalog_evidence, 'axes'), '旧档证据不含TB02逐轴结构');
  });
  await test('中间代轴引用非法即使末代补齐仍拒绝且不提交，健康重试可续', async () => {
    const server = sparseServer(sp), client = new API.Client({fetch: server.fetch, hooks: makeHooks(), limits: {pageSize: TEST_PAGE_SIZE}});
    await client.load(sparseDiscovery(sp));
    const before = canonical(client.committed);
    const cc6 = gen => sparseState(sp, gen).snapshot.catalog_evidence.axes['CC-006'].evidence.object_ids;
    const only5 = sparseState(sp, '5').snapshot.objects.map(o => o.object_id)
      .filter(id => !sparseState(sp, '4').snapshot.objects.some(o => o.object_id === id))
      .filter(id => !cc6('4').includes(id) && !cc6('5').includes(id));
    assert(only5.length > 0, '第5代应存在第4代尚不存在、且两侧轴都未引用过的对象');
    server.mutate = (p, r) => { if (r.payload.op === 'watch') {
      const d4 = p.deltas[0];
      d4.delta.catalog_evidence.axes['CC-006'].evidence.object_ids.push(only5[0]);
      d4.catalog_evidence = copy(d4.delta.catalog_evidence);
    } };
    await assert.rejects(client.watch(), /目录引用缺失\/重复/);
    assert.equal(canonical(client.committed), before, '拒绝后不得提交游标/状态');
    const late = copy(sparseDelta(sp, '5')).delta.catalog_evidence.axes;
    late['CC-006'].evidence.object_ids.push(only5[0]);
    assert.doesNotThrow(() => API.validateTB02Axes(late, sparseState(sp, '5').snapshot.objects), '末代完整集确有该对象：只按末代核会漏放');
    server.mutate = null;
    await client.watch();
    assert.equal(client.committed.cursor.after_generation, '5');
  });
  await test('旧Watch真实稀疏4/5逐代核引用后与同cut快照对拍提交，重复投递不重复生效', async () => {
    const legacy = sparseLegacy(sp);
    await vm.runInContext('load("current")', legacy.context);
    await vm.runInContext('applyStream()', legacy.context); // Gap → 按 cut-3 重建缓存
    assert.equal(vm.runInContext('lastAppliedGen', legacy.context), '3');
    await vm.runInContext('applyStream()', legacy.context); // 稀疏 4/5（含一条重复投递）
    assert.equal(vm.runInContext('lastAppliedGen', legacy.context), '5');
    assert.equal(vm.runInContext('appliedObjects.size', legacy.context), 15);
    assert.equal(vm.runInContext('appliedWithdrawn.size', legacy.context), 14);
    assert.match(legacy.elements.get('stream-result').innerHTML, /重复投递，不重复生效/);
    assert.match(legacy.elements.get('stream-cursor').textContent, /cut-5/);
  });
  await test('旧Watch中间代缺引用拒绝且游标/缓存不变', async () => {
    const legacy = sparseLegacy(sp, payload => {
      const d4 = payload.deltas[0];
      d4.delta.catalog_evidence.axes['CC-006'].evidence.object_ids.push('obj-1394-not-present');
      d4.catalog_evidence = copy(d4.delta.catalog_evidence);
    });
    await vm.runInContext('load("current")', legacy.context);
    await vm.runInContext('applyStream()', legacy.context); // Gap 重建 → gen 3
    await vm.runInContext('applyStream()', legacy.context); // 非法第4代：逐代核引用必须拦下
    assert.equal(vm.runInContext('lastAppliedGen', legacy.context), '3');
    assert.equal(vm.runInContext('appliedObjects.size', legacy.context), 9);
    assert.match(legacy.elements.get('stream-result').innerHTML, /目录引用缺失\/重复/);
  });
  if (tb02States !== null) {
    result.tb02_fixture = {path:path.resolve(argv[1]), sha256:hashBytes(tb02Bytes), states:tb02States.length,
      scope:'由正式S/Q另次运行取得的原始公开状态；本项只核生产makeHooks路径，不充当结构oracle/真实HTTP或GUI验收'};
    await test('生产HTTP helper的makeHooks完整校验实际TB02状态', async () => {
      assert(tb02States.length > 0); const hooks = makeHooks();
      for (const state of tb02States) {
        assert.equal(state.snapshot.profile_id, 'ohlc_integer_tb02a_v1');
        hooks.validateState(state, 's-session/2');
      }
    });
    await test('生产makeHooks拒绝实际TB02深层字段缺失', async () => {
      const state = copy(tb02States.find(s => s.snapshot.objects.some(o => o.kind === 'CC-004.inclusion_step')));
      assert(state, '真实状态未覆盖包含步骤');
      delete state.snapshot.objects.find(o => o.kind === 'CC-004.inclusion_step').payload.incoming.high;
      assert.throws(() => makeHooks().validateState(state, 's-session/2'));
    });
    await test('生产makeHooks拒绝实际TB02目录与固定cut不一致', async () => {
      const state = copy(tb02States.find(s => s.snapshot.generation !== '0'));
      assert(state, '真实状态未覆盖非空发布');
      state.catalog.items.find(i => i.id === 'CC-005').evidence = {};
      assert.throws(() => makeHooks().validateState(state, 's-session/2'));
    });
    await test('真实合并组序号与不连续原始锚分别校验', async () => {
      const shape = tb02States.flatMap(s => s.snapshot.objects).find(o => o.kind === 'CC-006.local_shape' &&
        o.input_refs.some((g, i) => g.merged_index !== [o.window_start, o.window_mid, o.window_end][i]));
      assert(shape, '真实状态缺少包含后的不连续原始锚');
      API.validateTB02ShapeRefs(shape);
      shape.input_refs.forEach((g, i) => assert.equal(g.raw_refs[0].source_coord, [shape.window_start, shape.window_mid, shape.window_end][i]));
    });
    await test('形态原始锚与合并组序号损坏分别拒绝', async () => {
      const shape = tb02States.flatMap(s => s.snapshot.objects).find(o => o.kind === 'CC-006.local_shape');
      assert(shape, '真实状态缺少形态');
      const anchor = copy(shape), ordinal = copy(shape);
      anchor.window_end = String(BigInt(anchor.window_end) + 1n);
      ordinal.input_refs[2].merged_index = String(BigInt(ordinal.input_refs[2].merged_index) + 1n);
      assert.throws(() => API.validateTB02ShapeRefs(anchor));
      assert.throws(() => API.validateTB02ShapeRefs(ordinal));
    });
    const nonempty = tb02States.find(s => s.snapshot.generation === '5');
    async function collectActual(state) {
      const client = new API.Client({fetch:actualTB02Server(state), hooks:makeHooks()});
      // 该自建S以session_generation=1初始化；纯state投影未附Q discovery字段。
      await client.load({...state, cut:{...state.cut, session_generation:'1'}}); return client;
    }
    await test('正式Client.load接受真实OHLC完整候选与撤回历史', async () => {
      for (const state of tb02States.filter(s => ['5','6'].includes(s.snapshot.generation))) {
        const client = await collectActual(state); assert.equal(client.committed.cursor.after_generation, state.snapshot.generation);
      }
    });
    for (const cid of ['CC-005','CC-008']) await test('正式Client.load拒绝完整目录缺项'+cid, async () => {
      const state = copy(nonempty); state.catalog.items = state.catalog.items.filter(i => i.id !== cid);
      await assert.rejects(collectActual(state), /目录数量/);
    });
    await test('正式Client.load拒绝CC006不存在的原始receipt', async () => {
      const state = copy(nonempty); state.snapshot.objects.find(o => o.kind === 'CC-006.local_shape').input_refs[0].raw_refs[0].receipt_id = 'rcpt-not-present';
      await assert.rejects(collectActual(state), /八键来源/);
    });
    await test('正式Client.load拒绝将边界依赖伪作成员', async () => {
      const state = copy(nonempty), shape = state.snapshot.objects.find(o => o.kind === 'CC-006.local_shape');
      const group = shape.input_refs.find(g => g.dependency_refs.some(r => BigInt(r.source_coord) > BigInt(g.raw_refs[0].source_coord)));
      assert(group); const i = group.dependency_refs.findIndex(r => BigInt(r.source_coord) > BigInt(group.raw_refs[0].source_coord));
      group.raw_refs.push(group.dependency_refs.splice(i,1)[0]); group.raw_refs.sort((a,b)=>Number(BigInt(a.source_coord)-BigInt(b.source_coord)));
      await assert.rejects(collectActual(state), /真实成员/);
    });
    await test('正式Client.load拒绝冻结目录重复身份', async () => {
      const state = copy(nonempty); state.catalog.items[1] = copy(state.catalog.items[0]);
      await assert.rejects(collectActual(state), /冻结目录身份/);
    });
    await test('正式Client.load拒绝非本叶目录静态内容篡改', async () => {
      const state = copy(nonempty); state.catalog.items.find(i=>i.id==='CC-008').title += '篡改';
      await assert.rejects(collectActual(state), /目录静态列摘要/);
    });
    await test('正式Client.load核CC006真实内容ID而非只核typed', async () => {
      const state = copy(nonempty); state.snapshot.objects.find(o=>o.kind==='CC-006.local_shape').comparisons[0].prev='999';
      await assert.rejects(collectActual(state), /对象内容身份 SHA/);
    });
    await test('正式Client.load拒绝整体平移组序号仍内部连续', async () => {
      const state = copy(nonempty), shape=state.snapshot.objects.find(o=>o.kind==='CC-006.local_shape');
      shape.input_refs.forEach(g=>{g.merged_index=String(BigInt(g.merged_index)+1n);});
      await assert.rejects(collectActual(state), /序号\/真实成员/);
    });
  }
  result.failed = result.cases.filter(c => !c.passed).map(c => c.name);
  console.log(JSON.stringify(result, null, 2)); if (result.failed.length) process.exitCode = 1;
}
module.exports = {makeHooks, htmlContext};
if (require.main === module) run().catch(error => { console.error(error.stack); process.exitCode = 1; });
