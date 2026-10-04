import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import zlib from 'node:zlib';
import {pathToFileURL} from 'node:url';
import {tradeInput, featuresAt as tradeFeaturesAt, timeNs, GRID_NS, WINDOW_NS, SCALE} from './trade_grid_features.mjs';

export const PROFILE = 'book-history-grid-b/1';
export const SOURCE_SHA = 'a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5';
export const NATIVE_SHA = '5c233367178b64ddbcd992bf869793f4bf12a415ed90d50d1ee1d481d04763cf';
export const NATIVE_PLAIN_SHA = '6a3c7eb8a47c5c2e9cdbbee77200286be6133349f2a5d328d3cec300c0496b30';
export const RAW_SHA = '8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd';
export const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const need = (ok, message) => {if (!ok) throw Error(message);};
const jsonl = rows => rows.map(x => JSON.stringify(x)).join('\n') + (rows.length ? '\n' : '');
const decode = bytes => bytes.toString('utf8').trimEnd().split('\n').filter(Boolean).map(JSON.parse);
const positive = x => {need(typeof x === 'string' && /^[1-9]\d*$/.test(x), 'invalid quote integer'); return BigInt(x);};
function quote(q) {
  if (q === null) return null;
  need(q && q.scale === SCALE, 'invalid quote scale');
  const b = positive(q.bid_price), a = positive(q.ask_price);
  positive(q.bid_quantity); positive(q.ask_quantity); need(b < a, 'invalid quote spread');
  return {bid_price: q.bid_price, ask_price: q.ask_price, bid_quantity: q.bid_quantity, ask_quantity: q.ask_quantity};
}

// Projection contains no label/eligibility/F1/F2 fields. Raw status controls
// continuity only; current quote validity is an input contract, not a row filter.
export function bookInput(rows) {
  const trades = tradeInput(rows), publications = [], buckets = [];
  rows.forEach((r, i) => {
    need(r.line === i + 1, 'noncontiguous lines');
    need(typeof r.status === 'string' && typeof r.transition === 'string', 'missing continuity state');
    need((r.status === 'ready') === (r.quote !== null), 'quote/status mismatch');
    const capture = timeNs(r.available_at_capture), q = quote(r.quote);
    const barrier = r.status !== 'ready' || !['single_event', 'no_new_book_application'].includes(r.transition);
    publications.push({capture_ns: String(capture), source_line: r.line, quote: q, barrier, status: r.status, transition: r.transition});
    const last = buckets.at(-1);
    if (last?.capture === capture) {last.quote = q; last.barrier ||= barrier; last.end = i + 1;}
    else buckets.push({capture, quote: q, barrier, end: i + 1});
  });
  return {product: trades.product, origin: trades.captures[0] ?? null,
    watermark: trades.captures.at(-1) ?? null, trades: trades.trades, publications, buckets};
}

// Independent callers get a prefix, never a pointer to future rows. Strictly
// t<=watermark makes every included capture bucket sealed even on physical EOF.
export function cutoffAt(input, t) {
  need(typeof t === 'bigint' && input.origin !== null && t > input.origin && t <= input.watermark, 'decision outside watermark');
  let lo = 0, hi = input.buckets.length;
  while (lo < hi) {const mid = (lo + hi) >> 1; if (input.buckets[mid].capture < t) lo = mid + 1; else hi = mid;}
  return {bucket_end: lo, publication_end: lo ? input.buckets[lo - 1].end : 0};
}
const sameQuote = (a, b) => JSON.stringify(a) === JSON.stringify(b);
export function featuresAt(input, t) {
  const {bucket_end: end} = cutoffAt(input, t), last = input.buckets[end - 1], q = last?.quote;
  const left = t - WINDOW_NS;
  let anchor = -1;
  for (let i = 0; i < end && input.buckets[i].capture <= left; i++) anchor = i;
  const full = left >= input.origin && anchor >= 0 && input.buckets[anchor].quote !== null &&
    !input.buckets.slice(anchor, end).some(b => b.capture >= left && b.barrier);
  let changes = null, delta = null;
  if (full) {
    changes = 0n;
    for (let i = anchor + 1; i < end; i++) if (!sameQuote(input.buckets[i - 1].quote, input.buckets[i].quote)) changes++;
    const first = input.buckets[anchor].quote;
    delta = BigInt(q.bid_price) + BigInt(q.ask_price) - BigInt(first.bid_price) - BigInt(first.ask_price);
  }
  return {...tradeFeaturesAt(input.trades, t, input.origin, input.watermark),
    bid_price: q?.bid_price ?? null, ask_price: q?.ask_price ?? null,
    bid_quantity: q?.bid_quantity ?? null, ask_quantity: q?.ask_quantity ?? null,
    spread: q ? String(BigInt(q.ask_price) - BigInt(q.bid_price)) : null,
    queue_imbalance: q ? {numerator: String(BigInt(q.bid_quantity) - BigInt(q.ask_quantity)), denominator: String(BigInt(q.bid_quantity) + BigInt(q.ask_quantity))} : null,
    published_l1_changes_1s: changes === null ? null : String(changes),
    published_mid2_delta_1s: delta === null ? null : String(delta)};
}
export function gridFeatures(input) {
  if (input.origin === null) return [];
  need((input.watermark - input.origin) / GRID_NS < 1_000_000n, 'grid cap');
  const out = [];
  for (let t = (input.origin / GRID_NS + 1n) * GRID_NS; t <= input.watermark; t += GRID_NS)
    out.push({profile: PROFILE, product_id: input.product, decision_ns: String(t), features: featuresAt(input, t)});
  return out;
}

