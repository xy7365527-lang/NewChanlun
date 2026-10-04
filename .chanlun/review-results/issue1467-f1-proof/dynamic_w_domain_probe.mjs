#!/usr/bin/env node
// #1467/#1468 DynamicWDomain-v1: 独立具名观察轴；不接生产，不续接旧失败。
import fs from 'node:fs';
import crypto from 'node:crypto';
import zlib from 'node:zlib';
import assert from 'node:assert/strict';
import path from 'node:path';
import vm from 'node:vm';
import {fileURLToPath} from 'node:url';
const self=fileURLToPath(import.meta.url), source=path.dirname(self);
const evidence='/Users/silencehan/Documents/Codex/research-evidence/issue1467';
const baseOut=evidence+'/dynamic-w-domain-v1';
assert.equal(process.argv[2],'--out');assert.equal(process.argv.length,4);
const out=path.resolve(process.argv[3]);
assert(out===baseOut||out.startsWith(baseOut+'-'));assert.equal(path.dirname(out),evidence);
assert(!fs.existsSync(out),'输出须为尚不存在的独立目录');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const clean=x=>JSON.parse(JSON.stringify(x));
const bindings={};
function frozen(name,expected){const p=source+'/'+name,b=fs.readFileSync(p);assert.equal(sha(b),expected);bindings[name]={path:p,sha256:expected};return b.toString();}
const relationSource=frozen('plateau_base_probe.mjs','0a630a6bbc418a0fa57be621ea2089123005a5b3dcd712622c81738f2a3515ac');
frozen('PlateauBaseCandidate-v2.md','ff4e1d062e78891ff2c563ae06ba3ac842ac10885a5d275795fcd852624daa43');
// 只求值固定 SHA 内纯关系前段；不执行原 main，不暴露固定参数 W。
const pure=relationSource.slice(relationSource.indexOf('const gcd ='),relationSource.indexOf('const add='));
const rel=vm.runInNewContext(pure+'\n({R,observePrefix,verifyResult});',{assert,createHash:crypto.createHash});
const adapterSource=frozen('stage42/plateau_data_domain_probe.mjs','8deeb0bbfb6f4110c610542fdf4e9c4bcb632168de937cb8187186f76d6a0a95');
// 仅复用原始快照→43补应用→B0 的冻结重建和受检查 apply；不执行旧 profiles 或输出。
const bootstrap=adapterSource.slice(0,adapterSource.indexOf('const initial=quote()'))
 .replace(/^import .*;\n/gm,'').replace('const out=path.dirname(fileURLToPath(import.meta.url));','');
const ad=vm.runInNewContext(bootstrap+'\n({raw,native,init,quote,apply,seq,frontier,initApplications,initMutations,files,bytes,nativeBytes});',
 {fs,crypto,zlib,assert,path,fileURLToPath});
