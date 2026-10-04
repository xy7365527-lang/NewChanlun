import assert from 'node:assert/strict';
import fs from 'node:fs';
import { labelRows, timeNs, GRID_NS, HORIZON_NS } from './order_label_probe.mjs';

const G = 100_000_000n, H = 100_000_000n;
const epoch = timeNs('2021-01-01T00:00:00Z');
function view(ms, mid = 101, status = 'ready', transition = 'single_event') {
  return {schema: 'coinbase-causal-view/1', product_id: 'TEST', line: 0,
    available_at_capture: ms === 0 ? '2020-12-31T23:59:59.999999999Z' : new Date(1609459200000 + ms).toISOString(), status, transition,
    quote: status === 'ready' ? {bid_price: String(mid - 1), ask_price: String(mid + 1),
      bid_quantity: '1', ask_quantity: '1', scale: 100000000} : null};
}
const numbered = rows => rows.map((r, i) => ({...r, line: i + 1}));
const actual = rows => labelRows(numbered(rows), G, H).decisions;
const decisionOnly = r => Object.fromEntries(Object.entries(r).filter(([k]) =>
  !['label', 'disposition', 'resolved_at_ns', 'witness_line'].includes(k)));
let checks = 0;
function check(name, test) { test(); checks++; }

// Independent linear scan: no use of bucketsOf, nextBoundary or outcome.
function reference(rows, grid, horizon) {
  if (!rows.length) return [];
  const points = [];
  let i = 0;
  while (i < rows.length) {
    const t = timeNs(rows[i].available_at_capture);
    let broken = false, last;
    do {
      last = rows[i++];
      broken ||= last.status !== 'ready' || ['snapshot_batch', 'catchup_batch'].includes(last.transition);
    } while (i < rows.length && timeNs(rows[i].available_at_capture) === t);
    points.push({t, broken, last, ok: last.status === 'ready',
      mid: last.quote ? BigInt(last.quote.bid_price) + BigInt(last.quote.ask_price) : null});
  }
  const out = [], end = points.at(-1).t;
  for (let t = (points[0].t / grid + 1n) * grid; t <= end; t += grid) {
    const known = points.filter(p => p.t < t).at(-1);
    let label = null, disposition = 'ineligible_at_decision', resolved = null, witness = null;
    if (known.ok) {
      disposition = end <= t + horizon ? 'censored_eof' : 'observed_no_change';
      label = disposition === 'observed_no_change' ? 'none' : null;
      resolved = (end <= t + horizon ? end : t + horizon).toString();
      for (const future of points) {
        if (future.t < t) continue;
        if (future.t >= end || future.t > t + horizon) break;
        if (future.broken || !future.ok || future.mid !== known.mid) {
          disposition = future.broken || !future.ok ? 'censored_continuity' : 'observed_change';
          label = disposition === 'observed_change' ? (future.mid > known.mid ? 'up' : 'down') : null;
          resolved = future.t.toString(); witness = future.last.line;
          break;
        }
      }
    }
    out.push({decision_ns: t.toString(), prefix_end_line: known.last.line,
      label, disposition, resolved_at_ns: resolved, witness_line: witness});
  }
  return out;
}
const comparisonFields = r => Object.fromEntries(['decision_ns', 'prefix_end_line', 'label',
  'disposition', 'resolved_at_ns', 'witness_line'].map(k => [k, r[k]]));

