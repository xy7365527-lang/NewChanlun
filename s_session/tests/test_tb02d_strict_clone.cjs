// #1414：私有 clone 的真实旧新实现对照，仅纯内存与可选归档 cut40。
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
for (const key of ['baseline-client', 'output']) assert.ok(args[key], 'missing ' + key);
const current = path.resolve(__dirname, '../browser/tb01c-client.js');
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const paths = [__filename, args['baseline-client'], current, ...(args.candidate ? [args.candidate] : [])];
const hashes = () => Object.fromEntries(paths.map(p => [p, hash(fs.readFileSync(p))]));
const before = hashes();
function load(filename) {
  const source = fs.readFileSync(filename, 'utf8');
  const marker = '  return Object.freeze({TB02_KINDS:';
  assert.equal(source.split(marker).length, 2, 'unique final export for test-only private access');
  const context = vm.createContext({module: {exports: {}}, TextEncoder, TextDecoder, performance});
  vm.runInContext(source.replace(marker, '  globalThis.__cloneTest = {clone, stateOf, arrayPrototype: Array.prototype};\n' + marker), context);
  return context.__cloneTest;
}
const oldAPI = load(args['baseline-client']), newAPI = load(current);
const results = {scope: 'strict private clone equivalence; no service/database/browser/runtime acceptance', source_before: before, tests: [], timings: {}};
function shape(value) {
  if (value === null || typeof value !== 'object') return typeof value;
  return Array.isArray(value) ? 'array' : Object.getPrototypeOf(value) === null ? 'null-prototype' : 'ordinary-object';
}
function equivalent(a, b) {
  assert.equal(shape(a), shape(b));
  if (a === null || typeof a !== 'object') { assert.equal(a, b); return; }
  const aa = Object.keys(a), bb = Object.keys(b);
  assert.deepEqual(aa, bb, 'own enumerable key order');
  if (Array.isArray(a)) assert.equal(a.length, b.length);
  for (const k of aa) equivalent(a[k], b[k]);
}
function sourceObjects(value, seen = new WeakSet()) {
  if (value && typeof value === 'object' && !seen.has(value)) {
    seen.add(value); for (const child of Object.values(value)) sourceObjects(child, seen);
  }
  return seen;
}
function independent(value, source, seen = new WeakSet(), strictPrototype = true) {
  if (!value || typeof value !== 'object') return;
  assert.ok(!source.has(value), 'output shares mutable tree with input');
  assert.ok(!seen.has(value), 'output reuses a mutable subtree'); seen.add(value);
  if (strictPrototype && !Array.isArray(value)) assert.equal(Object.getPrototypeOf(value), null);
  for (const child of Object.values(value)) independent(child, source, seen, strictPrototype);
}
function outcome(clone, value) {
  try { return {accepted: true, value: clone(value)}; }
  catch (error) { return {accepted: false, message: error.message, error_name: error.name}; }
}
function check(name, value, accepted) {
  const old = outcome(oldAPI.clone, value), fresh = outcome(newAPI.clone, value);
  assert.equal(old.accepted, accepted, 'baseline classification ' + name);
  assert.equal(fresh.accepted, old.accepted, 'new classification ' + name);
  if (accepted) {
    equivalent(old.value, fresh.value);
    const source = sourceObjects(value);
    independent(old.value, source); independent(fresh.value, source);
  }
  results.tests.push({name, accepted, ...(!accepted ? {old_error: old.message, new_error: fresh.message,
    old_error_name: old.error_name, new_error_name: fresh.error_name, same_message: old.message === fresh.message} : {})});
}
const textObject = Object.create(null);
for (const k of ['z', '\u{10000}', '\ue000', '__proto__', 'constructor', '', '10', '2']) textObject[k] = '值:' + k;
Object.defineProperty(textObject, 'hidden', {value: undefined, enumerable: false});
textObject[Symbol('ignored')] = undefined;
for (const [name, value] of [
  ['null', null], ['true', true], ['false', false], ['empty-text', ''],
  ['exact-numeric-text', '-9223372036854775808'], ['unicode-escapes', '汉字\u{1f600}\u0000\n\\"'],
  ['null-prototype-and-key-order', textObject], ['empty-object', {}], ['empty-array', []],
  ['nested', {z: [true, null, {c: '1'}], a: false}],
  ['one-array-hole-normalizes-empty', Array(1)], ['boxed-number-own-keys', Object(1)],
]) check(name, value, true);
const inherited = Object.create({ignored: undefined}); inherited.own = '1'; check('inherited-object-fields-ignored', inherited, true);
const extras = ['x']; extras.extra = undefined; check('array-extra-fields-ignored', extras, true);
const proto = Object.create(Array.prototype); proto[0] = 'inherited';
const inheritedIndex = Array(1); Object.setPrototypeOf(inheritedIndex, proto); check('inherited-array-index', inheritedIndex, true);
const shared = {leaf: ['same']}; check('repeated-input-alias', {a: shared, b: shared}, true);
for (const [name, value] of [
  ['number-zero', 0], ['number-negative-zero', -0], ['number-fraction', 0.5], ['number-nan', NaN],
  ['number-infinity', Infinity], ['bigint', 1n], ['undefined', undefined], ['symbol', Symbol('bad')],
  ['function', () => {}], ['nested-undefined', {a: undefined}], ['array-undefined', [undefined]],
  ['unpaired-high', '\ud800'], ['unpaired-low', '\udc00'], ['invalid-object-key', {'\ud800': 'x'}],
  ['two-array-holes', Array(2)], ['array-first-hole', [, 'x']], ['array-last-hole', ['x', ,]],
]) check(name, value, false);
function nest(n, leaf) { for (let i = 0; i < n; i++) leaf = {a: leaf}; return leaf; }
check('scalar-at-depth128', nest(128, 'leaf'), true);
check('empty-object-at-depth128', nest(128, {}), true);
check('empty-array-at-depth128', nest(128, []), true);
check('single-hole-at-depth128', nest(128, Array(1)), true);
check('scalar-at-depth129', nest(129, 'leaf'), false);
check('empty-object-at-depth129', nest(129, {}), false);
check('mixed-depth-and-invalid-number', {a: nest(128, 'leaf'), z: 1}, false);
check('mixed-hole-and-invalid-number', [, 1], false);
const cycle = {}; cycle.a = cycle; check('cycle-rejected', cycle, false);
for (const [label, api] of [['old', oldAPI], ['new', newAPI]]) {
  const input = {a: shared, b: shared};
  const first = api.clone(input); first.a.leaf[0] = 'changed-output';
  assert.equal(first.b.leaf[0], 'same'); assert.equal(input.a.leaf[0], 'same');
  input.a = {leaf: ['fresh-value']};
  const second = api.clone(input); assert.equal(second.a.leaf[0], 'fresh-value');
  assert.equal(second.b.leaf[0], 'same'); assert.notEqual(second.b, first.b);
  results.tests.push({name: label + '-fresh-values-and-alias-isolation', accepted: true});
}
if (args.candidate) {
  assert.ok(args['candidate-sha']); assert.equal(before[args.candidate], args['candidate-sha']);
  const archived = JSON.parse(fs.readFileSync(args.candidate, 'utf8'));
  assert.equal(archived.state.snapshot.generation, '40');
  assert.equal(archived.projection.fixed_cut.cut_generation, '40');
  function freeze(value) {
    if (value && typeof value === 'object' && !Object.isFrozen(value)) {
      for (const child of Object.values(value)) freeze(child); Object.freeze(value);
    }
    return value;
  }
  freeze(archived);
  results.cut = '40'; results.candidate_sha256 = before[args.candidate];
  for (const [name, method, input] of [['clone_snapshot', 'clone', archived.state.snapshot], ['stateOf_projection', 'stateOf', archived.projection]]) {
    const startedOld = performance.now(); const old = oldAPI[method](input); const oldMS = performance.now() - startedOld;
    const startedNew = performance.now(); const fresh = newAPI[method](input); const newMS = performance.now() - startedNew;
    equivalent(old, fresh);
    const source = sourceObjects(input);
    independent(old, source, new WeakSet(), method === 'clone'); independent(fresh, source, new WeakSet(), method === 'clone');
    results.timings[name] = {old_ms: oldMS, new_ms: newMS, saved_ms: oldMS - newMS, full_structure_key_order_prototype_equal: true, isolated_new_trees: true};
  }
  results.timing_limit = 'each real function once per arm, ordered old then new after small tests; includes JIT/order/GC effects; not statistical estimate or arm300 prediction';
}
results.source_after = hashes(); assert.deepEqual(results.source_after, before);
results.status = 'PASS';
fs.writeFileSync(args.output, JSON.stringify(results, null, 2) + '\n', {flag: 'wx'});
console.log(JSON.stringify({status: results.status, tests: results.tests.length, timings: results.timings,
  differing_errors: results.tests.filter(t => t.same_message === false)}));
