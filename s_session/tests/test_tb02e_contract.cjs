// #1419 TB-02-E：浏览器端中枢合同逐条判共享语料（与 test_tb02e_contract.py 同语料；对拍在 Python 侧）。
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const API = require('../browser/tb01c-client.js');
const corpus = require('./fixtures/tb02e-contract/corpus.json');

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

test('中枢合同逐条判共享语料：合法 cut 接受、反例按原因码拒绝', () => {
  const bases = Object.fromEntries(corpus.cases.filter(c => c.base === null).map(c => [c.name, c]));
  for (const c of corpus.cases) {
    const [records, axes] = expand(c, bases);
    if (c.expected === 'ACCEPT') API.validateCenterCut(records, c.rule_revision, axes);
    else assert.throws(() => API.validateCenterCut(records, c.rule_revision, axes), e => e.message.startsWith('TB02E_' + c.expected + ':'), c.name);
  }
});

test('中枢 kind 进入既有观察器 kind 表', () => {
  for (const k of ['CC-016.center_seed_attempt', 'CC-016.center', 'CC-017.center_roles', 'CC-022.unit_assignment']) assert.ok(API.TB02_KINDS.includes(k));
});
