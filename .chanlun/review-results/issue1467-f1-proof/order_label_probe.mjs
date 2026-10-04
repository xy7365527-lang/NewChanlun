import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';

export const PROFILE = 'published-mid-first-change/2';
export const GRID_NS = 100_000_000n;
export const HORIZON_NS = 1_000_000_000n;
const sha = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');

function requireThat(test, message) {
  if (!test) throw new Error(message);
}

export function timeNs(s) {
  const match = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d{1,9}))?Z$/.exec(s);
  requireThat(match, 'unsupported capture timestamp');
  const ms = Date.parse(`${match[1]}Z`);
  requireThat(Number.isSafeInteger(ms) && ms >= 0, 'invalid capture timestamp');
  requireThat(new Date(ms).toISOString().slice(0, 19) === match[1], 'normalized invalid date');
  return BigInt(ms) * 1_000_000n + BigInt((match[2] ?? '').padEnd(9, '0'));
}

function positiveInteger(s) {
  requireThat(typeof s === 'string' && /^[1-9]\d*$/.test(s), 'invalid positive integer string');
  return BigInt(s);
}

// This validates the view contract, not the underlying exchange book.
export function bucketsOf(rows) {
  const buckets = [];
  let previousLine = null;
  let product = null;
  for (const row of rows) {
    requireThat(row.schema === 'coinbase-causal-view/1', 'unexpected view schema');
    requireThat(Number.isSafeInteger(row.line) && row.line > 0, 'invalid line identity');
    requireThat(previousLine === null || row.line === previousLine + 1, 'noncontiguous input lines');
    previousLine = row.line;
    requireThat(typeof row.product_id === 'string' && row.product_id.length > 0, 'missing product');
    if (product === null) product = row.product_id;
    requireThat(row.product_id === product, 'mixed products');
    const t = timeNs(row.available_at_capture);
    const previous = buckets.at(-1);
    requireThat(!previous || t >= previous.t, 'backwards capture time');
    const ready = row.status === 'ready';
    let mid2 = null;
    if (ready) {
      requireThat(row.quote && row.quote.scale === 100_000_000, 'unsupported quote scale');
      const bid = positiveInteger(row.quote.bid_price);
      const ask = positiveInteger(row.quote.ask_price);
      positiveInteger(row.quote.bid_quantity);
      positiveInteger(row.quote.ask_quantity);
      requireThat(bid < ask, 'locked or crossed ready quote');
      mid2 = bid + ask;
    } else {
      requireThat(row.quote === null, 'unavailable view exposes quote');
    }
    const barrier = !ready || row.transition === 'snapshot_batch' || row.transition === 'catchup_batch';
    if (previous && previous.t === t) {
      previous.lastLine = row.line;
      previous.ready = ready;
      previous.status = row.status;
      previous.mid2 = mid2;
      previous.barrier ||= barrier;
      previous.rows += 1;
    } else {
      buckets.push({t, firstLine: row.line, lastLine: row.line, rows: 1, ready,
        status: row.status, mid2, barrier, product});
    }
  }
  return buckets;
}

// The as-of selector reads no future quote or structure.
export function decisionAt(bucket, t, horizonNs) {
  return {profile: PROFILE, product_id: bucket.product, decision_ns: t.toString(),
    prefix_end_line: bucket.lastLine, source_capture_ns: bucket.t.toString(),
    ready_at_decision: bucket.ready, status_at_decision: bucket.status,
    deadline_ns: (t + horizonNs).toString()};
}

function outcome(bucket, next, t, lastT, horizonNs) {
  if (!bucket.ready) return {label: null, disposition: 'ineligible_at_decision', resolved_at_ns: null, witness_line: null};
  const deadline = t + horizonNs;
  if (next && next.t < lastT && next.t <= deadline) {
    if (next.barrier || !next.ready) return {label: null, disposition: 'censored_continuity',
      resolved_at_ns: next.t.toString(), witness_line: next.lastLine};
    return {label: next.mid2 > bucket.mid2 ? 'up' : 'down', disposition: 'observed_change',
      resolved_at_ns: next.t.toString(), witness_line: next.lastLine};
  }
  if (lastT > deadline) return {label: 'none', disposition: 'observed_no_change',
    resolved_at_ns: deadline.toString(), witness_line: null};
  return {label: null, disposition: 'censored_eof', resolved_at_ns: lastT.toString(), witness_line: null};
}

