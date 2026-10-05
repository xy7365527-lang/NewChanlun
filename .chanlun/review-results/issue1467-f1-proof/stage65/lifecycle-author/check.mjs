import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

// Bounded analysis of frozen Stage64 H2; no parser, new history, or source edits.
const here = path.dirname(new URL(import.meta.url).pathname);
const R = '/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const E = '/Users/silencehan/Documents/Codex/research-evidence/issue1467';
const input = path.join(E, 'stage64/dynamic-author/v2/run');
const out = path.join(here, 'run');
fs.mkdirSync(out, {recursive: true});
const load = name => JSON.parse(fs.readFileSync(path.join(input, name), 'utf8'));
const write = (name, value) => fs.writeFileSync(path.join(out, name), JSON.stringify(value, null, 2) + '\n');
const hash = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
let checks = 0;
function check(test, label) { checks++; if (!test) throw Error(label); }
const source = load('source.json');
const segments = load('segments.json');
const reference = load('reference.stdout'); // Read an already reviewed result; never run R_W.
const old = load('result.json');
const S = Object.fromEntries(segments.map(s => [s.id, s]));
const price = i => source.observations[i].price;
const segIndex = id => Number(id.slice(1));
const same = (a,b) => JSON.stringify(a) === JSON.stringify(b);
check(source.observations.length === 583 && segments.length === 28, 'H2 input bounds');
check(reference.strokes.length === 144, 'frozen reference bounds');

// Only the six declared nucleus member sets are inherited. Geometry and polarity
// are reconstructed from their frozen segment material, without candidate Bool.
const kernels = old.initial_construction.cores.map(k => {
  const m = k.members.map(id => S[id]);
  const core = [Math.max(...m.map(x=>x.range[0])), Math.min(...m.map(x=>x.range[1]))];
  const outer = [Math.min(...m.map(x=>x.range[0])), Math.max(...m.map(x=>x.range[1]))];
  check(m.length === 3 && m[0].up !== m[1].up && m[1].up !== m[2].up, k.id+' alternating');
  check(m[0].end === m[1].start && m[1].end === m[2].start, k.id+' material contiguous');
  check(core[0] < core[1] && same(core,k.core) && same(outer,k.initial_outer), k.id+' geometry');
  const memberOwn = [m[0].start+1,m[2].end];
  check(same(memberOwn,k.member_own), k.id+' owned material');
  return {id:k.id,members:k.members,memberOwn,core,outer,
    seedHappenedAt:m[2].end,knownAt:Math.max(...m.map(x=>x.known_at)),
    polarity:m[0].up?'D':'U',pattern:m.map(x=>x.up?'U':'D').join('')};
});
const K = Object.fromEntries(kernels.map(k=>[k.id,k]));
const edgeDirection = (a,b) => b.outer[0]>a.outer[1]?'U':b.outer[1]<a.outer[0]?'D':'O';
const catalog = kernels.slice(1).map((k,i)=>({from:kernels[i].id,to:k.id,
  direction:edgeDirection(kernels[i],k),knownAt:k.knownAt}));

// M65-pol: maximal runs of the intrinsic three-segment polarity. A closed
// run needs an observed opposite-polarity nucleus and an exact two-segment
// connector. No Div, Completed, old object end, or old Owner is an input.
function polarityMap(cut) {
  const visible = kernels.filter(k=>k.knownAt<=cut);
  const blocks=[];
  for (const k of visible) {
    if (!blocks.length || blocks.at(-1).polarity!==k.polarity)
      blocks.push({id:'Y'+blocks.length,polarity:k.polarity,cores:[]});
    blocks.at(-1).cores.push(k.id);
  }
  let start=1;
  const closed=[], next=[], owner=[];
  for (let i=0;i<blocks.length;i++) {
    const b=blocks[i];b.start=start;
    b.cores.forEach(id=>owner.push({kernel:id,owner:b.id}));
    b.withinMoveNext=b.cores.slice(1).map((id,j)=>[b.cores[j],id]);
    b.type=b.cores.length===1?'P':b.polarity;
    check(b.withinMoveNext.every(([a,c])=>edgeDirection(K[a],K[c])===b.polarity), 'polarity run price direction');
    if (i+1<blocks.length) {
      const a=K[b.cores.at(-1)], c=K[blocks[i+1].cores[0]];
      const left=segIndex(a.members.at(-1))+1, right=segIndex(c.members[0]);
      const bridge=segments.slice(left,right);
      check(bridge.length===2, 'polarity boundary has exact two-segment bridge');
      check(bridge[0].up===(b.polarity==='U') && bridge[1].up!==(b.polarity==='U'), 'bridge directions');
      check(bridge[0].start===a.memberOwn[1] && bridge[0].end===bridge[1].start && bridge[1].end+1===c.memberOwn[0], 'bridge connectivity');
      const end=bridge[0].end;
      const knownAt=Math.max(c.knownAt,...bridge.map(s=>s.known_at));
      check(knownAt<=cut && end>start, 'causal strict boundary');
      b.end=end;b.own=[start+1,end];b.closedKnownAt=knownAt;
      b.bridge=bridge.map(s=>s.id);
      check(b.cores.every(id=>start<K[id].memberOwn[0] && K[id].memberOwn[1]<=end), 'closed polarity material containment');
      closed.push({id:b.id,start,end,own:b.own,cores:b.cores,type:b.type,knownAt,bridge:b.bridge});
      next.push({from:b.id,to:blocks[i+1].id,cut:end,knownAt});
      start=end;
    } else { b.end=null;b.buffer=[start+1,cut]; }
  }
  return {cut,blocks,closed,owner,sourceNext:next,buffer:[start+1,cut]};
}

