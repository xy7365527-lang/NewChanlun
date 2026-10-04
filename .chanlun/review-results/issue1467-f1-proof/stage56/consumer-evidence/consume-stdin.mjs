// Contract probe, not a trained predictor or OS sandbox. stdin contains one
// as-of packet; no source paths/labels/configuration are sent as argv or stdin.
import fs from 'node:fs';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
assert.equal(process.argv.length,2);
const bytes=fs.readFileSync(0), p=JSON.parse(bytes);
assert.equal(p.profile,'asof-consumer-packet/1');
assert.deepEqual(Object.keys(p),['profile','product_id','decision_ns','constants','as_of','initialization','features','history']);
const t=BigInt(p.decision_ns), end=p.as_of.inclusive_event_seq;
for(const k of ['receipts','publications']) for(const r of p.history[k]) {assert.ok(BigInt(r.capture_ns)<t);assert.ok(r.event_seq<=end);}
for(const a of p.history.applications) {assert.ok(BigInt(a.available_capture_ns)<t);assert.ok(a.available_event_seq<=end);}
for(const tr of p.history.trades) assert.ok(BigInt(tr.capture_ns)<t);
console.log(JSON.stringify({profile:p.profile,decision_ns:p.decision_ns,packet_sha256:crypto.createHash('sha256').update(bytes).digest('hex'),
  history_counts:Object.fromEntries(Object.entries(p.history).map(([k,v])=>[k,v.length])),last_receipt_seq:p.history.receipts.at(-1)?.event_seq??null}));