export function labelRows(rows, gridNs = GRID_NS, horizonNs = HORIZON_NS) {
  requireThat(typeof gridNs === 'bigint' && gridNs > 0n, 'grid must be positive');
  requireThat(typeof horizonNs === 'bigint' && horizonNs > 0n, 'horizon must be positive');
  const buckets = bucketsOf(rows);
  if (!buckets.length) return {buckets, decisions: []};
  const nextBoundary = new Array(buckets.length).fill(null);
  for (let i = buckets.length - 2; i >= 0; i--) {
    const next = buckets[i + 1];
    nextBoundary[i] = next.barrier || !next.ready || next.mid2 !== buckets[i].mid2
      ? i + 1 : nextBoundary[i + 1];
  }
  const end = buckets.at(-1).t;
  // Capture timestamps equal to the decision time are not yet feature inputs.
  const start = (buckets[0].t / gridNs + 1n) * gridNs;
  requireThat((end - start) / gridNs < 1_000_000n, 'probe decision limit exceeded');
  let i = 0;
  const decisions = [];
  for (let t = start; t <= end; t += gridNs) {
    while (i + 1 < buckets.length && buckets[i + 1].t < t) i++;
    const next = nextBoundary[i] === null ? null : buckets[nextBoundary[i]];
    decisions.push({...decisionAt(buckets[i], t, horizonNs),
      ...outcome(buckets[i], next, t, end, horizonNs)});
  }
  return {buckets, decisions};
}

export function summarize(buckets, decisions) {
  const dispositions = {}, labels = {};
  const witnesses = new Map();
  for (const row of decisions) {
    dispositions[row.disposition] = (dispositions[row.disposition] ?? 0) + 1;
    if (row.label !== null) labels[row.label] = (labels[row.label] ?? 0) + 1;
    if (row.disposition === 'observed_change') {
      witnesses.set(row.witness_line, (witnesses.get(row.witness_line) ?? 0) + 1);
    }
  }
  return {capture_buckets: buckets.length, equal_time_buckets: buckets.filter(b => b.rows > 1).length,
    continuity_barriers: buckets.filter(b => b.barrier).length, decisions: decisions.length,
    dispositions, labels, distinct_change_witnesses: witnesses.size,
    max_decisions_sharing_change: [...witnesses.values()].reduce((a, b) => Math.max(a, b), 0),
    zero_duration_change_labels: decisions.filter(r => r.disposition === 'observed_change' && r.resolved_at_ns === r.decision_ns).length};
}

export function run(input, outputDirectory, expectedSha) {
  const bytes = fs.readFileSync(input);
  requireThat(bytes.length <= 32 * 1024 * 1024, 'input exceeds probe cap');
  requireThat(/^[a-f0-9]{64}$/.test(expectedSha), 'expected SHA256 is required');
  requireThat(sha(bytes) === expectedSha, 'input SHA256 mismatch');
  requireThat(!fs.existsSync(outputDirectory), 'output directory already exists');
  const rows = bytes.toString('utf8').trimEnd().split('\n').filter(Boolean).map(JSON.parse);
  const {buckets, decisions} = labelRows(rows);
  const output = decisions.map(row => JSON.stringify(row)).join('\n') + (decisions.length ? '\n' : '');
  requireThat(Buffer.byteLength(output) < 16 * 1024 * 1024, 'output exceeds probe cap');
  fs.mkdirSync(outputDirectory, {recursive: true});
  fs.writeFileSync(path.join(outputDirectory, 'labels.jsonl'), output, {flag: 'wx'});
  const report = {profile: PROFILE, grid_ns: GRID_NS.toString(), horizon_ns: HORIZON_NS.toString(),
    input_sha256: expectedSha, input_rows: rows.length, output_sha256: sha(output),
    ...summarize(buckets, decisions), scope: 'exploratory target feasibility only; no features, model fit, C qualification, value or PnL',
    independent_review: 'not_reviewed'};
  fs.writeFileSync(path.join(outputDirectory, 'report.json'), JSON.stringify(report, null, 2) + '\n', {flag: 'wx'});
  return report;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    requireThat(process.argv.length === 5, 'usage: node order_label_probe.mjs INPUT NEW_OUTPUT_DIR EXPECTED_SHA256');
    console.log(JSON.stringify(run(...process.argv.slice(2)), null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
