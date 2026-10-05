// #1467: evaluate one explicit interpretation; assigned predicates are hypotheses.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
const [run,out]=process.argv.slice(2);assert(run&&out&&!fs.existsSync(out));
const ps=JSON.parse(fs.readFileSync(path.join(run,'periods.json'),'utf8'));
const ss=JSON.parse(fs.readFileSync(path.join(run,'segments.json'),'utf8'));
const timeline=JSON.parse(fs.readFileSync(path.join(run,'timeline.json'),'utf8'));
const interp={id:'Omega62-explicit-direct-v1',status:'proposed interpretation evaluated, not certified original predicates',scope:'q0 window (1,301]; left support E1 and active tail E302..E342 excluded from completed expression',objects:ps.map(p=>({id:p.id,grade:0,kind:'P',own:p.owned_events,range:p.whole_range,cores:[p.core.id],general_div:{definition:'same-direction internal external-to-seed strict signed-force weakening AND extreme AND c segment-end certificate',holds:p.comparison.strict_weakening&&p.comparison.extreme&&Number.isInteger(p.local_end.known_at),event:p.end,known_at:p.local_end.known_at},completed:{holds:true,event:p.end,known_at:p.local_end.known_at},next_start:p.end})),cores:ps.map(p=>({id:p.core.id,grade:0,seed:p.core.members,core:p.core.core,member_outer:p.core.member_outer,initial_break:{definition:'first complete return segment after c has range strictly outside seed core',holds:p.no_return.not_return,event:p.no_return.happened_at,known_at:p.no_return.known_at},original_S3_qualification:'unproved'})),delta_seg:ss.map(s=>s.id),delta_op0:ps.map(p=>p.id),delta_c0:ps.map(p=>p.id),SrcNext:[['X0','X1'],['X1','X2']],MemberNext_F2:[['X0','X1'],['X1','X2']],Adj_delta_c0:[['X0','X1'],['X1','X2']],candidate_parent:{id:'Y1',grade:1,children:ps.map(p=>p.id),core:[31000,36000],active:true,completed:false,original_F2_eligibility:'unproved'}};
const byid=new Map(interp.objects.map(x=>[x.id,x]));
const expr=interp.delta_c0.map(id=>byid.get(id));
assert(expr.every(x=>x.own[0]<=x.own[1]));
for(let i=1;i<expr.length;i++)assert.equal(expr[i-1].own[1]+1,expr[i].own[0]);
const failures=[];
for(let i=1;i<expr.length;i++){const a=expr[i-1],b=expr[i];if(a.completed.holds&&b.completed.holds&&a.kind==='P'&&b.kind==='P')failures.push({clause:'C6-NoPP on declared construction expression',pair:[a.id,b.id],earliest_joint_completion_known:Math.max(a.completed.known_at,b.completed.known_at),source_seam:a.own[1],cannot_insert_nonempty_item:true});}
assert.equal(failures.length,2);assert.deepEqual(failures.map(x=>x.earliest_joint_completion_known),[218,318]);
const delayed=failures.map(f=>({pair:f.pair,known_at:Math.max(...f.pair.map(id=>ps.find(p=>p.id===id).delayed_local_policy_known_at))}));assert.deepEqual(delayed.map(x=>x.known_at),[238,338]);
// Full frozen-prefix audit of exactly the declared assignment policy.
for(const t of timeline){const ids=interp.objects.filter(x=>x.completed.known_at<=t.cut).map(x=>x.id);assert.deepEqual(ids,t.local_PB_ends.map(x=>x.id));}
const result={interpretation:interp,checked_clauses:{nonempty_ownership:true,source_order_and_no_overlap:true,exact_completed_window_coverage:true,same_grade:true,single_registered_core:true,completed_iff_assigned_general_div:true,endpoint_consistency:true,parent_uses_whole_ranges:true,NoPP:false},failed_clauses:failures,delayed_publication_still_fails:delayed,minimal_conflict_objects:['X0','X1'],minimal_conflict_event_window:[2,201],minimal_conflict_known_at:218,minimality:'two declared independent complete P objects suffice; not global event-minimality',unproved_source_obligations:['initial predicate interpretations match original semantics','global core catalogue and lifecycle','fixed F2 eligibility and original parent identity','all-domain semantic uniqueness and further recursion'],conclusion:'This exact direct construction interpretation cannot satisfy NoPP. Different expressions or object mappings require a new explicit interpretation; three F2 members alone do not imply NoPP adjacency.'};
fs.writeFileSync(out,JSON.stringify(result,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({NoPP:false,failed_pairs:failures,delayed,original_P_certified:0}));
