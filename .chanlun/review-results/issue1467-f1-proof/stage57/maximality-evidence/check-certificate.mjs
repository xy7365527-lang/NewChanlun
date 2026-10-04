import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
const R='/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const D=path.join(R,'.chanlun/review-results/issue1467-f1-proof/stage57/maximality-evidence');
const source=path.join(R,'.chanlun/review-results/issue1467-f1-proof/stage54/candidate-evidence/ordinary-proofs-v1.md');
const text=fs.readFileSync(source,'utf8');
const wordLiteral='0,10,0,10,0,30,20,28,22,40,35,45';
assert(text.includes('`'+wordLiteral+'`'));
const x=wordLiteral.split(',').map(Number);
// Only this already-frozen, strictly alternating word is inspected.
const legs=[];
for(let i=0;i+2<x.length;i++){
  assert((x[i+1]-x[i])*(x[i+2]-x[i+1])<0);
  legs.push({index:i,start:i,end:i+1,knownAt:i+2,low:Math.min(x[i],x[i+1]),high:Math.max(x[i],x[i+1])});
}
const s=0;
const C={low:Math.max(...legs.slice(0,3).map(a=>a.low)),high:Math.min(...legs.slice(0,3).map(a=>a.high))};
assert(C.low<C.high);
const touch=i=>legs[i].low<=C.high&&C.low<=legs[i].high;
const span=k=>k>=s+2&&legs.slice(s,k+1).every((_,i)=>touch(s+i));
const terminals=[];
for(let k=s+2;k+1<legs.length;k++)if(span(k)&&!touch(k+1))terminals.push({j:k,r:k+1});
assert.deepEqual(terminals,[{j:4,r:5}]);
const {j,r}=terminals[0];
assert(legs[r].low>C.high);
assert(legs[j].end<legs[r].knownAt);
const prefixChecks=x.map((_,tau)=>({tau,rawTerminal:span(j)&&legs[r].knownAt<=tau&&!touch(r)}));
assert.equal(prefixChecks.find(x=>x.rawTerminal).tau,7);
const finiteLarger=Array.from({length:legs.length-j-1},(_,i)=>j+i+1).map(k=>({k,touchSpan:span(k),blockingIndex:r}));
assert(finiteLarger.every(a=>a.touchSpan===false));
const result={scope:'One existing scalar word, first seed only; no D54 implementation imported; no original Move qualification',source:{path:source,sha256:crypto.createHash('sha256').update(text).digest('hex'),wordLiteral},word:x,seed:{identity:'D54-existing-word:first-seed:legs-0-1-2',s,core:C},rawLegsThroughR:legs.slice(0,r+1),terminal:{j,r,L_end:legs[j].end,R_knownAt:legs[r].knownAt},contactChecks:legs.slice(s,j+1).map(a=>({index:a.index,range:[a.low,a.high],touch:touch(a.index)})),prefixChecks,finiteLarger,whole:{ownedEvents:[1,2,3,4,5],closedStateRange:[0,30]},limits:['Finite checks illustrate, not prove, the universal theorem.','Seed identity is a source ID, not inferred from equality of C.','Whole includes L; the contact theorem begins at s, not at an arbitrary NoCore prefix.']};
fs.writeFileSync(path.join(D,'certificate-v1.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({status:'passed',existingWords:1,terminal:result.terminal,checkedPrefixes:prefixChecks.length}));