const timeline=[];
let previous=[];
for(let cut=0;cut<=582;cut++) {
  const m=polarityMap(cut);
  check(same(m.closed.slice(0,previous.length),previous),'closed records retained '+cut);
  if(cut>=1) {
    const owned=m.closed.flatMap(x=>Array.from({length:x.end-x.start},(_,j)=>x.start+j+1));
    const tail=Array.from({length:Math.max(0,cut-m.buffer[0]+1)},(_,j)=>m.buffer[0]+j);
    check(same([...owned,...tail],Array.from({length:cut-1},(_,j)=>j+2)),'closed own plus buffer coverage '+cut);
  }
  previous=m.closed;
  timeline.push(m);
}
const pol=timeline.at(-1);
check(same(pol.owner.map(x=>x.owner),['Y0','Y0','Y1','Y1','Y2','Y2']),'six-kernel polarity mapping');
check(same(pol.closed.map(x=>[x.end,x.knownAt]),[[181,278],[361,458]]),'polarity closure timing');

// M65-force: recompute the six declared comparisons from frozen actual strokes.
// This verifies a concrete sequential reset, not an original GeneralDiv role.
const forces=[];
function force(id) {
  const s=S[id], pens=reference.strokes.slice(...s.members);
  check(pens.length===5 && pens[0].start===s.start && pens.at(-1).end===s.end,id+' stroke cover');
  const v=p=>(p.end_price-p.start_price)/(p.end-p.start);
  return {first:v(pens[0]),last:v(pens.at(-1)),L:v(pens.at(-1))-v(pens[0]),pens};
}
let resetStart=1;
for(const a of old.attempts) {
  const b=force(a.b), c=force(a.c), s=S[a.c];
  const prior=source.observations.slice(resetStart,s.start+1).map(x=>x.price);
  const priorExtreme=s.up?Math.max(...prior):Math.min(...prior);
  const extreme=s.up?price(s.end)>priorExtreme:price(s.end)<priorExtreme;
  const weak=c.L<b.L;
  const t=s.known_at;
  const stable=reference.prefixes[t].stable;
  check([...b.pens,...c.pens].every(p=>stable.some(v=>same(p,v))),'force stable input '+a.c);
  check(a.cores.every(id=>K[id].knownAt<=t),'force known kernels '+a.c);
  check(b.L===a.b_force.L && c.L===a.c_force.L && weak===a.strict_weakening && extreme===a.strict_extreme,'recomputed force '+a.c);
  const item={start:resetStart,b:a.b,c:a.c,cores:a.cores,Lb:b.L,Lc:c.L,weak,extreme,
    end:s.end,knownAt:t,accept:weak&&extreme,semanticDivCertified:false};
  forces.push(item);
  if(item.accept)resetStart=item.end;
}
const resets=forces.filter(x=>x.accept).map((x,i)=>({...x,id:'X'+i,own:[x.start+1,x.end]}));
check(same(resets.map(x=>[x.end,x.knownAt]),[[181,198],[361,378],[541,558]]),'actual force-reset timing');

// Non-tautological *finite* alignment check: polarity ownership is built without
// force flags; now test same-instance future nuclei on this single frozen H2.
const finiteNE=resets.map((x,i)=>({candidate:x.id,polarityOwner:'Y'+i,tau:x.end,
  observedThrough:582,ownedFutureKernels:pol.owner.filter(o=>o.owner==='Y'+i && K[o.kernel].seedHappenedAt>x.end).map(o=>o.kernel),
  completeFutureQuantification:false,semanticOwnerCertified:false,
  sourceBoundaryAvailableAtForcePublication:timeline[x.knownAt].closed.some(z=>z.start===x.start&&z.end===x.end),
  sourceClosureAt:pol.closed.find(z=>z.start===x.start)?.knownAt??null}));