const document=source+'/DynamicWDomain-v1.md';
const initial=clean(ad.quote());
const scale=100000000n;
function dynamicW(q){
 assert(q&&q.scale===Number(scale),'missing_or_wrong_scale_quote');
 const [b,a,qb,qa]=['bid_price','ask_price','bid_quantity','ask_quantity'].map(k=>{assert(/^\d+$/.test(q[k]),'missing_quote_field');return BigInt(q[k]);});
 assert(b>0n&&a>b&&qb>0n&&qa>0n,'invalid_two_sided_book');
 const w=rel.R((a*qb+b*qa)+'/'+((qb+qa)*scale));
 assert(w.cmp(rel.R(b+'/'+scale))>0&&w.cmp(rel.R(a+'/'+scale))<0,'W_not_between_bid_and_ask');
 return w;
}
function instant(s){assert.equal(typeof s,'string');const m=s.match(/^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?Z$/);assert(m,'missing_or_invalid_clock');return BigInt(Date.parse(m[1]+'Z'))*1000000n+BigInt((m[2]??'').padEnd(9,'0'));}
// 独立朴素方法：每个下标直接左右扫描最大等值区间，再从成熟值重建所有变号边块。
// 不调用原 platforms/validRelation；核初态、首次短封口、核心、完成区间、种子、方向、残余。
function naive(xs){
 const n=xs.length-1,eq=(a,b)=>xs[a].cmp(xs[b])===0;
 let failure=n&&!eq(0,1)?{at:1,reason:'initial_observation_differs'}:null;
 if(!failure)for(let i=1;i<=n;i++)if(i===1||!eq(i-1,i)){
  let j=i;while(j<n&&eq(i,j+1))j++;
  if(j<n&&j-i+1<3){failure={at:j+1,reason:'closed_short_platform'};break;}
 }
 const through=failure?failure.at-1:n,ps=[];
 for(let i=1;i<=through;i++)if(i===1||!eq(i-1,i)){
  let end=i;while(end<through&&eq(i,end+1))end++;
  ps.push({start:i,end,count:end-i+1,value:String(xs[i]),closed:end<through});
 }
 const cores=ps.filter(p=>p.count>=3),directions=cores.slice(1).map((p,i)=>rel.R(p.value).cmp(cores[i].value));
 const blocks=[];
 for(let i=0;i<directions.length;i++)if(i===0||directions[i]!==directions[i-1]){
  let end=i;while(end+1<directions.length&&directions[end+1]===directions[i])end++;
  blocks.push({a:i+1,b:end+1,d:directions[i]});
 }
 const completed=blocks.slice(0,-1).map((h,i)=>({after:i===0?0:cores[h.a-1].end,end:cores[h.b].end,direction:h.d,
  ids:Array.from({length:h.b-(h.a===1?0:h.a)+1},(_,j)=>(h.a===1?0:h.a)+j),
  knownAt:cores[h.b+1].start+2,seed:[0,1,2].map(j=>cores[h.b+1].start+j)}));
 return {n,through,failure,ps,cores,directions,completed,after:completed.at(-1)?.end??0};
}
const xs=[dynamicW(initial)],observations=[],continuity=[],prefixes=[];
let previous=null,checks=0;
function checkPrefix(){
 const r=rel.observePrefix(xs);rel.verifyResult(xs,r);const c=clean(r),ind=naive(xs);
 assert.equal(c.analysedThrough,ind.through);assert.equal(c.failure?.at??null,ind.failure?.at??null);
 assert.equal(c.failure?.reason??null,ind.failure?.reason??null);
 assert.deepEqual(c.coreCatalog.map(p=>[p.start,p.endObserved,p.count,p.value,p.closed]),ind.cores.map(p=>[p.start,p.end,p.count,p.value,p.closed]));
 assert.deepEqual(c.completed.map(m=>({after:m.raw.after,end:m.raw.end,direction:m.direction,ids:m.centreIds,knownAt:m.completion.knownAt,seed:m.completion.seedEvents})),ind.completed);
 assert.equal(c.unconsumed.after,ind.after);
 assert.deepEqual(c.unconsumed.events,Array.from({length:ind.n-ind.after},(_,i)=>ind.after+i+1));
 assert.equal(c.pendingPlatform?.count??null,ind.ps.at(-1)?.count<3?ind.ps.at(-1).count:null);
 if(previous)assert.deepEqual(c.completed.slice(0,previous.completed.length),previous.completed);
 c.clockedCompletion=c.completed.map(m=>({id:m.id,model_confirmed_at:m.completion.knownAt,
  native_event_at:observations[m.completion.knownAt-1].native_event_at,
  available_capture:observations[m.completion.knownAt-1].available_capture,
  seed:m.completion.seedEvents.map(e=>({event:e,sequence:observations[e-1].sequence,captured_line:observations[e-1].captured_line}))}));
 prefixes.push(c);previous=c;checks++;return c;
}
checkPrefix();let frontier=BigInt(ad.frontier),stop=null,postApplications=0,postMutations=0,quotePairs=ad.initMutations;
let lastAvailable=instant(ad.init.available_at_capture),lastLine=ad.init.line;
// 这里的连续账包含所有 post-B0 应用，包括 noop；它不是模型种子计数。
outer:for(const row of ad.native.filter(r=>r.line>ad.init.line)){
 if(row.line!==lastLine+1){stop={kind:'observation_domain',reason:'capture_row_gap',line:row.line};break;}
 if(row.status!=='ready'||row.pending!==0){stop={kind:'observation_domain',reason:'not_ready_or_pending',line:row.line};break;}
 if(!row.current_quote){stop={kind:'observation_domain',reason:'missing_current_quote',line:row.line};break;}
 if(instant(row.available_at_capture)<lastAvailable){stop={kind:'observation_domain',reason:'capture_clock_regression',line:row.line};break;}
 const pendingPrefixStart=prefixes.length;let platformFailure=null;
 for(const a of row.applications){
  if(a.type!=='applied'){stop={kind:'observation_domain',reason:'unsupported_application',line:row.line};break outer;}
  const x=ad.raw[a.captured_line-1];
  if(!x||ad.seq(x.m)!==frontier+1n){stop={kind:'observation_domain',reason:'source_sequence_gap',line:row.line};break outer;}
  assert.equal(String(ad.seq(x.m)),a.native_sequence);assert.equal(x.capture,a.captured_at);
  assert.equal(a.available_line,row.line);assert.equal(a.available_capture,row.available_at_capture);
  assert(a.captured_line<=row.line);assert(instant(a.captured_at)<=instant(a.available_capture));
  assert.equal(x.m.time,a.native_event_at);instant(a.native_event_at);
  if(a.disposition==='mutation'&&(!a.before_quote||!a.after_quote)){stop={kind:'observation_domain',reason:'missing_mutation_quote',line:row.line};break outer;}
  const before=clean(ad.quote());if(a.before_quote)assert.deepEqual(before,clean(a.before_quote));
  let normalization,current;
  try{normalization=ad.apply(x.m);current=clean(ad.quote());dynamicW(current);}
  catch{stop={kind:'observation_domain',reason:'adapter_or_two_sided_book_failure',line:row.line,sequence:a.native_sequence};break outer;}
  assert.equal(normalization,a.normalization);frontier++;postApplications++;
  if(a.after_quote){assert.deepEqual(current,clean(a.after_quote));quotePairs++;}
  const meta={application:postApplications,sequence:a.native_sequence,captured_line:a.captured_line,available_line:a.available_line,
   native_type:a.native_type,normalization,native_event_at:a.native_event_at,captured_at:a.captured_at,available_capture:a.available_capture,disposition:a.disposition};
  continuity.push(meta);
  if(a.disposition==='noop'){assert.deepEqual(before,current);continue;}
  assert.equal(a.disposition,'mutation');postMutations++;
  // 若同一 available 批先失败，余项只保留原始连续性，不重新分析或发布中间状态。
  if(platformFailure)continue;
  const w=dynamicW(current);xs.push(w);observations.push({...meta,event:observations.length+1,quote:current,W:String(w)});
  const r=checkPrefix();
  if(r.status==='outside_D')platformFailure={kind:'platform_domain',event:observations.length,line:row.line,sequence:a.native_sequence,
   reason:r.failure.reason,native_event_at:a.native_event_at,available_capture:a.available_capture};
 }
 assert.deepEqual(clean(ad.quote()),clean(row.current_quote));assert.equal(String(frontier),row.frontier);
 // 批内轨迹统一到完成整批后的 capture 知识时刻，不产生 native-time 发布流。
 for(let i=pendingPrefixStart;i<prefixes.length;i++)prefixes[i].publication={available_line:row.line,available_capture:row.available_at_capture,batch_committed:true};
 lastAvailable=instant(row.available_at_capture);lastLine=row.line;
 if(platformFailure){stop=platformFailure;break;}
}
if(!stop)stop={kind:'end_of_file',line:lastLine};
const fixedBytes=fs.readFileSync(evidence+'/f2-interface-v1/plateau-data-domain-summary.json');
assert.equal(sha(fixedBytes),'32e59e88edcbe18c8b90b5dcccf6de50a86177474f111cb1f8cadad61169ddc9');
const final=prefixes.at(-1),fixed=JSON.parse(fixedBytes);
const oldW=fixed.profiles.fixedW;assert.equal(oldW.exit.n,12);
const at12=prefixes[12];
// 固定哈希真实样本的回归断言：不据此选择参数或继续找窗口。
assert.equal(stop.kind,'platform_domain');assert.equal(stop.event,55);assert.equal(stop.line,422);
assert.equal(final.analysedThrough,54);assert.equal(final.completed.length,2);assert.equal(final.coreCatalog.length,6);
assert.equal(at12.status,'valid_D');assert.equal(observations[11].normalization,'resting_match');
assert.equal(xs[0].cmp(xs[1]),0);assert.notEqual(String(xs[11]),String(xs[12]));
for(let n=0;n<prefixes.length;n++)assert.equal(prefixes[n].completed.length,[20,45].filter(t=>t<=n).length);
for(const n of [18,19])assert.equal(prefixes[n].completed.length,0);
for(const n of [43,44])assert.equal(prefixes[n].completed.length,1);
assert.deepEqual(final.completed.map(m=>[m.raw.after,m.raw.end,m.completion.knownAt]),[[0,17,20],[17,42,45]]);
assert.deepEqual([final.pendingPlatform.start,final.pendingPlatform.count],[53,2]);
assert.deepEqual(final.unconsumed.events,Array.from({length:13},(_,i)=>43+i));
assert.deepEqual(final.unanalysedEvents,[55]);
const initContinuity=ad.init.applications.filter(a=>a.type==='applied').map(a=>({sequence:a.native_sequence,captured_line:a.captured_line,
 native_event_at:a.native_event_at,captured_at:a.captured_at,available_line:a.available_line,available_capture:a.available_capture,
 normalization:a.normalization,disposition:a.disposition,role:'B0_initialization_not_observation'}));
