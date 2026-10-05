import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
const old='/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage60/construction/run-v2';
const dest=process.argv[2];
if(!dest||!path.isAbsolute(dest))throw Error('absolute destination required');
fs.mkdirSync(dest,{recursive:true});
const inputs=[];
const read=p=>{const b=fs.readFileSync(p);inputs.push({path:p,bytes:b.length,sha256:crypto.createHash('sha256').update(b).digest('hex')});return JSON.parse(b);};
const results=[];
for(const name of ['T60-main','N60-equal-force']){
 const source=read(path.join(old,`${name}.source.json`)), ref=read(path.join(old,`${name}.reference.stdout`)), cert=read(path.join(old,`${name}.certificate.json`));
 const seg=cert.segments, b=seg[0], c=seg[4], coreMembers=seg.slice(1,4);
 const K=[Math.max(...coreMembers.map(s=>s.range[0])),Math.min(...coreMembers.map(s=>s.range[1]))];
 assert.deepEqual(K,[12500,15000]);
 const crossesUpper=s=>s.start_price<K[1]&&s.end_price>K[1];
 assert(b.up&&c.up&&crossesUpper(b)&&crossesUpper(c));
 assert(c.end_price>b.end_price);
 const impulse=s=>{const[first,last]=[ref.strokes[s.members[0]],ref.strokes[s.members[1]-1]]; const v=u=>(u.end_price-u.start_price)/(source.observations[u.end].time-source.observations[u.start].time);return v(last)-v(first);};
 const Lb=impulse(b),Lc=impulse(c),strict=Lc<Lb;
 const kReady=Math.max(...coreMembers.map(s=>s.known_at)), endReady=c.known_at;
 const owned=[b.start+1,c.end], wholeObs=source.observations.slice(b.start,c.end+1);
 const whole=[Math.min(...wholeObs.map(x=>x.price)),Math.max(...wholeObs.map(x=>x.price))];
 const prefixMap=ref.prefixes.map(p=>({cut:p.cut,k_ready:p.cut>=kReady,segment_tail_certificate:p.cut>=endReady,
   end61PB:p.cut>=endReady&&strict,
   // No label alone certifies the original generalized divergence relation.
   confirmed_PanWitness_available:p.cut>=endReady&&strict,
   original_GeneralDiv_qualification:'unassigned',original_completed_move_qualification:'unassigned'}));
 const back=source.observations.slice(c.end+1).find(o=>K[0]<=o.price&&o.price<=K[1]);
 results.push({template:name,interpretation:'Omega61-PB-init',raw_prefix_records:ref.prefixes.length,
   delta:{local_start:1,leading_segment:'S0',core_seed:['S1','S2','S3'],exit_segment:'S4'},
   K,core_owned_events:[22,81],core_member_hull:[12000,16000],whole_owned_events:owned,whole_range:whole,
   PanWitness:{b:'S0',c:'S4',K:'S1/S2/S3',Lb,Lc,strict,extreme:true,source_role_exclusion:['S2:core member'],clock:'t_i=i'},
   end_happened_at:strict?101:null,end_known_at:strict?118:null,
   emitted_completed_init:strict?[{id:`${name}/PBX`,owned_events:owned,kind:'P',core:'K',successor_start:101}]:[],
   active_tail:strict?{start:101,owned_events:[102,122],first_segment:'S5',first_segment_complete:false}:{start:1,owned_events:[2,122],initial_kernel:'K'},
   known_return_to_K:back?{time:back.time,price:back.price}:null,
   source_third_point_certificate:null,number_of_candidate_children_recorded:strict?1:0,
   number_of_qualified_F2_children:0,fixed_F2_seed_witness:null,observed_NoPP_violation:null,prefixMap});
}
assert.deepEqual(results.map(x=>[x.end_happened_at,x.end_known_at,x.emitted_completed_init.length]),[[101,118,1],[null,null,0]]);
assert(results.every(x=>x.prefixMap.filter(p=>p.cut<118).every(p=>!p.end61PB)));
const result={status:'finite-dynamic-initial-cut-partial-model; not an original-completion certificate',inputs,results,
 conditional_obstruction:{assumptions:['all emitted cycles have exactly one same-level original kernel','all emitted cycles are accepted complete','identity and same-level direct adjacency are preserved into selected non-same-level construction role','that consumer obeys NoPP','a nonempty F2 triple of these cycles exists'],
 conclusion:'inconsistent: first two of that triple are an adjacent P/P pair',not_claimed:'an actual second P exists in either Stage60 history'}};
fs.writeFileSync(path.join(dest,'pb-initial-model.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({status:result.status,results:results.map(x=>({template:x.template,end:x.end_happened_at,known:x.end_known_at,whole:x.whole_range,Lb:x.PanWitness.Lb,Lc:x.PanWitness.Lc,outputs:x.emitted_completed_init.length,return:x.known_return_to_K}))},null,2));
