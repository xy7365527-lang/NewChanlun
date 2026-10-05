// Post-run diagnosis only; immutable history and admission predicates unchanged.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
const H='/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage64/dynamic-author';
const R='/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const run=path.join(H,'run-v2');
const src=JSON.parse(fs.readFileSync(path.join(run,'source.json')));
const ref=JSON.parse(fs.readFileSync(path.join(run,'reference.stdout')));
const segments=JSON.parse(fs.readFileSync(path.join(run,'segments.json')));
const prices=src.observations.map(x=>x.price);
const put=(n,x)=>fs.writeFileSync(path.join(H,n),JSON.stringify(x,null,2)+'\n',{flag:'wx'});
const range=p=>[Math.min(p.start_price,p.end_price),Math.max(p.start_price,p.end_price)];
const ix=rs=>[Math.max(...rs.map(r=>r[0])),Math.min(...rs.map(r=>r[1]))];
const hull=rs=>[Math.min(...rs.map(r=>r[0])),Math.max(...rs.map(r=>r[1]))];
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const diagnostics=[7,25].map(s=>{
 const ids=[5*s+1,5*s+3,5*s+5,5*s+7];const p=ids.map(j=>({...ref.strokes[j],pen:j,range:range(ref.strokes[j])}));const [a,b]=p.slice(1,3);assert.equal(a.range[1],b.range[1]);assert(b.range[0]<a.range[0]);
 let contained=b.range[0]<=a.range[0]&&a.range[1]<=b.range[1];assert(contained);
 const firstPairKnown=ref.prefixes.find(t=>t.stable_count>ids[2]).cut;
 return {candidate_segment:`S${s}`,direction:'Down',intended_own:[2+20*s,21+20*s],actual_pens:p,first_rejection_pair:[ids[1],ids[2]],previous_feature:a.range,next_feature:b.range,next_contains_previous:contained,strict_bottom_neck_test:`${b.range[1]} < ${a.range[1]}`,strict_bottom_neck:false,first_pair_known_at:firstPairKnown,original_R_W_pen_certificate:true,declared_five_pen_segment_certificate:false,reason:'fixed no-inclusion/no-gap certificate rejects containment; equality independently violates strict bottom-neck condition; no full segment semantics impossibility asserted'};
});
// Execute frozen front end as far as it has real admitted segment identities.
const byid=new Map(segments.map(s=>[s.id,s]));
const K0mem=[1,2,3].map(i=>byid.get(`S${i}`));assert(K0mem.every(Boolean));
const core0={id:'K0',grade:0,members:K0mem.map(s=>s.id),member_own:[K0mem[0].own[0],K0mem.at(-1).own[1]],core:ix(K0mem.map(s=>s.range)),initial_outer:hull(K0mem.map(s=>s.range)),known_at:K0mem.at(-1).known_at,lifecycle:'seed_frozen; no certified whole completion'};
const force=s=>{let ps=ref.strokes.slice(s.members[0],s.members[1]),v=p=>(p.end_price-p.start_price)/(p.end-p.start);return {L:v(ps.at(-1))-v(ps[0]),first_pen:s.members[0],last_pen:s.members[1]-1,first_v:v(ps[0]),last_v:v(ps.at(-1))};};
const a=byid.get('S0'),c=byid.get('S4'),fa=force(a),fc=force(c);
assert.equal(fa.L,100);assert.equal(fc.L,200);
const firstPTest={candidate:'X-active',b:'S0',c:'S4',b_force:fa,c_force:fc,strict_weakening:fc.L<fa.L,strict_extreme:prices[c.end]>Math.max(...prices.slice(a.start,c.start+1)),c_end:c.end,c_known_at:c.known_at,general_div:false,completed:false};
assert(!firstPTest.strict_weakening&&firstPTest.strict_extreme);
const actual={input_events:582,input_observations:583,declared_profile:'D64-035-conservative-v2',history_count:1,actual_R_W_strokes:144,stable_at_eof:ref.prefixes.at(-1).stable_count,certified_five_pen_slots:segments.map(s=>s.id),slot_certificate_failures:diagnostics,initial_centers:[core0],candidate_partial_second_center:{id:'K1-pending',attempted_members:['S5','S6','S7'],qualified_members:['S5','S6'],missing_member:'S7',core:null,completed_seed:false},firstPTest,active_container:{id:'X-active',grade:0,own:[2,582],start_state:1,end_state:582,range:[Math.min(...prices.slice(1)),Math.max(...prices.slice(1))],admitted_core_ids:['K0'],provisional_kind:'P-development-only',general_div:false,completed:false,status:'construction_failed_on_this_input; retained raw container, not accepted permanent-active solution'},general_div_relation:[],complete_relation:[],delta_c0:[],source_next:[],F2:{candidate_input_triples:[],certified_input_triples:[],produced_centers:[],produced_completed:[]},ownership:{support:[1,1],retained_failure_container:[2,582],event_partition_complete:true},first_substantive_failure:diagnostics[0]};
put('actual-assignment.json',actual);
// Counterfactual fully assigned arithmetic for the *predeclared* template slots;
// the two unqualified slots remain prominently false. Never passed to real F2.
const slots=src.rows.map((row,s)=>({id:`S${s}`,start:1+20*s,end:21+20*s,own:[2+20*s,21+20*s],up:s%2===0,range:[Math.min(...row),Math.max(...row)],members:[5*s,5*s+5],qualified:byid.has(`S${s}`),known_at:byid.get(`S${s}`)?.known_at??null}));
const cores=[],objects=[],firsttests=[];
for(let j=0;j<3;j++){
 const off=9*j,local=[];for(const k of [off+1,off+5]){let mem=slots.slice(k,k+3);let C={id:`K${cores.length}`,grade:0,members:mem.map(s=>s.id),member_own:[mem[0].own[0],mem.at(-1).own[1]],core:ix(mem.map(s=>s.range)),initial_outer:hull(mem.map(s=>s.range)),qualified:mem.every(s=>s.qualified),known_at:mem.every(s=>s.qualified)?Math.max(...mem.map(s=>s.known_at)):null};cores.push(C);local.push(C);}
 const a=slots[off],b=slots[off+4],c=slots[off+8],af=force(a),bf=force(b),cf=force(c);let up=a.up;let previous=prices.slice(a.start,c.start+1);let extreme=up?prices[c.end]>Math.max(...previous):prices[c.end]<Math.min(...previous);let sep=up?local[0].initial_outer[1]<local[1].initial_outer[0]:local[1].initial_outer[1]<local[0].initial_outer[0];
 assert(sep&&extreme&&cf.L<bf.L);assert(!(bf.L<af.L));
 firsttests.push({object:`X${j}`,candidate_P_force_a:af.L,candidate_P_force_c:bf.L,strict_weakening:false});
 objects.push({id:`X${j}`,grade:0,type:up?'U':'D',own:[a.own[0],c.own[1]],start:a.start,end:c.end,whole_range:hull(slots.slice(off,off+9).map(s=>s.range)),cores:local.map(x=>x.id),members:slots.slice(off,off+9).map(s=>s.id),segment_qualification:slots.slice(off,off+9).every(s=>s.qualified),initial_outer_separation:sep,b:b.id,c:c.id,Lb:bf.L,Lc:cf.L,strict_extreme:extreme,arithmetic_general_div_test:true,candidate_completed_if_all_structure_admitted:true,actual_completed:false,c_end_certificate_known_at:c.known_at});
}
const G=objects.slice(0,-1).map((x,i)=>{let last=cores.find(k=>k.id===x.cores.at(-1)),next=cores.find(k=>k.id===objects[i+1].cores[0]);let same=x.type==='U'?next.initial_outer[0]>last.initial_outer[1]:next.initial_outer[1]<last.initial_outer[0];assert(same);return{pair:[x.id,objects[i+1].id],global_catalog_successor:[last.id,next.id],last_outer:last.initial_outer,next_outer:next.initial_outer,same_direction:true,source_implies_old_lifecycle_successor:false,actual_contradiction:false,conditional_incompatibility:'only if all slots admitted AND frozen global-catalog-successor inheritance A64 imposed; not an original Next theorem'};});
const conditional={status:'counterfactual arithmetic only; not a second F1 or changed gate',objects,cores,first_single_core_tests:firsttests,delta_c0_if_admitted:['X0','X1','X2'],source_next_if_admitted:[['X0','X1'],['X1','X2']],NoPP_if_admitted:true,F2_if_admitted:{children:['X0','X1','X2'],whole_ranges:objects.map(x=>x.whole_range),core:ix(objects.map(x=>x.whole_range)),strict_core:true,grade:1,completed:false,admitted:false,actual_certified_input_count:0},global_successor_conditional_conflicts:G};put('conditional-template-assignment.json',conditional);
const lines={
'docs/chanlun/text/blog/035-第35课.md':[[12,24],[38,38]],
'docs/chanlun/text/blog/084-第84课.md':[[12,12],[50,60],[68,68]],
'docs/chanlun/text/blog/018-第18课.md':[[12,12],[24,32]],
'docs/chanlun/text/blog/037-第37课.md':[[12,22]],
'docs/chanlun/text/blog/033-第33课.md':[[12,12],[24,26]],
'docs/chanlun/text/blog/061-第61课.md':[[12,12],[26,28]],
'.chanlun/definitions/beichi.md':[[309,323],[336,370]],
'.chanlun/definitions/zhongshu.md':[[67,95]],
};
put('source-excerpts.json',Object.entries(lines).map(([n,ranges])=>{let p=path.join(R,n),ls=fs.readFileSync(p,'utf8').split('\n');return{path:p,sha256:hash(p),excerpts:ranges.flatMap(([a,b])=>ls.slice(a-1,b).map((text,i)=>({line:a+i,text})))};}));
const refs=[path.join(R,'.chanlun/review-results/issue1467-f1-proof/Stage63CertificateTransport.md'),path.join(R,'.chanlun/review-results/issue1467-f1-proof/stage63/independent-transport-review.md'),path.join(R,'.chanlun/review-results/issue1467-f1-proof/stage58/ScopeCorrections-v1.md'),path.join(R,'.chanlun/review-results/issue1467-f1-proof/strict_point_reference.rs'),path.join(R,'.chanlun/review-results/issue1467-f1-proof/stage62/model-evidence/reference_probe.rs'),path.join(R,'.chanlun/review-results/issue1467-f1-proof/stage62/model-evidence/build_check.mjs'),path.join(H,'../completion-author/Report.md')];put('reference-manifest.json',refs.map(p=>({path:p,sha256:hash(p)})));
console.log(JSON.stringify({actual_completed:0,F2_triples:0,first_P_test:[fa.L,fc.L],first_failure:diagnostics[0],counterfactual_templates:objects.map(x=>({id:x.id,type:x.type,range:x.whole_range,Lb:x.Lb,Lc:x.Lc,qualified:x.segment_qualification})),conditional_parent_core:conditional.F2_if_admitted.core}));
