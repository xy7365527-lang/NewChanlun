// #1467/#1468: plateau_data_domain agent; read-only finite input probe.
// No raw payloads or order IDs are serialized. Only the existing first causal segment is used.
import fs from 'node:fs';
import crypto from 'node:crypto';
import zlib from 'node:zlib';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root='/Users/silencehan/Documents/Codex/research-evidence/issue1467';
const repo='/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const source=repo+'/.chanlun/review-results/issue1467-f1-proof/';
const out=path.dirname(fileURLToPath(import.meta.url));
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const files={raw:root+'/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson',views:root+'/causal-view-v1/rows.jsonl',native:root+'/native-quote-v1/baseline/rows.jsonl.gz'};
const bytes=Object.fromEntries(Object.entries(files).map(([k,p])=>[k,fs.readFileSync(p)]));
assert.equal(sha(bytes.raw),'8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd');
assert.equal(sha(bytes.views),'a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5');
assert.equal(sha(bytes.native),'5c233367178b64ddbcd992bf869793f4bf12a415ed90d50d1ee1d481d04763cf');
const nativeBytes=zlib.gunzipSync(bytes.native);
assert.equal(sha(nativeBytes),'6a3c7eb8a47c5c2e9cdbbee77200286be6133349f2a5d328d3cec300c0496b30');
const lines=b=>{const a=b.toString().split('\n');assert.equal(a.pop(),'');return a;};
const raw=lines(bytes.raw).map((s,i)=>{if(!s)return null;const k=s.indexOf(' ');return {line:i+1,capture:s.slice(0,k),m:JSON.parse(s.slice(k+1))};});
const native=lines(nativeBytes).map(JSON.parse),views=lines(bytes.views).map(JSON.parse);
const init=native.find(r=>r.status==='ready');assert.equal(init.line,340);
const dec=s=>{assert.equal(typeof s,'string');assert(/^\d+(\.\d{0,8})?$/.test(s));const [a,b='']=s.split('.');return BigInt(a)*100000000n+BigInt(b.padEnd(8,'0'));};
const seq=m=>{assert(Number.isSafeInteger(m.sequence));return BigInt(m.sequence);};
const orders=new Map(),bids=new Map(),asks=new Map();
function delta(buy,p,q){const map=buy?bids:asks,v=(map.get(p)??0n)+q;assert(v>=0n);if(v)map.set(p,v);else map.delete(p);}
function add(id,buy,p,q){assert(!orders.has(id)&&q>0n&&p>0n);orders.set(id,{buy,p,q});delta(buy,p,q);}
function quote(){assert(bids.size&&asks.size);const b=[...bids.keys()].reduce((a,v)=>a>v?a:v),a=[...asks.keys()].reduce((a,v)=>a<v?a:v);assert(b<a);return {bid_price:String(b),bid_quantity:String(bids.get(b)),ask_price:String(a),ask_quantity:String(asks.get(a)),scale:100000000};}
function apply(m){
  const id=m.type==='match'?m.maker_order_id:m.order_id,o=orders.get(id);
  if(m.type==='received')return 'received_no_book_effect';
  if((m.type==='done'||m.type==='change')&&!o)return 'nonresting_'+m.type;
  if(m.type==='open'){add(id,m.side==='buy',dec(m.price),dec(m.remaining_size));return 'resting_open';}
  assert(o);assert.equal(m.side,o.buy?'buy':'sell');assert.equal(dec(m.price),o.p);
  let next,kind;
  if(m.type==='match'){const q=dec(m.size);assert(q>0n&&q<=o.q);next=o.q-q;kind='resting_match';}
  else if(m.type==='done'){assert.equal(m.reason,'canceled');assert.equal(dec(m.remaining_size),o.q);next=0n;kind='resting_canceled_done';}
  else if(m.type==='change'){assert.equal(dec(m.old_size),o.q);next=dec(m.new_size);assert(next<=o.q);kind=next===o.q?'resting_size_unchanged':'resting_size_decrease';}
  else throw Error('unsupported message');
  delta(o.buy,o.p,next-o.q);if(next)orders.set(id,{...o,q:next});else orders.delete(id);return kind;
}
const snapshot=raw[init.line-1];assert.equal(snapshot.m.type,'full_snapshot');assert(snapshot.m.generated);
for(const [field,buy] of [['bids',true],['asks',false]])for(const [p,q,id] of snapshot.m[field])add(id,buy,dec(p),dec(q));
let frontier=seq(snapshot.m),initApplications=0,initMutations=0;
for(const a of init.applications.filter(a=>a.type==='applied')){
  const r=raw[a.captured_line-1];assert.equal(String(seq(r.m)),a.native_sequence);assert.equal(seq(r.m),frontier+1n);
  if(a.before_quote)assert.deepEqual(quote(),a.before_quote);assert.equal(apply(r.m),a.normalization);if(a.after_quote){assert.deepEqual(quote(),a.after_quote);initMutations++;}
  frontier++;initApplications++;
}
assert.equal(initApplications,43);assert.equal(String(frontier),init.frontier);assert.deepEqual(quote(),init.current_quote);assert.deepEqual(views[339].quote,init.current_quote);
const initial=quote(),b0=BigInt(initial.bid_price),literal=100n*100000000n;
const profiles={
  literalQ:{name:'literal-Q-100-add-cancel-prefix/1',q0:String(bids.get(literal)??0n),obs:[],exit:null},
  fixedQ:{name:'causal-init-fixed-b0-add-cancel/1',q0:initial.bid_quantity,obs:[],exit:null},
  fixedW:{name:'causal-init-fixed-b0-a0-K0-add-cancel/1',q0:initial.bid_quantity,obs:[],exit:null},
  extendedQ:{name:'causal-init-fixed-b0-native-removal-diagnostic/1',q0:initial.bid_quantity,obs:[],exit:null},
};
function relation(obs){
 const ps=[];for(const e of obs){let p=ps.at(-1);if(!p||p.q!==e.q){p={q:e.q,start:e.n,end:e.n,events:[]};ps.push(p);}p.end=e.n;p.events.push(e);}
 const mature=ps.filter(p=>p.events.length>=3),edges=[];for(let i=1;i<mature.length;i++)edges.push(BigInt(mature[i].q)>BigInt(mature[i-1].q)?1:-1);
 const blocks=[];edges.forEach((d,j)=>{let h=blocks.at(-1);if(!h||h.d!==d){h={d,a:j+1,b:j+1};blocks.push(h);}h.b=j+1;});
 const completed=blocks.slice(0,-1).map(h=>({start:h.a===1?1:mature[h.a-1].end+1,end:mature[h.b].end,confirmedAt:mature[h.b+1].start+2,direction:h.d}));
 const T=completed.at(-1)?.end??0;
 return {observations:obs.length,mature_cores:mature.length,core_support:mature.map((p,i)=>({id:i,birth:p.start+2,source_seed:p.events.slice(0,3).map(e=>({sequence:e.sequence,capture_line:e.line}))})),mature_edge_directions:edges,completed_count:completed.length,completed,parent_three_candidate_possible:completed.length>=3,platforms:ps.map(p=>({start:p.start,end:p.end,length:p.events.length,mature:p.events.length>=3,sealed:p!==ps.at(-1)})),pending:ps.at(-1)&&ps.at(-1).events.length<3?{start:ps.at(-1).start,length:ps.at(-1).events.length}:null,unconsumed_suffix:obs.length?{from:T+1,to:obs.length}:null};
}
let mutationOrdinal=0,appliedAfter=0,checkedQuotes=initMutations,stopLine=null;const typeCounts={};
const strictKinds=new Set(['resting_open','resting_canceled_done']);
outer:for(const r of native.filter(r=>r.line>init.line)){
 assert.equal(r.status,'ready');assert.equal(r.pending,0);
 for(const a of r.applications){
  assert.equal(a.type,'applied');assert.equal(a.available_line,r.line);assert.equal(a.captured_line,r.line);
  const x=raw[a.captured_line-1];assert.equal(String(seq(x.m)),a.native_sequence);assert.equal(seq(x.m),frontier+1n);assert.equal(x.capture,a.captured_at);assert.equal(a.available_capture,r.available_at_capture);
  if(a.before_quote)assert.deepEqual(quote(),a.before_quote);const normalization=apply(x.m);assert.equal(normalization,a.normalization);frontier++;appliedAfter++;typeCounts[normalization]=(typeCounts[normalization]??0)+1;
  if(a.after_quote){assert.deepEqual(quote(),a.after_quote);checkedQuotes++;}
  if(a.disposition==='noop')continue;
  assert.equal(a.disposition,'mutation');mutationOrdinal++;
  const meta={n:mutationOrdinal,line:r.line,sequence:a.native_sequence,native_type:a.native_type,normalization,native_time:a.native_event_at,available_at:a.available_capture};
  const current=quote(),q=String(bids.get(b0)??0n);
  for(const [key,p] of Object.entries(profiles)){
   if(p.exit)continue;const value=key==='literalQ'?String(bids.get(literal)??0n):q,reasons=[];
   if(key!=='extendedQ'&&!strictKinds.has(normalization))reasons.push('adapter_unsupported_native_type_not_DQ_counterexample');
   if(key==='fixedW'){
    if(current.bid_price!==initial.bid_price)reasons.push('best_bid_changed');if(current.ask_price!==initial.ask_price)reasons.push('best_ask_changed');if(current.ask_quantity!==initial.ask_quantity)reasons.push('actual_K_changed');if(q==='0')reasons.push('q_is_zero');
   }
   if(!p.obs.length&&value!==p.q0)reasons.push('first_observation_differs_from_initial');
   if(p.obs.length&&value!==p.obs.at(-1).q){let len=0;for(let j=p.obs.length-1;j>=0&&p.obs[j].q===p.obs.at(-1).q;j--)len++;if(len<3)reasons.push('closed_short_platform_'+len);}
   if(reasons.length){p.exit={...meta,status:reasons.some(r=>!r.startsWith('adapter_'))?'outside_mathematical_domain':'adapter_coverage_boundary',reasons};p.frozen=relation(p.obs);}else p.obs.push({...meta,q:value});
  }
 }
 assert.deepEqual(quote(),r.current_quote);assert.equal(String(frontier),r.frontier);
 if(Object.values(profiles).every(p=>p.exit)){stopLine=r.line;break outer;}
}
assert.equal(stopLine,422);assert.equal(profiles.fixedW.exit.n,12);assert.equal(profiles.fixedQ.exit.n,12);assert.equal(profiles.extendedQ.exit.n,55);assert(profiles.extendedQ.exit.reasons.includes('closed_short_platform_2'));
assert(Object.values(profiles).every(p=>p.frozen.completed_count<3));
const metadata={};for(const [k,p] of Object.entries(profiles))metadata[k]={profile:p.name,exit:p.exit,frozen:p.frozen,retained_unanalyzed_capture_suffix:{from:p.exit.line,to:raw.length},observations_sha256:sha(JSON.stringify(p.obs))};
const sourceFiles=['PlateauBaseCandidate-v2.md','plateau_base_probe.mjs','Stage16CausalViews.md','Stage26NativeQuote.md','Stage29NativeStateTime.md','native_quote_blocks.rs','stage26_inspect.mjs','stage29_native_time.mjs'];
const result={agent:'plateau_data_domain',scope:'first causal segment only; no reset; no profitability experiment',input_hashes:Object.fromEntries(Object.entries(files).map(([k,p])=>[k,{path:p,sha256:sha(bytes[k])}])),native_uncompressed_sha256:sha(nativeBytes),source_hashes:sourceFiles.map(p=>({path:source+p,sha256:sha(fs.readFileSync(source+p))})),probe_sha256:sha(fs.readFileSync(fileURLToPath(import.meta.url))),initialization:{line:init.line,frontier:init.frontier,capture:init.available_at_capture,parameters_scaled_1e8:initial,literal_Q100_initial_quantity:String(profiles.literalQ.q0),excluded_unknown_capture_lines:339,excluded_snapshot_count:1,excluded_initialization_applications:43},literalW:{status:'outside_D_at_initialization',reason:'actual_best_prices_and_K_are_not_literal_100_102_10'},checks:{raw_records:raw.length,native_rows:native.length,published_rows:views.length,checked_native_applications:initApplications+appliedAfter,checked_mutation_quote_pairs:checkedQuotes,post_init_applied:appliedAfter,post_init_mutations:mutationOrdinal,post_init_types:typeCounts,stopped_at_capture:stopLine,independent_BigInt_order_and_level_reconstruction:true,continuous_native_sequence_verified_to_stop:true},profiles:metadata,limitations:['no exchange protocol acceptance','no general recovery proof','synthetic add/cancel domain does not inherit execution support','one exploratory historical minute is not a confirmation set','raw logs and old outputs preserved','no candidate qualifies as canonical Chan Move or proves market value']};
fs.writeFileSync(out+'/plateau-data-domain-summary.json',JSON.stringify(result,null,2)+'\n',{mode:0o600});
console.log(JSON.stringify({passed:true,stopLine,profiles:Object.fromEntries(Object.entries(metadata).map(([k,v])=>[k,{exit:v.exit.n,reasons:v.exit.reasons,cores:v.frozen.mature_cores,completed:v.frozen.completed_count,pending:v.frozen.pending}]))}));
