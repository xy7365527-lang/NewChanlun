import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawn} from 'node:child_process';
import readline from 'node:readline';
import assert from 'node:assert/strict';
import {VERSION,CLASSES,SETTINGS,A,B,hash,convert,preprocess,transform,gradient,probs} from './learner-core.mjs';
const DIR=path.dirname(fileURLToPath(import.meta.url));
const E='/Users/silencehan/Documents/Codex/research-evidence/issue1467';
const OUT=path.resolve(process.argv[2]||path.join(DIR,'run-v1'));
assert(OUT.startsWith(DIR+'/'),'output must stay in author scope');fs.mkdirSync(OUT);
const write=(n,o)=>fs.writeFileSync(path.join(OUT,n),typeof o==='string'?o:JSON.stringify(o,null,2)+'\n',{flag:'wx'});
const jsonlines=xs=>xs.map(x=>JSON.stringify(x)).join('\n')+'\n';
const started=new Date().toISOString();
const cutoff='1609459220000000000',middleEnd='1609459240000000000';
const clone=x=>JSON.parse(JSON.stringify(x));
const before=JSON.parse(fs.readFileSync(path.join(DIR,'input-hashes-before.json'),'utf8'));
function inputHashes(){return before.inputs.map(s=>{const b=fs.readFileSync(s.path);return {path:s.path,bytes:b.length,sha256:hash(b.toString())};});}
assert.deepEqual(inputHashes(),before.inputs);
const card=fs.readFileSync(path.join(DIR,'WorkCard.md'),'utf8');assert.equal(hash(card),fs.readFileSync(path.join(DIR,'WorkCard.md.sha256'),'utf8').trim());
write('source-freeze.json',{frozen_at:new Date().toISOString(),files:['WorkCard.md','learner-core.mjs','learner-stdin.mjs','run-author.mjs'].map(name=>({name,sha256:hash(fs.readFileSync(path.join(DIR,name),'utf8'))}))});
const readRows=p=>fs.readFileSync(path.join(E,p),'utf8').trim().split('\n').map(JSON.parse);
const a=readRows('trade-grid-a-v1/features.jsonl'),b=readRows('book-grid-b-v1/features.jsonl'),labels=readRows('label-grid-v2/labels.jsonl');
const evaluation=JSON.parse(fs.readFileSync(path.join(E,'stage56/consumer-author/run-v4/evaluation-only.json'),'utf8'));
const receipt=JSON.parse(fs.readFileSync(path.join(E,'stage56/consumer-author/run-v4/receipt.json'),'utf8'));
assert.equal(receipt.partitions[0].end_ns,cutoff);assert.equal(receipt.partitions[1].end_ns,middleEnd);
const evalHash=receipt.outputs.find(x=>x.name==='evaluation-only.json');assert.equal(before.inputs[3].sha256,evalHash.sha256);
for(const rows of [a,b,labels,evaluation])assert.equal(rows.length,572);
for(let i=0;i<572;i++){
 for(const r of [a[i],b[i],labels[i],evaluation[i]]){assert.equal(r.product_id,'BTC-USD');assert.equal(r.decision_ns,a[i].decision_ns);}
 assert.equal(a[i].profile,'trade-history-grid-a/1');assert.equal(b[i].profile,'book-history-grid-b/1');assert.equal(labels[i].profile,'published-mid-first-change/2');
 assert.equal(evaluation[i].label,labels[i].label);assert.equal(evaluation[i].disposition,labels[i].disposition);assert.equal(evaluation[i].label_event_ns,labels[i].resolved_at_ns);
 assert.deepEqual(a[i].features,Object.fromEntries(A.map(k=>[k,b[i].features[k]])));
 if(i)assert.equal(BigInt(a[i].decision_ns)-BigInt(a[i-1].decision_ns),100000000n);
 if(evaluation[i].label_known_ns!==null){assert(BigInt(evaluation[i].label_known_ns)>BigInt(labels[i].resolved_at_ns));assert(BigInt(labels[i].resolved_at_ns)>=BigInt(labels[i].decision_ns));}
 const p=BigInt(a[i].decision_ns)<BigInt(cutoff)?'early':BigInt(a[i].decision_ns)<BigInt(middleEnd)?'middle':'late';assert.equal(evaluation[i].partition,p);
}
function qualifies(row){return BigInt(row.decision_ns)<BigInt(cutoff)&&['observed_change','observed_no_change'].includes(row.disposition)&&CLASSES.includes(row.label)&&row.label_known_ns!==null&&BigInt(row.label_known_ns)<BigInt(cutoff);}
function trainingRequest(rows,evals,arm){const fields=arm==='A'?A:B;return {op:'fit',version:VERSION,arm,fit_cutoff_ns:cutoff,rows:rows.filter((r,i)=>qualifies(evals[i])).map((r)=>{const i=rows.indexOf(r);return {product_id:r.product_id,decision_ns:r.decision_ns,features:Object.fromEntries(fields.map(k=>[k,r.features[k]])),label:evals[i].label};})};}
function predictionRequest(r,arm){const fields=arm==='A'?A:B;return {op:'predict',version:VERSION,product_id:r.product_id,decision_ns:r.decision_ns,features:Object.fromEntries(fields.map(k=>[k,r.features[k]]))};}
const requests={A:trainingRequest(a,evaluation,'A'),B:trainingRequest(b,evaluation,'B')};
for(const arm of ['A','B'])assert.equal(requests[arm].rows.length,169);
const eligibility=evaluation.map(r=>({product_id:r.product_id,decision_ns:r.decision_ns,decision_before_cutoff:BigInt(r.decision_ns)<BigInt(cutoff),label:r.label,disposition:r.disposition,label_event_ns:r.label_event_ns,label_known_ns:r.label_known_ns,known_strict_before_cutoff:r.label_known_ns!==null&&BigInt(r.label_known_ns)<BigInt(cutoff),used_for_fit:qualifies(r),reason:qualifies(r)?'historical-label-known-before-fit':BigInt(r.decision_ns)>=BigInt(cutoff)?'decision-at-or-after-fit':r.disposition}));
assert.equal(eligibility.filter(x=>x.used_for_fit).length,169);assert.equal(eligibility.filter(x=>x.reason==='ineligible_at_decision').length,3);
write('host-training-eligibility.jsonl',jsonlines(eligibility));
const controls=[];
function checked(name,fn){const evidence=fn();controls.push({name,passed:true,evidence});}
checked('label-known strict cutoff including equality',()=>{const t={decision_ns:'1609459219900000000',disposition:'observed_change',label:'up',label_known_ns:'1609459219999999999',label_event_ns:'1609459219900000001'};assert(qualifies(t));assert(!qualifies({...t,label_known_ns:cutoff}));assert(!qualifies({...t,label_known_ns:'1609459220000000001'}));assert(!qualifies({...t,label:null,label_known_ns:null,disposition:'ineligible_at_decision'}));return {cases:4};});
checked('strict A/B scalar schema and finite conversion',()=>{assert.throws(()=>convert({...a[100].features,bid_price:'1'},'A'));assert.throws(()=>convert({...a[100].features,order_seq:'1'},'A'));assert.throws(()=>convert({...a[100].features,last_trade_price:'1'+'0'.repeat(400)},'A'));assert.throws(()=>convert({...b[100].features,queue_imbalance:{numerator:'1',denominator:'0'}},'B'));assert.throws(()=>convert({...a[100].features,last_trade_price:1},'A'));assert.throws(()=>convert({...a[100].features,last_trade_price:undefined},'A'));const f={...b[100].features,published_mid2_delta_1s:'200000000'};assert.equal(convert(f,'B').at(-1),1);return {rejections:6,mid2_200000000_dollars:1};});
checked('all-missing and constant preprocessing retain dimensions',()=>{const xs=[[null,2],[null,2]],p=preprocess(xs,['empty','constant']);assert.deepEqual(p.map(x=>[x.mean,x.scale,x.observed]),[[0,1,0],[2,1,2]]);assert.deepEqual(transform(xs[0],p),[1,0,0,1,0]);return p;});
checked('missing class gradient keeps all three heads',()=>{const w=CLASSES.map(()=>[0,0]),g=gradient(w,[[1,0]],[0],SETTINGS.l2);assert(g[0][0]<0&&g[1][0]>0&&g[2][0]>0);const next=w.map((r,k)=>r.map((v,j)=>v-SETTINGS.learning_rate*g[k][j]));const p=probs(next,[1,0]);assert(p[0]>p[1]&&p[1]===p[2]);return {gradient:g,probabilities:p,synthetic_control_only:true,full_fit:false};});
checked('future feature/label mutations leave fit request byte-identical',()=>{const aa=clone(a),bb=clone(b),ev=clone(evaluation);for(let i=0;i<572;i++)if(BigInt(a[i].decision_ns)>=BigInt(cutoff)){for(const rows of [aa,bb])for(const k of Object.keys(rows[i].features))rows[i].features[k]=k==='queue_imbalance'?{numerator:'0',denominator:'1'}:'7';ev[i]={...ev[i],label:ev[i].label==='up'?'down':'up',label_known_ns:String(BigInt(cutoff)+1n),partition:'transformed-control',loss_eligible:false};}const fitA=trainingRequest(aa,ev,'A'),fitB=trainingRequest(bb,ev,'B');assert.deepEqual(fitA,requests.A);assert.deepEqual(fitB,requests.B);return {changed_saved_future_rows:400,original_request_sha256:{A:hash(requests.A),B:hash(requests.B)},mutated_request_sha256:{A:hash(fitA),B:hash(fitB)},extra_fit_count:0,not_new_market_history:true};});
write('author-controls-before-fit.json',controls);
const processReceipts=[],allPredictions={},models={},rejections=[],prefixEvidence=[];
let hostPeak=process.memoryUsage().rss,childPeak=0;
async function startProcess(arm){const logIn=fs.openSync(path.join(OUT,arm+'-stdin.jsonl'),'wx'),logOut=fs.openSync(path.join(OUT,arm+'-stdout.jsonl'),'wx'),logEvents=fs.openSync(path.join(OUT,arm+'-events.jsonl'),'wx'),logErr=fs.openSync(path.join(OUT,arm+'-stderr.jsonl'),'wx');
 const args=['--max-old-space-size=64',path.join(DIR,'learner-stdin.mjs')],child=spawn(process.execPath,args,{cwd:DIR,env:{PATH:process.env.PATH,LANG:'C'},stdio:['pipe','pipe','pipe']});let pending=null,closed=false;const start=new Date().toISOString();
 const close=new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',(code,signal)=>{closed=true;if(pending){pending.reject(Error('unexpected child exit'));pending=null;}resolve({code,signal});});});
 const lines=readline.createInterface({input:child.stdout,crlfDelay:Infinity});lines.on('line',line=>{fs.writeSync(logOut,line+'\n');if(!pending)throw Error('unsolicited stdout');const p=pending;pending=null;fs.writeSync(logEvents,JSON.stringify({direction:'out',wall_at:new Date().toISOString(),pid:child.pid,sha256:hash(line),for_input_sha256:p.input_hash})+'\n');p.resolve(JSON.parse(line));});
 const errors=readline.createInterface({input:child.stderr,crlfDelay:Infinity});errors.on('line',line=>{fs.writeSync(logErr,line+'\n');try{childPeak=Math.max(childPeak,JSON.parse(line).rss_bytes||0);}catch{}});
 async function ask(req){assert(!pending&&!closed);const line=JSON.stringify(req);fs.writeSync(logIn,line+'\n');fs.writeSync(logEvents,JSON.stringify({direction:'in',wall_at:new Date().toISOString(),pid:child.pid,sha256:hash(line)})+'\n');hostPeak=Math.max(hostPeak,process.memoryUsage().rss);return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{child.kill('SIGTERM');reject(Error('child response exceeded 15000ms'));},15000);pending={input_hash:hash(line),resolve:x=>{clearTimeout(timer);resolve(x);},reject:e=>{clearTimeout(timer);reject(e);}};child.stdin.write(line+'\n');});}
 async function finish(){child.stdin.end();const r=await close;for(const fd of [logIn,logOut,logEvents,logErr])fs.closeSync(fd);assert.equal(r.code,0);processReceipts.push({arm,pid:child.pid,command:[process.execPath,...args],cwd:DIR,started_at:start,finished_at:new Date().toISOString(),...r,input_sha256:hash(fs.readFileSync(path.join(OUT,arm+'-stdin.jsonl'),'utf8')),output_sha256:hash(fs.readFileSync(path.join(OUT,arm+'-stdout.jsonl'),'utf8'))});}
 return {ask,finish,pid:child.pid};}
