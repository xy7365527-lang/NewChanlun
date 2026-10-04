import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

const here=path.dirname(fileURLToPath(import.meta.url));
const lock=JSON.parse(fs.readFileSync(path.join(here,'source-lock.json')));
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
for(const s of lock.sources) assert.equal(sha(fs.readFileSync(path.join(lock.root,s.path))),s.sha256,s.path);
const raw=JSON.parse(fs.readFileSync(path.join(lock.root,'.chanlun/review-results/issue1467-f1-proof/stage49/candidate-evidence/results-v1/same_up_counterexample.json')));
const q=raw.source.states.map(s=>s.q);
assert.deepEqual(q.slice(0,16),[1040,1040,1000,1030,1010,1140,1100,1130,1110,1140,1135,1138,1136,1240,1200,1230]);
const hull=(s,t)=>[Math.min(...q.slice(s,t+1)),Math.max(...q.slice(s,t+1))];
const std=(s,t)=>{
  const [lo,hi]=hull(s,t);
  return lo<hi && ((q[s]===lo && q[t]===hi)||(q[s]===hi && q[t]===lo));
};
const current=[[0,5],[5,8],[8,12]].map(([s,t],i)=>({id:`P${i}`,s,t,range:hull(s,t),start:q[s],end:q[t],std:std(s,t)}));
assert.deepEqual(current.map(r=>r.std),[false,false,false]);
// This is a finite readback, not the proof for arbitrary future extensions.
const possibleFirstEnds=Array.from({length:q.length-4},(_,i)=>i+4);
assert.ok(possibleFirstEnds.every(t=>!std(0,t)));
assert.ok(q[2]<q[0] && q[0]<q[5]);
const firstCore=raw.result.coreScan.completed[0];
assert.equal(firstCore.sourceStart,0);assert.equal(firstCore.sourceEnd,4);
assert.deepEqual(hull(0,4),[1000,1040]);
assert.notEqual(q[4],1000);assert.notEqual(q[4],1040);
// A conditional, purely numerical model of the specific 039:28 return branch.
// These endpoints do not certify any Chan-theory moves or new order histories.
const model039={a:[0,10],A1:[10,5],A2:[5,15],A3:[15,7]};
const ranges039=[model039.A1,model039.A2,model039.A3].map(v=>[Math.min(...v),Math.max(...v)]);
const intersection039=[Math.max(...ranges039.map(r=>r[0])),Math.min(...ranges039.map(r=>r[1]))];
assert.ok(model039.A1[1]>=model039.a[0]);
assert.ok(model039.A2[1]>model039.a[1]);
assert.ok(model039.A3[1]<model039.a[1]);
assert.deepEqual(intersection039,[7,10]);
// Payload-insensitivity: ownership count + the same break record has two candidate endpoints.
// No semantic validity is asserted for either interval.
const equalBreakInputs=[4,5].map(end=>({end,ownedIDs:['K0'],core:firstCore.core,exit:firstCore.exit}));
assert.deepEqual(equalBreakInputs[0].ownedIDs,equalBreakInputs[1].ownedIDs);
assert.deepEqual(equalBreakInputs[0].exit,equalBreakInputs[1].exit);
assert.notDeepEqual(hull(0,4),hull(0,5));
const result={scope:'Source-bound ordinary-lemma checks; reads stored coordinates and catalog, does not replay events or certify core/move identity.',
  checkedSourceFiles:lock.sources.length,sourceLockSHA256:sha(fs.readFileSync(path.join(here,'source-lock.json'))),
  current,firstObjectFiniteCheck:{ends:possibleFirstEnds,count:possibleFirstEnds.length,allFailStd:true},
  arbitraryContinuationObstruction:{fixedStart:0,ownedSupport:[0,4],end4Fails:true,permanentStrictInteriorWitness:[2,0,5],values:[q[2],q[0],q[5]],proof:'Any future hull containing indices 2,0,5 keeps q0 strictly internal.'},
  source039ConditionalGeometry:{model:model039,ranges:ranges039,intersection:intersection039,completeInputsAssumed:true,orderEventRealizationClaimed:false},
  sameBreakPayload:{candidateIntervals:[[0,4],[0,5]],ranges:[hull(0,4),hull(0,5)],payload:equalBreakInputs[0].exit,semanticCompletionClaimed:false},allAssertionsPassed:true};
fs.writeFileSync(path.join(here,'source-claim-results.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({checkedSources:lock.sources.length,allAssertionsPassed:true,firstObjectFiniteEndCount:possibleFirstEnds.length,conditional039Intersection:intersection039}));
