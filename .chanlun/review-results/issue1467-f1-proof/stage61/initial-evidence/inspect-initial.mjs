import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';

// New Stage61 object-mapping computation; does not run or change the Stage60 parser.
const root = '/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const old = '/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage60/construction/run-v2';
const out = process.argv[2];
if (!out || !path.isAbsolute(out)) throw new Error('Give a fresh absolute output directory');
fs.mkdirSync(out, { recursive: true });
const inputs = [];
const read = p => {
  const bytes = fs.readFileSync(p);
  inputs.push({ path: p, bytes: bytes.length, sha256: crypto.createHash('sha256').update(bytes).digest('hex') });
  return JSON.parse(bytes);
};
const eq = (a,b) => assert.deepEqual(a,b);
const range = s => [Math.min(s.start_price,s.end_price), Math.max(s.start_price,s.end_price)];
const meet = ss => [Math.max(...ss.map(s=>range(s)[0])), Math.min(...ss.map(s=>range(s)[1]))];
const cross = (s,c) => (Math.min(s.start_price,s.end_price)<c[0] && c[0]<Math.max(s.start_price,s.end_price)) ||
  (Math.min(s.start_price,s.end_price)<c[1] && c[1]<Math.max(s.start_price,s.end_price));
const all = [];
for (const name of ['T60-main','N60-equal-force']) {
  const source = read(path.join(old,`${name}.source.json`));
  const ref = read(path.join(old,`${name}.reference.stdout`));
  const cert = read(path.join(old,`${name}.certificate.json`));
  const rows = cert.segments.map((s,index)=>{
    const [a,b] = s.members, members = ref.strokes.slice(a,b);
    assert.equal(members.length,5);
    const own=[members[0].start+1,members.at(-1).end];
    eq(own,s.source.owned_events);
    const raw=source.observations.slice(members[0].start,members.at(-1).end+1);
    const whole=[Math.min(...raw.map(x=>x.price)),Math.max(...raw.map(x=>x.price))];
    eq(whole,s.range);
    for(const u of members){
      eq([source.observations[u.start].price,source.observations[u.end].price],[u.start_price,u.end_price]);
      assert.equal(source.observations[u.end].time-source.observations[u.start].time,4);
    }
    const seed=meet(members.slice(0,3));
    eq(seed,s.initial_three_core);
    assert(seed[0]<seed[1]);
    const windows=[0,1,2].map(j=>({relative_start:j,ids:[a+j,a+j+1,a+j+2],range:meet(members.slice(j,j+3))}));
    // Every same-direction pen outside the initial seed remains in the inventory;
    // no single-pen impulse argument is needed for the role obstruction.
    const outside=members.map((u,j)=>({u,id:a+j,seed:j<3})).filter(x=>!x.seed && x.u.up===s.up);
    const outsideSame=outside.map(x=>({id:x.id,range:range(x.u),strictly_crosses_seed:cross(x.u,seed)}));
    const first=members[0], last=members.at(-1);
    const v=u=>(u.end_price-u.start_price)/(source.observations[u.end].time-source.observations[u.start].time);
    const firstSeedStable=ref.prefixes.find(p=>p.stable_count>=a+3)?.cut ?? null;
    const wholeAtomsStable=ref.prefixes.find(p=>p.stable_count>=b)?.cut ?? null;
    const known=ref.prefixes.find(p=>p.cut===s.known_at);
    assert(s.first_selected.every(id=>known.stable_count>id));
    const closedLoop=[s.start,s.end];
    if(index>0)eq(closedLoop[0],cert.segments[index-1].end);
    return {
      object:`${name}/X${index}`,segment_source:s.id,seed_object:`${name}/C${index}`,
      proposed_kind:'P0 in I61 only; not accepted as original move',
      own_events:own,source_endpoints:closedLoop,whole_range:whole,
      segment_orientation:s.up?'up':'down',conceptual_P_direction:null,
      seed_pen_ids:[a,a+1,a+2],seed_range:seed,seed_owned_events:[first.start+1,members[2].end],
      seed_stable_known_at:firstSeedStable,whole_atoms_stable_at:wholeAtomsStable,
      end_happened_at:s.end,segment_end_known_at:s.known_at,
      segment_end_feature_ids:s.first_selected,segment_end_support_events:s.evidence_events,
      successor_start:s.end,successor_owned_events_from:s.end+1,
      impulse:v(last)-v(first),all_three_pen_windows:windows,
      seed_and_third_window_share_pen:a+2,
      outside_same_direction_pens:outsideSame,
      eligible_internal_D3_pair:outsideSame.filter(x=>x.strictly_crosses_seed).length>=2,
      post_seed_ranges:members.slice(3).map(range),
      post_seed_pair_both_strictly_outside:members.slice(3).every(u=>range(u)[0]>seed[1]) || members.slice(3).every(u=>range(u)[1]<seed[0]),
      inherited_dynamics_certificate:null,
      canonical_center_eligibility:'rejected-by-812-S5-S2-if-Chan-stroke-role-retained'
    };
  });
  const f2geometry=meet(cert.segments.slice(0,3).map(s=>({start_price:s.range[0],end_price:s.range[1]})));
  const cut114=ref.prefixes.find(p=>p.cut===114);
  const boundaryMap=ref.prefixes.map(p=>({cut:p.cut,emitted_segment_cycles:cert.segments.filter(s=>s.known_at<=p.cut).map(s=>s.id),active_from:cert.segments.filter(s=>s.known_at<=p.cut).at(-1)?.end??1}));
  all.push({template:name,rows,first_parent_geometry:f2geometry,parent_eligibility:false,
    prefix114:{stable_count:cut114.stable_count,needed_feature:27,feature_stable:cut114.stable_count>27,emitted_X4:false},
    finite_prefix_emit_map:boundaryMap,
    inherited_parent_level_comparison:{b:'X0',c:'X4',core_seed:'X1,X2,X3',core:cert.core.core,Lb:rows[0].impulse,Lc:rows[4].impulse,strict:rows[4].impulse<rows[0].impulse,justifies_own_core_completion:false}});
}
assert(all.every(x=>x.rows.every(r=>!r.eligible_internal_D3_pair)));
eq(all.map(x=>x.rows.map(r=>r.impulse)),[[250,-125,125,-125,-125],[250,-125,125,-125,250]]);
const result={status:'finite-new-object-mapping-checked; canonical-eligibility-rejected; no-completion-certification',
  algorithm_scope:'new arithmetic and ownership mapping on frozen Stage60 outputs; no old parser/Lean/control rerun',
  inputs,templates:all};
fs.writeFileSync(path.join(out,'initial-mapping.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({status:result.status,templates:all.map(x=>({name:x.template,rows:x.rows.length,cores:x.rows.map(r=>r.seed_range),impulses:x.rows.map(r=>r.impulse),parent:x.first_parent_geometry,cut114:x.prefix114}))},null,2));