try {
for(const arm of ['A','B']){
 const rows=arm==='A'?a:b;const child=await startProcess(arm);
 const earlyReq=predictionRequest(rows[172],arm);assert.equal((await child.ask(earlyReq)).error,'predict:not-fitted');
 const invalidFits=[];
 const withPath={...clone(requests[arm]),label_path:'/forbidden/not-read'};invalidFits.push({name:'fit-extra-path',request:withPath});
 const withKnown=clone(requests[arm]);withKnown.rows[0].label_known_ns='1609459219999999999';invalidFits.push({name:'fit-row-known-field',request:withKnown});
 const withFuture=clone(requests[arm]);withFuture.rows.at(-1).decision_ns=cutoff;invalidFits.push({name:'fit-decision-at-cutoff',request:withFuture});
 const withFeature=clone(requests[arm]);withFeature.rows[0].features[arm==='A'?'bid_price':'native_sequence']='1';invalidFits.push({name:'fit-forbidden-feature',request:withFeature});
 for(const item of invalidFits){const response=await child.ask(item.request);assert.equal(response.status,'rejected');rejections.push({arm,...item,response});}
 const fitStart=new Date().toISOString();const fitted=await child.ask(requests[arm]);assert.equal(fitted.status,'fitted');assert.equal(fitted.fit_steps,200);assert.equal(fitted.model_sha256,hash(fitted.model));assert.equal(fitted.model.training_request_sha256,hash(requests[arm]));models[arm]=fitted.model;write(arm+'-model.json',fitted.model);write(arm+'-fit-receipt.json',{pid:child.pid,fit_started_at:fitStart,fit_responded_at:new Date().toISOString(),logical_cutoff_ns:cutoff,...fitted});
 const predictions=[];let prefix=null;for(let i=0;i<572;i++){
  const r=rows[i];if(BigInt(r.decision_ns)<BigInt(cutoff)){predictions.push({arm,product_id:r.product_id,decision_ns:r.decision_ns,status:'not-yet-fitted',probabilities:null});continue;}
  const request=predictionRequest(r,arm);const response=await child.ask(request);assert.equal(response.status,'predicted');assert.equal(response.decision_ns,r.decision_ns);assert.equal(response.model_sha256,fitted.model_sha256);assert.equal(response.probabilities.length,3);assert(Math.abs(response.probabilities.reduce((s,p)=>s+p,0)-1)<1e-12);assert(response.probabilities.every(p=>Number.isFinite(p)&&p>=0&&p<=1));predictions.push(response);
  if(i===271)prefix=JSON.stringify(predictions.slice(172));
 }
 assert.equal(predictions.filter(x=>x.status==='predicted').length,400);assert.equal(JSON.stringify(predictions.slice(172,272)),prefix);
 // Re-ask the exact first 100 previously emitted inputs, after the stream was extended.
 // This is read-only inference with the same frozen model, never a second fit.
 const replay=[];for(let i=172;i<272;i++)replay.push(await child.ask(predictionRequest(rows[i],arm)));assert.deepEqual(replay,predictions.slice(172,272));prefixEvidence.push({arm,prefix_decisions:100,extended_by_decisions:300,original_prefix_sha256:hash(prefix),after_extension_sha256:hash(JSON.stringify(replay)),extra_fit_count:0});
 const valid=predictionRequest(rows[172],arm),bad=[];
 for(const field of ['label','partition','loss_eligible','label_known_ns','label_path','future_watermark_ns','history','order_seq'])bad.push({name:'top-level-'+field,request:{...valid,[field]:'forbidden'}});
 for(const field of arm==='A'?['bid_price','native_sequence','label']:['native_sequence','label'])bad.push({name:'feature-'+field,request:{...valid,features:{...valid.features,[field]:'1'}}});
 bad.push({name:'decision-before-fit',request:{...valid,decision_ns:rows[0].decision_ns}});
 bad.push({name:'repeat-fit-disallowed',request:requests[arm]});
 for(const item of bad){const out=await child.ask(item.request);assert.equal(out.status,'rejected');rejections.push({arm,...item,response:out});}
 const missing={...valid,features:Object.fromEntries((arm==='A'?A:B).map(k=>[k,null]))};const missingOut=await child.ask(missing);assert.equal(missingOut.status,'predicted');write(arm+'-all-missing-control.json',{synthetic_interface_control:true,request:missing,response:missingOut});
 const inspect=await child.ask({op:'inspect',version:VERSION});assert.equal(inspect.model_sha256,fitted.model_sha256);assert.deepEqual(inspect.model,fitted.model);write(arm+'-frozen-after-inference.json',inspect);
 write(arm+'-predictions.jsonl',jsonlines(predictions));allPredictions[arm]=predictions;await child.finish();
}
// No label evaluation before both arms completed all primary predictions.
const evaluationJoinAt=new Date().toISOString();const joined=[];const diagnostics={};
for(const arm of ['A','B']){diagnostics[arm]={};for(const partition of ['middle','late']){const group=[];for(let i=0;i<572;i++){const e=evaluation[i],p=allPredictions[arm][i];if(e.partition!==partition)continue;assert.equal(p.status,'predicted');joined.push({...p,evaluation:e});if(e.loss_eligible){const y=CLASSES.indexOf(e.label);assert(y>=0);group.push({correct:p.prediction===e.label,logloss:-Math.log(Math.max(p.probabilities[y],1e-15)),brier:p.probabilities.reduce((s,q,k)=>s+(q-(k===y?1:0))**2,0)});}}assert.equal(group.length,200);diagnostics[arm][partition]={rows:group.length,accuracy:group.filter(x=>x.correct).length/group.length,mean_logloss:group.reduce((s,x)=>s+x.logloss,0)/group.length,multiclass_brier:group.reduce((s,x)=>s+x.brier,0)/group.length};}}
write('host-evaluation-after-predictions.jsonl',jsonlines(joined));write('diagnostics.json',{scope:'same-seen-minute reproduction diagnostics only; no B-minus-A, significance, generalization or profit inference',evaluation_join_at:evaluationJoinAt,metrics:diagnostics});write('rejected-examples.json',rejections);write('prefix-extension-controls.json',prefixEvidence);write('process-receipts.json',processReceipts);
const after=inputHashes();assert.deepEqual(after,before.inputs);write('input-hashes-after.json',{verified_at:new Date().toISOString(),inputs:after});
const sourceFreeze=JSON.parse(fs.readFileSync(path.join(OUT,'source-freeze.json'),'utf8'));for(const s of sourceFreeze.files)assert.equal(hash(fs.readFileSync(path.join(DIR,s.name),'utf8')),s.sha256);
write('receipt.json',{profile:'stage69-actual-learner-author/1',started_at:started,finished_at:new Date().toISOString(),status:'author-checks-passed',independent_review:'pending',version:VERSION,settings:SETTINGS,source_freeze_unchanged:true,fit_count:2,fit_rows_per_arm:169,common_decisions:572,predicted_per_arm:400,not_yet_fitted_per_arm:172,loss_rows:{early:169,middle:200,late:200},parameter_counts:{A:33,B:81},classes:CLASSES,class_counts:{A:models.A.class_counts,B:models.B.class_counts},model_sha256:{A:hash(models.A),B:hash(models.B)},checks:{pure_controls:controls.length,process_rejections:rejections.length,prefix_extensions:prefixEvidence.length,frozen_model_inspections:2,all_missing_inferences:2,input_hashes_unchanged:after.length},memory:{host_sampled_peak_rss_bytes:hostPeak,child_sampled_peak_rss_bytes:childPeak,conservative_sum_of_sampled_peaks:hostPeak+childPeak,not_os_enforced:true},diagnostics,scope:'fixed finite-feature named trusted-code protocol; same seen minute; no OS sandbox, full-history strong B, recursive C, statistical or profit confirmation'});
const outputFiles=fs.readdirSync(OUT).map(name=>{const p=path.join(OUT,name),v=fs.readFileSync(p);return {name,bytes:v.length,sha256:hash(v.toString())};});const size=outputFiles.reduce((s,x)=>s+x.bytes,0);assert(size<128*1024*1024);write('manifest.json',{files:outputFiles,bytes_before_manifest:size});console.log(JSON.stringify({status:'passed',output:OUT,fit_count:2,bytes:size,diagnostics}));
}catch(error){write('failure.json',{failed_at:new Date().toISOString(),message:error.message,stack:error.stack,completed_processes:processReceipts});throw error;}
