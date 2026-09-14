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
  const discovery = {cut: {session_id: f.a3.snapshot.session_id, session_generation: '1'}};
  async function test(name, fn) { try { await fn(); result.cases.push({name, passed: true}); }
    catch (error) { result.cases.push({name, passed: false, error: error.stack}); } }
  async function setup() {
    const server = fakeServer(f), commits = [];
    const client = new API.Client({fetch: server.fetch, hooks: makeHooks(), limits: {pageSize: TEST_PAGE_SIZE}, onCommit: (c, e) => commits.push({c, e})});
    await client.load(discovery); return {client, server, commits};
  }
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