const result={profile:'DynamicWDomain-v1',verdict:'finite_first_segment_diagnostic_only',axis:'R_W',formula:'(ask*bidQty+bid*askQty)/(bidQty+askQty)',
 scope:'single first-ready segment; no reset, no window search, no Q-order-isomorphism claim',
 input_hashes:Object.fromEntries(Object.entries(ad.files).map(([k,p])=>[k,{path:p,sha256:sha(ad.bytes[k])}])),native_uncompressed_sha256:sha(ad.nativeBytes),
 source_hashes:bindings,script_sha256:sha(fs.readFileSync(self)),document_sha256:fs.existsSync(document)?sha(fs.readFileSync(document)):null,
 fixed_summary_sha256:sha(fs.readFileSync(evidence+'/f2-interface-v1/plateau-data-domain-summary.json')),
 initialization:{line:ad.init.line,frontier:String(ad.frontier),available_capture:ad.init.available_at_capture,applications_excluded:ad.initApplications,quote:initial,W0:String(xs[0]),x0_equals_x1:xs.length>1?xs[0].cmp(xs[1])===0:null},
 stop,analysedThrough:final.analysedThrough,completed_count:final.completed.length,clocked_completion:final.clockedCompletion,
 mature_cores:final.coreCatalog.length,pending:final.pendingPlatform,unconsumed:final.unconsumed,
 fixedW_comparison:{old_exit:oldW.exit,dynamic_at_e12:at12?{status:at12.status,completed:at12.completed.length,analysedThrough:at12.analysedThrough}:null},
 checks:{prefix_count:checks,independent_naive_prefix_comparisons:checks,raw_records:ad.raw.length,initialization_applications:ad.initApplications,post_init_applications:postApplications,post_init_mutations:postMutations,quote_pairs:quotePairs,last_committed_line:lastLine,continuous_sequence_through:String(frontier),every_legal_state_exact_W_between_quote:true,completion_and_residual_coverage:true,confirmation_not_before_three_mutation_observations:true,model_clock_separate_from_native_and_capture:true},
 retained_unanalysed_capture_suffix:{from:stop.line,to:ad.raw.length,raw_path:ad.files.raw},
 limitations:['historical exploratory minute, not held-out confirmation set','no general exchange protocol or recovery proof','no Q/W partition equivalence','no canonical F2 qualification','no training, market value or profit comparison','unfinished or unknown is not zero benefit']};
fs.mkdirSync(out,{mode:0o700});const hashes=[];
for(const [name,data] of Object.entries({'summary.json':result,'observations.json':observations,'continuity.json':{initialization:initContinuity,post_initialization:continuity},'prefixes.json':prefixes})){
 const b=JSON.stringify(data,null,2)+'\n';fs.writeFileSync(out+'/'+name,b,{mode:0o600});hashes.push(sha(b)+'  '+name);
}
fs.writeFileSync(out+'/output-hashes.sha256',hashes.join('\n')+'\n',{mode:0o600});
console.log(JSON.stringify({passed:true,out,stop,analysedThrough:final.analysedThrough,completed:final.completed.length,cores:final.coreCatalog.length,prefixChecks:checks}));
