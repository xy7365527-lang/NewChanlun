import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';

const here=path.dirname(fileURLToPath(import.meta.url));
const P=path.resolve(here,'../..');
const out=process.argv[2]??path.join(here,'results-v1');
const sha=f=>crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const json=f=>JSON.parse(fs.readFileSync(f,'utf8'));
const lock=json(path.join(here,'input-lock.json'));
for(const f of lock.files) assert.equal(sha(path.join(P,f.path)),f.sha256,f.path);
fs.mkdirSync(out,{recursive:true});
const save=(n,x)=>fs.writeFileSync(path.join(out,n),JSON.stringify(x,null,2)+'\n');
const indices=(a,b)=>Array.from({length:b-a+1},(_,i)=>a+i);
const events=(a,b)=>indices(a+1,b);
const hull=xs=>[Math.min(...xs),Math.max(...xs)];
const sign=x=>x>0?'U':x<0?'D':null;
const gcd=(a,b)=>b===0n?a:gcd(b,a%b);
const rat=(n,d=1n)=>{assert.ok(d>0n);const g=gcd(n<0n?-n:n,d);return `${n/g}/${d/g}`;};
const W=q=>rat(1000n+102n*BigInt(q),10n+BigInt(q));
const cmp=(a,b)=>{const [an,ad]=a.split('/').map(BigInt),[bn,bd]=b.split('/').map(BigInt);const d=an*bd-bn*ad;return d>0n?1:d<0n?-1:0;};

function replay(source) {
  const book=new Map([[source.initial.bid.id,{...source.initial.bid,side:'bid'}],[source.initial.ask.id,{...source.initial.ask,side:'ask'}]]);
  const states=[];const snapshots=[];
  function observe(event) {
    for(const o of book.values())assert.ok(Number.isSafeInteger(o.quantity)&&o.quantity>0);
    const bids=[...book.values()].filter(o=>o.side==='bid'),asks=[...book.values()].filter(o=>o.side==='ask');
    assert.equal(Math.max(...bids.map(o=>o.price)),100);
    assert.equal(Math.min(...asks.map(o=>o.price)),102);
    const q=bids.filter(o=>o.price===100).reduce((s,o)=>s+o.quantity,0);
    const askQ=asks.filter(o=>o.price===102).reduce((s,o)=>s+o.quantity,0);
    assert.equal(askQ,10);assert.ok(q>0);
    states.push({event,q,w:W(q)});
    snapshots.push({event,reservoirQuantity:book.get(source.initial.bid.id).quantity,bestBidQuantity:q,bestAskQuantity:askQ,liveIDs:[...book.keys()]});
  }
  observe(0);
  for(const [i,e] of source.events.entries()) {
    assert.equal(e.seq,i+1);assert.ok(Number.isSafeInteger(e.quantity)&&e.quantity>0);
    if(e.kind==='add') {assert.ok(!book.has(e.id));assert.equal(e.side,'bid');assert.ok(e.price<=100);book.set(e.id,{...e});}
    else {assert.equal(e.kind,'reduce');const o=book.get(e.id);assert.ok(o&&o.quantity>e.quantity);o.quantity-=e.quantity;}
    observe(e.seq);
  }
  if(source.states)assert.deepEqual(states,source.states);
  let orderComparisons=0;
  for(const a of states)for(const b of states){assert.equal(cmp(a.w,b.w),Math.sign(a.q-b.q));orderComparisons++;}
  return {states,snapshots,orderComparisons};
}

