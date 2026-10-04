import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Bounded source-scope audit, not a replacement scanner or semantic certifier.
const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../');
const read = p => JSON.parse(fs.readFileSync(path.join(root, p), 'utf8'));
const oldScalar = read('stage53/candidate-evidence/results-v1/after-touch-comparison.json');
const input = read('stage53/candidate-evidence/reanchor-control-input.json');
const nativeOld = read('stage53/candidate-evidence/results-v1/reanchor_115_control.json');
const extent = xs => [Math.min(...xs), Math.max(...xs)];
const gcd = (a, b) => b ? gcd(b, a % b) : a;
const W = q => { const n = 1000n + 102n * BigInt(q), d = 10n + BigInt(q), g = gcd(n, d); return `${n/g}/${d/g}`; };

// These three controls alternate at every coordinate. This is not a general parser.
function legs(q) {
  for (let i = 1; i < q.length; i++) {
    assert.notEqual(q[i], q[i-1]);
    if (i > 1) assert.notEqual(Math.sign(q[i]-q[i-1]), Math.sign(q[i-1]-q[i-2]));
  }
  return q.slice(1, -1).map((v, i) => ({id: i, name: `X${i+1}`, start: i, end: i+1,
    direction: Math.sign(v-q[i]), low: Math.min(v,q[i]), high: Math.max(v,q[i]), knownAt: i+2}));
}
function summarize(q, lastMember, departure, returning, old) {
  const ls = legs(q);
  const seed = [0,1,2], core = [Math.max(...seed.map(i=>ls[i].low)), Math.min(...seed.map(i=>ls[i].high))];
  assert.ok(core[0] < core[1]);
  const members = Array.from({length:lastMember+1}, (_,i)=>i);
  const zs = members.filter(i=>ls[i].direction===ls[0].direction);
  const memberHull = extent(members.flatMap(i=>[ls[i].low,ls[i].high]));
  const zHull = extent(zs.flatMap(i=>[ls[i].low,ls[i].high]));
  const L=ls[departure], R=ls[returning];
  assert.equal(R.id,L.id+1);
  assert.ok(L.direction===1 && R.direction===-1 && q[L.start]<=core[1] && q[L.end]>core[1] && R.low>core[1]);
  const fullRange=extent(q.slice(0,L.end+1));
  const result={q,legs:ls,seed:seed.map(i=>ls[i].name),core,
    membershipHypothesis:members.map(i=>ls[i].name),formationDirection:ls[0].direction,
    sameDirectionWithinHypothesis:zs.map(i=>ls[i].name),memberHull,zHull,
    coverage:members.map(i=>({member:ls[i].name,covered:ls[i].low>=zHull[0]&&ls[i].high<=zHull[1]})),
    departure:L.name,return:R.name,departureSource:[L.start,L.end],
    departureKnownAt:L.knownAt,returnKnownAt:R.knownAt,
    seedOccurredAt:ls[2].end,seedRawKnownAt:ls[2].knownAt,
    successfulRelationRawKnownAt:Math.max(ls[2].knownAt,L.knownAt,R.knownAt),
    completeOutputSource:[0,L.end],completeOutputRange:fullRange,
    candidateCenterEvents:members.flatMap(i=>[ls[i].end]),
    connectionCandidateEvents:[L.end],tailEvents:Array.from({length:q.length-1-L.end},(_,i)=>L.end+i+1),
    originalCoreIdentity:null,originalMemberCertificates:null,originalConnectionIdentity:null,originalOuterIdentity:null};
  if(old) {
    assert.deepEqual(core,old.core);
    assert.equal(L.id,old.departure); assert.equal(R.id,old.return);
    assert.equal(result.successfulRelationRawKnownAt,old.relationshipPublishedAt);
    assert.deepEqual(fullRange,old.fullRange);
  }
  return result;
}

