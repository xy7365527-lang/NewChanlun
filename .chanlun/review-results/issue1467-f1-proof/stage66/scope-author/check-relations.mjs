import fs from 'node:fs';
import crypto from 'node:crypto';
import path from 'node:path';
import assert from 'node:assert/strict';
const E = path.dirname(new URL(import.meta.url).pathname);
const B = '/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage65/local-domain-author/run';
const files = ['source.json', 'scan.json', 'timeline.json'];
const bytes = Object.fromEntries(files.map(f => [f, fs.readFileSync(path.join(B, f))]));
const hashes = Object.fromEntries(files.map(f => [f, crypto.createHash('sha256').update(bytes[f]).digest('hex')]));
const source = JSON.parse(bytes['source.json']);
const scan = JSON.parse(bytes['scan.json']);
const timeline = JSON.parse(bytes['timeline.json']);
const X = scan.objects;
const byId = Object.fromEntries(X.map(x => [x.id, x]));
const checks = [];
function verify(name, actual, expected = true) { assert.deepEqual(actual, expected, name); checks.push({name, pass: true}); }
function noPP(ids) { return ids.every((id, i) => i === 0 || byId[id].type !== 'P' || byId[ids[i-1]].type !== 'P'); }
function expandIntervals(intervals) { return intervals.flatMap(([a,b]) => Array.from({length:b-a+1}, (_,i) => a+i)); }
for (const x of X) {
  const prices = source.observations.slice(x.start, x.end + 1).map(o => o.price);
  verify(`${x.id}: actual whole range includes start/end`, [Math.min(...prices), Math.max(...prices)], x.whole_range);
  verify(`${x.id}: own event bounds`, x.own, [x.start+1, x.end]);
  verify(`${x.id}: actual local publication`, timeline[x.general_div.known_at].completed_candidates.includes(x.id));
  verify(`${x.id}: no earlier local publication`, timeline[x.general_div.known_at-1].completed_candidates.includes(x.id), false);
  verify(`${x.id}: source completion uncertified`, x.original_completed_certified, false);
}
const states = [218, 318, 342].map(t => {
  const available = X.filter(x => x.general_div.known_at <= t);
  const ids = available.map(x => x.id);
  const last = available.at(-1).end;
  const cover = [[1,1], ...available.map(x => x.own), ...(last < t ? [[last+1,t]] : [])];
  verify(`t${t}: complete raw prefix coverage without overlap`, expandIntervals(cover), Array.from({length:t}, (_,i)=>i+1));
  verify(`t${t}: declaration matches frozen candidate timeline`, ids, timeline[t].completed_candidates);
  verify(`t${t}: direct expression violates NoPP`, noPP(ids), false);
  verify(`t${t}: actual source completed list remains empty`, timeline[t].original_completed, []);
  return {
    cut:t,
    record_ids:ids,
    local_completed_ids:ids,
    source_completed_ids:[],
    source_completion_status:'not certified; local field is not substituted',
    raw_member_table_q0:ids,
    factor_input_direct_q0:ids,
    direct_delta:{ids, grades:available.map(x=>x.grade), Own:available.map(x=>x.own), NoPP:false},
    accepted_canonical_delta_q0:null,
    accepted_canonical_status:'unconstructed; not an empty accepted expression',
    raw_prefix_cover:{prelude:[[1,1]],completed_local:available.map(x=>x.own),physical_tail:last<t?[[last+1,t]]:[]},
    f2_material_cache:{member_refs:ids,status:ids.length<3?'two-member prefix; no three-member center':'three-member geometry only; input semantics and parent completion unproved',owns_events:false},
    parent_completed_ids:[],
    parent_frontier_replacement_permitted:false,
  };
});
const meet = [Math.max(...X.map(x=>x.whole_range[0])),Math.min(...X.map(x=>x.whole_range[1]))];
verify('actual strict triple whole intersection',meet,[62000,72000]);
verify('strict triple intersection nonempty',meet[0]<meet[1]);
verify('joined owned events',expandIntervals(X.map(x=>x.own)),Array.from({length:300},(_,i)=>i+2));
const whole = [Math.min(...X.map(x=>x.whole_range[0])),Math.max(...X.map(x=>x.whole_range[1]))];
verify('parent proposed whole union range',whole,[40000,72800]);
const symbolicParentState = {
  name:'S66-CERTIFIED-PARENT-CONDITIONAL-v1',
  instantiated:false,
  actual_source_certificate_count:0,
  missing:['original Complete(Xi) and grades/ownership','independent original parent completion over Own=(1,301]','original parent one-center catalogue if kind=P','consumer-authoritative canonical input bridge'],
  preconditions:{source_children:'independently certified X0,X1,X2 with unchanged Own',parent:'independent true parent certificate KappaW for this exact Whole; otherwise no W',clock:'tauW >= max(tauX0,tauX1,tauX2,tauKappaW), never set to 318 merely because children are locally visible'},
  record_ids:['X0','X1','X2','W'],
  W:{Own:[2,301],whole_range:whole,grade:1,kind:'P only if a complete one-center parent certificate is supplied',completed:'true only under KappaW',happened_at:301,known_at:'tauW'},
  parent_child_edges:[['W','X0'],['W','X1'],['W','X2']],
  raw_member_table_q0:['X0','X1','X2'],
  delta_same_q0:{ids:['X0','X1','X2'],NoPP:false},
  mixed_frontier:{ids:['W'],grades:[1],Own:[[2,301]],structural_NoPP:true,uniform_q0:false,acceptance:'conditional expression only; singleton does not certify F2/NoPP bridge'},
  q1_frontier:{ids:['W'],coverage:[2,301],grade:1},
  q0_exact_cover_claim:false,
  activity:{physical_tail:'(301,tauW] stays separate',pending_parent:null,completed_child_refs_not_active:true},
  preservation:'X0/X1/X2 unchanged; add W and edges, then replace a derived frontier at tauW, do not rewrite prior frontiers'
};
verify('immutable input bytes',Object.fromEntries(files.map(f => [f,crypto.createHash('sha256').update(fs.readFileSync(path.join(B,f))).digest('hex')])),hashes);
const output={claim:'S66-SCOPE-v1',input_hashes:hashes,original_records:X,actual_states:states,geometry:{triple_whole_intersection:meet,parent_union_whole:whole},conditional_parent_state:symbolicParentState,checks,check_count:checks.length,scope:'finite relation and interval self-check only; no new history, parser, true completion certificate, or independent review'};
fs.writeFileSync(path.join(E,'relation-structure.json'),JSON.stringify(output,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({checks:checks.length,pass:true,actual_cuts:states.map(s=>s.cut),actual_parent_completed:0,conditional_parent_instantiated:false}));