// Complete receipt history is retained as opaque source strings: decimal native
// JSON numbers are never rounded by JSON.parse. Sequence never drives cutoff.
export function rawInput(bytes, input) {
  const lines = bytes.toString('utf8').split('\n'); if (lines.at(-1) === '') lines.pop();
  need(lines.length === input.publications.length, 'raw line count mismatch');
  let offset = 0;
  return lines.map((line, i) => {
    const split = line.indexOf(' '); need(split > 0, 'invalid raw capture prefix');
    const capture = timeNs(line.slice(0, split));
    need(String(capture) === input.publications[i].capture_ns, 'raw capture alignment mismatch');
    const start = offset; offset += Buffer.byteLength(line) + (i < lines.length - 1 || bytes.at(-1) === 10 ? 1 : 0);
    return {capture_ns: String(capture), source_line: i + 1, byte_start: start, byte_end: offset, raw_message: line.slice(split + 1)};
  });
}

// Strip all Stage26 model-state, model-event, active and confirmed structures.
// Preserve every application (including noops and snapshot anchors) and its
// source location. Full original applications remain hash-addressed separately.
export function nativeInput(rows, input) {
  need(rows.length === input.publications.length, 'native row count mismatch');
  const out = [];
  rows.forEach((r, i) => {
    const p = input.publications[i];
    need(r.profile === 'rd-q-native-available/1' && r.line === i + 1 && timeNs(r.available_at_capture).toString() === p.capture_ns, 'native row alignment mismatch');
    need(Array.isArray(r.applications), 'missing applications');
    r.applications.forEach((a, j) => {
      const capture = a.type === 'anchor' ? a.quote?.available_capture : a.available_capture;
      const availableLine = a.type === 'anchor' ? a.quote?.available_line : a.available_line;
      need(timeNs(capture).toString() === p.capture_ns && availableLine === r.line, 'native application availability mismatch');
      need(['anchor', 'applied'].includes(a.type), 'unsupported native application');
      const item = {available_capture_ns: p.capture_ns, available_line: r.line, application_index: j, type: a.type};
      if (a.type === 'anchor') {item.reason = a.reason; item.quote = a.quote.value; item.quote_status = a.quote.status;}
      else {
        need(Number.isSafeInteger(a.captured_line) && a.captured_line > 0 && a.captured_line <= r.line, 'invalid original capture line');
        need(timeNs(a.captured_at).toString() === input.publications[a.captured_line - 1].capture_ns, 'original capture alignment mismatch');
        for (const key of ['native_sequence', 'native_type', 'native_event_at', 'captured_at', 'captured_line', 'disposition', 'normalization', 'buy', 'price', 'signed_delta', 'level_before', 'level_after', 'before_status', 'after_status', 'before_quote', 'after_quote'])
          if (Object.hasOwn(a, key)) item[key] = a[key];
      }
      out.push(item);
    });
  });
  return out;
}
export function historyAt(input, native, raw, t) {
  const {publication_end} = cutoffAt(input, t);
  return {publications: input.publications.slice(0, publication_end),
    trades: input.trades.filter(x => x.capture < t).map(x => ({capture_ns: String(x.capture), price: String(x.price), quantity: String(x.quantity)})),
    applications: native.filter(x => BigInt(x.available_capture_ns) < t),
    receipts: raw.slice(0, publication_end)};
}