function pairs(states,a,b,H,d,coordinate='q') {
  const value=i=>states[i][coordinate];
  const compare=coordinate==='q'?((a,b)=>Math.sign(a-b)):cmp;
  const band=coordinate==='q'?H:H.map(W);
  const found=[];
  for(let i=a;i<=b;i++)for(let j=i+1;j<=b;j++) {
    const ok=d==='U'?compare(value(i),band[0])<=0&&compare(value(j),band[1])>=0:
      compare(value(i),band[1])>=0&&compare(value(j),band[0])<=0;
    if(ok)found.push([i,j]);
  }
  return found;
}
function triples(states,cuts) {
  assert.equal(cuts.length,4);for(let i=1;i<4;i++)assert.ok(cuts[i]>cuts[i-1]);
  const ranges=cuts.slice(0,3).map((a,r)=>hull(states.slice(a,cuts[r+1]+1).map(s=>s.q)));
  const H=[Math.max(...ranges.map(r=>r[0])),Math.min(...ranges.map(r=>r[1]))];
  if(H[0]>=H[1])return {cuts,qRanges:ranges,numericalH:H,strict:false,allEightAssignments:[],selections:[]};
  const children=cuts.slice(0,3).map((a,r)=>{
    const b=cuts[r+1],witnesses={};
    for(const d of ['U','D']) {
      const ps=pairs(states,a,b,H,d);assert.deepEqual(ps,pairs(states,a,b,H,d,'w'));
      witnesses[d]=ps.map(([i,j])=>({indices:[i,j],q:[states[i].q,states[j].q],w:[states[i].w,states[j].w],pointsAvailableAt:j}));
    }
    const path=states.slice(a,b+1),range=ranges[r];
    const ownEvents=events(a,b),support=indices(a,b);
    return {start:a,end:b,ownedEvents:ownEvents,priceSupport:support,sourceStates:path,qRange:range,wRange:range.map(W),
      extrema:{minIndices:path.filter(s=>s.q===range[0]).map(s=>s.event),maxIndices:path.filter(s=>s.q===range[1]).map(s=>s.event)},
      endpointDirection:sign(states[b].q-states[a].q),strictEndpointIncrease:states[b].q>states[a].q,
      witnesses,directions:['U','D'].filter(d=>witnesses[d].length>0),conceptDirection:null,originalCompletionEvidence:null,originalZ3Evidence:null};
  });
  assert.deepEqual(children.flatMap(c=>c.ownedEvents),events(cuts[0],cuts[3]));
  assert.equal(new Set(children.flatMap(c=>c.ownedEvents)).size,cuts[3]-cuts[0]);
  for(let r=0;r<2;r++) {
    assert.equal(children[r].sourceStates.at(-1).event,children[r+1].sourceStates[0].event);
    assert.deepEqual(children[r].sourceStates.at(-1),children[r+1].sourceStates[0]);
    assert.deepEqual(children[r].priceSupport.filter(i=>children[r+1].priceSupport.includes(i)),[cuts[r+1]]);
  }
  const allEightAssignments=[],selections=[];
  for(const a of ['U','D'])for(const b of ['U','D'])for(const c of ['U','D']) {
    const ds=[a,b,c],perChild=ds.map((d,i)=>children[i].directions.includes(d));
    const feasible=perChild.every(Boolean),alternating=a!==b&&b!==c;
    const row={word:ds.join(''),perChildHasWitness:perChild,feasible,alternating,accepted:feasible&&alternating};
    allEightAssignments.push(row);
    if(row.accepted) {
      const tuples=[];
      for(const wa of children[0].witnesses[a])for(const wb of children[1].witnesses[b])for(const wc of children[2].witnesses[c])
        tuples.push([wa.indices,wb.indices,wc.indices]);
      selections.push({word:row.word,witnessTupleCount:tuples.length,allWitnessIndexTuples:tuples});
    }
  }
  return {cuts,qRanges:ranges,wRanges:ranges.map(r=>r.map(W)),numericalH:H,wNumericalH:H.map(W),strict:true,children,
    allEightAssignments,selections,selectionCount:selections.length,witnessTupleCount:selections.reduce((n,s)=>n+s.witnessTupleCount,0),
    sourceRangesAvailableAt:cuts[3],semanticEligibleAt:null,originalParentCore:null,
    qualification:'numerical RDW-v1 only; no completed same-level moves or original Z3 bridge certified'};
}

