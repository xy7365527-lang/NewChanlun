import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

const out = path.dirname(fileURLToPath(import.meta.url));
const R = '/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const P = `${R}/.chanlun/review-results/issue1467-f1-proof`;
const E = '/Users/silencehan/Documents/Codex/research-evidence/issue1467';
const began = new Date().toISOString();
const inputs = [
  `${R}/.agents/skills/rigorous-open-math-research/SKILL.md`,
  `${P}/GoalReframe-v4.md`, `${P}/Stage66NestedExpressionAndConsumerScope.md`,
  `${P}/stage66/adoption/ScopeCorrections-v2.md`, `${P}/stage66/nested-review/review.md`,
  `${P}/Stage61CompletionAndRankContracts.md`, `${P}/stage61/initial-completion-model.md`,
  `${P}/Stage64DynamicCandidatesAndCausalFrontier.md`,
  `${P}/stage66/nested-author/run/source.json`,
  ...['independent-parent.json','independent-objects.json','independent-pens.json',
      'independent-narrow-segments.json','H_b-evidence-versus-semantics.json'].map(f=>`${P}/stage66/nested-review/${f}`),
  `${P}/stage66/scope-author/issue812.json`,
  `${E}/stage64/completion-author/issue865.json`, `${E}/stage64/completion-author/issue873.json`,
  ...['018','035','037','057','061','064','067','083','084'].map(n=>`${R}/docs/chanlun/text/blog/${n}-第${Number(n)}课.md`),
  ...['beichi','level_recursion','zhongshu'].map(n=>`${R}/.chanlun/definitions/${n}.md`),
];
const sha = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const hashes = () => inputs.map(p=>({path:p, bytes:fs.statSync(p).size, sha256:sha(p)}));
const write = (p,v) => fs.writeFileSync(path.join(out,p), JSON.stringify(v,null,2)+'\n');
const before = hashes(); write('inputhash-before.json',{phase:'before finite author check; not before earlier read-only research',files:before});
const read = p=>JSON.parse(fs.readFileSync(p,'utf8'));
const source = read(`${P}/stage66/nested-author/run/source.json`);
const pens = read(`${P}/stage66/nested-review/independent-pens.json`);
const segs = read(`${P}/stage66/nested-review/independent-narrow-segments.json`).certificates;
const objs = read(`${P}/stage66/nested-review/independent-objects.json`);
const parent = read(`${P}/stage66/nested-review/independent-parent.json`);
const b = segs.find(x=>x.id==='S0');
const selected = objs.filter(x=>['P0','P1','P2'].includes(x.id));
const c = objs.find(x=>x.id==='c0');
const x = source.observations;
const intersect = rs=>[Math.max(...rs.map(r=>r[0])),Math.min(...rs.map(r=>r[1]))];
const checks=[];
function check(name, actual, expected) {assert.deepEqual(actual,expected,name);checks.push({name,actual,expected,passed:true});}
const prange=p=>[Math.min(p.start_price,p.end_price),Math.max(p.start_price,p.end_price)];
const vel=p=>(p.end_price-p.start_price)/(p.end-p.start);
const actualBRange=[Math.min(...x.slice(1,22).map(o=>o.price)),Math.max(...x.slice(1,22).map(o=>o.price))];
const core=intersect(selected.map(o=>o.whole));
const earlier=intersect([actualBRange,...selected.slice(0,2).map(o=>o.whole)]);
check('one existing history',source.name,'N66-low-prefix-nested-v1');
check('observation and event counts',[x.length,source.events.length],[443,442]);
check('b actual boundary',[x[1].price,x[21].price],[76000,40000]);
check('b whole from observations',actualBRange,[40000,76000]);
check('b certificate fields',[b.start,b.end,b.known_at,b.members,b.own],[1,21,38,[0,5],[2,21]]);
check('b five pen directions',pens.slice(0,5).map(p=>p.up),[false,true,false,true,false]);
check('b force from actual pens',[vel(pens[0]),vel(pens[4]),vel(pens[4])-vel(pens[0])],[-4000,-5000,-1000]);
check('b selected evidence intervals',b.first_selected.map(i=>prange(pens[i])),[[46000,60000],[40000,48000],[44000,60000]]);
const f=b.first_selected.map(i=>prange(pens[i]));
check('067 no gap',intersect(f.slice(0,2))[0]<=intersect(f.slice(0,2))[1],true);
check('067 bottom feature both coordinates',f[1][0]<f[0][0]&&f[1][0]<f[2][0]&&f[1][1]<f[0][1]&&f[1][1]<f[2][1],true);
check('b initial three pen overlap only a raw/class-like geometry',intersect(pens.slice(0,3).map(prange)),[60000,68000]);
check('selected K1 core',core,[62000,72000]);
check('K1 formation directions',selected.map(o=>o.direction),['Up','Down','Up']);
check('local source-contiguity',[b.end,selected[0].start,selected[0].end,selected[1].start,selected[1].end,selected[2].start,selected[2].end,c.start],[21,21,121,121,221,221,321,321]);
check('b outside selected member list',selected.some(o=>o.id==='b_low'||o.id==='S0'),false);
check('b own disjoint K1 own',b.own[1]<selected[0].own[0],true);
check('b same actual direction as c',[pens[0].up,c.direction],[false,'Down']);
check('b traverses both core boundaries',[x[1].price>core[1],x[21].price<core[0]],[true,true]);
check('c traverses both core boundaries',[x[321].price>core[1],x[421].price<core[0]],[true,true]);
check('parent force comparison',[vel(pens[104])-vel(pens[80]),parent.b_force.L],[-3500,-1000]);
check('parent strict numerical inequality',-3500 < -1000,true);
check('parent new low',x[421].price < Math.min(...x.slice(1,322).map(o=>o.price)),true);
check('source-own decomposition sizes',[b.own[1]-b.own[0]+1,selected.reduce((s,o)=>s+o.own[1]-o.own[0]+1,0),c.own[1]-c.own[0]+1],[20,300,100]);
check('earlier b P0 P1 triple',earlier,[62000,72000]);
check('earlier geometry end',[selected[1].end,selected[2].end],[221,321]);
check('old local times only',[Math.max(b.known_at,...selected.slice(0,2).map(o=>o.local_known_at)),Math.max(...selected.map(o=>o.local_known_at))],[238,338]);
check('no c as same-direction Zn',c.direction===selected[0].direction,false);
check('source original null preserved',[parent.original_completed,...objs.map(o=>o.original_completed)],[null,null,null,null,null]);

