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
// #1371 R11：请求上下文绑定。真实HTTP源 + 模拟DOM/TestOnly交付，不代表GUI/TCP故障。
async function runR11() {
  const fs = require('fs'), vm = require('vm'), assert = require('assert'), crypto = require('crypto');
  const [,, ,base,outfile,root,phase='green'] = process.argv;
  const htmlBytes = fs.readFileSync(root+'/s_session/browser/index.html');
  const source = htmlBytes.toString().match(/<script>([\s\S]*?)<\/script>/)[1].replace(/load\("current"\);\s*$/,'');
  const evidence = {phase, source_sha256:crypto.createHash('sha256').update(htmlBytes).digest('hex'), http:[], cases:{}};
  function context() {
    const els=new Map();
    function element() {const flags={};return {innerHTML:'',textContent:'',value:'3',placeholder:'',checked:false,
      flags,classList:{toggle(k,v){flags[k]=v}},addEventListener(){},focus(){},scrollIntoView(){}};}
    const document={getElementById(id){if(!els.has(id))els.set(id,element());return els.get(id)},
      querySelector(q){return this.getElementById(q)},createElement(){const e=element();Object.defineProperty(e,'innerHTML',{get(){return this.textContent}});return e}};
    const c=vm.createContext({document,structuredClone,console});vm.runInContext(source,c);return {c,els};
  }
  function snapshot(c) {return JSON.parse(vm.runInContext('JSON.stringify({currentState,lastAppliedGen,sessionBinding,historyMode,objects:[...appliedObjects],withdrawn:[...appliedWithdrawn],witnesses:[...appliedWitnesses],relations:[...appliedRelations],observations:[...appliedObservations],raw:[...appliedRawHistory]})',c));}
  function ui(els) {return Object.fromEntries([...els].map(([k,v])=>[k,{html:v.innerHTML,text:v.textContent,value:v.value,flags:v.flags}]));}
  function diff(a,b,p='') {
    if(JSON.stringify(a)===JSON.stringify(b))return [];
    if(a&&b&&typeof a==='object'&&typeof b==='object')return [...new Set([...Object.keys(a),...Object.keys(b)])].flatMap(k=>diff(a[k],b[k],p+'/'+k));
    return [p];
  }
  async function get(path) {
    const r=await fetch(base+path), body=await r.text();
    evidence.http.push({url:base+path,status:r.status,body,sha256:crypto.createHash('sha256').update(body).digest('hex')});
    assert.equal(r.status,200,'实际源HTTP必须200');return JSON.parse(body);
  }
  function install(c,transform,trace) {
    c.fetch=async path=>{
      const original=await get(path), delivered=await transform(path,structuredClone(original));
      trace.push({path,original,delivered:structuredClone(delivered),differences:diff(original,delivered)});
      return {ok:true,status:200,json:async()=>structuredClone(delivered)};
    };
  }
  function seed(c,st) {c.seed=st;vm.runInContext('validateState(seed); rebuildCommittedFromState(seed); syncModeUI("asof",seed.snapshot); renderAllFromState(seed)',c);}
  async function record(name,fn) {
    const r={};evidence.cases[name]=r;
    try {await fn(r);r.passed=true;} catch(e) {r.passed=false;r.error=String(e);r.stack=e.stack;}
  }
  const s3=await get('/api/state?as_of=3'), s4=await get('/api/state?as_of=4'), s5=await get('/api/state?as_of=5');
  for(const [mode,g] of [['current',null],['asof','0'],['asof','3']]) {
    await record('healthy_load_'+mode+'_'+g,async r=>{
      const {c,els}=context();r.trace=[];install(c,async(p,v)=>v,r.trace);
      await vm.runInContext(mode==='current'?'load("current")':`load("asof",${JSON.stringify(g)})`,c);
      r.after=snapshot(c);r.ui=ui(els);const snap=r.after.currentState.snapshot;
      assert.equal(snap.history_mode,mode==='current'?'RecomputedWithRevision':'AsKnown');
      if(g!==null)assert.equal(snap.generation,g);assert.equal(els.get('status-section').innerHTML,'');
    });
  }
  for(const warm of [false,true]) for(const kind of ['wrong_cut','wrong_mode']) {
    await record('load_'+kind+'_'+(warm?'warm':'cold'),async r=>{
      const {c,els}=context();r.trace=[];
      if(warm){install(c,async(p,v)=>v,[]);await vm.runInContext('load("asof","3")',c);}
      r.before=snapshot(c);r.before_ui=ui(els);
      install(c,async(p,v)=>kind==='wrong_cut'?s4:s3,r.trace);
      await vm.runInContext(kind==='wrong_cut'?'load("asof","3")':'load("current")',c);
      r.after=snapshot(c);r.ui=ui(els);
      assert.equal(r.after.currentState,null,'load必须拒绝本次完整但错误的响应');
      assert.equal(r.after.lastAppliedGen,'0','load反例没有预先提交流缓存');
      assert(els.get('status-section').innerHTML.includes('读取失败'));
      assert(els.get('objects').innerHTML.includes('未渲染'));
      install(c,async(p,v)=>v,r.trace);
      await vm.runInContext(kind==='wrong_cut'?'load("asof","3")':'load("current")',c);
      r.retry=snapshot(c);r.retry_ui=ui(els);
      assert.equal(r.retry.currentState.snapshot.history_mode,kind==='wrong_cut'?'AsKnown':'RecomputedWithRevision');
      assert.equal(els.get('status-section').innerHTML,'');
    });
  }
  for(const kind of ['healthy','mode_pair','wrong_cut','profile','request_failure','session_mismatch']) {
    await record('stream_'+kind,async r=>{
      const {c,els}=context();seed(c,s3);r.before=snapshot(c);r.before_ui=ui(els);r.trace=[];
      const transform=async(path,v)=>{
        if(path.startsWith('/api/delta'))return {...v,generation:'4',structure_cut:'cut-4',session_id:kind==='session_mismatch'?'different-session':v.session_id,deltas:v.deltas.filter(d=>BigInt(d.generation)<=4n)};
        assert.equal(path,'/api/state?as_of=4');
        if(kind==='wrong_cut')return s5;
        if(kind==='mode_pair'){v.snapshot.history_mode='RecomputedWithRevision';v.snapshot.as_of_generation=null;assert.deepEqual(diff(s4,v).sort(),['/snapshot/as_of_generation','/snapshot/history_mode']);}
        if(kind==='profile')v.snapshot.profile_hash='f'.repeat(64);
        if(kind==='request_failure'){r.trace.push({path,original:v,transport_error:'TestOnly响应读取失败'});throw Error('TestOnly响应读取失败（非TCP故障）');}
        return v;
      };
      install(c,transform,r.trace);await vm.runInContext('applyStream()',c);
      r.after=snapshot(c);r.ui=ui(els);
      if(kind==='healthy'){assert.equal(r.after.lastAppliedGen,'4');assert.deepEqual(r.after.currentState,s4);assert(els.get('stream-result').innerHTML.includes('ok-box'));}
      else {
        assert.deepEqual(r.after,r.before,'stream失败必须保持完整逻辑状态');
        assert(!els.get('stream-result').innerHTML.includes('ok-box'));
        install(c,async(path,v)=>path.startsWith('/api/delta')?{...v,generation:'4',structure_cut:'cut-4',deltas:v.deltas.filter(d=>BigInt(d.generation)<=4n)}:v,r.trace);
        await vm.runInContext('applyStream()',c);r.retry=snapshot(c);r.retry_ui=ui(els);
        assert.equal(r.retry.lastAppliedGen,'4');assert.deepEqual(r.retry.currentState,s4);
      }
    });
  }
  // Gap源是服务真实after_generation=99的合法Gap；TestOnly重送这个完整信封，非正常服务自行错投。
  const gap=await get('/api/delta?after_generation=99'), gapGen=gap.gap.rebuild_cut.slice(4), gapState=await get('/api/state?as_of='+gapGen);
  for(const kind of ['healthy','mode_pair','wrong_cut','request_failure']) {
    await record('gap_'+kind,async r=>{
      const {c,els}=context();seed(c,s3);r.before=snapshot(c);r.before_ui=ui(els);r.trace=[];
      install(c,async(path,v)=>{
        if(path.startsWith('/api/delta'))return gap;
        assert.equal(path,'/api/state?as_of='+gapGen);
        if(kind==='wrong_cut')return s4;
        if(kind==='mode_pair'){v.snapshot.history_mode='RecomputedWithRevision';v.snapshot.as_of_generation=null;}
        if(kind==='request_failure'){r.trace.push({path,original:v,transport_error:'TestOnly响应读取失败'});throw Error('TestOnly响应读取失败（非TCP故障）');}
        return v;
      },r.trace);await vm.runInContext('applyStream()',c);r.after=snapshot(c);r.ui=ui(els);
      if(kind==='healthy'){assert.equal(r.after.lastAppliedGen,gapGen);assert.deepEqual(r.after.currentState,gapState);}
      else{
        assert.deepEqual(r.after,r.before,'Gap失败必须保持完整逻辑状态');
        install(c,async(path,v)=>path.startsWith('/api/delta')?gap:v,r.trace);await vm.runInContext('applyStream()',c);
        r.retry=snapshot(c);r.retry_ui=ui(els);assert.equal(r.retry.lastAppliedGen,gapGen);assert.deepEqual(r.retry.currentState,gapState);
      }
    });
  }
  for(const g of ['03','+3',' 3','', '-1', '9223372036854775808',3]) {
    await record('noncanonical_'+JSON.stringify(g),async r=>{
      const {c,els}=context();let requests=0;c.fetch=async()=>{requests++;throw Error('不应发请求')};c.input=g;
      await vm.runInContext('load("asof",input)',c);r.requests=requests;r.after=snapshot(c);r.ui=ui(els);
      assert.equal(requests,0);assert.equal(r.after.currentState,null);assert(els.get('status-section').innerHTML.includes('读取失败'));
    });
  }
  await record('unknown_mode_rejected_before_fetch',async r=>{
    const {c,els}=context();let requests=0;c.fetch=async()=>{requests++;throw Error('不应发请求')};
    await vm.runInContext('load("other")',c);r.requests=requests;r.after=snapshot(c);r.ui=ui(els);
    assert.equal(requests,0);assert.equal(r.after.currentState,null);assert(els.get('status-section').innerHTML.includes('读取失败'));
  });
  // current没有预定generation：真实早先current可接受，不新增新鲜度策略。
  const api=JSON.parse(fs.readFileSync(require('path').join(require('path').dirname(outfile),'api-cuts.json'))).results;
  const earlierCurrent=JSON.parse(api.earlier_current.body);
  assert.equal(earlierCurrent.snapshot.history_mode,'RecomputedWithRevision');
  await record('current_without_generation_constraint',async r=>{
    const {c,els}=context();r.trace=[];r.source=api.earlier_current;
    install(c,async()=>earlierCurrent,r.trace);await vm.runInContext('load("current")',c);
    r.after=snapshot(c);r.ui=ui(els);assert.deepEqual(r.after.currentState,earlierCurrent);
  });
  // 获胜current必须是真实current来源；旧成功/失败均不能覆盖已完成的新请求。
  const current=await get('/api/state');
  for(const failure of [false,true])await record('late_'+(failure?'failure':'success'),async r=>{
    const {c,els}=context();let resolve,reject,count=0;const old=earlierCurrent;
    c.fetch=async path=>{assert.equal(path,'/api/state');const body=++count===1?await new Promise((res,rej)=>{resolve=res;reject=rej}):current;return {ok:true,status:200,json:async()=>structuredClone(body)}};
    const pending=vm.runInContext('load("current")',c);await Promise.resolve();await vm.runInContext('load("current")',c);
    r.winner=snapshot(c);r.winner_ui=ui(els);assert.equal(r.winner.currentState.snapshot.history_mode,'RecomputedWithRevision');
    if(failure)reject(Error('TestOnly晚到失败'));else resolve(old);await pending;
    r.after=snapshot(c);r.ui=ui(els);assert.deepEqual(r.after,r.winner);assert.deepEqual(r.ui,r.winner_ui);
  });
  evidence.failed=Object.entries(evidence.cases).filter(([,v])=>!v.passed).map(([k])=>k);
  fs.writeFileSync(outfile,JSON.stringify(evidence,null,2));console.log(JSON.stringify({phase,cases:Object.keys(evidence.cases).length,failed:evidence.failed}));
  if(evidence.failed.length)process.exitCode=1;
}

