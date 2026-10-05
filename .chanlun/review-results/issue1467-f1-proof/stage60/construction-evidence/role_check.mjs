import fs from'node:fs';import path from'node:path';import assert from'node:assert/strict';
const [run,out]=process.argv.slice(2);if(!run||!out)throw Error('run and output required');
const checks=[];
for(const name of ['T60-main','N60-equal-force']){
 const x=JSON.parse(fs.readFileSync(path.join(run,name+'.certificate.json'))),r=JSON.parse(fs.readFileSync(path.join(run,name+'.reference.stdout')));
 const [lo,hi]=x.core.core,coreMembers=new Set(x.core.members);
 const rows=x.segments.map((s,i)=>({id:i,source:s.source.observations,up:s.up,core_member:coreMembers.has(i),crosses_lower:s.range[0]<lo&&s.range[1]>=lo,crosses_upper:s.range[0]<=hi&&s.range[1]>hi,eligible_external_role:!coreMembers.has(i)}));
 const geom=rows.filter(s=>s.id<4&&s.up&&(s.crosses_lower||s.crosses_upper));
 const role=geom.filter(s=>s.eligible_external_role);
 assert.deepEqual(geom.map(s=>s.id),[0,2]);assert.deepEqual(role.map(s=>s.id),[0]);
 const a=r.strokes[10],z=r.strokes[14],L2=(z.end_price-z.start_price)/4-(a.end_price-a.start_price)/4;assert.equal(L2,125);
 checks.push({name,contract:{delta:1,core_members:[1,2,3],external_left:[0],external_right:[4],source:'057:26,40 local analysis regrouping; beichi:265 excludes core construction members',not_claimed:'global earliest core, unique global seed, fixed F2 tower output'},rows,
 role_aware_previous:0,pure_geometric_previous:2,S2_L:L2,positive_comparison_against_S2:x.pair.force.c.L<L2,
 correction:'S2 does cross the lower boundary. Exclusion in this declared analysis view uses its actual core-member role, not no-crossing geometry. The earlier upper-boundary-only argument was insufficient.'});
}
fs.writeFileSync(out,JSON.stringify(checks,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(checks.map(x=>({name:x.name,role:x.role_aware_previous,pure_geometry:x.pure_geometric_previous,S2_L:x.S2_L}))));