check(finiteNE.every(x=>x.ownedFutureKernels.length===0),'H2 finite independent-owner no extension');

// M65-mono: disjoint maximal runs of matching outer displacement.
// On a flip, its new nucleus starts a fresh run (the turn nucleus is not shared).
const mono=[];
for(const k of kernels) {
  if(!mono.length){mono.push({id:'M0',direction:null,cores:[k.id]});continue;}
  const b=mono.at(-1),d=edgeDirection(K[b.cores.at(-1)],k);
  if(d==='O')throw Error('M65-mono outside declared separated-core domain');
  if(b.direction===null || d===b.direction){b.direction=d;b.cores.push(k.id);}
  else mono.push({id:'M'+mono.length,direction:null,cores:[k.id]});
}
const first=resets[0], m0=mono[0];
const escaped=m0.cores.map(id=>K[id]).filter(k=>k.memberOwn[1]>first.end);
check(same(m0.cores,['K0','K1','K2']) && escaped.length===1 && escaped[0].id==='K2','mono containment counterexample');
const failures={
  primary:{id:'M65-mono-certificate-transport',firstKnownAt:278,
    contract:'Transport X0 completed record unchanged to M0, with every owned nucleus member event contained in its Whole Own',
    owner:m0,transportedEnd:181,transportedOwn:[2,181],
    escapingKernel:'K2',escapingMaterial:[202,261],firstEscapingEvent:202,
    impossibleInequality:'261 <= 181',
    minimumMaterialCoveringEnd:261,
    inheritedCertificateC:'S8',inheritedCertificateEndpoint:181,
    endpointsBeforeFirstUnownedKernel:[261,281].map(u=>({u,price:price(u),priorObservedHigh:80000,certCMatches:u===181})),
    scope:'Refutes this certificate-preserving transport, not all monotone lifecycle models or all F1'},
  timing:{id:'M65-pol-early-publication',firstKnownAt:198,
    contract:'M65-pol closure certificate exists when the first force-reset is published',
    claimedCandidate:'X0',forcePublishedAt:198,polarityClosureAt:278,
    closedAt198:timeline[198].closed,scope:'False for this concrete closure algorithm; does not disprove a semantic end at 181'},
  f2:{at558:timeline[558].closed.length,atEOF:pol.closed.length,
    requiredCompletedChildren:3,geometricallyClosedTriples:0,semanticCompletedChildren:0,
    pendingThird:'Y2; S27 is a raw segment and no opposite-polarity successor nucleus is present'}
};

// The common alternative of sharing a turn nucleus is explicitly a relation,
// not a function Owner, and double-counts actual same-level material.
const sharedRuns=[{id:'O0',cores:['K0','K1','K2']},{id:'O1',cores:['K2','K3','K4']},{id:'O2',cores:['K4','K5']}];
const doubleOwnership=kernels.map(k=>({kernel:k.id,owners:sharedRuns.filter(r=>r.cores.includes(k.id)).map(r=>r.id),material:k.memberOwn})).filter(x=>x.owners.length>1);
check(same(doubleOwnership.map(x=>x.kernel),['K2','K4']),'shared-turn double ownership');
write('polarity-timeline.json',timeline);
write('mapping.json',{method:'M65-pol-v1',kernels,catalog,polarity:pol,forceResets:resets,forceComparisons:forces,finiteNE,
  monotone:mono,sharedTurn:{runs:sharedRuns,doubleOwnership},
  noOriginalCompletionCertified:true,noOriginalF2TripleCertified:true});
write('first-failures.json',failures);
write('result.json',{observations:583,segments:28,kernels:6,prefixesChecked:583,assertions:checks,
  rawParserExecuted:false,newHistories:0,
  forceResetEnds:resets.map(x=>[x.end,x.knownAt]),polarityClosed:pol.closed.map(x=>[x.end,x.knownAt]),
  polarityOpen:pol.blocks.at(-1).id,finiteSameOwnerNEChecks:finiteNE.length,
  semanticNECertified:false,semanticF1Certified:false,semanticF2Triples:0,
  primaryCounterexample:failures.primary,timingCounterexample:failures.timing});
const inputs=['source.json','segments.json','reference.stdout','result.json'].map(n=>({path:path.join(input,n),sha256:hash(path.join(input,n))}));
write('input-bindings.json',inputs);
console.log(JSON.stringify({status:'completed-bounded-author-check',assertions:checks,
  polarityClosed:pol.closed.map(x=>[x.end,x.knownAt]),primaryFailure:failures.primary.id,
  primaryFailureKnownAt:278,sourceCertification:false}));