if (process.argv[2] === '--r11') {
 runR11().catch(e=>{console.error(e);process.exitCode=1});
} else if (process.argv[2] === '--r10') {
 runR10().catch(e=>{console.error(e);process.exitCode=1});
} else {
// #1371 R9：运行实际 HTML 函数 + 原 HTTP payload；模拟DOM，不是GUI。
const fs=require('fs'),vm=require('vm'),assert=require('assert');
const dir=process.argv[2],r3dir=process.argv[3],root=process.argv[4];
const api=JSON.parse(fs.readFileSync(dir+'/api-cuts.json')).results;
const states=Object.fromEntries(Object.entries(api.states).map(([g,r])=>[g,JSON.parse(r.body)]));
const watch=JSON.parse(api.watches['0'].body);
const current=JSON.parse(api.current.body);
const earlierCurrent=JSON.parse(api.earlier_current.body);
assert.equal(earlierCurrent.snapshot.history_mode,'RecomputedWithRevision');
assert.equal(current.snapshot.history_mode,'RecomputedWithRevision');
assert.equal(current.snapshot.as_of_generation,null);
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
   let resolve,count=0;fetcher(c,url=>++count===1?new Promise(r=>resolve=r):current);
   const a=vm.runInContext('load("current")',c);await Promise.resolve();const b=vm.runInContext('load("current")',c);await b;resolve(earlierCurrent);await a;
   assert.equal(vm.runInContext('currentState.snapshot.generation',c),'7');
   assert.equal(vm.runInContext('currentState.snapshot.history_mode',c),'RecomputedWithRevision');
   assert.equal(vm.runInContext('currentState.snapshot.as_of_generation',c),null);
 });
 fs.writeFileSync(dir+'/node-results.json',JSON.stringify(output,null,2));console.log('实际HTML函数回归通过（非GUI）');
})().catch(e=>{fs.writeFileSync(dir+'/node-results.json',JSON.stringify({...output,error:String(e),stack:e.stack},null,2));console.error(e);process.exitCode=1});

}