const model={
  name:'Omega67-closed-external', status:'finite partial role interpretation; source semantic admission unresolved',
  history:source.name, history_count:1, new_history_count:0, candidate_mapping_count:1,
  distinction:{raw_a0:'R_W pens and End067 raw segments; no original Movement completion assigned',lowest_a1:'P0/P1/P2/c0 are local lowest-whole candidates built from segments; original qualification remains null',project_a0:'the #812 smallest center, corresponding to a1 center in 084 notation, not raw input a0',below_minimum_view:'view-relative compression only; not a rank assignment',closed_observation:'fixed raw support and endpoint with source certificate',completed_original:'dynamic Movement completion under #865; not equated to closed observation'},
  objects:{b_low:{raw_type:'Raw067Segment',raw_rank:-1,original_rank:null,own:b.own,start_price:x[1].price,end_price:x[21].price,whole:actualBRange,raw_end:21,raw_known:38,closed_observation:true,owns_original_minimum_core:null,GeneralDiv_original:null,Completed_original:null,original_qualified_known_at:null,original_published_at:null,L:-1000},K1:{selected_members:selected.map(o=>o.id),core,formation_direction:'Up',member_local_evidence_by:338,original_center_qualified:null},parent:{expression:'Seq(b_low,Core1([P0,P1,P2]),c0)',own:[2,421],selected_entry:'b_low',selected_exit:'c0',GeneralDiv_original:null,Completed_original:null,original_known_at:null,original_published_at:null}},
  relations:{raw_entry_root:true,finite_raw_closed_root:true,same_class_completed_root:null,raw_before_core:true,source_disjoint_from_members:true,crosses_core_down:true,same_actual_direction_as_exit:true,selected_geometry_entry:true,nearest_prior_down_external_among_named_outer_factors:true,nearest_prior_original_arm:null,Member_q0_b_K1:false,Member_q0_b_K1_scope:'chosen raw mapping only; not universal source exclusion',is_original_completed_entry_arm:null},
  minimal_missing_bridge:{id:'RootArm67',statement:'For this declared below-minimum Raw067Segment, End067 plus external entry relation suffices for the already-completed b role in #865/#814, while it is not a Completed_original Movement and not a K1 q0 center member.',independent_geometry_and_closure_premises:true,source_proven:false,not_allowed_as_silent_axiom:true,why_not_derived:'067 supplies raw endpoint; 035/061 allow low entry/comparison roles; 057 supplies a viewpoint but not this typed closure bridge; #865 does not explicitly exempt or identify a raw closed root.'},
  exact_obstruction:{target:'RawEnd067(b,21;38) -> OriginalCompletedEntryArm(b,K1,c0)',result:'not established for the named model',if_role_requires_completed_movement:'no GeneralDiv certificate or original own center assigned; setting Completed_original=true from raw end changes #865 arrow',if_role_means_only_closed_observation:'finite mapping exists but identification with source completed-entry role remains RootArm67; replacing semantic target by closure is not proof',not_proven:'No universal nonexistence of F1, low arms, root interpretation, or full source model'},
  uniform_view_conditional:{is_second_candidate:false,purpose:'same candidate stress test under alternate justification',if_b_and_P0_P1_are_same_view_constructive_members:true,earlier_triple:['b_low','P0','P1'],core:earlier,geometry_end:221,planned_geometry_end:321,old_local_certificate_aggregate:238,original_qualification_known_at:null,first_global_core_proven:false,required_action:'recompute candidate kernel/member allocation consistently; cannot use uniform view to admit b then exclude b merely to preserve K1'},
  obligation_domain:{movement_completion:'only objects actually asserted as original Movement inherit #865 dynamic completion; not every raw input',raw_root:'closed observation root exists at 21/38 independently of original MotionCompleted',for_all_completed_requires_earlier_completed_same_class_b:'not asserted and not inferred from #865',policy827:'unchanged; MemberNext is not normalized Adj; no NoPP pass assigned'},
  finite_checks:checks,
};
write('relation/model-or-obstruction.json',model);

