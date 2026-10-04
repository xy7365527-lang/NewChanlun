import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
import {relation,check} from '../../stage54/candidate-evidence/relation-reference.mjs';

const here=path.dirname(fileURLToPath(import.meta.url));
const specifications=[
  {id:'five-core-raw-15',coordinates:[0,0,0,10,10,10,5,5,5,15,15,15,4,4,4],tag:'AwaitPair',core:[105,110],
    expectedLegs:[{id:0,start:0,end:6,direction:1,low:100,high:110,knownAt:7},{id:1,start:6,end:9,direction:-1,low:105,high:110,knownAt:10},{id:2,start:9,end:12,direction:1,low:105,high:115,knownAt:13}]},
  {id:'equal-return-raw-7',coordinates:[0,0,0,1,0,0,0],tag:'NoCore',core:null,
    expectedLegs:[{id:0,start:0,end:4,direction:1,low:100,high:101,knownAt:5}]}
];
const controls=specifications.map(spec=>{
  const input={kind:'synthetic-trade-model',marketData:false,coordinateShift:100,initial:{price:100,isTrade:false},events:spec.coordinates.map((x,i)=>({seq:i+1,id:`${spec.id}-e${i+1}`,kind:'trade',price:100+x,quantity:1}))};
  const values=[input.initial.price,...input.events.map(e=>e.price)];
  let previous; const prefixes=[];
  for(let n=0;n<values.length;n++){
    const h=values.slice(0,n+1),r=relation(h); check(h,r,previous);
    assert.equal(r.outputs.length,0); assert.deepEqual(r.tail.ownedEvents,Array.from({length:n},(_,i)=>i+1));
    prefixes.push({prefixAt:n,result:r}); previous=r;
  }
  const final=prefixes.at(-1).result;
  assert.equal(final.terminal.tag,spec.tag); assert.deepEqual(final.legs,spec.expectedLegs);
  assert.deepEqual(final.terminal.core??null,spec.core);
  if(spec.core){
    assert.deepEqual(final.terminal.seed,[0,1,2]); assert.deepEqual(final.terminal.developmentLegs,[0,1,2]);
    assert.equal(prefixes[12].result.terminal.tag,'NoCore');
    assert.equal(prefixes[13].result.terminal.tag,'AwaitPair');
  }
  return {id:spec.id,input,prefixes,summary:{events:input.events.length,prefixes:prefixes.length,outputs:final.outputs.length,terminal:final.terminal,unpublishedWholeRange:[Math.min(...values),Math.max(...values)],fixedF2EligibleInputs:0,originalP:null}};
});
const result={schema:'stage56-lifecycle-two-controls-v1',controls:controls,totals:{controls:controls.length,events:22,prefixes:24},scope:'Two synthetic raw trade controls only; numeric D54 checks are not original completion or F2 certification.'};
fs.writeFileSync(path.join(here,'two-controls-v1.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({ok:true,controls:result.totals,terminalTags:controls.map(c=>c.summary.terminal.tag),semanticStatus:'unproved'}));
