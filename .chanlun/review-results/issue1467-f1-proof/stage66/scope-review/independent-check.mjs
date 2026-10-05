import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';

// 独立审查：不导入或执行作者检查器，也不运行 R_W/解析器。
const O = path.dirname(new URL(import.meta.url).pathname);
const E = '/Users/silencehan/Documents/Codex/research-evidence/issue1467';
const R = '/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const A = `${E}/stage66/scope-author`;
const B = `${E}/stage65/local-domain-author/run`;
const read = p => JSON.parse(fs.readFileSync(p, 'utf8'));
const sha = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const save = (name, value) => fs.writeFileSync(path.join(O, name), JSON.stringify(value, null, 2) + '\n', {flag:'wx'});
const before = read(`${O}/input-hashes-before.json`);
const source = read(`${B}/source.json`);
const scan = read(`${B}/scan.json`);
const timeline = read(`${B}/timeline.json`);
const relation = read(`${A}/relation-structure.json`);
const nested = read(`${E}/stage66/nested-author/source-supplement/actual-relations-v2.json`);
const basic = [], extra = [], sourceChecks = [];
function eq(group, name, actual, expected) {
  group.push({name, pass:isDeepStrictEqual(actual, expected), actual, expected});
}
function whole(x) {
  const prices = source.observations.filter(o => x.start <= o.index && o.index <= x.end).map(o => o.price);
  return [Math.min(...prices), Math.max(...prices)];
}
function exactCover(intervals, left, right) {
  const counts = new Map();
  for (const [a,b] of intervals) for (let j=a;j<=b;j++) counts.set(j, (counts.get(j)||0)+1);
  return counts.size === right-left+1 && [...counts].every(([j,n])=>left<=j && j<=right && n===1);
}
function directCheck(xs) {
  return xs.slice(1).every((x,i) => !(x.grade===xs[i].grade && x.type==='P' && xs[i].type==='P'));
}
for (const x of scan.objects) {
  eq(basic, `${x.id}: actual whole range includes start/end`, whole(x), x.whole_range);
  eq(basic, `${x.id}: own event bounds`, x.own, [x.start+1,x.end]);
  eq(basic, `${x.id}: actual local publication`, timeline.find(t=>t.cut===x.general_div.known_at).completed_candidates.includes(x.id), true);
  // 比作者的 known_at-1 更强：扫描冻结 timeline 全部早期项。
  eq(basic, `${x.id}: no earlier local publication`, timeline.filter(t=>t.cut<x.general_div.known_at).some(t=>t.completed_candidates.includes(x.id)), false);
  eq(basic, `${x.id}: source completion uncertified`, x.original_completed_certified, false);
}
const states = [];
for (const cut of [218,318,342]) {
  const t = timeline.find(t=>t.cut===cut);
  const xs = t.completed_candidates.map(id=>scan.objects.find(x=>x.id===id));
  const ids = xs.map(x=>x.id);
  const tail = [xs.at(-1).end+1, cut];
  const intervals = [[1,1], ...xs.map(x=>x.own), tail];
  eq(basic, `t${cut}: complete raw prefix coverage without overlap`, exactCover(intervals,1,cut), true);
  eq(basic, `t${cut}: declaration matches frozen candidate timeline`, scan.objects.filter(x=>x.general_div.known_at<=cut).map(x=>x.id), ids);
  eq(basic, `t${cut}: direct expression violates NoPP`, directCheck(xs), false);
  eq(basic, `t${cut}: actual source completed list remains empty`, t.original_completed, []);
  const author = relation.actual_states.find(t=>t.cut===cut);
  eq(extra, `t${cut}: Rec I0 and direct factors are exact frozen references`, [author.record_ids,author.local_completed_ids,author.raw_member_table_q0,author.factor_input_direct_q0,author.direct_delta.ids], Array(5).fill(ids));
  eq(extra, `t${cut}: direct grades and Own match frozen objects`, [author.direct_delta.grades,author.direct_delta.Own], [xs.map(x=>x.grade),xs.map(x=>x.own)]);
  eq(extra, `t${cut}: canonical unconstructed and no actual parent claim`, [author.accepted_canonical_delta_q0,author.parent_completed_ids,author.parent_frontier_replacement_permitted], [null,[],false]);
  eq(extra, `t${cut}: cache does not own children and raw tail is exact`, [author.f2_material_cache.member_refs,author.f2_material_cache.owns_events,author.raw_prefix_cover.physical_tail], [ids,false,[tail]]);
  states.push({cut, Rec:ids, Cert_original:t.original_completed, I0:ids, delta0:{ids,grades:xs.map(x=>x.grade),NoPP:directCheck(xs)}, N_certified:null, A:tail, Own:xs.map(x=>x.own), direct_adjacent_pairs:xs.slice(1).map((x,i)=>({pair:[xs[i].id,x.id],shared_boundary:x.start,touching:xs[i].end===x.start,same_grade:xs[i].grade===x.grade})), parent_completed:false});
}
const ranges = scan.objects.map(whole);
const intersection = [Math.max(...ranges.map(x=>x[0])),Math.min(...ranges.map(x=>x[1]))];
const envelope = [Math.min(...ranges.map(x=>x[0])),Math.max(...ranges.map(x=>x[1]))];
eq(basic, 'actual strict triple whole intersection', intersection, [62000,72000]);
eq(basic, 'strict triple intersection nonempty', intersection[0]<intersection[1], true);
eq(basic, 'joined owned events', exactCover(scan.objects.map(x=>x.own),2,301), true);
eq(basic, 'parent proposed whole union range', envelope, [40000,72800]);
eq(basic, 'immutable input bytes', before.inputs.filter(x=>x.path.startsWith(B+'/')).map(x=>sha(x.path)), before.inputs.filter(x=>x.path.startsWith(B+'/')).map(x=>x.sha256));
eq(extra, 'original records copied without mutation', relation.original_records, scan.objects);
eq(extra, 'all selected objects really are grade zero P candidates', scan.objects.map(x=>[x.grade,x.type,x.completed]), [[0,'P',true],[0,'P',true],[0,'P',true]]);
eq(extra, 'known clock is endpoint plus confirmation lag; not endpoint clock', scan.objects.map(x=>[x.end,x.general_div.happened_at,x.general_div.known_at]), [[101,101,118],[201,201,218],[301,301,318]]);
eq(extra, 'frozen observations have explicit sequential event indices', source.observations.every((x,i)=>x.index===i && x.time===i), true);
eq(extra, 'candidate publication persists through all later frozen prefixes', scan.objects.every(x=>timeline.filter(t=>t.cut>=x.general_div.known_at).every(t=>t.completed_candidates.includes(x.id))), true);
eq(extra, 'no original completion was supplied at any frozen prefix', timeline.every(t=>t.original_completed.length===0), true);
eq(extra, 'strict center needs alternating directions, satisfied geometrically here', scan.objects.map(x=>x.technical_direction), ['Up','Down','Up']);
eq(extra, 'conditional parent remains uninstantiated', [relation.conditional_parent_state.instantiated,relation.conditional_parent_state.actual_source_certificate_count], [false,0]);
eq(extra, 'conditional W is new q1 object over same owned interval, not q0', [relation.conditional_parent_state.W.grade,relation.conditional_parent_state.W.Own,relation.conditional_parent_state.W.whole_range,relation.conditional_parent_state.q0_exact_cover_claim], [1,[2,301],envelope,false]);
eq(extra, 'conditional parent time stays symbolic', relation.conditional_parent_state.W.known_at, 'tauW');
eq(extra, 'parent record addition preserves exact child IDs', relation.conditional_parent_state.record_ids, [...scan.objects.map(x=>x.id),'W']);
eq(extra, 'parent-child edges retained', relation.conditional_parent_state.parent_child_edges, scan.objects.map(x=>['W',x.id]));
eq(extra, 'all 32 independent checks map to author names', basic.map(x=>x.name), relation.checks.map(x=>x.name));
eq(extra, 'actual geometry agrees with packet', relation.geometry, {triple_whole_intersection:intersection,parent_union_whole:envelope});

