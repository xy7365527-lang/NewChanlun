// Four hand-specified scalar proof controls; never counted as native orders or
// as members of the bounded four-letter enumeration.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {relation,checkInvariants} from './relation-reference.mjs';
import {LegStream} from './streaming-prefix.mjs';
const seeds=[{name:'point-contact-first-window',q:[0,1,0,3,2,4,3,6,5,6]},{name:'empty-first-window',q:[0,1,0,3,2,5,4,7,6,7]}];
const cases=seeds.flatMap(x=>[x,{name:x.name+'-mirror',q:x.q.map(v=>-v)}]);
const results=[];
for(const c of cases){
  const r=relation(c.q,'B53'),stream=new LegStream(c.q[0]);for(const q of c.q.slice(1))stream.push(q);
  assert.deepEqual(r,stream.parse('B53'));checkInvariants(c.q,r);
  assert.equal(r.outputs.length,2);assert.equal(r.outputs[1].seed[0],r.outputs[0].return+1);
  results.push({...c,result:r,oldCutValue:c.q[r.outputs[0].end]});
}
const out=process.argv[2]??new URL('./results-v1/skipped-window-witnesses.json',import.meta.url);
fs.writeFileSync(out,JSON.stringify({ledger:'four-explicit-scalar-proof-controls',nativeEventCount:0,enumerationWordCount:0,cases:results},null,2)+'\n');
console.log('4 explicit scalar skipped-window controls passed');
