import fs from 'node:fs';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {tradeInput, gridFeatures, featuresAt, auditRows, timeNs, run} from './trade_grid_features.mjs';
import {labelRows} from './order_label_probe.mjs';
const sha = x => crypto.createHash('sha256').update(x).digest('hex');
const SOURCE_SHA = 'a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5';
const B = 1_609_459_200_000_000_000n;
const stamp = n => `2021-01-01T00:00:${String(n / 1_000_000_000n).padStart(2, '0')}.${String(n % 1_000_000_000n).padStart(9, '0')}Z`;
function row(ns, sequence = null, price = '100', quantity = '1', status = 'ready') {
  return {schema: 'coinbase-causal-view/1', product_id: 'BTC-USD', line: 1,
    available_at_capture: stamp(ns), status, transition: status === 'ready' ? 'single_event' : 'unavailable',
    quote: status === 'ready' ? {scale: 100_000_000, bid_price: '90', ask_price: '110', bid_quantity: '1', ask_quantity: '1'} : null,
    new_trade: sequence === null ? null : {scale: 100_000_000, sequence: String(sequence),
      known_at_capture: stamp(ns), price, quantity, maker_buy: false}};
}
const numbered = rows => rows.map((r, i) => ({...r, line: i + 1}));
const produce = rows => gridFeatures(tradeInput(rows));
// Independent slow reference: per-decision raw-row filters and fresh reductions.
// Uses neither production timestamp parser, projection, buckets nor accumulator.
function refTime(s) {
  const [whole, fraction = ''] = s.slice(0, -1).split('.');
  return BigInt(Date.parse(`${whole}Z`)) * 1_000_000n + BigInt((fraction + '000000000').slice(0, 9));
}
function reference(rows) {
  if (!rows.length) return [];
  const origin = refTime(rows[0].available_at_capture), end = refTime(rows.at(-1).available_at_capture);
  const out = [];
  for (let t = origin - origin % 100_000_000n + 100_000_000n; t <= end; t += 100_000_000n) {
    const past = rows.filter(r => r.new_trade !== null && refTime(r.new_trade.known_at_capture) < t).map(r => r.new_trade);
    const window = past.filter(x => refTime(x.known_at_capture) >= t - 1_000_000_000n);
    const full = t - origin >= 1_000_000_000n;
    const last = past.at(-1);
    out.push({profile: 'trade-history-grid-a/1', product_id: rows[0].product_id, decision_ns: String(t), features: {
      last_trade_price: last ? last.price : null, seen_trade_count: String(past.length),
      since_last_trade_ns: last ? String(t - refTime(last.known_at_capture)) : null,
      observed_trade_count_1s: full ? String(window.length) : null,
      observed_trade_quantity_1s: full ? String(window.reduce((sum, x) => sum + BigInt(x.quantity), 0n)) : null}});
  }
  return out;
}
const checks = [];
function check(name, fn) { fn(); checks.push(name); }
const sample = numbered([row(0n, 1, '9007199254740993123', '9007199254740993124', 'waiting_snapshot'),
  row(100_000_000n, 2, '102', '2', 'waiting_snapshot'), row(100_000_000n, 3, '103', '3', 'waiting_snapshot'),
  row(250_000_000n), row(1_100_000_000n), row(1_200_000_000n, 4, '104')]);
