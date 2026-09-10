// #1371 R10：实际同源HTTP，TestOnly只控制交付末代及单字段响应变换；DOM是模拟。
async function runR10() {
 const fs=require('fs'),vm=require('vm'),assert=require('assert');
 const base=process.argv[3], out=process.argv[4], root=process.argv[5];
 const source=fs.readFileSync(root+'/s_session/browser/index.html','utf8').match(/<script>([\s\S]*?)<\/script>/)[1].replace(/load\("current"\);\s*$/,'');
 const results={cases:{},http:[]};
 function context(){
   const els=new Map();function el(){return {innerHTML:'',textContent:'',value:'0',placeholder:'',checked:false,classList:{toggle(){}},addEventListener(){},focus(){},scrollIntoView(){}}}
   const document={getElementById(id){if(!els.has(id))els.set(id,el());return els.get(id)},querySelector(){return el()},createElement(){let x=el();Object.defineProperty(x,'innerHTML',{get(){return this.textContent}});return x}};
   const c=vm.createContext({document,structuredClone,console});vm.runInContext(source,c);return {c,els};
 }
 async function get(path){const response=await fetch(base+path);const text=await response.text();results.http.push({url:base+path,status:response.status,body:text});assert.equal(response.status,200);return JSON.parse(text)}
 function stateOf(c){return JSON.parse(vm.runInContext('JSON.stringify({cursor:lastAppliedGen,binding:sessionBinding,state:currentState,objects:[...appliedObjects],withdrawn:[...appliedWithdrawn],witnesses:[...appliedWitnesses],relations:[...appliedRelations],observations:[...appliedObservations],raw:[...appliedRawHistory]})',c))}
 function differences(a,b,path=''){if(JSON.stringify(a)===JSON.stringify(b))return [];if(a&&b&&typeof a==='object'&&typeof b==='object')return [...new Set([...Object.keys(a),...Object.keys(b)])].flatMap(k=>differences(a[k],b[k],path+'/'+k));return [path]}
 for(const field of [null,'profile_hash','profile_id']) {
   const {c,els}=context(), name=field||'healthy';const initial=await get('/api/state?as_of=1');c.initial=initial;
   vm.runInContext('validateState(initial);rebuildCommittedFromState(initial);renderAllFromState(initial)',c);
   const before=stateOf(c);const trace=[];
   c.fetch=async path=>{
     let body=await get(path);const original=structuredClone(body);
     if(path.startsWith('/api/delta')) {
       body={...body,generation:'2',structure_cut:'cut-2',deltas:body.deltas.filter(d=>BigInt(d.generation)<=2n)};
     } else {
       assert.equal(path,'/api/state?as_of=2');
       if(field)body.snapshot[field]=field==='profile_hash'?'f'.repeat(64):'wrong-profile-id';
       const diff=differences(original,body);
       assert.deepEqual(diff,field?['/snapshot/'+field]:[]);
     }
     trace.push({path,original,delivered:structuredClone(body)});
     return {ok:true,status:200,json:async()=>structuredClone(body)};
   };
   await vm.runInContext('applyStream()',c);const after=stateOf(c);const message=els.get('stream-result').innerHTML;
   results.cases[name]={before,after,message,trace};
   if(field){assert.deepEqual(after,before);assert(message.includes(field));assert(!message.includes('ok-box'))}
   else{assert.equal(after.cursor,'2');assert(message.includes('ok-box'))}
 }
 // cut0完整健康返回必须能在当前HTML中正常加载。
 const {c}=context();c.zero=await get('/api/state?as_of=0');vm.runInContext('validateState(zero)',c);
 fs.writeFileSync(out,JSON.stringify(results,null,2));console.log('R10 actual HTTP + HTML函数断言通过（非GUI）');
}
if (process.argv[2] === '--r10') {
 runR10().catch(e=>{console.error(e);process.exitCode=1});
} else {
// #1371 R9：运行实际 HTML 函数 + 原 HTTP payload；模拟DOM，不是GUI。
const fs=require('fs'),vm=require('vm'),assert=require('assert');
const dir=process.argv[2],r3dir=process.argv[3],root=process.argv[4];
const api=JSON.parse(fs.readFileSync(dir+'/api-cuts.json')).results;
const states=Object.fromEntries(Object.entries(api.states).map(([g,r])=>[g,JSON.parse(r.body)]));
const watch=JSON.parse(api.watches['0'].body);
const script=fs.readFileSync(root+'/s_session/browser/index.html','utf8').match(/<script>([\s\S]*?)<\/script>/)[1].replace(/load\("current"\);\s*$/,'');
function ctx() {
 const els=new Map();function el(){return {innerHTML:'',textContent:'',value:'0',placeholder:'',checked:false,classList:{toggle(){}},addEventListener(){},focus(){},scrollIntoView(){}}}
 const document={getElementById(id){if(!els.has(id))els.set(id,el());return els.get(id)},querySelector(){return el()},createElement(){let x=el();Object.defineProperty(x,'innerHTML',{get(){return this.textContent}});return x}};
 const c=vm.createContext({document,structuredClone,console});vm.runInContext(script,c);return {c,els};
}
function fetcher(c,fn){c.fetch=async url=>({ok:true,status:200,json:async()=>structuredClone(await fn(url))});}
function stateFetch(url){const match=url.match(/^\/api\/state\?as_of=(\d+)$/);assert(match,'必须取指定cut：'+url);return states[match[1]];}
const output={};
async function test(name,fn){await fn();output[name]='断言通过';}
(async()=>{
 await test('gen0',async()=>{let {c}=ctx();c.s=states['0'];vm.runInContext('validateState(s)',c)});
 await test('seven_cut_incremental',async()=>{
   const {c,els}=ctx();
   for(let g=1;g<=7;g++){
     const d=watch.deltas[g-1];fetcher(c,url=>url.startsWith('/api/delta')?{...watch,generation:String(g),structure_cut:'cut-'+g,deltas:[d]}:stateFetch(url));
     await vm.runInContext('applyStream()',c);assert.equal(vm.runInContext('lastAppliedGen',c),String(g),els.get('stream-result').innerHTML);
     assert(!els.get('stream-result').innerHTML.includes('err-box'));
   }
   output.final=JSON.parse(vm.runInContext('JSON.stringify(currentState)',c));
 });
 await test('new_writer_advances_while_fetching',async()=>{
   const {c,els}=ctx();fetcher(c,url=>url.startsWith('/api/delta')?{...watch,generation:'2',structure_cut:'cut-2',deltas:watch.deltas.slice(0,2)}:stateFetch(url));
   await vm.runInContext('applyStream()',c);assert.equal(vm.runInContext('lastAppliedGen',c),'2',els.get('stream-result').innerHTML);
 });
 await test('old_actual_wrong_root_is_rejected',async()=>{
   const old=JSON.parse(fs.readFileSync(r3dir+'/probe-results.json'));const bad=old.corrupt.wrong_current_root.http;
   const {c,els}=ctx();fetcher(c,url=>url.startsWith('/api/delta')?bad['/api/delta?after_generation=0'][1]:bad['/api/state'][1]);
   await vm.runInContext('applyStream()',c);assert.equal(vm.runInContext('lastAppliedGen',c),'0');assert(!els.get('stream-result').innerHTML.includes('ok-box'));
 });
 for(const field of ['index_frontier','scope','input_frontier','seq_range','catalog_run_status','catalog_evidence','session_id','generation','structure_cut','catalog_revision']){
   await test('tuple_'+field,async()=>{
     const {c,els}=ctx();c.s=states['1'];vm.runInContext('validateState(s); rebuildCommittedFromState(s); renderAllFromState(s)',c);
     const before=vm.runInContext('JSON.stringify([currentState,[...appliedObjects],sessionBinding])',c);
     const bad=structuredClone(states['2']);bad.snapshot[field]='bad';
     fetcher(c,url=>url.startsWith('/api/delta')?{...watch,generation:'2',deltas:[watch.deltas[1]]}:bad);
     await vm.runInContext('applyStream()',c);assert.equal(vm.runInContext('lastAppliedGen',c),'1');
     assert.equal(vm.runInContext('JSON.stringify([currentState,[...appliedObjects],sessionBinding])',c),before);
   });
 }
 await test('coherent_state_wrong_root_and_numeric_predecessor',async()=>{
   for (const kind of ['root','numeric']) {
     const {c,els}=ctx();const bad=structuredClone(states['7']);
     if(kind==='root') {
       for(const p of [bad.snapshot,bad.catalog,bad.cut])p.index_frontier=states['1'].snapshot.index_frontier;
       bad.snapshot.catalog_evidence.batch_id=bad.snapshot.index_frontier;
       bad.catalog.catalog_evidence.batch_id=bad.snapshot.index_frontier;
       bad.catalog.items.find(x=>x.id==='CC-006').evidence.batch_id=bad.snapshot.index_frontier;
     } else {
       const w=bad.snapshot.witnesses.find(w=>w.raw_bars.some(r=>r.supersedes_revision==='9007199254740993'));
       w.raw_bars.find(r=>r.supersedes_revision==='9007199254740993').supersedes_revision=9007199254740993;
     }
     fetcher(c,url=>url.startsWith('/api/delta')?watch:bad);await vm.runInContext('applyStream()',c);
     assert.equal(vm.runInContext('lastAppliedGen',c),'0',els.get('stream-result').innerHTML);
   }
 });
 await test('duplicate_empty_failed_gap_late',async()=>{
   const {c,els}=ctx();c.s=states['7'];vm.runInContext('validateState(s); rebuildCommittedFromState(s); renderAllFromState(s)',c);
   const before=vm.runInContext('JSON.stringify(sessionBinding)',c);
   for(const deltas of [watch.deltas,[]]) {
     fetcher(c,url=>url.startsWith('/api/delta')?{...watch,deltas}:stateFetch(url));await vm.runInContext('applyStream()',c);
     assert.equal(vm.runInContext('JSON.stringify(sessionBinding)',c),before);
   }
   fetcher(c,url=>{if(url.startsWith('/api/delta'))return {...watch,deltas:[]};throw Error('断连')});
   await vm.runInContext('applyStream()',c);assert.equal(vm.runInContext('JSON.stringify(sessionBinding)',c),before);
   fetcher(c,url=>url.startsWith('/api/delta')?JSON.parse(api.gap.body):stateFetch(url));
   await vm.runInContext('applyStream()',c);assert.equal(vm.runInContext('lastAppliedGen',c),'7');
   let resolve,count=0;fetcher(c,url=>++count===1?new Promise(r=>resolve=r):states['7']);
   const a=vm.runInContext('load("current")',c);await Promise.resolve();const b=vm.runInContext('load("current")',c);await b;resolve(states['1']);await a;
   assert.equal(vm.runInContext('currentState.snapshot.generation',c),'7');
 });
 fs.writeFileSync(dir+'/node-results.json',JSON.stringify(output,null,2));console.log('实际HTML函数回归通过（非GUI）');
})().catch(e=>{fs.writeFileSync(dir+'/node-results.json',JSON.stringify({...output,error:String(e),stack:e.stack},null,2));console.error(e);process.exitCode=1});

}
