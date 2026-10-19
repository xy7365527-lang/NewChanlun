// #1493 段合同 v4（s-segment/4）浏览器侧同 cut 引用锁：PENDING_CASE_ONE 绑 FIRST 开窗候选、候选态/窗成员绑 FIRST 检查、
// 段端门失败（parse_invalid）在段版本与逐轴证据标阻断。载荷取 test_tb02c_turn_contract.py 的同一构造（Python 与 JS 同输入）。
// 人工构造只检合同，不作结构语义或运行验收。
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const {execFileSync} = require('node:child_process');
const API = require('../browser/tb01c-client.js');

const RULE_V4 = 's2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1', PROFILE = 'ohlc_integer_tb02a_v1';
const SEQ = 'CC-012.feature_sequence', SEG = 'CC-013.segment', VER = 'CC-056.segment_version';
const S = JSON.parse(execFileSync('python3', ['-B', '-c', `import json, test_tb02c_turn_contract as t
print(json.dumps(dict(first5=t.first(5), first7=t.first(7), pending=t.segment_pending(), confirmed=t.segment_confirmed(),
  invalid=t.segment_invalid(), version=t.version(), blocked=t.version(["segment_parse_invalid", "top_not_above_bottom"]))))`], {cwd: __dirname, encoding: 'utf8'}));
const clone = structuredClone;

function privateClient() {
  const source = fs.readFileSync(path.join(__dirname, '../browser/tb01c-client.js'), 'utf8');
  const marker = '  return Object.freeze({TB02_KINDS:';
  assert.equal(source.split(marker).length, 2, 'unique final export for test-only private access');
  const context = vm.createContext({module: {exports: {}}, TextEncoder, TextDecoder, crypto: require('node:crypto').webcrypto});
  vm.runInContext(source.replace(marker, '  globalThis.__tb02x = {validateTB02Candidate, TB02_AXES};\n' + marker), context);
  return context.__tb02x;
}
function catalog() {
  const declared = JSON.parse(fs.readFileSync(path.join(__dirname, '../catalog/signed-catalog.json'), 'utf8'));
  const wire = v => typeof v === 'number' ? String(v) : Array.isArray(v) ? v.map(wire) : v && typeof v === 'object' ? Object.fromEntries(Object.entries(v).map(([k, x]) => [k, wire(x)])) : v;
  const items = declared.items.map(i => ({id: i.id, ...wire(Object.fromEntries(['kind', 'title', 'domain', 'branches'].map(k => [k, i[k] ?? (k === 'branches' ? [] : '')]))),
    implementation_status: 'not_implemented', proof_status: 'not_proved', run_status: 'not_run', evidence: {}}));
  const n = String(items.length);
  return {catalog_revision: API.TB02_CATALOG.catalog_revision, items, counts: {implemented: '0', not_implemented: n, run: '0', not_run: n}};
}
async function typed(kind, payload) {
  const fact_key = API.canonical([kind, payload.slot]);
  const object_id = 'obj-' + await API.sha256({kind, fact_key, payload, input_refs: [], profile_id: PROFILE, rule_revision: RULE_V4});
  return {object_id, object_revision: '1', kind, batch_id: 'batch', fact_key, payload, input_refs: [], source_coords: [],
    first_known_generation: '1', first_known_cut: 'cut-1', published_generation: '1', withdrawn_generation: null, withdrawal_reason: null, superseded_by: null, lifecycle: 'active'};
}
async function cut(firstPayload, segmentPayload, versionPayload = null) {
  const a = await typed(SEQ, firstPayload), s = clone(segmentPayload);
  s.data.sequence_object_id = a.object_id;
  const objects = [a, await typed(SEG, s)];
  if (versionPayload !== null) objects.push(await typed(VER, versionPayload));
  return {objects, withdrawn_objects: [], raw_history: [], generation: '1'};
}

test('v4 单对象载荷与 Python 同构造同判定', () => {
  for (const [kind, p] of [[SEQ, S.first5], [SEQ, S.first7], [SEG, S.pending], [SEG, S.confirmed], [SEG, S.invalid], [VER, S.version]]) API.validateSegmentPayload(kind, p);
});

test('同cut引用：PENDING_CASE_ONE 绑开窗候选、候选态绑 FIRST 检查、段端门失败须标阻断', async () => {
  const {validateTB02Candidate, TB02_AXES} = privateClient(), state = {catalog: catalog()};
  const axes = reasons => Object.fromEntries(TB02_AXES.map(a => [a, {evidence: {waiting_reasons: reasons}}]));
  const run = async (snapshot, reasons = []) => validateTB02Candidate({...state, snapshot}, {cut_generation: '1', rule_revision: RULE_V4, profile_id: PROFILE, catalog_evidence: {axes: axes(reasons)}}, () => {});
  await run(await cut(S.first5, S.pending));
  await run(await cut(S.first7, S.confirmed));
  await assert.rejects(run(await cut(S.first7, S.pending)), {message: 'PENDING_CASE_ONE未绑定FIRST开窗候选'});
  const unbound = clone(S.confirmed); unbound.data.window_members = [];
  await assert.rejects(run(await cut(S.first7, unbound)), {message: '段候选态/窗成员未绑定FIRST检查'});
  await run(await cut(S.first5, S.invalid, S.blocked), ['segment_parse_invalid']);
  await assert.rejects(run(await cut(S.first5, S.invalid, S.version), ['segment_parse_invalid']), {message: '段端门失败未在段版本标阻断'});
  await assert.rejects(run(await cut(S.first5, S.invalid)), {message: '段端门失败未在段版本标阻断'});
  await assert.rejects(run(await cut(S.first5, S.invalid, S.blocked), []), {message: '段端门失败未在逐轴证据标阻断'});
});

test('v4 逻辑slot不被页面当成见证整数；CC-013 页面显示候选态与阻断', () => {
  const {htmlContext} = require('./test_tb01c_browser.cjs');
  const {context} = htmlContext();
  for (const p of [S.version, S.first5, S.pending]) assert.doesNotThrow(() => context.validateExactRecords(p));
  const pending = context.renderSegmentTurn(S.pending), confirmed = context.renderSegmentTurn(S.confirmed), invalid = context.renderSegmentTurn(S.invalid);
  assert.match(pending, /PENDING_CASE_ONE/); assert.match(pending, /OPEN_AT_PREFIX_END/); assert.match(pending, /窗成员 \[5\]/);
  assert.match(confirmed, /CASE_ONE/); assert.match(confirmed, /CONFIRMED/);
  assert.match(invalid, /段划分阻断（fail-closed）：自笔 0 起划分无效/); assert.doesNotMatch(pending, /段划分阻断/);
});
