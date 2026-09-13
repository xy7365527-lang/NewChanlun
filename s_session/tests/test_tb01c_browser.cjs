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
  const {context} = htmlContext();
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
    case 'objects': return [BigInt(r.window_start), BigInt(r.window_mid), BigInt(r.window_end), r.object_id, BigInt(r.object_revision)];
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
  return {fixed_cut: fixed, catalog: copy(state.catalog), rows, counts, scope: copy(s.scope), omissions: [], order_version: 's-record-order/1'};
}
function token(p, offset, pageSize = TEST_PAGE_SIZE) {
  const t = Object.fromEntries(['session_id', 'session_generation', 'cut_generation', 'scope', 'history_mode', 'structure_cut', 'catalog_revision', 'index_frontier', 'profile_hash', 'rule_revision'].map(k => [k, copy(p.fixed_cut[k])]));
  Object.assign(t, {token_schema: 's-snapshot-token/1', order_version: 's-record-order/1', page_size: String(pageSize), offset: String(offset),
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
async function run() {
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
  result.failed = result.cases.filter(c => !c.passed).map(c => c.name);
  console.log(JSON.stringify(result, null, 2)); if (result.failed.length) process.exitCode = 1;
}
module.exports = {makeHooks, htmlContext};
if (require.main === module) run().catch(error => { console.error(error.stack); process.exitCode = 1; });
