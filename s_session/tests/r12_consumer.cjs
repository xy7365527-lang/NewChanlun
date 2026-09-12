// #1371 R12：原 HTTP 字节 + 实际 HTML 函数；模拟 DOM，无服务或网络请求。
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '../..');
const html = fs.readFileSync(path.join(root, 's_session/browser/index.html'));
const fixtureBytes = fs.readFileSync(path.join(__dirname, 'fixtures/r12_consumer.json'));
const fixture = JSON.parse(fixtureBytes).records;
const hash = b => crypto.createHash('sha256').update(b).digest('hex');
const source = html.toString().match(/<script>([\s\S]*?)<\/script>/)[1].replace(/load\("current"\);\s*$/, '');
function response(name) {
  const r = fixture[name];
  assert.equal(r.status, 200);
  assert.equal(hash(Buffer.from(r.body)), r.sha256, name + ' 原 HTTP 字节');
  return JSON.parse(r.body);
}
const a1 = response('a1'), a3 = response('a3'), a4 = response('a4');
const b1 = response('b1'), gapB = response('gap_b'), watch3 = response('watch3');
assert.notEqual(a1.snapshot.session_id, b1.snapshot.session_id);
assert.equal(gapB.session_id, b1.snapshot.session_id);
// TestOnly：只交付真实 watch3 中的第四代，明确限定外层交付末代。
const delta4 = {...watch3, generation:'4', structure_cut:'cut-4', deltas:watch3.deltas.filter(d => d.generation === '4')};
assert.equal(delta4.deltas.length, 1);
assert(delta4.deltas[0].delta.replaces.length > 0);
function context() {
  const elements = new Map();
  function element() { return {innerHTML:'', textContent:'', value:'3', placeholder:'', checked:false,
    classList:{toggle() {}}, addEventListener() {}, focus() {}, scrollIntoView() {}}; }
  const document = {
    getElementById(id) { if (!elements.has(id)) elements.set(id, element()); return elements.get(id); },
    querySelector(q) { return this.getElementById(q); },
    createElement() { const e = element(); Object.defineProperty(e, 'innerHTML', {get() { return this.textContent; }}); return e; }
  };
  const c = vm.createContext({document, structuredClone, console});
  vm.runInContext(source, c);
  c.seed = structuredClone(a3);
  vm.runInContext('validateState(seed); rebuildCommittedFromState(seed); syncModeUI("asof",seed.snapshot); renderAllFromState(seed)', c);
  return {c, elements};
}
function state(c) {
  return JSON.parse(vm.runInContext('JSON.stringify({currentState,lastAppliedGen,sessionBinding,historyMode,objects:[...appliedObjects],withdrawn:[...appliedWithdrawn],witnesses:[...appliedWitnesses],relations:[...appliedRelations],observations:[...appliedObservations],raw:[...appliedRawHistory]})', c));
}
async function deliver(c, delta, snapshot) {
  const requests = [], cursor = state(c).lastAppliedGen;
  c.fetch = async url => {
    requests.push(url);
    assert(['/api/delta?after_generation=' + cursor, '/api/state?as_of=' + snapshot.snapshot.generation].includes(url), url);
    const body = url.startsWith('/api/delta') ? delta : snapshot;
    return {ok:true, status:200, json:async () => structuredClone(body)};
  };
  await vm.runInContext('applyStream()', c);
  return requests;
}
const result = {html_sha256:hash(html), fixture_sha256:hash(fixtureBytes), cases:[]};
async function test(name, fn) {
  try { await fn(); result.cases.push({name, passed:true}); }
  catch (e) { result.cases.push({name, passed:false, error:String(e).split('\n')[0]}); }
}
(async () => {
  await test('健康更正流提交第四代', async () => {
    const {c, elements} = context(); await deliver(c, delta4, a4);
    assert.deepEqual(state(c).currentState, a4); assert.equal(state(c).lastAppliedGen, '4');
    assert(elements.get('stream-result').innerHTML.includes('ok-box'));
  });
  const mutations = {
    '替代集合缺失': d => { delete d.delta.replaces; },
    '替代集合漏项': d => { d.delta.replaces = []; },
    '替代旧端点错误': d => { d.delta.replaces[0].old_object_id = 'wrong-old-object'; },
    '替代新端点错误': d => { d.delta.replaces[0].new_object_id = 'wrong-new-object'; },
    '替代集合重复': d => { d.delta.replaces.push(structuredClone(d.delta.replaces[0])); },
    '替代成员多字段': d => { d.delta.replaces[0].unexpected_semantic_field = 'wrong'; }
  };
  for (const [name, mutate] of Object.entries(mutations)) await test(name + '拒绝并同页重试', async () => {
    const {c, elements} = context(), before = state(c), bad = structuredClone(delta4);
    mutate(bad.deltas[0]); await deliver(c, bad, a4);
    assert.deepEqual(state(c), before, '失败后全部逻辑状态必须保持');
    assert(!elements.get('stream-result').innerHTML.includes('ok-box'));
    const requests = await deliver(c, delta4, a4);
    assert.equal(requests[0], '/api/delta?after_generation=3');
    assert.deepEqual(state(c).currentState, a4); assert.equal(state(c).lastAppliedGen, '4');
  });
  await test('合法新会话Gap允许重建', async () => {
    const {c} = context(); await deliver(c, gapB, b1);
    assert.deepEqual(state(c).currentState, b1); assert.equal(state(c).sessionBinding.session_id, gapB.session_id);
    assert.equal(state(c).lastAppliedGen, '1');
  });
  for (const kind of ['另一会话快照', '缺少Gap会话身份']) await test('Gap' + kind + '拒绝并同页重试', async () => {
    const {c} = context(), before = state(c), gap = structuredClone(gapB);
    if (kind === '缺少Gap会话身份') delete gap.session_id;
    await deliver(c, gap, kind === '另一会话快照' ? a1 : b1);
    assert.deepEqual(state(c), before, 'Gap声明与快照未绑定时不可提交');
    const requests = await deliver(c, gapB, b1);
    assert.equal(requests[0], '/api/delta?after_generation=3');
    assert.deepEqual(state(c).currentState, b1); assert.equal(state(c).sessionBinding.session_id, gapB.session_id);
  });
  result.failed = result.cases.filter(x => !x.passed).map(x => x.name);
  console.log(JSON.stringify(result, null, 2));
  process.exitCode = result.failed.length ? 1 : 0;
})().catch(e => { console.error(e.stack); process.exitCode = 1; });