export function run(viewPath, nativePath, rawPath, outputDirectory) {
  need(!fs.existsSync(outputDirectory), 'output directory already exists');
  const sources = [viewPath, nativePath, rawPath].map(p => fs.readFileSync(p));
  [SOURCE_SHA, NATIVE_SHA, RAW_SHA].forEach((s, i) => need(sha(sources[i]) === s, 'input SHA256 mismatch'));
  const plain = zlib.gunzipSync(sources[1]); need(sha(plain) === NATIVE_PLAIN_SHA, 'native plain SHA mismatch');
  const rows = decode(sources[0]), input = bookInput(rows), native = nativeInput(decode(plain), input), raw = rawInput(sources[2], input);
  const features = gridFeatures(input);
  const audit = features.map(d => {
    const t = BigInt(d.decision_ns), cut = cutoffAt(input, t), p = input.publications[cut.publication_end - 1];
    const n = native.filter(x => BigInt(x.available_capture_ns) < t).length;
    return {product_id: d.product_id, decision_ns: d.decision_ns, prefix_end_line: cut.publication_end,
      source_capture_ns: p.capture_ns, ready_at_decision: p.status === 'ready', status_at_decision: p.status,
      raw_prefix_byte_end: raw[cut.publication_end - 1].byte_end, native_application_end: n,
      trade_stream_completeness: 'not_established', source_market_completeness: 'not_established',
      published_window_valid: d.features.published_l1_changes_1s !== null};
  });
  const manifest = {profile: PROFILE, source_history_start_ns: String(input.origin), watermark_ns: String(input.watermark), cutoff_rule: 'capture < decision <= watermark; EOF never seals final capture bucket',
    sources: [viewPath, nativePath, rawPath].map((p, i) => ({path: path.resolve(p), sha256: sha(sources[i]), bytes: sources[i].length})),
    native_uncompressed_sha256: NATIVE_PLAIN_SHA,
    raw_history: 'all received source messages including pending, snapshot and payload; source byte prefix is exclusive',
    native_history: 'all Stage26 applications by available_capture, including noops; derived F1/F2 stripped from projection',
    gap: 'source receipt history is complete relative to this saved file, exchange feed completeness is not established; pending receipt is known before application; no recovered state is backdated',
    missing: ['pre-capture history', 'unobserved exchange messages', 'full four-arm history/initialization/model-budget contract and qualified F1/F2']};
  const payloads = {'features.jsonl': Buffer.from(jsonl(features)), 'audit.jsonl': Buffer.from(jsonl(audit)),
    'quote_history.jsonl.gz': zlib.gzipSync(jsonl(input.publications)), 'native_history.jsonl.gz': zlib.gzipSync(jsonl(native)),
    'trade_history.jsonl': Buffer.from(jsonl(input.trades.map(x => ({capture_ns: String(x.capture), price: String(x.price), quantity: String(x.quantity)})))),
    'history-manifest.json': Buffer.from(JSON.stringify(manifest, null, 2) + '\n')};
  const report = {profile: PROFILE, input_sha256: manifest.sources.map(s => s.sha256), code_sha256: sha(fs.readFileSync(new URL(import.meta.url))),
    imported_trade_code_sha256: sha(fs.readFileSync(new URL('./trade_grid_features.mjs', import.meta.url))), decisions: features.length, input_rows: rows.length,
    applications: native.length, applied: native.filter(x => x.type === 'applied').length, anchors: native.filter(x => x.type === 'anchor').length,
    late_applications: native.filter(x => x.type === 'applied' && x.captured_line < x.available_line).length,
    observed_trades: input.trades.length, unavailable_quote_rows: features.filter(x => x.features.bid_price === null).length,
    unavailable_quote_rows_with_trade: features.filter(x => x.features.bid_price === null && x.features.last_trade_price !== null).length,
    null_rolling_windows: features.filter(x => x.features.published_l1_changes_1s === null).length,
    output_sha256: Object.fromEntries(Object.entries(payloads).map(([k, v]) => [k, sha(v)])),
    scope: 'fixed nonrecursive L1 compression plus complete saved-history cutoff interfaces; no training, selection, profit or strongest-B claim'};
  payloads['report.json'] = Buffer.from(JSON.stringify(report, null, 2) + '\n');
  const outputBytes = Object.values(payloads).reduce((n, b) => n + b.length, 0);
  need(outputBytes <= 16 * 1024 * 1024 - 65536, 'output cap exceeded');
  fs.mkdirSync(outputDirectory, {recursive: false, mode: 0o700});
  for (const [name, bytes] of Object.entries(payloads)) fs.writeFileSync(path.join(outputDirectory, name), bytes, {flag: 'wx', mode: 0o600});
  return {...report, output_bytes: outputBytes};
}
if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  need(process.argv.length === 6, 'usage: node book_grid_features.mjs VIEW NATIVE_GZ RAW NEW_DIR');
  console.log(JSON.stringify(run(...process.argv.slice(2)), null, 2));
}