check('nanosecond timestamp', () => assert.equal(timeNs(stamp(123_456_789n)), B + 123_456_789n));
check('capture equal to decision excluded', () => assert.equal(produce(sample)[0].features.seen_trade_count, '1'));
check('trade before book initialization retained', () => { const out = produce(sample); assert.equal(out[0].features.last_trade_price, '9007199254740993123'); assert.equal(auditRows(sample, out)[0].ready_at_decision, false); });
check('complete same-time bucket includes both in arrival order', () => { const f = produce(sample)[1].features; assert.equal(f.seen_trade_count, '3'); assert.equal(f.last_trade_price, '103'); });
check('full integer quantity precision', () => assert.equal(produce(sample)[9].features.observed_trade_quantity_1s, '9007199254740993129'));
check('window left endpoint inclusive', () => assert.equal(produce(sample)[10].features.observed_trade_count_1s, '2'));
check('left-truncated window null', () => assert.equal(produce(sample)[0].features.observed_trade_quantity_1s, null));
check('observed no-trade history has null price and age', () => { const f = produce(numbered([row(0n), row(1_000_000_000n)]))[9].features; assert.equal(f.last_trade_price, null); assert.equal(f.since_last_trade_ns, null); assert.equal(f.seen_trade_count, '0'); assert.equal(f.observed_trade_quantity_1s, '0'); });
check('EOF bucket not consumed', () => assert.equal(produce(sample).at(-1).features.seen_trade_count, '3'));
check('empty or single bucket has no decisions', () => { assert.deepEqual(produce([]), []); assert.deepEqual(produce([row(0n, 1)]), []); });
check('same-time truncation does not seal partial bucket', () => { const prefix = sample.slice(0, 2); assert.equal(produce(prefix).length, 1); assert.deepEqual(produce(prefix), produce(sample).slice(0, 1)); });
check('duplicate sequence rejected', () => assert.throws(() => produce(numbered([row(0n, 1), row(1n, 1)])), /duplicate/));
check('distinct identical-price prints retained', () => assert.equal(produce(numbered([row(0n, 1), row(0n, 2), row(100_000_000n)]))[0].features.seen_trade_count, '2'));
check('invalid scale, precision, order, known time rejected', () => { for (const mutation of [r => {r.new_trade.scale = 1;}, r => {r.new_trade.price = 1.1;}, r => {r.new_trade.known_at_capture = stamp(1n);}]) { const r = row(0n, 1); mutation(r); assert.throws(() => produce([r])); } assert.throws(() => produce([row(2n), row(1n)]), /backwards/); });
check('quote/status/side/line/future-known-field isolation', () => { const altered = structuredClone(sample); altered.forEach(r => {r.quote = {bid_price: 'bad'}; r.status = 'sequence_gap'; r.line = 999; r.pending = 123456; r.transition = 'snapshot_batch'; r.label = 'up'; r.resolved_at_ns = 'future'; if (r.new_trade) {r.new_trade.maker_buy = true; r.new_trade.known_at_line = -99; r.new_trade.event_at = '2099-01-01T00:00:00Z';}}); assert.deepEqual(produce(altered), produce(sample)); });
check('gap is audit-only and completeness never assumed', () => { const rows = numbered([row(0n, 1), row(200_000_000n, null, '100', '1', 'sequence_gap'), row(1_100_000_000n)]); const a = auditRows(rows, produce(rows)).at(-1); assert.equal(a.window_has_sequence_gap, true); assert.equal(a.trade_stream_completeness, 'not_established'); });
check('future suffix perturbation preserves past features', () => { const altered = structuredClone(sample); altered.at(-1).new_trade.price = '999999'; altered.push({...row(1_300_000_000n, 5), line: 7}); assert.deepEqual(produce(altered).slice(0, produce(sample).length), produce(sample)); });
check('manual input matches independent reference and cutoff API', () => { assert.deepEqual(produce(sample), reference(sample)); const x = tradeInput(sample); for (const d of produce(sample)) assert.deepEqual(d.features, featuresAt(x.trades, BigInt(d.decision_ns), x.captures[0], x.captures.at(-1))); assert.throws(() => featuresAt(x.trades, x.captures.at(-1) + 1n, x.captures[0], x.captures.at(-1)), /watermark/); });
check('64 deterministic mixed histories vs independent reference', () => { let seed = 1487; const rand = () => (seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0); for (let trial = 0; trial < 64; trial++) { let ns = 0n, seq = 0; const rows = []; for (let j = 0; j < 80; j++) {ns += BigInt(rand() % 4) * 10_000_000n; rows.push(row(ns, rand() % 3 === 0 ? ++seq : null, String(rand() + 1), String(rand() + 1)));} assert.deepEqual(produce(rows), reference(rows)); } });
let real = null;
if (process.argv[2]) {
  const bytes = fs.readFileSync(process.argv[2]);
  assert.equal(sha(bytes), SOURCE_SHA);
  const rows = bytes.toString('utf8').trimEnd().split('\n').map(JSON.parse);
  const out = produce(rows);
  check('fixed SHA real sample vs independent raw-row reference', () => assert.deepEqual(out, reference(rows)));
  const labels = labelRows(rows).decisions, audit = auditRows(rows, out);
  check('572 identities and shared cutoff/eligibility match profile/2', () => { assert.equal(out.length, 572); assert.deepEqual(out.map(x => [x.product_id, x.decision_ns]), labels.map(x => [x.product_id, x.decision_ns])); for (let i = 0; i < audit.length; i++) for (const key of ['prefix_end_line', 'source_capture_ns', 'ready_at_decision', 'status_at_decision']) assert.equal(audit[i][key], labels[i][key]); });
  check('real prefix cuts preserve all watermark-known feature rows', () => { for (const n of [50, 339, 340, 1000, 10000, rows.length - 1]) {const prefix = produce(rows.slice(0, n)); assert.deepEqual(prefix, out.slice(0, prefix.length));} });
  check('real future trade suffix mutations preserve past features', () => { const cut = BigInt(out[250].decision_ns); const changed = structuredClone(rows); for (const r of changed) if (r.new_trade && refTime(r.available_at_capture) >= cut) {r.new_trade.price = '123456789012345678901'; r.new_trade.quantity = '987654321098765432109';} assert.deepEqual(produce(changed).slice(0, 251), out.slice(0, 251)); });
  if (process.argv[3]) {
    check('saved exports match recomputation and hashes', () => {
      const dir = process.argv[3], report = JSON.parse(fs.readFileSync(`${dir}/report.json`, 'utf8'));
      assert.deepEqual(fs.readFileSync(`${dir}/features.jsonl`, 'utf8').trimEnd().split('\n').map(JSON.parse), out);
      for (const [name, expected] of Object.entries(report.output_sha256)) assert.equal(sha(fs.readFileSync(`${dir}/${name}`)), expected);
    });
    check('wrong source SHA and existing output directory rejected', () => {
      assert.throws(() => run(process.argv[2], process.argv[3], '0'.repeat(64)), /SHA256/);
      assert.throws(() => run(process.argv[2], process.argv[3], SOURCE_SHA), /already exists/);
    });
  }
  real = {source_sha256: SOURCE_SHA, rows: rows.length, decisions: out.length, observed_trades: tradeInput(rows).trades.length};
}
console.log(JSON.stringify({passed: checks.length, checks, real, test_code_sha256: sha(fs.readFileSync(new URL(import.meta.url)))}, null, 2));