let replayed=0,comparisonCount=0;const main=[];const mainSummary=[];
for(const name of ['same_up_counterexample','same_down_mirror']) {
  const old=json(path.join(P,`stage49/candidate-evidence/results-v1/${name}.json`));
  const stage50=json(path.join(P,`stage50/candidate-evidence/results-v1/${name}.json`));
  const {states,orderComparisons}=replay(old.source);replayed+=old.source.events.length;comparisonCount+=orderComparisons;
  assert.deepEqual(stage50.source,old.source);
  const records=[];
  for(const snapshot of stage50.snapshots)for(const necessary of snapshot.leftWidenedNecessary.necessary) {
    const r=triples(states,necessary.cuts);
    assert.equal(r.strict,necessary.strictCore);assert.deepEqual(r.numericalH,necessary.qParentCore);
    r.catalogueCheckedAt=snapshot.t;
    r.outsideThisPartition={knownLeftEvents:events(0,r.cuts[0]),knownRightEvents:events(r.cuts[3],snapshot.t),
      futureEventsNotConsumed:events(snapshot.t,old.source.events.length)};
    assert.deepEqual([...r.outsideThisPartition.knownLeftEvents,...events(r.cuts[0],r.cuts[3]),
      ...r.outsideThisPartition.knownRightEvents,...r.outsideThisPartition.futureEventsNotConsumed],events(0,old.source.events.length));
    r.originalCompletionSupportAvailableAt=null;
    if(r.strict) {
      const ownIds=[];let catalogueSupportAvailableAt=0;
      for(const [i,ch] of r.children.entries()) {
        const ref=necessary.children[i];assert.deepEqual(ch.qRange,ref.qRange);assert.deepEqual(ch.wRange,ref.wRange);
        assert.deepEqual(ch.ownedEvents,ref.sourceEvents);assert.deepEqual(ch.priceSupport,ref.priceSupportEvents);
        assert.equal(ref.originalCompletionEvidence,null);assert.equal(ref.dirForZ3Evidence,null);assert.equal(ref.conceptDirection,null);
        const cores=old.result.coreScan.completed.filter(c=>c.sealedAt<=snapshot.t&&ch.start<=c.sourceStart&&c.sourceEnd<=ch.end);
        assert.deepEqual(cores.map(c=>c.id),ref.ownedCoreIDs);assert.equal(cores.length,1);
        ch.kindIfCompleted=ref.type;assert.equal(ch.kindIfCompleted,'P');ch.ownedCatalogueCoreIDs=cores.map(c=>c.id);
        ch.coreFormationDiagnostics=cores.map(c=>{
          ownIds.push(c.id);catalogueSupportAvailableAt=Math.max(catalogueSupportAvailableAt,c.sealedAt);
          const supportLegs=[...new Set([...c.members,c.exit.departure,c.exit.return])].map(j=>old.result.legs.completed[j]);
          assert.ok(supportLegs.every(l=>l.knownAt<=c.sealedAt&&l.knownAt<=snapshot.t));
          const deltas=states.slice(c.sourceStart+1,c.sourceEnd+1).map((s,j)=>sign(s.q-states[c.sourceStart+j].q)).filter(Boolean);
          const expected=c.startDirection==='Up'?'U':'D';assert.equal(deltas[0],expected);
          return {id:c.id,sourceStart:c.sourceStart,sourceEnd:c.sourceEnd,rawNonzeroIncrementSigns:deltas,firstFormationSign:deltas[0],sealedAt:c.sealedAt,
            supportLegIndices:[...new Set([...c.members,c.exit.departure,c.exit.return])],supportKnownAt:supportLegs.map(l=>l.knownAt),originalCenterQualification:null};
        });
      }
      assert.equal(new Set(ownIds).size,3);r.catalogueSupportAvailableAt=catalogueSupportAvailableAt;
      assert.ok(r.sourceRangesAvailableAt<=snapshot.t&&catalogueSupportAvailableAt<=snapshot.t);
      r.earliestNumericalAndFrozenCatalogueReadbackAt=Math.max(r.sourceRangesAvailableAt,catalogueSupportAvailableAt);
      // This is a retrospective check of already named cuts, not a claim that an online decomposition emitted them then.
      r.onlinePartitionPublicationAt=null;
      mainSummary.push({name,cuts:r.cuts,H:r.numericalH,directionSets:r.children.map(c=>c.directions),words:r.selections.map(s=>s.word),
        witnessTupleCount:r.witnessTupleCount,catalogueCheckedAt:snapshot.t,rangeAvailableAt:r.sourceRangesAvailableAt,
        catalogueSupportAvailableAt,semanticEligibleAt:null});
    }
    records.push(r);
  }
  save(`${name}.json`,{candidate:'RDW-v1',sourceFile:`stage49/candidate-evidence/results-v1/${name}.json`,sourceSHA256:sha(path.join(P,`stage49/candidate-evidence/results-v1/${name}.json`)),records});
  main.push(...records);
}
assert.equal(replayed,80);assert.equal(main.length,24);assert.equal(main.filter(r=>r.strict).length,12);
assert.deepEqual(mainSummary[0].directionSets,[['U'],['D'],['U']]);assert.equal(mainSummary[0].witnessTupleCount,10);
assert.deepEqual(mainSummary[1].directionSets,[['U'],['U','D'],['D']]);assert.equal(mainSummary[1].words.length,0);
assert.equal(mainSummary.filter(r=>r.words.length===1).length,6);assert.equal(mainSummary.filter(r=>r.words.length===0).length,6);