check('nanoseconds survive parsing', () => {
  assert.equal(timeNs('2021-01-01T00:00:00.000000001Z') - epoch, 1n);
  assert.equal(timeNs('2021-01-01T00:00:00.1234567Z') - epoch, 123456700n);
  assert.throws(() => timeNs('2021-02-30T00:00:00Z'));
});
check('first change is not endpoint return', () => {
  const rows = actual([view(0), view(20, 99), view(100)]);
  assert.equal(rows[0].label, 'down');
  assert.equal(rows[0].witness_line, 2);
});
check('no-change requires complete horizon', () => {
  assert.equal(actual([view(0), view(101)])[0].label, 'none');
  const tail = actual([view(0), view(99)])[0];
  assert.equal(tail.label, null); assert.equal(tail.disposition, 'censored_eof');
});
check('observed change remains valid before EOF', () => {
  assert.equal(actual([view(0), view(20, 102), view(21, 102)])[0].label, 'up');
});
check('gap cannot be bridged even at the same later price', () => {
  const rows = actual([view(0), view(20, 101, 'sequence_gap'), view(50), view(100)]);
  assert.equal(rows[0].disposition, 'censored_continuity');
  assert.equal(rows[0].label, null);
});
check('resolved move survives later gap', () => {
  assert.equal(actual([view(0), view(10, 102), view(20, 101, 'sequence_gap'), view(100)])[0].label, 'up');
});
check('same-time transients cannot become a label', () => {
  const rows = actual([view(0), view(20, 102), view(20), view(101)]);
  assert.equal(rows[0].label, 'none');
});
check('all closed same-time rows are visible at decision', () => {
  const rows = actual([view(0), view(0, 102), view(20, 101), view(100)]);
  assert.equal(rows[0].prefix_end_line, 2); assert.equal(rows[0].label, 'down');
});
check('same-time gap and repair still break past target continuity', () => {
  const rows = actual([view(0), view(20, 101, 'sequence_gap'), view(20, 102, 'ready', 'catchup_batch'), view(100, 102), view(201, 102)]);
  assert.equal(rows[0].disposition, 'censored_continuity');
  assert.equal(rows[1].label, 'none');
});
check('new snapshot breaks previous target but permits new starting state', () => {
  const rows = actual([view(0), view(50, 102, 'ready', 'snapshot_batch'), view(201, 102)]);
  assert.equal(rows[0].disposition, 'censored_continuity'); assert.equal(rows[1].label, 'none');
});
check('horizon endpoint included; future after it excluded', () => {
  assert.equal(actual([view(0), view(100, 102), view(101, 102)])[0].label, 'up');
  assert.equal(actual([view(0), view(101, 102)])[0].label, 'none');
});
check('unavailable decision retained', () => {
  assert.equal(actual([view(0, 101, 'awaiting_snapshot'), view(200)])[0].disposition, 'ineligible_at_decision');
});
check('future mutation changes target but not as-of record', () => {
  const a = actual([view(0), view(20, 102), view(200)]);
  const b = actual([view(0), view(20, 100), view(200)]);
  assert.deepEqual(decisionOnly(a[0]), decisionOnly(b[0]));
  assert.notEqual(a[0].label, b[0].label);
});
check('physical cutoff preserves decisions and already-resolved targets', () => {
  const data = numbered([view(0), view(20, 102), view(100, 102), view(150, 100), view(400)]);
  const full = labelRows(data, G, H).decisions;
  const partial = labelRows(data.slice(0, 3), G, H).decisions;
  for (const row of partial) {
    const complete = full.find(x => x.decision_ns === row.decision_ns);
    assert.deepEqual(decisionOnly(row), decisionOnly(complete));
    if (complete.resolved_at_ns && BigInt(complete.resolved_at_ns) < epoch + 100_000_000n) {
      assert.deepEqual(row, complete);
    }
  }
  assert.equal(partial.at(-1).disposition, 'censored_eof');
});
check('terminal time bucket is unclosed, and a same-time append cannot revise a settled label', () => {
  const rows = numbered([view(0), view(20, 102), view(20), view(101)]);
  const partial = labelRows(rows.slice(0, 2), G, H).decisions[0];
  const full = labelRows(rows, G, H).decisions[0];
  assert.equal(partial.disposition, 'censored_eof');
  assert.equal(full.label, 'none');
  assert.deepEqual(decisionOnly(partial), decisionOnly(full));
});
check('capture exactly on the grid is target data, never a feature input', () => {
  const rows = actual([view(0), view(100, 102), view(101, 102), view(201, 102)]);
  assert.equal(rows[1].prefix_end_line, 1);
  assert.equal(rows[1].label, 'up');
  assert.equal(rows[1].resolved_at_ns, rows[1].decision_ns);
});
check('invalid schema, missing line, crossing and reversed time rejected', () => {
  const rows = numbered([view(0), view(100)]);
  assert.throws(() => labelRows([{...rows[0], schema: 'wrong'}]));
  assert.throws(() => labelRows([rows[0], {...rows[1], line: 3}]));
  assert.throws(() => labelRows(numbered([view(100), view(0)])));
  assert.throws(() => labelRows([{...rows[0], quote: {...rows[0].quote, ask_price: '99'}}]));
});
check('deterministic varied traces match separate linear reference', () => {
  let seed = 146735;
  const rand = n => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed % n; };
  for (let trial = 0; trial < 64; trial++) {
    let t = 0, mid = 101;
    const data = [];
    for (let n = 0; n < 60; n++) {
      t += rand(50); mid += rand(3) - 1;
      data.push(view(t, mid, rand(19) === 0 ? 'sequence_gap' : 'ready', rand(23) === 0 ? 'catchup_batch' : 'single_event'));
    }
    const rows = numbered(data);
    assert.deepEqual(labelRows(rows, G, H).decisions.map(comparisonFields), reference(rows, G, H));
  }
});

let realRows = 0, realDecisions = 0;
if (process.argv[2]) {
  const rows = fs.readFileSync(process.argv[2], 'utf8').trimEnd().split('\n').map(JSON.parse);
  const generated = labelRows(rows);
  assert.deepEqual(generated.decisions.map(comparisonFields), reference(rows, GRID_NS, HORIZON_NS));
  realRows = rows.length; realDecisions = generated.decisions.length;
  for (const cut of [339, 340, 1000, 10000]) {
    const truncated = rows.slice(0, cut);
    const frontier = timeNs(truncated.at(-1).available_at_capture);
    const partial = labelRows(truncated).decisions;
    for (const decision of partial) {
      const full = generated.decisions.find(r => r.decision_ns === decision.decision_ns);
      assert.deepEqual(decisionOnly(decision), decisionOnly(full));
      if (full.resolved_at_ns && BigInt(full.resolved_at_ns) < frontier) assert.deepEqual(decision, full);
    }
  }
}
console.log(JSON.stringify({checks, varied_traces: 64, real_rows: realRows, real_decisions: realDecisions,
  asof_prefix_checks: realRows ? 4 : 0, passed: true, reviewer: 'author_only'}, null, 2));
