import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
const here=process.argv[2] || path.dirname(new URL(import.meta.url).pathname);
if(process.argv[2]) fs.mkdirSync(here,{recursive:false});
const input='/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage62/run-v1';
const load=f=>JSON.parse(fs.readFileSync(path.join(input,f),'utf8'));
const source=load('T62-joint.source.json'), seg=load('segments.json'), periods=load('periods.json');
const obs=source.observations, price=i=>obs[i].price;
assert.equal(obs.length,343);assert.equal(seg.length,16);
const whole=(a,b)=>[Math.min(...obs.slice(a,b+1).map(x=>x.price)),Math.max(...obs.slice(a,b+1).map(x=>x.price))];
const hull=rs=>[Math.min(...rs.map(x=>x[0])),Math.max(...rs.map(x=>x[1]))];
const intersect=rs=>[Math.max(...rs.map(x=>x[0])),Math.min(...rs.map(x=>x[1]))];
const ref=load('T62-joint.reference.stdout');
const final=ref.prefixes.at(-1).stable;
const force=s=>{
 const first=final[s.members[0]], last=final[s.members[1]-1];
 const t0=first.start,t1=first.end,t2=last.start,t3=last.end;
 assert.equal(first.start_price,price(t0));assert.equal(first.end_price,price(t1));
 assert.equal(last.start_price,price(t2));assert.equal(last.end_price,price(t3));
 const v0=(price(t1)-price(t0))/(obs[t1].time-obs[t0].time);
 const v1=(price(t3)-price(t2))/(obs[t3].time-obs[t2].time);
 return {coordinates:[t0,t1,t2,t3],v0,v1,L:v1-v0};
};
const segments=seg.map(s=>{
 assert.deepEqual(s.range,whole(s.start,s.end));
 return {...s,force:force(s)};
});
for (const p of periods) {
 assert.deepEqual(p.whole_range,whole(p.start,p.end));
 assert.equal(segments[Number(p.comparison.b.slice(1))].force.L,p.comparison.b_force.L);
 assert.equal(segments[Number(p.comparison.c.slice(1))].force.L,p.comparison.c_force.L);
}
const kernels=periods.map(p=>{
 const members=p.core.members.map(x=>segments[Number(x.slice(1))]);
 const core=intersect(members.map(s=>s.range)), outer=hull(members.map(s=>s.range));
 assert.deepEqual(core,p.core.core);assert.deepEqual(outer,p.core.member_outer);
 return {id:p.core.id,members:p.core.members,core,member_outer:outer,known_at:p.core.known_at,grade:0,outer_status:'initial member envelope; DD/GG role conditional'};
});
const pair=(a,b)=>({a:a.id,b:b.id,strict_up:b.member_outer[0]>a.member_outer[1],strict_down:b.member_outer[1]<a.member_outer[0],outer_intersection:intersect([a.member_outer,b.member_outer]),core_up:b.core[0]>a.core[1],core_down:b.core[1]<a.core[0],expansion_if_original_roles:(b.core[1]<a.core[0]&&b.member_outer[1]>=a.member_outer[0])||(b.core[0]>a.core[1]&&b.member_outer[0]<=a.member_outer[1])});
const pairs=[pair(kernels[0],kernels[1]),pair(kernels[1],kernels[2]),pair(kernels[0],kernels[2])];
const windows=Array.from({length:14},(_,i)=>({members:segments.slice(i,i+3).map(x=>x.id),core:intersect(segments.slice(i,i+3).map(x=>x.range)),outer:hull(segments.slice(i,i+3).map(x=>x.range)),known_at:segments[i+2].known_at}));
const object=(id,start,end,ks,kind,certificate)=>({id,grade:0,proposed_kind:kind,Own:[start+1,end],whole:whole(start,end),start_price:price(start),end_price:price(end),kernels:ks,proposed_completed:true,accepted_completed:false,certificate,endpoint_at:end,known_at:certificate==='X2'?318:218});
const A=object('A01',1,201,['K0','K1'],'U','X1');
const B=object('B2',201,301,['K2'],'P','X2');
B.accepted_completed='local policy only; original/F2 unproved';
const assignment={id:'S63-reassociate-A01-B2-v1',objects:[A,B],kernels,delta_c0_attempt:['A01','B2'],Adj_delta_c0:[['A01','B2']],NoPP_on_proposed_tags:true,nonempty:true,source_owned:[2,301],source_excluded_left_support:[0,1],active_tail:{Own:[302,342],segments:['S15'],uncertified_remainder:[322,342],grade:0,completed:false},F2:{attempted_consumption:['A01','B2'],completed_same_grade_triple:false,first_rejected_object:'A01',rejection:'retained K0/K1 envelopes are not strictly separated; X1 local end does not certify A01',parent_core:null,parent_completed:false},first_failure:{predicate:'strict_up(K0,K1)',left:32000,operator:'>',right:32000,value:false,conditional_scope:'original DD/GG roles or this v1 initial member-envelope trend rule'}};
const partitions=[{blocks:[['X0'],['X1'],['X2']],delta:['X0','X1','X2'],failure:'NoPP at X0/X1'}, {blocks:[['X0','X1'],['X2']],delta:['A01','B2'],failure:'A01 retained cores fail strict same-grade trend relation'}, {blocks:[['X0'],['X1','X2']],delta:['X0','A12'],failure:'A12 retained cores fail strict same-grade trend relation'}, {blocks:[['X0','X1','X2']],delta:['A012'],failure:'three retained cores neither one-core P nor monotone strictly separated U/D'}];
const shifts=[181,201,221].map(cut=>{
 const leftLast=segments.find(s=>s.end===cut), rightFirst=segments.find(s=>s.start===cut);
 return {cut,left:{Own:[2,cut],whole:whole(1,cut),terminal_segment:leftLast.id,terminal_up:leftLast.up,endpoint:price(cut),up_endpoint_new_high:price(cut)===whole(1,cut)[1]},right:{Own:[cut+1,301],whole:whole(cut,301),first_segment:rightFirst.id,first_up:rightFirst.up,terminal_segment:'S14',terminal_up:true,same_direction_arms:rightFirst.up===true,first_arm_is_K2_member:kernels[2].members.includes(rightFirst.id)},old_certificate_transport:cut===201?'X1 endpoint preserved only for original X1; left aggregate still not certified':'old X1 event201 endpoint not equal new left endpoint; old X2 start201 not equal new right start'};
});
const parentCore=intersect(periods.map(p=>p.whole_range));
const lift={id:'Y012',Own:[2,301],grade:1,whole:whole(1,301),core:parentCore,core_members:['X0','X1','X2'],member_outer:hull(periods.map(p=>p.whole_range)),completed:false,active:true,borrowed_arms:['S0','S14'],arms_inside_members:true,old_X2_certificate_scope:[202,301],whole_scope:[2,301],right_return:{S15:segments[15].range,intersection_with_parent_core:intersect([parentCore,segments[15].range]),not_return:false},delta_c0_replacement_valid:false,F2_input:'conditional only: three source X are not authenticated construction children'};
assert.deepEqual(parentCore,[31000,36000]);assert.equal(pairs[0].strict_up,false);assert.equal(pairs[0].expansion_if_original_roles,true);assert.equal(shifts[0].left.up_endpoint_new_high,false);assert.equal(shifts[2].left.up_endpoint_new_high,false);
const out={scope:'same-source finite transport test, no general impossibility claim',counts:{events:342,segments:16,partitions:4,boundary_positions:3,windows:14},segments,kernels,pairs,assignment,partitions,shifts,windows,lift};
fs.writeFileSync(path.join(here,'result.json'),JSON.stringify(out,null,2)+'\n',{flag:'wx'});
const names=['T62-joint.source.json','segments.json','periods.json','T62-joint.reference.stdout'];
const manifest=names.map(f=>({path:path.join(input,f),sha256:crypto.createHash('sha256').update(fs.readFileSync(path.join(input,f))).digest('hex')}));
fs.writeFileSync(path.join(here,'input-manifest.json'),JSON.stringify(manifest,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({exit:'pass',meaning:'finite negative certificates reproduced',counts:out.counts,first_failure:assignment.first_failure,pairs,shifts,lift},null,2));
