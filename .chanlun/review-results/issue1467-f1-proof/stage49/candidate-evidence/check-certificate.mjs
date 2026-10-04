import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const dir=process.argv[2]??path.join(here,'results-v1');
const read=n=>JSON.parse(fs.readFileSync(path.join(dir,n+'.json'),'utf8'));
const h=xs=>[Math.min(...xs),Math.max(...xs)];
const inter=rs=>[Math.max(...rs.map(r=>r[0])),Math.min(...rs.map(r=>r[1]))];
const rng=(a,b)=>Array.from({length:b-a},(_,i)=>a+i+1);
const findings=[];
for(const [name,type] of [['same_up_counterexample','U'],['same_down_mirror','D']]) {
  const f=read(name),s=f.source,r=f.result;
  const orders=new Map([[s.initial.bid.id,{...s.initial.bid,side:'bid'}],
    [s.initial.ask.id,{...s.initial.ask,side:'ask'}]]);
  const qs=[s.initial.bid.quantity];
  for(const e of s.events) {
    if(e.kind==='add') {assert.ok(!orders.has(e.id));orders.set(e.id,{...e});}
    else {assert.equal(e.kind,'reduce');const o=orders.get(e.id);assert.ok(o&&o.quantity>e.quantity);o.quantity-=e.quantity;}
    const bids=[...orders.values()].filter(o=>o.side==='bid'),asks=[...orders.values()].filter(o=>o.side==='ask');
    assert.equal(Math.max(...bids.map(o=>o.price)),100);
    assert.equal(Math.min(...asks.map(o=>o.price)),102);
    assert.equal(asks.filter(o=>o.price===102).reduce((a,o)=>a+o.quantity,0),10);
    qs.push(bids.filter(o=>o.price===100).reduce((a,o)=>a+o.quantity,0));
  }
  assert.deepEqual(qs,s.states.map(x=>x.q));
  for(const l of r.legs.completed) {
    assert.deepEqual(l.range,h(qs.slice(l.start,l.end+1)));
    const sign=l.direction==='Up'?1:-1;
    assert.ok(sign*(qs[l.end]-qs[l.start])>0);
    for(let j=l.start+1;j<=l.end;j++) assert.ok(sign*(qs[j]-qs[j-1])>=0);
    assert.equal(l.knownAt,l.end+1);
    assert.ok(sign*(qs[l.knownAt]-qs[l.end])<0);
  }
  const ms=r.outputs;
  assert.equal(ms.length,3);
  assert.deepEqual(ms.map(m=>m.type),[type,type,type]);
  assert.deepEqual(ms.map(m=>m.level),[0,0,0]);
  assert.deepEqual(ms.map(m=>[m.start,m.end,m.confirmedAt]),[[0,8,15],[8,16,23],[16,24,31]]);
  assert.deepEqual(ms.flatMap(m=>m.sourceEvents),rng(0,24));
  assert.deepEqual([...ms.flatMap(m=>m.sourceEvents),...r.tail.sourceEvents],rng(0,s.events.length));
  assert.equal(new Set(ms.flatMap(m=>m.ownedCoreIDs)).size,6);
  for(let i=0;i<ms.length;i++) {
    const m=ms[i],a=m.cores[0],b=m.cores[1];
    assert.deepEqual(m.sourceEvents,rng(m.start,m.end));
    assert.deepEqual(m.range,h(qs.slice(m.start,m.end+1)));
    assert.equal(m.startValue,qs[m.start]);assert.equal(m.endValue,qs[m.end]);
    assert.ok(type==='U'?b.outer[0]>a.outer[1]:b.outer[1]<a.outer[0]);
    const witness=r.coreScan.completed.find(c=>c.id===m.completion.witnessCore);
    assert.equal(witness.sealedAt,m.confirmedAt);
    assert.ok(type==='U'?witness.core[0]>b.core[1]:witness.core[1]<b.core[0]);
    assert.ok(Math.max(witness.outer[0],b.outer[0])<=Math.min(witness.outer[1],b.outer[1]));
    for(const c of [a,b,witness]) {
      assert.deepEqual(c.core,inter(c.seed.map(j=>r.legs.completed[j].range)));
      assert.deepEqual(c.outer,h(c.members.flatMap(j=>r.legs.completed[j].range)));
      assert.deepEqual(c.outer,h(c.members.filter((_,k)=>k%2===0).flatMap(j=>r.legs.completed[j].range)));
      assert.ok(c.core[0]<c.core[1]);
      assert.ok(c.exit.returnRange[0]>c.core[1]||c.exit.returnRange[1]<c.core[0]);
      assert.ok(c.sealedAt<=m.confirmedAt);
    }
    if(i) {assert.equal(ms[i-1].end,m.start);assert.equal(ms[i-1].endValue,m.startValue);}
  }
  const at23=ms.filter(m=>m.confirmedAt<=23);
  assert.equal(at23.length,2);
  assert.deepEqual(at23.flatMap(m=>m.sourceEvents),rng(0,16));
  findings.push({name,eventsReplayed:s.events.length,firstForbiddenAdjacentCompletionKnownAt:23,
    objectsAt23:at23.map(m=>({id:m.id,type:m.type,level:m.level,start:m.start,end:m.end,
      range:m.range,confirmedAt:m.confirmedAt,sourceEvents:m.sourceEvents})),
    noEventGap:true,disjointOwnedCores:true,threeCandidateObjectsKnownAt:31,
    originalCompletedSuccession:false});
}
const overlap=read('p_completion_overlap'),split=read('p_completion_disjoint');
assert.deepEqual(overlap.source.events.slice(0,7),split.source.events.slice(0,7));
assert.deepEqual(overlap.source.states.slice(0,8),split.source.states.slice(0,8));
assert.deepEqual(overlap.result.coreScan.completed[0],split.result.coreScan.completed[0]);
assert.equal(overlap.result.outputs[0].type,'P');
assert.equal(overlap.result.outputs[0].end,4);
assert.equal(overlap.result.outputs[0].confirmedAt,11);
assert.equal(split.result.outputs.length,0);
const oscillation=read('long_10_20');
assert.equal(oscillation.result.coreScan.active.members.length,77);
const report={scope:'separate author arithmetic/source checker; not independent review',
  findings,pFork:{commonEvents:7,oldCoreEnd:4,oldCoreSealedAt:7,
    overlapCandidatePConfirmedAt:11,disjointBranchNoP:true},
  allAssertionsPassed:true};
fs.writeFileSync(path.join(dir,'certificate-check.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report,null,2));
