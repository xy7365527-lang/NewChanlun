import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {bookInput,nativeInput,rawInput,SOURCE_SHA,NATIVE_SHA,RAW_SHA} from '../../book_grid_features.mjs';
import {labelRows} from '../../order_label_probe.mjs';
import {packetAt,partitionLabels} from './consumer-boundary.mjs';
const sha=x=>crypto.createHash('sha256').update(x).digest('hex');
const decode=b=>b.toString().trimEnd().split('\n').map(JSON.parse);
const [root,out]=process.argv.slice(2);
assert.ok(root&&out&&process.argv.length===4,'usage: node verify-consumer.mjs EVIDENCE_ROOT NEW_OUTPUT');
assert.ok(!fs.existsSync(out),'output exists');
const inputs={
  view:['causal-view-v1/rows.jsonl',SOURCE_SHA],native:['native-quote-v1/baseline/rows.jsonl.gz',NATIVE_SHA],
  raw:['data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson',RAW_SHA],
  features:['book-grid-b-v1/features.jsonl','be957271f54b1b912e537bd6a2802ab0646d6e77b13d623ed070dd0ad3659adc'],
  labels:['label-grid-v2/labels.jsonl','b945b5c6e0148b67ee6b1dda958b9200b91e16db4fff3c25f55622f6e83805fa']};
