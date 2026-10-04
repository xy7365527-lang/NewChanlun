import fs from 'node:fs';
import zlib from 'node:zlib';
import assert from 'node:assert/strict';
import {bookInput, gridFeatures, featuresAt, historyAt, nativeInput, rawInput, cutoffAt, run, sha, SOURCE_SHA, RAW_SHA, NATIVE_SHA} from './book_grid_features.mjs';
import {tradeInput, gridFeatures as tradeGrid} from './trade_grid_features.mjs';
const B = 1609459200000000000n;
const stamp = n => `2021-01-01T00:00:${String(n / 1000000000n).padStart(2, '0')}.${String(n % 1000000000n).padStart(9, '0')}Z`;
const quote = (b = '100', a = '110', bq = '10', aq = '20') => ({scale: 100000000, bid_price: b, ask_price: a, bid_quantity: bq, ask_quantity: aq});
const row = (ns, q = quote(), transition = 'single_event', tr = null) => ({schema: 'coinbase-causal-view/1', line: 1, product_id: 'BTC-USD', available_at_capture: stamp(ns), status: q ? 'ready' : 'awaiting_snapshot', transition, quote: q, new_trade: tr ? {scale: 100000000, sequence: tr, known_at_capture: stamp(ns), price: '9007199254740993123', quantity: '9007199254740993127'} : null});
const numbered = rows => rows.map((x, i) => ({...x, line: i + 1}));
const produce = rows => gridFeatures(bookInput(rows));
// No production timestamp/cutoff/bucket/A implementation is used here. Each
// decision filters raw rows anew, then folds a timestamp map and fresh sums.
function clock(s) {const [w, f = ''] = s.slice(0, -1).split('.'); return BigInt(Date.parse(w + 'Z')) * 1000000n + BigInt((f + '000000000').slice(0, 9));}
function reference(rows) {
  if (!rows.length) return [];
  const prepared = rows.map(r => ({r, c: clock(r.available_at_capture)}));
  const origin = prepared[0].c, end = prepared.at(-1).c, out = [];
  for (let t = origin - origin % 100000000n + 100000000n; t <= end; t += 100000000n) {
    const past = prepared.filter(x => x.c < t), left = t - 1000000000n;
    const trades = past.filter(x => x.r.new_trade), last = trades.at(-1), win = trades.filter(x => x.c >= left);
    const states = new Map();
    for (const x of past) {
      const old = states.get(String(x.c));
      states.set(String(x.c), {c: x.c, q: x.r.quote, bad: !!old?.bad || x.r.status !== 'ready' || !['single_event', 'no_new_book_application'].includes(x.r.transition)});
    }
    const bs = [...states.values()], base = bs.filter(x => x.c <= left).at(-1), q = bs.at(-1)?.q;
    const known = origin <= left && !!base?.q && !bs.some(x => x.c >= left && x.bad);
    let count = null, delta = null;
    if (known) {
      let prior = base.q; count = 0;
      for (const x of bs.filter(x => x.c > left)) {if (JSON.stringify(x.q) !== JSON.stringify(prior)) count++; prior = x.q;}
      delta = BigInt(q.bid_price) + BigInt(q.ask_price) - BigInt(base.q.bid_price) - BigInt(base.q.ask_price);
    }
    out.push({profile: 'book-history-grid-b/1', product_id: rows[0].product_id, decision_ns: String(t), features: {
      last_trade_price: last?.r.new_trade.price ?? null, seen_trade_count: String(trades.length), since_last_trade_ns: last ? String(t - last.c) : null,
      observed_trade_count_1s: origin <= left ? String(win.length) : null,
      observed_trade_quantity_1s: origin <= left ? String(win.reduce((n, x) => n + BigInt(x.r.new_trade.quantity), 0n)) : null,
      bid_price: q?.bid_price ?? null, ask_price: q?.ask_price ?? null, bid_quantity: q?.bid_quantity ?? null, ask_quantity: q?.ask_quantity ?? null,
      spread: q ? String(BigInt(q.ask_price) - BigInt(q.bid_price)) : null,
      queue_imbalance: q ? {numerator: String(BigInt(q.bid_quantity) - BigInt(q.ask_quantity)), denominator: String(BigInt(q.bid_quantity) + BigInt(q.ask_quantity))} : null,
      published_l1_changes_1s: count === null ? null : String(count), published_mid2_delta_1s: delta === null ? null : String(delta)}});
  }
  return out;
}
const checks = []; const check = (name, f) => {f(); checks.push(name);};
const qBig = quote('9007199254740993123', '9007199254740993140', '9007199254740993160', '9007199254740993110');
const rows = numbered([row(0n, null, 'unavailable', '1'), row(100000000n, qBig, 'snapshot_batch'), row(100000000n, quote(), 'single_event', '2'), row(200000001n, quote('101', '111')), row(1200000000n, quote('102', '112')), row(1300000000n)]);
check('manual history and independent reference agree', () => assert.deepEqual(produce(rows), reference(rows)));
check('capture=t excluded and uninitialized A retained', () => {const f = produce(rows)[0].features; assert.equal(f.bid_price, null); assert.equal(f.seen_trade_count, '1'); assert.equal(f.last_trade_price, '9007199254740993123');});
check('same-time bucket uses final quote and includes every trade', () => {const f = produce(rows)[1].features; assert.equal(f.bid_price, '100'); assert.equal(f.seen_trade_count, '2');});
check('nanosecond cutoff and >Number integer precision', () => {const x = bookInput(numbered([row(0n, qBig), row(100000001n, quote()), row(200000000n)])); const f = featuresAt(x, B + 100000001n); assert.equal(f.bid_price, qBig.bid_price); assert.equal(f.spread, '17'); assert.deepEqual(f.queue_imbalance, {numerator:'50',denominator:'18014398509481986270'});});
check('initialization and window shortage produce null summaries', () => {const out = produce(rows); assert.equal(out[0].features.published_l1_changes_1s, null); assert.equal(out[10].features.published_l1_changes_1s, null); assert.equal(out[11].features.published_l1_changes_1s, '1');});
check('window anchor inclusive; future EOF quote excluded', () => {const f = produce(rows).at(-1).features; assert.equal(f.bid_price, '102'); assert.equal(f.published_mid2_delta_1s, '2');});
check('gap, unknown, snapshot and catchup invalidate rolling without dropping A', () => {
  for (const kind of ['gap', 'unknown', 'snapshot_batch', 'catchup_batch']) {
    const r = row(500000000n, kind.includes('batch') ? quote() : null, kind.includes('batch') ? kind : 'unavailable', '2');
    if (kind === 'gap') r.status = 'sequence_gap'; if (kind === 'unknown') r.status = 'unknown';
    const xs = numbered([row(0n, quote(), 'single_event', '1'), r, row(700000000n), row(1600000000n)]), out = produce(xs);
    assert.equal(out[9].features.published_l1_changes_1s, null); assert.equal(out[9].features.seen_trade_count, '2');
    assert.deepEqual(out, reference(xs));
  }
});
check('unknown transition and reinitialization cannot produce zero-filled windows', () => {for(const kind of ['reinitialized_or_batch','unknown_transition']){const xs=numbered([row(0n),row(500000000n,quote(),kind),row(1100000000n)]);assert.equal(produce(xs).at(-1).features.published_l1_changes_1s,null);}});
check('within-bucket gap then recovery remains a barrier', () => {const xs = numbered([row(0n), row(500000000n, null), row(500000000n), row(1100000000n)]); assert.equal(produce(xs).at(-1).features.published_mid2_delta_1s, null);});
check('physical same-time truncation cannot seal partial bucket', () => {const prefix = produce(rows.slice(0,2)); assert.equal(prefix.length,1); assert.deepEqual(prefix,produce(rows).slice(0,1));});
check('public cutoff rejects beyond watermark and origin', () => {const x=bookInput(rows); assert.throws(()=>featuresAt(x,B+1300000001n),/watermark/); assert.throws(()=>featuresAt(x,B),/watermark/);});
check('labels, eligibility and native chronology cannot enter features', () => {const xs = structuredClone(rows); xs.forEach(x=>{x.label='up';x.ready_at_decision=false;x.resolved_at_ns='9999';x.native_sequence='99999999999999999999';x.native_event_at='2099-01-01T00:00:00Z';}); assert.deepEqual(produce(xs),produce(rows));});
check('future perturbation preserves earlier features', () => {const xs=structuredClone(rows);xs.at(-1).quote=qBig;assert.deepEqual(produce(xs),produce(rows));});
check('malformed scale, quote, chronology and lines rejected', () => {for(const change of [r=>r.quote.scale=1,r=>r.quote.bid_quantity='0',r=>r.quote.ask_price='99',r=>r.line=2]){const xs=numbered([row(0n),row(100000000n)]);change(xs[0]);assert.throws(()=>produce(xs));}assert.throws(()=>produce(numbered([row(1n),row(0n)])),/backwards/);});
check('64 deterministic mixed histories vs independent reference',()=>{let seed=44;const rand=()=>seed=(Math.imul(seed,1664525)+1013904223)>>>0;for(let k=0;k<64;k++){let ns=0n;const xs=[];for(let j=0;j<80;j++){ns+=BigInt(rand()%4)*10000000n;xs.push(row(ns,rand()%13===0?null:quote(String(100+rand()%5),'120'),rand()%19===0?'catchup_batch':'single_event',rand()%3===0?String(j+1):null));}assert.deepEqual(produce(numbered(xs)),reference(numbered(xs)));}});
check('empty and single-time history produce no rows',()=>{assert.deepEqual(produce([]),[]);assert.deepEqual(produce(numbered([row(0n)])),[]);});
const root = process.argv[2]; let real = null;
if (root) {
  const view=fs.readFileSync(root+'/causal-view-v1/rows.jsonl'), nativeBytes=fs.readFileSync(root+'/native-quote-v1/baseline/rows.jsonl.gz'), rawBytes=fs.readFileSync(root+'/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson');
  assert.equal(sha(view),SOURCE_SHA);assert.equal(sha(nativeBytes),NATIVE_SHA);assert.equal(sha(rawBytes),RAW_SHA);
  const rs=view.toString().trimEnd().split('\n').map(JSON.parse), nativeRows=zlib.gunzipSync(nativeBytes).toString().trimEnd().split('\n').map(JSON.parse);
  const input=bookInput(rs), native=nativeInput(nativeRows,input), raw=rawInput(rawBytes,input), out=gridFeatures(input);
  check('all real 572 rows match independent reference',()=>{assert.equal(out.length,572);assert.deepEqual(out,reference(rs));});
  check('A is preserved exactly at every decision including unready',()=>{const a=tradeGrid(tradeInput(rs));for(let i=0;i<out.length;i++){assert.equal(a[i].decision_ns,out[i].decision_ns);for(const [k,v] of Object.entries(a[i].features))assert.equal(out[i].features[k],v);}assert.equal(out.filter(x=>x.features.bid_price===null&&x.features.last_trade_price!==null).length,2);});
  // Saved labels are read ONLY here for identity and never passed to a feature API.
  const labelBytes=fs.readFileSync(root+'/label-grid-v2/labels.jsonl'), labels=labelBytes.toString().trimEnd().split('\n').map(JSON.parse);
  check('saved A and Stage35 label identities agree, no label values consumed',()=>{const a=fs.readFileSync(root+'/trade-grid-a-v1/features.jsonl','utf8').trimEnd().split('\n').map(JSON.parse);const ids=xs=>xs.map(x=>[x.product_id,x.decision_ns]);assert.deepEqual(ids(out),ids(a));assert.deepEqual(ids(out),ids(labels));});
  check('complete history cutoff vs independent row and application filters for all 572',()=>{for(const d of out){const t=BigInt(d.decision_ns),h=historyAt(input,native,raw,t); const expectedRows=rs.filter(r=>clock(r.available_at_capture)<t);assert.equal(h.receipts.length,expectedRows.length);assert.equal(h.publications.length,expectedRows.length);let count=0;for(const r of nativeRows)if(clock(r.available_at_capture)<t)count+=r.applications.length;assert.equal(h.applications.length,count);assert(h.applications.every(a=>BigInt(a.available_capture_ns)<t));assert.equal(h.receipts.at(-1).byte_end,Buffer.byteLength(rawBytes.toString().split('\n').slice(0,expectedRows.length).join('\n')+'\n'));}});
  check('all original native applications retained without F1/F2',()=>{assert.equal(native.length,20572);assert.equal(native.filter(a=>a.type==='applied').length,20571);assert.equal(native.filter(a=>a.captured_line<a.available_line).length,43);assert(!/model_event|model_state|confirmed|active|geometry/.test(JSON.stringify(native)));});
  check('pending receipt known before application; delayed application never backdated',()=>{const early=historyAt(input,native,raw,BigInt(out[2].decision_ns));assert(early.receipts.length>0);assert.equal(early.applications.length,0);const later=historyAt(input,native,raw,BigInt(out[3].decision_ns));assert(later.applications.some(a=>a.captured_line<a.available_line));});
  check('future native and raw suffix mutation leaves earlier full-history cutoff unchanged',()=>{const t=BigInt(out[250].decision_ns),ns=structuredClone(native),rr=structuredClone(raw);for(const a of ns)if(BigInt(a.available_capture_ns)>=t)a.native_sequence='99999999999999999999';for(const r of rr)if(BigInt(r.capture_ns)>=t)r.raw_message='future changed';assert.deepEqual(historyAt(input,ns,rr,t),historyAt(input,native,raw,t));});
  check('real physical prefixes preserve decisions and history, including bucket cuts',()=>{for(const n of [50,339,340,1000,10000,rs.length-1]){const p=bookInput(rs.slice(0,n));const ps=gridFeatures(p);assert.deepEqual(ps,out.slice(0,ps.length));const pn=nativeInput(nativeRows.slice(0,n),p),pr=raw.slice(0,n);for(const d of ps.slice(-2))assert.deepEqual(historyAt(p,pn,pr,BigInt(d.decision_ns)),historyAt(input,native,raw,BigInt(d.decision_ns)));}});
  check('real future quote/trade perturbation preserves 251 decisions',()=>{const cut=BigInt(out[250].decision_ns),xs=structuredClone(rs);for(const r of xs)if(clock(r.available_at_capture)>=cut){if(r.quote)r.quote=qBig;if(r.new_trade){r.new_trade.price='99999999999999999999';r.new_trade.quantity='8888888888888888888';}}assert.deepEqual(produce(xs).slice(0,251),out.slice(0,251));});
  if(process.argv[3]){
    const dir=process.argv[3],report=JSON.parse(fs.readFileSync(dir+'/report.json'));
    check('saved exports, hashes and <=16MiB cap',()=>{for(const [p,s]of Object.entries(report.output_sha256))assert.equal(sha(fs.readFileSync(dir+'/'+p)),s);assert.deepEqual(fs.readFileSync(dir+'/features.jsonl','utf8').trimEnd().split('\n').map(JSON.parse),out);assert(fs.readdirSync(dir).reduce((n,p)=>n+fs.statSync(dir+'/'+p).size,0)<16*1024*1024);});
    check('existing output is never overwritten',()=>assert.throws(()=>run('','','',dir),/already exists/));
  }
  real={decisions:out.length,applications:native.length,label_identity_sha256:sha(labelBytes),source_sha256:SOURCE_SHA};
}
console.log(JSON.stringify({passed:checks.length,checks,real,test_code_sha256:sha(fs.readFileSync(new URL(import.meta.url)))},null,2));
