import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';

export const PROFILE = 'trade-history-grid-a/1';
export const GRID_NS = 100_000_000n;
export const WINDOW_NS = 1_000_000_000n;
export const SCALE = 100_000_000;
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const requireThat = (ok, message) => { if (!ok) throw new Error(message); };

export function timeNs(s) {
  const m = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d{1,9}))?Z$/.exec(s);
  requireThat(m, 'unsupported capture timestamp');
  const ms = Date.parse(`${m[1]}Z`);
  requireThat(Number.isSafeInteger(ms) && ms >= 0 && new Date(ms).toISOString().slice(0, 19) === m[1], 'invalid capture date');
  return BigInt(ms) * 1_000_000n + BigInt((m[2] ?? '').padEnd(9, '0'));
}
function integer(s) {
  requireThat(typeof s === 'string' && /^[1-9]\d*$/.test(s), 'invalid positive integer string');
  return BigInt(s);
}

// Only this projection enters A. Quote, status, line, transition, native event
// time and side are deliberately inaccessible to the feature accumulator.
export function tradeInput(rows) {
  let product = null, previous = null;
  const captures = [], trades = [], seen = new Set();
  for (const row of rows) {
    requireThat(row.schema === 'coinbase-causal-view/1', 'unsupported schema');
    requireThat(typeof row.product_id === 'string' && row.product_id.length > 0, 'missing product');
    product ??= row.product_id;
    requireThat(row.product_id === product, 'mixed products');
    const t = timeNs(row.available_at_capture);
    requireThat(previous === null || t >= previous, 'backwards capture time');
    if (previous === null || t !== previous) captures.push(t);
    previous = t;
    requireThat(Object.hasOwn(row, 'new_trade'), 'missing new_trade field');
    if (row.new_trade === null) continue;
    const tr = row.new_trade;
    requireThat(tr.scale === SCALE, 'unsupported trade scale');
    requireThat(timeNs(tr.known_at_capture) === t, 'trade was not newly known at this capture');
    integer(tr.sequence);
    requireThat(!seen.has(tr.sequence), 'duplicate trade sequence');
    seen.add(tr.sequence);
    // sequence is only a validation identity and never enters trades/features.
    trades.push({capture: t, price: integer(tr.price), quantity: integer(tr.quantity)});
  }
  return {product, captures, trades};
}

// Caller supplies a common observation origin and a timestamp watermark.
// A bucket is sealed iff its capture < watermark. No EOF sealing is allowed.
export function featuresAt(trades, t, origin, watermark) {
  requireThat(t > origin && t <= watermark, 'decision outside observed watermark');
  let count = 0n, last = null, windowCount = 0n, quantity = 0n;
  for (const tr of trades) {
    if (tr.capture >= t) break;
    count++;
    last = tr;
    if (tr.capture >= t - WINDOW_NS) { windowCount++; quantity += tr.quantity; }
  }
  const fullWindow = t - WINDOW_NS >= origin;
  return {last_trade_price: last?.price.toString() ?? null,
    seen_trade_count: count.toString(),
    since_last_trade_ns: last ? (t - last.capture).toString() : null,
    observed_trade_count_1s: fullWindow ? windowCount.toString() : null,
    observed_trade_quantity_1s: fullWindow ? quantity.toString() : null};
}

// Incremental accumulator; standalone featuresAt is a simple public cutoff
// interface. Tests also use an independent raw-row filter/reduce reference.
export function gridFeatures(input) {
  const {product, captures, trades} = input;
  if (!captures.length) return [];
  const origin = captures[0], end = captures.at(-1);
  const start = (origin / GRID_NS + 1n) * GRID_NS;
  requireThat((end - start) / GRID_NS < 1_000_000n, 'decision cap exceeded');
  const decisions = [];
  let right = 0, left = 0, quantity = 0n;
  for (let t = start; t <= end; t += GRID_NS) {
    while (right < trades.length && trades[right].capture < t) {
      quantity += trades[right].quantity;
      right++;
    }
    while (left < right && trades[left].capture < t - WINDOW_NS) {
      quantity -= trades[left].quantity;
      left++;
    }
    const last = right ? trades[right - 1] : null;
    const fullWindow = t - WINDOW_NS >= origin;
    decisions.push({profile: PROFILE, product_id: product, decision_ns: t.toString(),
      features: {last_trade_price: last?.price.toString() ?? null,
        seen_trade_count: BigInt(right).toString(),
        since_last_trade_ns: last ? (t - last.capture).toString() : null,
        observed_trade_count_1s: fullWindow ? BigInt(right - left).toString() : null,
        observed_trade_quantity_1s: fullWindow ? quantity.toString() : null}});
  }
  return decisions;
}