const bytes={},identity={};
for(const [k,[rel,expected]] of Object.entries(inputs)){const p=path.resolve(root,rel),b=fs.readFileSync(p);assert.equal(sha(b),expected);bytes[k]=b;identity[k]={path:p,sha256:expected,bytes:b.length};}
const rows=decode(bytes.view),nr=decode(zlib.gunzipSync(bytes.native)), input=bookInput(rows),native=nativeInput(nr,input),raw=rawInput(bytes.raw,input);
const features=decode(bytes.features),labels=decode(bytes.labels);
assert.equal(features.length,572);assert.equal(labels.length,572);
fs.mkdirSync(out,{recursive:true,mode:0o700});
const save=(name,x)=>fs.writeFileSync(path.join(out,name),typeof x==='string'||Buffer.isBuffer(x)?x:JSON.stringify(x,null,2)+'\n',{flag:'wx',mode:0o600});
const checks=[],check=(name,fn)=>{fn();checks.push(name);};
const packetHashes=[],selected=new Map();
check('572 feature/label decision identities align without selecting by outcome',()=>{for(let i=0;i<572;i++){assert.equal(features[i].decision_ns,labels[i].decision_ns);assert.equal(features[i].product_id,labels[i].product_id);}});
check('7 named packets retain saved B/A fields; prefixes clock-and-sequence bounded',()=>{
  for(const i of [0,1,2,3,250,570,571]){
    const f=features[i],t=BigInt(f.decision_ns),s=packetAt(input,native,raw,t),p=JSON.parse(s);
    assert.equal(p.product_id,labels[i].product_id);assert.equal(p.decision_ns,labels[i].decision_ns);assert.deepEqual(p.features,f.features);
    const eligibleP=input.publications.filter(x=>BigInt(x.capture_ns)<t),eligibleA=native.filter(x=>BigInt(x.available_capture_ns)<t);
    assert.equal(p.as_of.inclusive_event_seq,eligibleP.length);
    assert.deepEqual(p.history.publications.map(x=>x.event_seq),eligibleP.map(x=>x.source_line));
    assert.deepEqual(p.history.receipts.map(x=>x.event_seq),eligibleP.map(x=>x.source_line));
    assert.deepEqual(p.history.applications.map(x=>[x.available_event_seq,x.application_index]),eligibleA.map(x=>[x.available_line,x.application_index]));
    assert.deepEqual(p.history.trades,input.trades.filter(x=>x.capture<t).map(x=>({capture_ns:String(x.capture),price:String(x.price),quantity:String(x.quantity)})));
    for(const a of p.history.applications){assert.ok(BigInt(a.available_capture_ns)<t);assert.ok(a.available_event_seq<=p.as_of.inclusive_event_seq);if(a.captured_event_seq)assert.ok(a.captured_event_seq<=a.available_event_seq);}
    for(const k of ['receipts','publications'])for(const r of p.history[k])assert.ok(BigInt(r.capture_ns)<t&&r.event_seq<=p.as_of.inclusive_event_seq);
    assert.deepEqual(Object.keys(p),['profile','product_id','decision_ns','constants','as_of','initialization','features','history']);
    assert.deepEqual(Object.keys(p.initialization),['history_origin_ns','rule','market_completeness']);
    assert.deepEqual(Object.keys(p.as_of),['exclusive_capture_ns','inclusive_event_seq']);
    packetHashes.push({product_id:p.product_id,decision_ns:p.decision_ns,sha256:sha(s),bytes:Buffer.byteLength(s),prefix_events:p.as_of.inclusive_event_seq,applications:p.history.applications.length});
    selected.set(i,s);
  }
});
check('initial state contains no future snapshot or anchor; received trades survive missing quote',()=>{
  const p=JSON.parse(selected.get(0));assert.equal(p.features.bid_price,null);assert.equal(p.history.applications.length,0);assert.ok(p.history.receipts.length>0);assert.equal(p.features.last_trade_price,null);
  assert.ok(JSON.parse(selected.get(1)).features.last_trade_price!==null);
  assert.ok(!p.history.receipts.some(x=>JSON.parse(x.raw_message).type==='snapshot'));
});
check('43 late applications are absent before available time even if original receipt is present',()=>{
  const late=native.filter(x=>x.type==='applied'&&x.captured_line<x.available_line);assert.equal(late.length,43);
  for(const a of late){const t=BigInt(a.available_capture_ns);if(t<=input.origin)continue;const p=JSON.parse(packetAt(input,native,raw,t));
    assert.ok(!p.history.applications.some(x=>x.available_event_seq===a.available_line&&x.application_index===a.application_index));
    if(BigInt(input.publications[a.captured_line-1].capture_ns)<t)assert.ok(p.history.receipts.some(x=>x.event_seq===a.captured_line));}
});
const suffix=[];
check('three valid finite-prefix alternatives have different nonempty future suffixes and identical model bytes',()=>{
  for(const i of [0,250,570]){
    const t=BigInt(features[i].decision_ns),firstLater=rows.findIndex(r=>BigInt(input.publications[r.line-1].capture_ns)>=t);
    // Retain >1 future receipt after t. This is a literal finite capture prefix,
    // not edited prices/sequences; it cannot invalidate past protocol state.
    const n=Math.min(rows.length-1,firstLater+3);assert.ok(n>firstLater&&n<rows.length);
    const pi=bookInput(rows.slice(0,n)),pn=nativeInput(nr.slice(0,n),pi),pr=rawInput(bytes.raw.subarray(0,raw[n-1].byte_end),pi);
    const a=packetAt(input,native,raw,t),b=packetAt(pi,pn,pr,t);assert.equal(a,b);
    assert.notEqual(input.watermark,pi.watermark);assert.notEqual(input.publications.length,pi.publications.length);
    const shortLabel=labelRows(rows.slice(0,n)).decisions.find(x=>x.decision_ns===String(t));
    suffix.push({decision_ns:String(t),common_prefix_event_end:JSON.parse(a).as_of.inclusive_event_seq,short_source_rows:n,long_source_rows:rows.length,short_watermark_ns:String(pi.watermark),long_watermark_ns:String(input.watermark),packet_sha256:sha(a),short_source_sha256:sha(bytes.raw.subarray(0,raw[n-1].byte_end)),short_label:shortLabel,long_label:labels[i]});
  }
  assert.ok(suffix.some(x=>x.short_label.label!==x.long_label.label));
});
check('detached JSON has no aliases into future store or later packet',()=>{const s=selected.get(250),p=JSON.parse(s);p.history.publications[0].status='tampered';p.features.bid_price='0';assert.equal(packetAt(input,native,raw,BigInt(p.decision_ns)),s);});
check('host-only future metadata and label-like fields are not copied by projection',()=>{
  const t=BigInt(features[0].decision_ns),fake={...input,total_rows:999999,source_sha:'future_hash',label:'up',resolved_at_ns:'99999999999999999999'};
  assert.equal(packetAt(fake,native,raw,t),selected.get(0));
});
check('beyond-watermark and initialization-origin requests fail',()=>{assert.throws(()=>packetAt(input,native,raw,input.watermark+1n),/watermark/);assert.throws(()=>packetAt(input,native,raw,input.origin),/watermark/);});
const parts=[{name:'early',start_ns:'1609459200000000000',end_ns:'1609459220000000000'},{name:'middle',start_ns:'1609459220000000000',end_ns:'1609459240000000000'},{name:'late',start_ns:'1609459240000000000',end_ns:'1609459260000000000'}];
// Engineering split probe only. This explored minute is never a held-out set.
const evalRows=partitionLabels(labels,input.publications,parts);
// Deliberately adversarial engineering boundary inside the first shared-label
// run already documented by Stage35; not a fitted or confirmatory split.
const stressParts=[{name:'early',start_ns:parts[0].start_ns,end_ns:'1609459203500000000'},{name:'middle',start_ns:'1609459203500000000',end_ns:parts[1].end_ns},parts[2]];
const stressRows=partitionLabels(labels,input.publications,stressParts);
check('all labels retained evaluation-side; cross-boundary knowledge excluded; no future-event group split',()=>{
  assert.equal(evalRows.length,572);const groups=new Map();
  for(const e of evalRows){if(e.loss_eligible){const part=parts.find(x=>x.name===e.partition);assert.ok(BigInt(e.label_known_ns)<BigInt(part.end_ns));if(e.event_group){if(groups.has(e.event_group))assert.equal(groups.get(e.event_group),e.partition);groups.set(e.event_group,e.partition);}}}
  assert.ok(stressRows.some(x=>x.exclusion==='cross_boundary_or_unsealed'));
});
check('naive alternating random-row assignment splits shared future-event labels',()=>{
  const groups=new Map();labels.forEach((l,i)=>{if(l.disposition==='observed_change'){const k=l.witness_line;groups.set(k,new Set([...(groups.get(k)??[]),i%2]));}});
  assert.ok([...groups.values()].some(x=>x.size>1));
});
// Valid causal-view contract, not an asserted reconstructed exchange stream.
const quote=(b)=>({scale:100000000,bid_price:String(b),ask_price:String(b+2),bid_quantity:'10',ask_quantity:'20'});
const synthetic=(price)=>[0,100000000,200000000,300000000].map((n,i)=>({schema:'coinbase-causal-view/1',line:i+1,product_id:'BTC-USD',available_at_capture:`2021-01-01T00:00:00.${String(n).padStart(9,'0')}Z`,status:'ready',transition:'single_event',quote:quote(i<2?100:price),new_trade:null}));
const syntheticView=synthetic(110);
let sealCounterexample;
check('resolved timestamp before split is insufficient: sealing capture equal to boundary is purged',()=>{
  const r=syntheticView,bs=bookInput(r),l=labelRows(r).decisions.find(x=>x.decision_ns==='1609459200200000000');
  const split=[{name:'train_probe',start_ns:'1609459200000000000',end_ns:'1609459200300000000'},{name:'next_probe',start_ns:'1609459200300000000',end_ns:'1609459200400000000'}];
  assert.ok(BigInt(l.resolved_at_ns)<BigInt(split[0].end_ns));
  const e=partitionLabels([l],bs.publications,split)[0];assert.equal(e.label_known_ns,split[0].end_ns);assert.equal(e.loss_eligible,false);
  sealCounterexample={view_rows:r,label:l,split,naive_resolved_before_end:true,corrected:e};
});
check('none requires strictly later than deadline and unknown cannot become loss',()=>{
  const base=labels.find(x=>x.label==='none'),e=evalRows.find(x=>x.decision_ns===base.decision_ns);assert.ok(BigInt(e.label_known_ns)>BigInt(base.deadline_ns));
  const short=syntheticView.slice(0,3),shortLabel=labelRows(short).decisions.find(x=>x.decision_ns==='1609459200200000000');
  assert.equal(shortLabel.disposition,'censored_eof');
  const noSeal=partitionLabels([shortLabel],bookInput(short).publications,[{name:'probe',start_ns:'1609459200000000000',end_ns:'1609459200400000000'}])[0];assert.equal(noSeal.loss_eligible,false);assert.equal(noSeal.label_known_ns,null);
  assert.equal(evalRows.filter(x=>x.disposition==='ineligible_at_decision').length,3);
});
const child=[];
check('actual consumer receives one stdin packet, no source/label arguments',()=>{
  for(const [i,s] of selected){const p=spawnSync(process.execPath,[fileURLToPath(new URL('./consume-stdin.mjs',import.meta.url))],{input:s,encoding:'utf8',timeout:30000,maxBuffer:65536,env:{PATH:process.env.PATH}});assert.equal(p.status,0,p.stderr);const rec=JSON.parse(p.stdout);assert.equal(rec.packet_sha256,sha(s));child.push({decision_index:i,status:p.status,stdout:rec,stderr:p.stderr});if([0,3,250].includes(i))save(`packet-${i}.json.gz`,zlib.gzipSync(s));}
});
save('packet-identities.json',packetHashes);save('evaluation-only.json',evalRows);save('evaluation-stress-only.json',{partitions:stressParts,rows:stressRows});save('suffix-comparisons.json',suffix);save('seal-counterexample.json',sealCounterexample);save('consumer-processes.json',child);
const totals=Object.fromEntries(parts.map(p=>[p.name,{all:evalRows.filter(e=>e.partition===p.name).length,loss_eligible:evalRows.filter(e=>e.partition===p.name&&e.loss_eligible).length,purged:evalRows.filter(e=>e.partition===p.name&&e.exclusion==='cross_boundary_or_unsealed').length}]));
const files=['consumer-boundary.mjs','consume-stdin.mjs','verify-consumer.mjs'].map(x=>fileURLToPath(new URL(x,import.meta.url)));
const pbase=fileURLToPath(new URL('../../',import.meta.url));
for(const rel of ['book_grid_features.mjs','trade_grid_features.mjs','order_label_probe.mjs','GoalReframe-v4.md','Stage35ExternalTarget.md','Stage44BaseCompletionAndB.md','BookGridB-v1.md','ExperimentDesignCandidate-v1.md','stage44/independent-book-grid-b-review.md'])files.push(path.join(pbase,rel));
const fileIds=files.map(p=>({path:p,sha256:sha(fs.readFileSync(p))}));
const outputs=fs.readdirSync(out).sort().map(name=>({name,sha256:sha(fs.readFileSync(path.join(out,name))),bytes:fs.statSync(path.join(out,name)).size}));
const receipt={profile:'stage56-causal-consumer-check/1',passed:checks.length,checks,input_identity:identity,author_packet_files:fileIds,outputs,decisions:572,partitions:parts,partition_counts:totals,late_applications_checked:43,
  stress_partition_counts:Object.fromEntries(stressParts.map(p=>[p.name,{all:stressRows.filter(e=>e.partition===p.name).length,loss_eligible:stressRows.filter(e=>e.partition===p.name&&e.loss_eligible).length,purged:stressRows.filter(e=>e.partition===p.name&&e.exclusion==='cross_boundary_or_unsealed').length}])),
  scope:'offline named serialized API contract only; not OS isolation, strong-B fairness, training, candidate qualification or confirmation',independent_review:'pending',reproduce:`node ${fileURLToPath(import.meta.url)} ${path.resolve(root)} NEW_OUTPUT_DIRECTORY`};
save('receipt.json',receipt);console.log(JSON.stringify(receipt,null,2));