// Independent raw ID replay of the one existing synthetic native-message control.
const orders=new Map(['bid','ask'].map(side=>[input.initial[side].id,{...input.initial[side],side}]));
const nativeStates=[];
function observe(t) {
  const bs=[...orders.values()].filter(o=>o.side==='bid'), as=[...orders.values()].filter(o=>o.side==='ask');
  const bid=Math.max(...bs.map(o=>o.price)),ask=Math.min(...as.map(o=>o.price));
  assert.equal(bid,100); assert.equal(ask,102);
  assert.equal(as.filter(o=>o.price===ask).reduce((s,o)=>s+o.quantity,0),10);
  const q=bs.filter(o=>o.price===bid).reduce((s,o)=>s+o.quantity,0);
  nativeStates.push({event:t,q,w:W(q)});
}
observe(0);
input.events.forEach((e,i)=>{
  assert.equal(e.seq,i+1); assert.ok(Number.isSafeInteger(e.quantity)&&e.quantity>0);
  if(e.kind==='add') {assert.ok(!orders.has(e.id));orders.set(e.id,{...e});}
  else {assert.equal(e.kind,'reduce');const o=orders.get(e.id);assert.ok(o&&o.quantity>=e.quantity);o.quantity-=e.quantity;if(o.quantity===0)orders.delete(e.id);}
  observe(i+1);
});
assert.deepEqual(nativeStates,nativeOld.states);
const nq=nativeStates.map(s=>s.q);
assert.deepEqual(nq,[115,100,110,104,112,111,113,112]);

const scalarOld=oldScalar.results.C53.outputs[0];
assert.deepEqual(oldScalar.q,[0,1,0,1,0,3,2,3,0]);
assert.deepEqual(scalarOld.members,[0,1,2,3,4]);
assert.equal(scalarOld.departureIsMember,true);
const scalar = summarize(oldScalar.q,3,4,5,scalarOld);
scalar.oldC53={members:scalarOld.members.map(i=>`X${i+1}`),memberHull:scalarOld.memberOuter,
  absorbedAt:6,absorptionPair:['X4','X5'],priorLRange:[0,1],priorLStrictlyLeftCore:false};
assert.deepEqual(scalar.memberHull,[0,1]); assert.deepEqual(scalar.zHull,[0,1]);
assert.deepEqual(scalar.completeOutputRange,[0,3]);
assert.deepEqual([...scalar.candidateCenterEvents,...scalar.connectionCandidateEvents,...scalar.tailEvents],[1,2,3,4,5,6,7,8]);

const nativeC=nativeOld.prefixes.at(-1).results.C53.outputs[0];
const native = summarize(nq,2,3,4,nativeC);
assert.deepEqual(native.memberHull,[100,115]); assert.deepEqual(native.zHull,[100,115]);
assert.deepEqual(nativeC.members,[0,1,2]);
assert.equal(nativeC.departureIsMember,false);
native.states=nativeStates;
native.B53=nativeOld.prefixes.at(-1).results.B53.outputs[0];
assert.deepEqual(native.B53.members,[0,1,2,3]);
assert.equal(native.B53.departureIsMember,true);
assert.deepEqual(native.B53.memberOuter,nativeC.memberOuter);
assert.deepEqual([...native.candidateCenterEvents,...native.connectionCandidateEvents,...native.tailEvents],[1,2,3,4,5,6,7]);
native.rawPrefixes=nativeOld.prefixes.map(p=>({prefix:p.prefixAt,
  C53MemberIDs:(p.results.C53.outputs[0]?.members??p.results.C53.terminal.members??[]),
  C53Published:p.results.C53.outputs.length>0,
  B53MemberIDs:(p.results.B53.outputs[0]?.members??p.results.B53.terminal.members??[])}));

const unequal=summarize([0,2,1,3,-1,4,3,5,0],3,4,5);
assert.deepEqual(unequal.core,[1,2]);
assert.deepEqual(unequal.memberHull,[-1,3]); assert.deepEqual(unequal.zHull,[0,3]);
assert.equal(unequal.coverage.find(x=>x.member==='X4').covered,false);
assert.deepEqual(unequal.completeOutputRange,[-1,4]);

const output={scope:'source-membership-and-range-check-only',nativeLedger:{existingControlCount:1,eventReferences:7,
  statesIncludingInitial:8,newNativeEvents:0,marketSamples:0,fullIDReplayOperations:7},
  scalarLedger:{controls:2,reusedStage53Words:1,newHandWrittenWords:1,totalCoordinateValues:18,nativeEvents:0,enumeratedWords:0},
  scalarAfterTouch:scalar,reanchorNativeControl:native,memberHullVsZnHullCounterexample:unequal,
  semantics:'All original semantic identity/certification fields remain null. Arithmetic checks do not qualify a source role.'};
const target=path.join(here,'member-source-checks.json');
fs.writeFileSync(target,JSON.stringify(output,null,2)+'\n');
console.log(JSON.stringify({ok:true,nativeLedger:output.nativeLedger,scalarLedger:output.scalarLedger,
  scalarHulls:[scalar.memberHull,scalar.zHull,scalar.completeOutputRange],
  nativeHulls:[native.memberHull,native.zHull,native.completeOutputRange],
  unequalHulls:[unequal.memberHull,unequal.zHull,unequal.completeOutputRange]}));