function lift(qs,label) {
  const shift=1000,q=qs.map(x=>x+shift);
  const source={epoch:label,synthetic:true,marketData:false,coordinates:qs,shift,
    initial:{bid:{id:'reservoir',price:100,quantity:q[0]},ask:{id:'ask',price:102,quantity:10}},events:[]};
  for(let i=1;i<q.length;i++) {
    const d=q[i]-q[i-1];assert.notEqual(d,0);
    source.events.push(d>0?{seq:i,kind:'add',id:`rise-${i}`,side:'bid',price:100,quantity:d}:
      {seq:i,kind:'reduce',id:'reservoir',quantity:-d});
  }
  const result=replay(source);assert.deepEqual(result.states.map(s=>s.q),q);source.states=result.states;
  return {source,replayEvidence:result.snapshots,orderComparisons:result.orderComparisons};
}
const negative=lift([0,10,5,2,10,5,2,12],'synthetic-rdw-v1-geometric-negative');
negative.result=triples(negative.source.states,[0,2,5,7]);
assert.deepEqual(negative.result.numericalH,[1002,1010]);
assert.deepEqual(negative.result.children.map(c=>c.directions),[['U'],['U'],['U']]);
assert.equal(negative.result.selectionCount,0);
negative.scope='strict three-range intersection does not imply alternating RDW; no original completion/core/level qualification, hence no original F2 counterexample';
save('negative-order-flow.json',negative);

const netMismatch=lift([0,10,5,10,0,6,0,10],'synthetic-rdw-v1-net-mismatch');
netMismatch.result=triples(netMismatch.source.states,[0,2,5,7]);
assert.deepEqual(netMismatch.result.children.map(c=>c.endpointDirection),['U','U','U']);
assert.deepEqual(netMismatch.result.children.map(c=>c.directions),[['U'],['D'],['U']]);
save('net-direction-mismatch.json',netMismatch);

const dual=lift([0,10,0,10,0,10,0],'synthetic-rdw-v1-two-selections');
dual.result=triples(dual.source.states,[0,2,4,6]);
assert.deepEqual(dual.result.children.map(c=>c.directions),[['U','D'],['U','D'],['U','D']]);
assert.deepEqual(dual.result.selections.map(s=>s.word),['UDU','DUD']);
save('two-selections.json',dual);

const bandExample=[0,10,5,8].map((q,event)=>({event,q:q+1000,w:W(q+1000)}));
const extremaBand=[1000,1010],narrowBand=[1005,1008];
const bandDependence={path:bandExample,fullRange:extremaBand,narrowBand,
  full:{U:pairs(bandExample,0,3,extremaBand,'U'),D:pairs(bandExample,0,3,extremaBand,'D')},
  narrow:{U:pairs(bandExample,0,3,narrowBand,'U'),D:pairs(bandExample,0,3,narrowBand,'D')}};
assert.equal(bandDependence.full.D.length,0);assert.ok(bandDependence.narrow.D.length>0);
save('band-dependence.json',bandDependence);

// Exhaustive truth table for the three nonempty per-child direction sets: 3^3 = 27 windows.
const setOptions=[['U'],['D'],['U','D']];const truthTable=[];
for(const a of setOptions)for(const b of setOptions)for(const c of setOptions) {
  const sets=[a,b,c];const choices=['UDU','DUD'].filter(word=>[...word].every((d,i)=>sets[i].includes(d)));
  truthTable.push({sets,choices,count:choices.length});
}
assert.equal(truthTable.length,27);
save('direction-set-truth-table.json',truthTable);
const summary={candidate:'RDW-v1',sourceEventsReplayed:replayed,sourceCoordinatePairComparisons:comparisonCount,
  necessaryTriplesChecked:main.length,strictTriplesChecked:mainSummary.length,assignmentsChecked:mainSummary.length*8,
  selectedDirectionWords:mainSummary.reduce((n,r)=>n+r.words.length,0),zeroSelectionTriples:mainSummary.filter(r=>!r.words.length).length,
  main:mainSummary,newSyntheticOrderEvents:{negative:negative.source.events.length,netMismatch:netMismatch.source.events.length,dual:dual.source.events.length},
  negativeStrictIntersection:true,negativeAlternatingSelections:0,dualAlternatingSelections:2,
  originalCompletionCertificatesIssued:0,originalZ3CertificatesIssued:0,originalParentCoresIssued:0,
  assertionsPassed:true,independentReview:false};
save('summary.json',summary);
for(const f of lock.files)assert.equal(sha(path.join(P,f.path)),f.sha256,f.path);
console.log(JSON.stringify(summary,null,2));
