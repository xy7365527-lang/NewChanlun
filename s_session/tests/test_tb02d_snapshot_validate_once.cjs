// #1414：真实已归档Snapshot的HTML旧/新校验对照；不启动DB/服务/浏览器。
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const {performance} = require('node:perf_hooks');
const args = Object.fromEntries(process.argv.slice(2).reduce((pairs, value, i, values) => {
  if (i % 2 === 0) pairs.push([value.replace(/^--/, ''), values[i + 1]]); return pairs;
}, []));
for (const key of ['baseline-html', 'candidate', 'candidate-sha', 'output']) assert.ok(args[key], 'missing ' + key);
const root = path.resolve(__dirname, '..');
const html = path.join(root, 'browser/index.html');
const client = path.join(root, 'browser/tb01c-client.js');
const API = require(client);
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const sourcePaths = [args['baseline-html'], html, client, args.candidate];
const before = Object.fromEntries(sourcePaths.map(p => [p, hash(fs.readFileSync(p))]));
assert.equal(before[args.candidate], args['candidate-sha']);
function hooks(filename) {
  const elements = new Map();
  function element() { return {innerHTML: '', textContent: '', value: '3', checked: true, disabled: false,
    classList: {toggle() {}}, addEventListener() {}, focus() {}, scrollIntoView() {}}; }
  const document = {getElementById(id) { if (!elements.has(id)) elements.set(id, element()); return elements.get(id); },
    querySelector(q) { return this.getElementById(q); }, createElement() { return element(); }};
  const context = vm.createContext({document, TB01C: API, structuredClone, console, setTimeout, clearTimeout, performance});
  const original = fs.readFileSync(filename, 'utf8').match(/<script>([\s\S]*?)<\/script>/)[1];
  assert.match(original, /load\("current"\);\s*$/);
  vm.runInContext(original.replace(/load\("current"\);\s*$/, ''), context);
  assert.equal(vm.runInContext('typedKinds === TB01C.TB02_KINDS', context), true);
  return context.validateState;
}
const oldValidate = hooks(args['baseline-html']), newValidate = hooks(html);
const archived = JSON.parse(fs.readFileSync(args.candidate, 'utf8'));
const state = archived.state;
assert.equal(state.snapshot.generation, '40');
assert.equal(state.snapshot.profile_id, 'ohlc_integer_tb02a_v1');
function freeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) freeze(child); Object.freeze(value);
  }
  return value;
}
freeze(state);
const result = {scope: 'one saved cut40 HTML validateState pair plus targeted corruptions; no runtime acceptance',
  candidate_sha256: before[args.candidate], source_before: before, tests: [], valid_pair: {},
  cut: state.snapshot.generation, families: Object.fromEntries(['objects','withdrawn_objects','witnesses','relations','observations','raw_history'].map(k => [k, state.snapshot[k].length]))};
for (const [name, validate] of [['old', oldValidate], ['new', newValidate]]) {
  const start = performance.now(); const returned = validate(state, 's-session/2');
  result.valid_pair[name] = {elapsed_ms: performance.now() - start, accepted: true};
  assert.equal(returned, undefined);
}
const hasTyped = r => API.TB02_KINDS.includes(r.kind);
const active = state.snapshot.objects.findIndex(hasTyped);
const withdrawn = state.snapshot.withdrawn_objects.findIndex(hasTyped);
const shape = state.snapshot.objects.findIndex(r => r.kind === 'CC-006.local_shape');
assert.ok(active >= 0 && withdrawn >= 0 && shape >= 0, 'saved cut40 must cover typed active/withdrawn and shape');
function changedRecord(family, index, change) {
  const records = [...state.snapshot[family]], record = structuredClone(records[index]); change(record); records[index] = record;
  return {...state, snapshot: {...state.snapshot, [family]: records}};
}
const mutations = [
 ['active_typed_revision', () => changedRecord('objects', active, r => { r.object_revision = '0'; })],
 ['withdrawn_typed_lifecycle', () => changedRecord('withdrawn_objects', withdrawn, r => { r.withdrawn_generation = r.first_known_generation; })],
 ['typed_missing_payload', () => changedRecord('objects', active, r => { delete r.payload; })],
 ['typed_source_coords', () => changedRecord('objects', active, r => { r.source_coords = ['999']; })],
 ['typed_raw_binding', () => changedRecord('objects', active, r => { assert.ok(r.input_refs.length); r.input_refs[0].receipt_id = 'wrong'; })],
 ['shape_group_missing', () => changedRecord('objects', shape, r => { r.input_refs.pop(); })],
 ['shape_member_identity', () => changedRecord('objects', shape, r => { r.input_refs[0].raw_refs[0].receipt_id = 'wrong'; })],
 ['raw_schema', () => changedRecord('raw_history', 0, r => { r.schema_revision = 'wrong'; })],
 ['raw_geometry', () => changedRecord('raw_history', 0, r => { r.low = String(BigInt(r.high) + 1n); })],
 ['raw_negative_volume', () => changedRecord('raw_history', 0, r => { r.volume = '-1'; })],
 ['raw_missing_open', () => changedRecord('raw_history', 0, r => { delete r.open; })],
 ['raw_integer_number', () => changedRecord('raw_history', 0, r => { r.revision = 1; })],
 ['raw_history_missing', () => ({...state, snapshot: {...state.snapshot, raw_history: state.snapshot.raw_history.slice(1)}})],
 ['catalog_publication', () => ({...state, catalog: {...state.catalog, generation: '39'}})],
 ['cut_identity', () => ({...state, cut: {...state.cut, generation: '39'}})],
];
for (const [name, build] of mutations) {
  const value = freeze(build()), errors = [];
  for (const validate of [oldValidate, newValidate]) {
    let error; try { validate(value, 's-session/2'); } catch (e) { error = String(e.message); }
    assert.ok(error, name + ' must reject'); errors.push(error);
  }
  assert.equal(errors[0], errors[1], name + ' error changed');
  result.tests.push({name, both_rejected: true, same_message: true, message: errors[0]});
}
result.source_after = Object.fromEntries(sourcePaths.map(p => [p, hash(fs.readFileSync(p))]));
assert.deepEqual(result.source_after, before);
result.valid_pair.saved_ms = result.valid_pair.old.elapsed_ms - result.valid_pair.new.elapsed_ms;
result.valid_pair.interpretation = 'single ordered pair, no warmup/repetitions; not statistical estimate or arm300 prediction';
result.status = 'PASS';
fs.writeFileSync(args.output, JSON.stringify(result, null, 2) + '\n', {flag: 'wx'});
console.log(JSON.stringify({status: result.status, tests: result.tests.length, cut: result.cut, families: result.families, valid_pair: result.valid_pair}));