const nd = nested.expressions.find(x=>x.id==='D1-nested-attempt');
const nc = nested.expressions.find(x=>x.id==='D0-direct-counterfactual');
eq(extra, 'D1 really is nested mixed-rank AST', nd.ast, {Seq:['b_low',{Core1:{members:['P0','P1','P2']}},'c0']});
eq(extra, 'D1 does not claim a NoPP pass', [nd.NoPP.applied,nd.NoPP.holds], [false,null]);
eq(extra, 'nested parent 438 remains local only', [nested.parent.candidate_known_at,nested.parent.original_known_at,nested.parent.original_completed,nested.parent.local_completion.typed_completed_original], [438,null,false,false]);
eq(extra, 'two histories use different first direct pair clock', [scan.objects[1].general_div.known_at,nc.NoPP.first_known_at], [218,238]);
eq(extra, 'two histories use different Own, never interchangeable', [scan.objects[0].own,nested.raw_initial_carrier.find(x=>x.id==='P0').own], [[2,101],[22,121]]);
eq(extra, 'nested Core1 does not discharge construction-role policy', nested.expressions.find(x=>x.id==='K1-member-arguments').canonical_adj, null);

const excerptResults = [];
for(const record of read(`${A}/source-evidence.json`).records) {
  const p = `${R}/${record.path}`, lines=fs.readFileSync(p,'utf8').split('\n');
  eq(sourceChecks, `${record.path}:${record.range}: file identity`, sha(p),record.sha256);
  const isBlog=record.path.startsWith('docs/chanlun/text/blog/');
  if(isBlog) eq(sourceChecks, `${record.path}:${record.range}: actual body boundary`, lines.findIndex(x=>x.includes('↑正文'))+1,record.body_end_line);
  for(const excerpt of record.excerpts) {
    const actual=lines[excerpt.line-1];
    eq(sourceChecks, `${record.path}:${excerpt.line}: text unchanged`,actual,excerpt.text);
    if(isBlog) {
      const marked=/[（(](?:娇注|注)[：:]|^注[：:]/.test(actual);
      eq(sourceChecks, `${record.path}:${excerpt.line}: note marker classification`,marked,excerpt.editor_note_marker_present);
      eq(sourceChecks, `${record.path}:${excerpt.line}: within body boundary`,excerpt.line<record.body_end_line,true);
      excerptResults.push({path:record.path,line:excerpt.line,text:actual,body_end_line:record.body_end_line,editor_note_present:marked,allowed_attribution:marked?'author text before inline annotation only':'author_body'});
    }
  }
}
for(const [n,id] of [[812,5147131478],[827,5153724096]]) {
  const live=read(`${O}/issue${n}-comment-live.json`);
  const frozen=read(`${A}/issue${n}.json`).comments.find(x=>x.url.endsWith(String(id)));
  eq(sourceChecks, `#${n}: exact original comment identity`, live.id,id);
  eq(sourceChecks, `#${n}: captured comment body matches live GitHub`, frozen.body,live.body);
  const decisive=fs.readFileSync(`${A}/issue${n}-decisive-comment.md`,'utf8').trim();
  eq(sourceChecks, `#${n}: decisive excerpt is whole matching comment`, decisive,`${live.html_url}\n\n${live.body}`.trim());
}
const authorManifest=read(`${A}/manifest.json`);
const manifestMismatches=[...authorManifest.inputs.map(x=>({path:x.path,expected:x.sha256})),...authorManifest.outputs.map(x=>({path:`${A}/${x.file}`,expected:x.sha256}))].filter(x=>sha(x.path)!==x.expected);
eq(extra,'author manifest input/output identity',manifestMismatches,[]);
eq(extra,'frozen Report SHA256',sha(`${A}/Report.md`),'76c0a8f99d864a7dc42abe0ffdb4f04f26a41bf702ce0b788007874e69790dce');
eq(extra,'frozen manifest SHA256',sha(`${A}/manifest.json`),'000c0984e3afdb5080379f67b3d198b51781f880866b1e2f67a13dfed0294df5');
const after={captured_at:new Date().toISOString(),inputs:before.inputs.map(x=>({...x,sha256:sha(x.path)}))};
eq(extra,'all declared input bytes unchanged across independent check',after.inputs.map(x=>x.sha256),before.inputs.map(x=>x.sha256));
save('input-hashes-after-check.json',after);
save('source-audit.json',{claim:'source attribution checks, not a mathematical proof',primary_excerpts:excerptResults,checks:sourceChecks});
save('independent-relations.json',{claim:'finite frozen relation audit; no parser run or new history',author_check_count:32,independent_basic_count:basic.length,actual_states:states,geometry:{intersection,envelope},clocks:{stage65:scan.objects.map(x=>({id:x.id,own:x.own,known_at:x.general_div.known_at})),nested:nested.raw_initial_carrier.filter(x=>x.completion).map(x=>({id:x.id,own:x.own,known_at:x.completion.known_at})),conditional_parent:'tauW remains symbolic; 438 belongs only to a different history and local candidate'},checks:basic,additional_checks:extra});
const all=[...basic,...extra,...sourceChecks];
const result={pass:all.every(x=>x.pass),basic_checks:basic.length,additional_checks:extra.length,source_checks:sourceChecks.length,total:all.length,failures:all.filter(x=>!x.pass)};
save('check-result.json',result);
console.log(JSON.stringify(result));
process.exitCode=result.pass?0:1;