const selections={
 '018':[18,20,22,24,26,28,30,32,34,36,40,50,52,54,56],
 '035':[16,20,22,24], '037':[16,18,20], '057':[26,28,30,32],
 '061':[26], '064':[34], '067':[18,20,22,24,28], '083':[14,16], '084':[52,54,56],
};
const excerpts=[];
for(const [lesson,lines] of Object.entries(selections)){
 const p=`${R}/docs/chanlun/text/blog/${lesson}-第${Number(lesson)}课.md`;
 const all=fs.readFileSync(p,'utf8').split('\n'); const boundary=all.findIndex(s=>s.includes('↑正文'))+1;
 for(const line of lines){
  const raw=all[line-1]; assert(line<boundary,`${lesson}:${line} before body delimiter`);
  const marker=raw.search(/[（(]娇(?:注)?[：:]/);
  const adopted=marker>=0?raw.slice(0,marker).trim():raw;
  const beginsAnnotation=/^\s*[（(](?:注|娇)/.test(raw);
  assert(!beginsAnnotation,`${lesson}:${line} editor-leading annotation`);
  excerpts.push({source:p,line,raw,adopted,authority:'author primary text; checked body boundary and annotation scope',body_boundary:boundary,inline_editor_note_excluded:marker>=0});
 }
}
for(const [name,start,end] of [['beichi',263,268],['beichi',309,338],['level_recursion',27,59],['level_recursion',91,112],['zhongshu',132,170]]){
 const p=`${R}/.chanlun/definitions/${name}.md`,all=fs.readFileSync(p,'utf8').split('\n');
 excerpts.push({source:p,start,end,text:all.slice(start-1,end).join('\n'),authority:'project settled doctrine; later rulings govern old implementation notation; not author quotation'});
}
const tickets=[];
for(const [n,p,ids] of [[812,`${P}/stage66/scope-author/issue812.json`,['5148532848']], [865,`${E}/stage64/completion-author/issue865.json`,['5169396438','5169594298']], [873,`${E}/stage64/completion-author/issue873.json`,['5171403143','5171515876']]]){
 const d=read(p);
 for(const comment of d.comments.filter(c=>ids.some(id=>(c.url||'').includes(id)))) tickets.push({issue:n,cache:p,url:comment.url,text:comment.body,authority:'cached original project ruling, not new live status; later force and ratio revisions apply, completion arrow retained; full old quote is not wholly current doctrine'});
}
check('all requested decisive cached comments found',tickets.length,5);
write('source-evidence.json',{excerpts,tickets,excluded:['027:854/856','reader text after body boundaries','inline Jia editor notes'],source_layer_rule:'author primary semantics > settled project applied contract > historical research claims; early implementation aliases are not Completed proofs'});
fs.writeFileSync(path.join(out,'source-excerpts.md'),'# Stage67 来源摘录与等级\n\n'+excerpts.map(e=>`## ${e.source}:${e.line??`${e.start}-${e.end}`}\n\n等级：${e.authority}\n\n${e.adopted??e.text}\n${e.inline_editor_note_excluded?'\n已排除同行娇注；完整原行见source-evidence.json。\n':''}`).join('\n')+'\n'+tickets.map(t=>`## #${t.issue} 原票裁定\n\n${t.url}\n\n${t.text}`).join('\n'));
const after=hashes(); assert.deepEqual(after,before,'all frozen inputs unchanged');write('inputhash-after.json',{files:after,identical_to_before:true});
write('relation/model-or-obstruction.json',model);
const receipt={started_at:began,finished_at:new Date().toISOString(),command:'node check.mjs',check_count:checks.length,passed:true,input_files:inputs.length,inputhash_unchanged:true,new_histories:0,candidate_mappings:1,lean_environments_created:0,external_network_requests:0,scope:'author finite arithmetic/source extraction checks only; not original semantic proof or independent review',max_rss_bytes:process.resourceUsage().maxRSS*1024,output_budget_bytes:96*1024*1024};
write('receipt.json',receipt);
console.log(JSON.stringify(receipt,null,2));
