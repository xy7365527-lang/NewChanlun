import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

// Independent finite review. Does not import, evaluate, or run the author script.
const O=path.dirname(fileURLToPath(import.meta.url));
const E='/Users/silencehan/Documents/Codex/research-evidence/issue1467';
const R='/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const A=E+'/stage67/base-role-author';
const P=R+'/.chanlun/review-results/issue1467-f1-proof/stage66';
const start=new Date().toISOString();
const read=p=>JSON.parse(fs.readFileSync(p,'utf8'));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const write=(p,v)=>fs.writeFileSync(path.join(O,p),JSON.stringify(v,null,2)+'\n');
const before=read(O+'/inputhash-before.json').files;
for(const f of before) assert.equal(sha(f.path),f.sha256,'frozen input '+f.path);
const manifest=read(A+'/manifest.json');
assert.equal(sha(A+'/manifest.json'),'678f261effaf7f23b45e97ccbb1fe5944b368f28993261dff39536ceda980ffd');
for(const f of manifest.files){assert.equal(sha(A+'/'+f.path),f.sha256);assert.equal(fs.statSync(A+'/'+f.path).size,f.bytes);}

const s=read(P+'/nested-author/run/source.json');
const pens=read(P+'/nested-review/independent-pens.json');
const certs=read(P+'/nested-review/independent-narrow-segments.json').certificates;
const obs=new Map(s.observations.map(o=>[o.index,o]));
const eventIds=s.events.map(e=>e.id);
const objects=read(P+'/nested-review/independent-objects.json');
const byId=new Map(objects.map(o=>[o.id,o]));
const members=['P0','P1','P2'].map(id=>byId.get(id));
const c=byId.get('c0');
const b=certs.find(c=>c.id==='S0');
const parent=read(P+'/nested-review/independent-parent.json');
const model=read(A+'/relation/model-or-obstruction.json');
const endpoint=p=>[obs.get(p.start).price,obs.get(p.end).price];
const direction=(a,z)=>obs.get(z).price>obs.get(a).price?'Up':'Down';
const whole=(a,z)=>{
  let lo=Infinity,hi=-Infinity;
  for(let k=a;k<=z;k++){const p=obs.get(k).price;lo=Math.min(lo,p);hi=Math.max(hi,p);}
  return [lo,hi];
};
const meet=rs=>rs.reduce(([lo,hi],[a,z])=>[Math.max(lo,a),Math.min(hi,z)],[-Infinity,Infinity]);
const penRange=p=>endpoint(p).sort((a,z)=>a-z);
const inSpan=(a,z)=>pens.map((p,i)=>({p,i})).filter(({p})=>p.start>=a&&p.end<=z);
const velocity=p=>{
 const [a,z]=endpoint(p);const num=BigInt(z)-BigInt(a),den=BigInt(obs.get(p.end).time-obs.get(p.start).time);
 assert(den>0n);assert.equal(num%den,0n,'these selected velocities are exact integers');return Number(num/den);
};
const force=(a,z)=>{const ps=inSpan(a,z);assert.equal(ps[0].p.start,a);assert.equal(ps.at(-1).p.end,z);const first=velocity(ps[0].p),last=velocity(ps.at(-1).p);return {first_pen:ps[0].i,last_pen:ps.at(-1).i,first,last,L:last-first};};
const bPens=inSpan(b.start,b.end),bf=force(b.start,b.end),cf=force(c.start,c.end);
const br=whole(b.start,b.end),mrs=members.map(o=>whole(o.start,o.end));
const core=meet(mrs),early=meet([br,...mrs.slice(0,2)]);
const features=b.first_selected.map(i=>penRange(pens[i]));
const rows=[];
function check(id,label,actual,expected,method,scope='finite arithmetic / chosen mapping only'){
 assert.deepEqual(actual,expected,label);rows.push({id,label,actual,expected,passed:true,method,scope});
}
// Checks follow the author's 29 claim slots, but each value is derived here.
check(1,'既有历史身份',s.name,'N66-low-prefix-nested-v1','Source identity additionally pinned to its Stage66 SHA-256.');
assert(s.observations.every((o,i)=>o.index===i&&o.time===i));assert(eventIds.every((id,i)=>id===i+1));
check(2,'观测及事件数',[obs.size,eventIds.length],[443,442],'Count keyed observations and event IDs; verify contiguous source clocks.');
for(const p of pens) assert.deepEqual(endpoint(p),[p.start_price,p.end_price],'frozen pen endpoints agree with raw observations');
check(3,'b实际边界',endpoint(b),[76000,40000],'Look up raw observations at the frozen S0 endpoints.');
check(4,'b整片价格区间',br,[40000,76000],'Scan every observation from index 1 through 21 inclusive.');
assert.deepEqual(bPens.map(x=>x.i),[0,1,2,3,4]);
check(5,'b局部证书字段',[b.start,b.end,b.known_at,b.members,b.own],[1,21,38,[0,5],[2,21]],'Read the previously audited S0 certificate; independently verify its pen support. 38 is inherited raw knowledge, not freshly re-proved minimal time.','audited certificate identity plus support check');
check(6,'b五笔方向',bPens.map(({p})=>endpoint(p)[1]>endpoint(p)[0]),[false,true,false,true,false],'Derive sign from observation endpoints, not stored up flags.');
check(7,'b力度',[bf.first,bf.last,bf.L],[-4000,-5000,-1000],'Exact integer quotient of raw price differences by the common observation time; subtract last minus first.');
assert.deepEqual(b.first_selected,[3,5,7]);
check(8,'b所选特征区间',features,[[46000,60000],[40000,48000],[44000,60000]],'Rebuild each selected frozen pen interval from raw observation prices.');
check(9,'067第一二特征无缺口',meet(features.slice(0,2)),[46000,48000],'Actual intersection is nonempty, stronger than checking the cached gap flag.');
check(10,'067底分型双坐标',features[1].every((v,j)=>v<features[0][j]&&v<features[2][j]),true,'Check lower low and lower high against both neighbors.');
check(11,'首三笔几何交集',meet(bPens.slice(0,3).map(({p})=>penRange(p))),[60000,68000],'Intersect three raw pen ranges; no original center role is inferred.');
for(let i=0;i<members.length;i++)assert.deepEqual(mrs[i],members[i].whole);
check(12,'所选K1三交',core,[62000,72000],'Recompute P0/P1/P2 wholes from observations, then intersect.');
check(13,'K1所选成员方向',members.map(o=>direction(o.start,o.end)),['Up','Down','Up'],'Derive member directions independently from raw start/end prices.');
check(14,'来源接续',[b.end,members[0].start,members[0].end,members[1].start,members[1].end,members[2].start,members[2].end,c.start],[21,21,121,121,221,221,321,321],'Use frozen object support endpoints; does not identify expression Adj with MemberNext.');
check(15,'b不在所选成员名单',model.objects.K1.selected_members.includes('b_low')||model.objects.K1.selected_members.includes('S0'),false,'Directly inspect this explicit finite mapping; exclusion is chosen, not a theorem about every source view.','candidate choice only');
const spans=[b,...members,c].map(o=>({id:o.id,own:o.own}));
const ownSet=o=>new Set(eventIds.filter(i=>i>=o.own[0]&&i<=o.own[1]));
const bOwn=ownSet(b),kOwn=new Set(members.flatMap(o=>[...ownSet(o)]));
check(16,'b与K1 Own不重计',[...bOwn].every(id=>!kOwn.has(id)),true,'Explicit event-ID sets, independent of interval inequality shortcut.');
check(17,'b与c0实际方向',[direction(b.start,b.end),direction(c.start,c.end)],['Down','Down'],'Derive both signs from observations.');
check(18,'b跨K1双边界',[obs.get(b.start).price>core[1],obs.get(b.end).price<core[0]],[true,true],'76000 > 72000 and 40000 < 62000.');
check(19,'c0跨K1双边界',[obs.get(c.start).price>core[1],obs.get(c.end).price<core[0]],[true,true],'72800 > 72000 and 37800 < 62000.');
check(20,'父比较臂力度',[cf.L,bf.L],[-3500,-1000],'Identify first/last pens by whole support: c0 indices 80/104, b indices 0/4; do not reuse parent stored force.');
check(21,'严格数值不等式',cf.L<bf.L,true,'Independent exact arithmetic: -3500 < -1000; no semantic divergence inference.');
const prior=whole(b.start,c.start)[0];
check(22,'父比较末端新低',[obs.get(c.end).price,prior,obs.get(c.end).price<prior],[37800,40000,true],'Scan all raw prices before c0 starts; compare final price.');
const allOwn=spans.flatMap(o=>[...ownSet(o)]);assert.equal(new Set(allOwn).size,allOwn.length);assert.deepEqual([...new Set(allOwn)].sort((a,z)=>a-z),eventIds.filter(i=>i>=2&&i<=421));
check(23,'Own分配计数',[bOwn.size,kOwn.size,ownSet(c).size],[20,300,100],'Enumerate all 420 event IDs, prove no duplicates and no holes within chosen parent support.');
check(24,'较早b/P0/P1三交',early,[62000,72000],'Intersect independently reconstructed raw wholes; eligibility remains conditional.');
check(25,'较早三元组与旧K1末端',[members[1].end,members[2].end],[221,321],'Read supports of P1 and P2; makes no claim of global first kernel.');
check(26,'旧局部证齐聚合',[Math.max(b.known_at,members[0].local_known_at,members[1].local_known_at),Math.max(...members.map(o=>o.local_known_at))],[238,338],'Max of frozen local clocks only. Does not grant transformed-role qualification or publication time.','inherited local certificates, not semantic clocks');
check(27,'c0非同向Zn',direction(c.start,c.end)===direction(members[0].start,members[0].end),false,'Down differs from selected K1 formation Up; intersection alone cannot restore old extension reasoning.');
check(28,'原义完成空值保持',[parent.original_completed,...objects.map(o=>o.original_completed)],[null,null,null,null,null],'Read Stage66 audited object nulls; separately verify Stage67 b GeneralDiv/Completed/role and parent semantic clocks below.','semantic status preservation, not falsehood');
for(const v of [model.objects.b_low.GeneralDiv_original,model.objects.b_low.Completed_original,model.objects.b_low.owns_original_minimum_core,model.relations.is_original_completed_entry_arm,model.relations.same_class_completed_root,model.relations.nearest_prior_original_arm,model.objects.K1.original_center_qualified,model.objects.parent.GeneralDiv_original,model.objects.parent.Completed_original,model.objects.parent.original_known_at,model.objects.parent.original_published_at])assert.equal(v,null);

