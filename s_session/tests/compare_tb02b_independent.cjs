// #1392 独立原件对照：读取冻结手算 Oracle 与公开产物，不导入生产 reducer。
'use strict';
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const read=p=>JSON.parse(fs.readFileSync(p,'utf8'));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const canon=v=>JSON.stringify(v,(_,x)=>x&&typeof x==='object'&&!Array.isArray(x)?Object.fromEntries(Object.keys(x).sort().map(k=>[k,x[k]])):x);
function ok(test,why){if(!test)throw Error(why);}
function eq(a,b,why){ok(canon(a)===canon(b),why);}
function n(v){ok(/^-?\d+$/.test(String(v)),'非规范整数');const r=Number(v);ok(Number.isSafeInteger(r),'非安全整数');return r;}
const labels=['merged_gap_below_3','raw_between_below_minimum','top_not_above_bottom'];
function decoded(snapshot){
 const of=k=>snapshot.objects.filter(o=>o.kind===k).map(o=>o.payload);
 const groups=of('CC-005.inclusion_group').sort((a,b)=>n(a.group_index)-n(b.group_index));
 const index=new Map(groups.map(g=>[g.group_anchor,n(g.group_index)]));
 const descriptions=of('CC-007.fractal_description');
 const endpoints=of('CC-008.endpoint').map(p=>p.data).sort((a,b)=>n(a.merged_index)-n(b.merged_index));
 const fractals=endpoints.map(e=>{
  ok(e.extreme_roots.length===1,'端点必须有唯一原始极值根');
  const ds=descriptions.filter(d=>d.window[1]===e.group_anchor);ok(ds.length===1,'分型窗口说明不唯一');
  eq(ds[0].branch,e.kind,'分型说明与端点类型不一致');
  return [n(e.merged_index),e.kind,n(e.price),n(e.extreme_roots[0]),ds[0].window.map(a=>{ok(index.has(a),'窗口缺组');return index.get(a);})];
 });
 return {groups:groups.map(g=>[g.members.map(n),n(g.high),n(g.low)]),fractals,endpoints,
  opposite:of('CC-008.new_bi_pair').map(p=>p.data),same:of('CC-009.same_kind').map(p=>p.data),
  bis:of('CC-010.bi').map(p=>p.data),relations:of('CC-055.relation').map(p=>p.data)};
}
function endpoint(e,expected){
 ok(e.extreme_roots.length===1,'实际候选根不唯一');
 const f=expected.fractals.find(f=>f[3]===n(e.extreme_roots[0]));ok(f,'候选端点没有冻结原始分型依据');
 eq([n(e.merged_index),e.kind,n(e.price)],[f[0],f[1],f[2]],'候选端点不符冻结分型');
 eq(n(e.group_anchor),expected.groups[f[0]][0][0],'候选组锚不符原始组');
 eq(n(e.raw_position),f[3],'候选原始位置不符实际根');
 eq(e.waiting_reasons,[],'已声明无同价域出现端点缺证');return f;
}
function condition(a,b,c,expected){
 const x=endpoint(a,expected),y=endpoint(b,expected);ok(x[1]!==y[1],'异型候选混入同型');
 const gap=y[0]-x[0],between=y[3]-x[3]-1;
 const top=x[1]==='TOP'?x[2]:y[2],bottom=x[1]==='BOTTOM'?x[2]:y[2];
 const bits=[gap>=3,between>=3,top>bottom];
 eq([n(c.merged_gap),n(c.raw_between_actual_extrema),n(c.top_price),n(c.bottom_price)],[gap,between,top,bottom],'完整条件原始数值不匹配');
 eq(c.vector,bits,'条件向量不匹配');eq(c.failed_conditions,labels.filter((_,i)=>!bits[i]),'失败位不完整');
 eq(c.waiting_reasons,[],'条件出现未定证据');return {pair:[x[3],y[3]],bits:bits.map(Number).join('')};
}
// 下面是冻结 Oracle formationAndExtensionExpectations 的具名端点断言，不实现状态机。
function selected(trace,prefix){
 const base=trace.replace('_REFLECTED','');
 if(['A','B','C','D'].includes(base)){
  if(base==='B'||base==='C')return [];
  return prefix>=(base==='A'?7:8)?[[1,base==='A'?5:6]]:[];
 }
 if(trace.includes('STRICT'))return [[1,prefix>=9&&(trace==='A_STRICT_HIGHER_TOP'||trace==='A_STRICT_LOWER_BOTTOM_REFLECTED')?7:5]];
 if(trace==='CORRECTION_WITHDRAW_TOP')return [[1,5]];
 if(trace==='APPEND_ABOVE_OLD_TOP'&&prefix>=17)return [[1,7],[7,11],[11,15]];
 return prefix>=13?[[1,7],[7,11]]:[[1,7]];
}
function projectionCheck(trace,prefix,snapshot,expected){
 const d=decoded(snapshot);eq(d.groups,expected.groups,'完整包含组/成员/高低点不匹配');eq(d.fractals,expected.fractals,'全部局部分型/实际极值根/窗口不匹配');
 const pairs=d.opposite.map(p=>{eq(p.waiting_reasons,[],'候选出现缺证');return condition(p.old,p.new,p.conditions,expected);});
 for(const p of d.same){
  const a=endpoint(p.old,expected),b=endpoint(p.new,expected);eq(a[1],b[1],'同型域混入异型');ok(a[2]!==b[2],'进入本叶外同价域');
  const stronger=a[1]==='TOP'?b[2]>a[2]:b[2]<a[2];
  eq(p.conditions,null,'同型不得伪装异型失败');eq(p.comparison,b[2]>a[2]?'GT':'LT','同型价格比较错误');
  eq(p.selection,stronger?'REPLACE':'KEEP','同型选择错误');eq(p.retained_anchors,[stronger?p.new.group_anchor:p.old.group_anchor],'同型保留锚错误');
 }
 const actual=d.bis.map(b=>{condition(b.start,b.end,b.formation,expected);eq(b.formation.vector,[true,true,true],'成笔非111');return [n(b.start.extreme_roots[0]),n(b.end.extreme_roots[0])];}).sort((a,b)=>a[0]-b[0]);
 eq(actual,selected(trace,prefix),'具名轨迹实际选中笔不符独立预期');
 if(trace.includes('STRICT')&&prefix===9)ok(d.same.length>0,'同型取舍证据缺失');
 if((trace==='B'||trace==='B_REFLECTED')&&prefix===6||(trace==='C'||trace==='C_REFLECTED')&&prefix===7)ok(pairs.length>0,'失败候选正路径缺失');
 for(const b of d.bis){
  const root=n(b.start.extreme_roots[0]);
  const expectConfirmed=root===1&&prefix>=14&&['RIGHT_GROUP_SEALED_COMPONENT','APPEND_ABOVE_OLD_TOP','CORRECTION_CONFIRMED_COMPONENT'].includes(trace);
  if(expectConfirmed){
   ok(b.confirmation!==null,'充分证书组件已具备的具名确认正路径缺失');eq(b.state,'CONFIRMED','确认状态错误');
   const c=b.confirmation;eq(n(c.successor_start),n(b.end.group_anchor),'证书未锚前笔末端');eq(n(c.successor_end),11,'具名后继证书末端错误');eq(n(c.right_group_sealed_at),13,'具名封口原始坐标错误');
   const start=d.endpoints.find(e=>e.group_anchor===c.successor_start),end=d.endpoints.find(e=>e.group_anchor===c.successor_end);
   ok(start&&end,'确认证书缺少同版本原始端点');condition(start,end,c.successor_conditions,expected);
   eq(c.successor_conditions.vector,[true,true,true],'后继三条件不全真');
   eq([...new Set(c.source_coords.map(n))].sort((a,b)=>a-b),[...new Set([...start.source_coords,...end.source_coords].map(n))].sort((a,b)=>a-b),'确认证书来源坐标不完整');
   ok(d.relations.some(r=>r.relation_kind==='confirms'&&r.target_id===b.entity_id&&r.source_id===`bi:${b.entity_id.split(':')[1]}:${c.successor_start}`),'确认关系与证书不一致');
  }
  if(b.confirmation===null)eq(b.state,'FORMED_UNCONFIRMED','无证书却报已确认');
 }
 const actualKeys=new Set(pairs.map(p=>p.pair.join(':')));
 return {vectors:[...new Set(pairs.map(p=>p.bits))].sort(),same_kind:d.same.length,selected_bis:actual,
  constructible_not_candidate:expected.pairs.filter(p=>!actualKeys.has(p.slice(0,2).join(':'))).map(p=>p.slice(0,2))};
}
function correctionCheck(trace,cs,oracle){
 const spec=oracle.correctionsAndHistory.find(t=>t.trace===trace);if(!spec)return null;
 const before=spec.sameRawCount,after=before+1,prior=oracle.traces.find(t=>t.id===spec.beforeTrace);
 const expected=prior.expectedPrefixCuts.find(c=>c.prefixCount===before).expected;
 const old=cs.filter(c=>n(c.candidate.state.snapshot.generation)===before);
 eq(old.length,3,'纠错会话缺少旧cut的live/AsKnown/事后重访');
 eq(old.filter(c=>c.command.client==='live').length,1,'纠错旧cut缺live');
 eq(old.filter(c=>c.command.mode==='AsKnown').length,2,'纠错旧cut缺两次AsKnown');
 ok(old.some(c=>c.command.id.startsWith('revisit-')),'纠错后未重访旧cut');
 for(const c of old)projectionCheck(spec.beforeTrace,before,c.candidate.state.snapshot,expected);
 const fresh=cs.filter(c=>n(c.candidate.state.snapshot.generation)===after);eq(fresh.length,3,'纠错新cut采样不完整');
 const oldData=decoded(old[0].candidate.state.snapshot),oldIds=oldData.bis.map(b=>b.entity_id).sort();
 for(const c of fresh){
  const s=c.candidate.state.snapshot,d=decoded(s);
  const revisions=s.raw_history.filter(r=>r.input_revision==='2'&&r.source_coord===String(spec.changedInput.rawOrdinal));
  eq(revisions.length,1,'新cut缺少唯一具名raw修订');const revision=revisions[0];
  eq([n(revision.high),n(revision.low)],spec.changedInput.afterHL,'修订原始高低点不匹配');
  function known(k){ok(k,'新版本缺首次获知');eq(n(k.generation),after,'新版本沿用旧首次获知cut');eq(n(k.input_frontier),after-1,'新版本获知输入前沿错误');eq(k.receipt_id,revision.receipt_id,'获知未绑定修订持久回执');eq(k.received_at,revision.received_at,'获知接收时刻不符修订');ok(/^\d+$/.test(k.semantic_commit_ns),'缺精确提交时钟');}
  for(const b of d.bis){eq(b.entity_id.split(':')[1],'2','纠错未开启新实体代际');ok(!oldIds.includes(b.entity_id),'新版本沿用旧实体');known(b.formed_known_at);if(b.confirmation!==null){known(b.confirmed_known_at);const oldBi=oldData.bis.find(o=>o.start.group_anchor===b.start.group_anchor);if(oldBi?.confirmed_known_at)ok(BigInt(b.confirmed_known_at.semantic_commit_ns)>BigInt(oldBi.confirmed_known_at.semantic_commit_ns),'新确认时钟未晚于旧确认');}}
  const changes=s.objects.filter(o=>o.kind==='CC-055.change_event').map(o=>o.payload.data);
  const withdrawals=changes.filter(e=>e.change==='withdrawn');eq(withdrawals.map(e=>e.entity_id).sort(),oldIds,'旧代际对象撤回不完整');
  for(const e of withdrawals){ok(e.causes.includes('raw_fact_revision'),'撤回缺修订因果');known(e.known_at);}
  const versions=s.objects.filter(o=>o.kind==='CC-056.version').map(o=>o.payload.data);eq(versions.length,1,'版本事实缺失');ok(versions[0].version_causes.includes('raw_fact_revision'),'版本原因未记录原始修订');known(versions[0].known_at);
 }
 return {trace,before_cut:before,after_cut:after,old_entity_ids:oldIds,status:'passed'};
}
function compare(planPath,output,traces=[]){
 const plan=read(planPath),planHash=sha(planPath);eq(sha(plan.oracle_path),plan.oracle_sha256,'Oracle发生漂移');
 const oracle=read(plan.oracle_path);eq(plan.oracle_sha256,'0450031d1201f64e1cfcd5f347a2597070e6c61f31f9376434382ac09245ac56','未使用签署独立Oracle');
 const wanted=traces.length?traces:Object.keys(plan.traces),errors=[],checked=[],runResults=[],corrections=[];
 eq(new Set(wanted).size,wanted.length,'重复trace选择');
 function attempt(context,fn){try{return fn();}catch(e){errors.push({...context,error:String(e.message).slice(0,500)});return null;}}
 for(const trace of wanted){
  const frozen=oracle.traces.find(t=>t.id===trace);ok(frozen,'未知trace');const runs=plan.runs.filter(r=>r.trace===trace);eq(runs.length,2,'缺少两arm计划');
  for(const run of runs)attempt({trace,arm:run.arm},()=>{
   const dir=run.directory,ev=path.join(dir,'evidence');const runtime=read(path.join(ev,'RESULT.json')),receipt=read(path.join(dir,'EXIT.json'));
   eq(receipt.exit,0,'运行未成功');eq(receipt.plan_sha256,planHash,'运行计划hash不符');
   for(const [file,key] of Object.entries({'config.json':'config_sha256','messages.jsonl':'messages_sha256','clock.json':'clock_file_sha256','prefix-map.json':'prefix_map_sha256','provenance.json':'provenance_sha256'}))eq(sha(path.join(dir,file)),run[key],'输入hash漂移:'+file);
   eq(runtime.status,'CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER','采集未完整');eq(runtime.cleanup.errors,[],'进程清理失败');
   eq(read(path.join(ev,'browser','RESULT.json')).status,'passed','浏览器未通过');
   const records=fs.readFileSync(path.join(ev,'semantic-core.jsonl'),'utf8').trim().split('\n').map(JSON.parse);
   const cs=records.filter(r=>r.kind==='public_candidate');
   // 同cut跨load/watch/AsKnown/revisit比较完整快照，仅去掉两个明确的查询视图标记。
   const known=new Map();for(const c of cs){const s={...c.candidate.state.snapshot};delete s.as_of_generation;delete s.history_mode;const cut=String(s.generation),bytes=canon(s);if(known.has(cut))eq(bytes,known.get(cut),'同cut完整公开事实在重访或模式切换中变化:'+cut);else known.set(cut,bytes);}
   for(const binding of plan.traces[trace].prefix_to_cut)attempt({trace,arm:run.arm,prefix:binding.prefix,cut:binding.cut},()=>{
    const expected=frozen.expectedPrefixCuts.find(x=>x.prefixCount===binding.prefix).expected;
    const matches=cs.filter(c=>n(c.candidate.state.snapshot.generation)===binding.cut);
    eq(matches.length,binding.prefix===0?2:3,'声明前缀缺少live/AsKnown/revisit采样');
    if(binding.prefix!==0){eq(matches.filter(c=>c.command.client==='live').length,1,'缺live采样');eq(matches.filter(c=>c.command.mode==='AsKnown').length,2,'缺AsKnown及重访');}
    const details=matches.map(c=>projectionCheck(trace,binding.prefix,c.candidate.state.snapshot,expected));
    checked.push({trace,arm:run.arm,...binding,candidates:matches.length,...details[0]});
   });
   const correction=correctionCheck(trace,cs,oracle);if(correction)corrections.push({...correction,arm:run.arm});
   runResults.push({trace,arm:run.arm,core_sha256:sha(path.join(ev,'semantic-core.jsonl')),candidate_count:cs.length});
  });
  attempt({trace},()=>{const pair=runResults.filter(r=>r.trace===trace);eq(pair.length,2,'缺有效双跑');eq(pair[0].core_sha256,pair[1].core_sha256,'两次新进程完整语义核心不相等');});
 }
 const full=wanted.length===17&&checked.length===164;
 const result={status:errors.length?'FAIL_INDEPENDENT_COMPARISON':full?'PASS_BOUNDED_17_TRACES_82_PREFIXES':'PASS_SELECTED_SUBSET',oracle_sha256:plan.oracle_sha256,plan_sha256:planHash,
  counts:{traces:wanted.length,checked_arm_prefixes:checked.length,declared_prefixes:checked.length/2,candidates:checked.reduce((n,r)=>n+r.candidates,0)},
  actual_candidate_vectors:[...new Set(checked.flatMap(x=>x.vectors))].sort(),runs:runResults,checked,corrections,errors,
  scope:'冻结原始Oracle的公开投影、实际候选条件、具名选取、确认组件、完整同cut重访和双进程重放；确认充分性另由源码证明和追加反例测试支撑。',
  not_covered:['真实SIGKILL/恢复矩阵','同价未定域及全部市场输入','S-4跨实现等价','完整#1392关票或#1323总图验收']};
 fs.writeFileSync(output,JSON.stringify(result,null,2)+'\n',{flag:'wx'});return result;
}
module.exports={decoded,projectionCheck,correctionCheck,compare};
if(require.main===module){const [plan,out,...traces]=process.argv.slice(2);const r=compare(path.resolve(plan),path.resolve(out||path.join(path.dirname(plan),'INDEPENDENT-COMPARISON.json')),traces);process.stdout.write(JSON.stringify({status:r.status,counts:r.counts,vectors:r.actual_candidate_vectors,errors:r.errors})+'\n');if(r.errors.length)process.exitCode=1;}