// Audit is an independent path. It cannot feed gridFeatures/featuresAt.
export function auditRows(rows, decisions) {
  let i = -1, previousLine = null;
  for (const row of rows) {
    requireThat(Number.isSafeInteger(row.line) && row.line > 0 && (previousLine === null || row.line === previousLine + 1), 'noncontiguous audit lines');
    previousLine = row.line;
    requireThat(typeof row.status === 'string', 'missing audit status');
  }
  return decisions.map(d => {
    const t = BigInt(d.decision_ns);
    while (i + 1 < rows.length && timeNs(rows[i + 1].available_at_capture) < t) i++;
    const row = rows[i];
    return {product_id: d.product_id, decision_ns: d.decision_ns,
      prefix_end_line: row.line, source_capture_ns: timeNs(row.available_at_capture).toString(),
      ready_at_decision: row.status === 'ready', status_at_decision: row.status,
      window_1s_observed: t - WINDOW_NS >= timeNs(rows[0].available_at_capture),
      window_has_sequence_gap: rows.slice(0, i + 1).some(r => r.status === 'sequence_gap' && timeNs(r.available_at_capture) >= t - WINDOW_NS),
      trade_stream_completeness: 'not_established'};
  });
}

export function run(inputPath, outputDirectory, expectedSha) {
  const bytes = fs.readFileSync(inputPath);
  requireThat(bytes.length <= 32 * 1024 * 1024, 'input cap exceeded');
  requireThat(/^[a-f0-9]{64}$/.test(expectedSha) && sha(bytes) === expectedSha, 'input SHA256 mismatch');
  requireThat(!fs.existsSync(outputDirectory), 'output directory already exists');
  const rows = bytes.toString('utf8').trimEnd().split('\n').filter(Boolean).map(JSON.parse);
  const input = tradeInput(rows), decisions = gridFeatures(input), audits = auditRows(rows, decisions);
  const history = input.trades.map(t => ({capture_ns: t.capture.toString(), price: t.price.toString(), quantity: t.quantity.toString()}));
  const contents = {'features.jsonl': decisions, 'audit.jsonl': audits, 'trade_history.jsonl': history};
  const output_sha256 = {};
  fs.mkdirSync(outputDirectory, {recursive: false});
  for (const [name, records] of Object.entries(contents)) {
    const out = records.map(x => JSON.stringify(x)).join('\n') + (records.length ? '\n' : '');
    fs.writeFileSync(path.join(outputDirectory, name), out, {flag: 'wx'});
    output_sha256[name] = sha(out);
  }
  const report = {profile: PROFILE, alignment_profile: 'published-mid-first-change/2',
    grid_ns: GRID_NS.toString(), window_ns: WINDOW_NS.toString(), price_quantity_scale: SCALE,
    source_sha256: expectedSha, source_rows: rows.length, source_bytes: bytes.length,
    source_capture_buckets: input.captures.length, observed_trades: input.trades.length,
    trades_in_unsealed_final_bucket: input.trades.filter(t => t.capture === input.captures.at(-1)).length,
    decisions: decisions.length, common_ready_decisions: audits.filter(a => a.ready_at_decision).length,
    unready_decisions_with_trade: decisions.filter((d, i) => !audits[i].ready_at_decision && d.features.last_trade_price !== null).length,
    decisions_without_trade: decisions.filter(d => d.features.last_trade_price === null).length,
    decisions_with_left_truncated_window: decisions.filter(d => d.features.observed_trade_count_1s === null).length,
    output_sha256, code_sha256: sha(fs.readFileSync(new URL(import.meta.url))),
    scope: '探索用固定成交特征；不训练、不调参、不判断预测或交易价值；不声称最终A或最强B',
    history_start_ns: input.captures[0]?.toString() ?? null, watermark_ns: input.captures.at(-1)?.toString() ?? null};
  fs.writeFileSync(path.join(outputDirectory, 'report.json'), JSON.stringify(report, null, 2) + '\n', {flag: 'wx'});
  return report;
}
if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    requireThat(process.argv.length === 5, 'usage: node trade_grid_features.mjs INPUT NEW_OUTPUT_DIR EXPECTED_SHA256');
    console.log(JSON.stringify(run(...process.argv.slice(2)), null, 2));
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