const ticketSpecs=[
 [812,P+'/scope-author/issue812.json',['5148532848']],
 [865,E+'/stage64/completion-author/issue865.json',['5169396438','5169594298']],
 [873,E+'/stage64/completion-author/issue873.json',['5171403143','5171515876']],
];
const tickets=ticketSpecs.flatMap(([issue,file,ids])=>{
 const d=read(file);return ids.map(id=>{
  const cc=d.comments.filter(c=>c.url.endsWith('issuecomment-'+id));assert.equal(cc.length,1);
  return {issue,cache_path:file,cache_sha256:sha(file),live:false,url:cc[0].url,created_at:cc[0].createdAt,author:cc[0].author,body:cc[0].body};
 });
});
const authorSources=read(A+'/source-evidence.json');
for(const t of tickets){const at=authorSources.tickets.find(x=>x.url===t.url);assert(at);assert.equal(at.text,t.body);}
check(29,'五条决定性原票缓存',tickets.length,5,'Read original comment objects from frozen caches and compare exact bodies; source conclusions are assessed in review.md, not inferred from comment count.','cached original rulings, not live tracker status');

const lessonSpans=[['018',18,56],['035',16,26],['037',14,22],['057',26,36],['061',26,28],['064',34,34],['067',14,28],['083',14,20],['084',50,60]];
const sourceEvidence=[];
for(const [n,a,z] of lessonSpans){
 const file=R+`/docs/chanlun/text/blog/${n}-第${Number(n)}课.md`;const lines=fs.readFileSync(file,'utf8').split('\n');
 const bodyEnd=lines.findIndex(l=>l.includes('↑正文'))+1;
 const selected=[];
 for(let i=a;i<=z;i++){
  const raw=lines[i-1];if(!raw.trim())continue;
  const editorBlock=(n==='018'&&i>=46&&i<=48)||/^\s*[（(](?:注|娇)/.test(raw);
  const adopted=editorBlock?null:raw.replace(/[（(](?:娇(?:注)?|注)[：:][^）)]*[）)]/g,'');
  selected.push({line:i,raw,before_body_end:i<bodyEnd,editor_block:editorBlock,adopted_author_text:adopted,inline_editor_note_removed:adopted!==null&&adopted!==raw});
  assert(i<bodyEnd);
 }
 sourceEvidence.push({path:file,sha256:sha(file),authority:'canonical blog author text; note exclusions separately stated',body_end:bodyEnd,lines:selected});
}
const doctrineSpans=[['beichi',263,268],['beichi',309,338],['level_recursion',27,59],['level_recursion',91,112],['zhongshu',132,170]];
for(const [n,a,z] of doctrineSpans){const file=R+'/.chanlun/definitions/'+n+'.md';const ls=fs.readFileSync(file,'utf8').split('\n');sourceEvidence.push({path:file,sha256:sha(file),authority:'project settled doctrine; not author prose',lines:ls.slice(a-1,z).map((raw,i)=>({line:a+i,raw}))});}
// Direct source comparisons establish excerpt fidelity, not semantic completeness.
let excerptComparisons=0;
for(const e of authorSources.excerpts){
 const lines=fs.readFileSync(e.source,'utf8').split('\n');
 if(e.line){assert.equal(e.raw,lines[e.line-1]);assert(e.line<lines.findIndex(s=>s.includes('↑正文'))+1);}
 else assert.equal(e.text,lines.slice(e.start-1,e.end).join('\n'));
 excerptComparisons++;
}
write('source-direct-reading.json',{reader:'fresh independent Stage67 reviewer',live_tracker:false,sourceEvidence,tickets,author_excerpt_fidelity_comparisons:excerptComparisons,excluded:['MEMORY','ResearchProgress','author conversation','author helper conclusions','027:854/856'],note:'All semantic conclusions are in review.md. The full cached comments preserve superseded claims; only current specified clauses are adopted.'});
write('independent-checks.json',{status:'passed',check_count:rows.length,rows,supplementary:{all_pen_endpoints_match_raw:true,all_source_prices_reused:true,b_force:bf,c0_whole_force:cf,source_own_spans:spans,chosen_wholes:[br,...mrs,whole(c.start,c.end)],old_local_known_c0:c.local_known_at,original_semantic_nulls_preserved:true,author_excerpt_fidelity_comparisons:excerptComparisons},scope:{history_count:1,new_history_count:0,candidate_mapping_count:1,R_W_parser_reexecuted:false,source_model_proved:false,RootArm67_proved:false}});
const after=before.map(f=>({path:f.path,bytes:fs.statSync(f.path).size,sha256:sha(f.path)}));assert.deepEqual(after,before);write('inputhash-after.json',{phase:'after independent arithmetic and direct source capture',files:after,identical_to_before:true});
write('receipt.json',{status:'passed',started_at:start,finished_at:new Date().toISOString(),command:'node independent-check.mjs',independent:true,author_script_executed:false,check_count:rows.length,input_file_count:before.length,input_hashes_unchanged:true,history_count:1,new_history_count:0,candidate_mapping_count:1,R_W_reexecuted:false,live_tracker_requests:0,max_rss_bytes:process.resourceUsage().maxRSS*1024,output_budget_bytes:96*1024*1024,semantic_admission_pass:false});
console.log(JSON.stringify({check_count:rows.length,passed:true,b_force:bf,c0_force:cf,input_hashes_unchanged:true,max_rss_bytes:process.resourceUsage().maxRSS*1024},null,2));
