// #1424 TB-02-F：浏览器端中枢对合同逐条判共享语料（与 test_tb02f_contract.py 同语料；对拍在 Python 侧）。
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const API = require('../browser/tb01c-client.js');
const corpus = require('./fixtures/tb02f-contract/corpus.json');

function expand(c, bases) {
  if (c.base === null) return [c.records, c.axes];
  const b = bases[c.base], seen = new Set(), records = [];
  for (const r of b.records) {
    if (c.remove.includes(r.object_id)) continue;
    records.push(Object.hasOwn(c.replace, r.object_id) && !seen.has(r.object_id) ? c.replace[r.object_id] : r); seen.add(r.object_id);
  }
  records.push(...c.add);
  return [records, c.axes ?? b.axes];
}

test('中枢对合同逐条判共享语料：合法 cut 接受、反例按原因码拒绝', () => {
  const bases = Object.fromEntries(corpus.cases.filter(c => c.base === null).map(c => [c.name, c]));
  for (const c of corpus.cases) {
    const [records, axes] = expand(c, bases);
    if (c.expected === 'ACCEPT') API.validatePairCut(records, c.rule_revision, axes);
    else {
      const expected = c.expected.startsWith('TB02E_') ? c.expected : 'TB02F_' + c.expected;
      assert.throws(() => API.validatePairCut(records, c.rule_revision, axes), e => e.message.startsWith(expected + ':'), c.name);
    }
  }
});

test('#1424 C1：12 成员的合法 pending 保留，角色与 pair 合谋伪装被整 cut 拒绝', () => {
  const bases = Object.fromEntries(corpus.cases.filter(c => c.base === null).map(c => [c.name, c]));
  const good = bases.w9_k15;
  API.validatePairCut(good.records, good.rule_revision, good.axes);
  const bad = corpus.cases.find(c => c.name === 'ge9_roles_and_pair_forged_evaluated');
  assert.ok(bad);
  const [records, axes] = expand(bad, bases);
  const center = records.find(r => r.kind === 'CC-016.center').payload.data;
  assert.equal(center.members.length, 12);
  assert.equal(center.status, 'UPGRADE_PENDING_TB02H');
  assert.equal(records.find(r => r.kind === 'CC-020.center_pair').payload.data.status, 'EVALUATED');
  assert.throws(() => API.validatePairCut(records, bad.rule_revision, axes), /^Error: TB02E_ROLE_NULL:/);
});

test('中枢对 kind 进入既有观察器 kind 表', () => {
  for (const k of ['CC-018.unit_core_position', 'CC-020.center_pair', 'CC-055.pair_relation', 'CC-055.pair_change', 'CC-056.pair_version']) assert.ok(API.TB02_KINDS.includes(k));
});

// 相邻 cut 的真实状态绑定与单快照校验分开，稳定 cut 无需重复出生事件。
test('真实多 cut 历史：出生、输入/叶修订、稳定、撤回与坏前态', () => {
  const bases = Object.fromEntries(corpus.cases.filter(c => c.base === null).map(c => [c.name, c]));
  const records = Object.fromEntries(corpus.cases.map(c => [c.name, expand(c, bases)[0]]));
  assert.ok(corpus.transitions.length >= 10);
  for (const c of corpus.transitions) {
    const run = () => API.validatePairTransition(c.before === null ? [] : records[c.before], records[c.after]);
    if (c.expected === 'ACCEPT') run();
    else assert.throws(run, e => e.message.startsWith('TB02F_' + c.expected + ':'), c.name);
  }
});

test('冷快照用 active/withdrawn 生命周期核实际出生与修订历史', () => {
  const bases = Object.fromEntries(corpus.cases.filter(c => c.base === null).map(c => [c.name, c]));
  const history = [], pairs = new Map();
  let previous = null;
  for (const name of ['history_w12_k11', 'history_w12_k12', 'history_w12_k13', 'history_stable', 'history_withdrawn']) {
    const rows = bases[name].records;
    const pair = rows.find(r => r.kind === 'CC-020.center_pair');
    const event = rows.find(r => r.kind === 'CC-055.pair_change');
    if (event) {
      const generation = event.payload.data.known_at.generation;
      if (previous) previous.withdrawn_generation = generation;
      history.push({...structuredClone(event), first_known_generation: generation, withdrawn_generation: null});
      previous = null;
    }
    if (pair && !pairs.has(pair.object_id)) {
      const value = {...structuredClone(pair), first_known_generation: pair.payload.data.known_at.generation, withdrawn_generation: null};
      pairs.set(pair.object_id, value); history.push(value); previous = value;
    }
  }
  API.validatePairHistory(history);
  const noBirth = history.filter(r => !(r.kind === 'CC-055.pair_change' && r.payload.data.change === 'formed'));
  assert.throws(() => API.validatePairHistory(noBirth), /TB02F_CHANGE:/);
  const bad = structuredClone(history);
  bad.find(r => r.kind === 'CC-055.pair_change' && r.payload.data.change === 'inputs_revised').payload.data.before.a_revision = '2';
  assert.throws(() => API.validatePairHistory(bad), /TB02F_CHANGE: 变化前后件须绑定相邻 cut/);
});

test('#1424 M1：冷历史仅访问受影响实体，操作计数保持线性且确定', () => {
  const base = corpus.cases.find(c => c.name === 'history_w12_k11');
  const originalPair = base.records.find(r => r.kind === 'CC-020.center_pair');
  const originalEvent = base.records.find(r => r.kind === 'CC-055.pair_change');
  function count(n) {
    let reads = 0;
    const history = [];
    for (let i = 0; i < n; i++) {
      const pair = structuredClone(originalPair), event = structuredClone(originalEvent), d = pair.payload.data;
      const a = 100 + i * 2, b = a + 1, generation = String(i + 1), oid = 'count-pair-' + i;
      d.entity_id = `pair:1:${a}:1:${b}`; d.a_center_id = `center:1:${a}`; d.b_center_id = `center:1:${b}`;
      pair.payload.slot = d.entity_id;
      d.known_at.generation = generation; d.formed_known_at = structuredClone(d.known_at);
      pair.first_known_generation = generation; pair.withdrawn_generation = null;
      event.payload.data.entity_id = d.entity_id; event.payload.data.after = structuredClone(d);
      event.payload.data.known_at = structuredClone(d.known_at); event.payload.data.object_id = oid;
      event.payload.slot = [generation, d.entity_id, 'formed']; event.first_known_generation = generation; event.withdrawn_generation = null;
      Object.defineProperty(pair, 'object_id', {enumerable: true, get() {reads++; return oid;}});
      history.push(pair, event);
    }
    API.validatePairHistory(history);
    return reads;
  }
  for (const n of [4, 8, 16, 32, 64]) {
    const first = count(n), second = count(n);
    assert.equal(first, second, '同样历史应有确定的访问次数');
    assert.ok(first <= 6 * n, `n=${n}, object_id reads=${first}，不得逐代扫描全部活动对`);
  }
});
