import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';

const here=path.dirname(fileURLToPath(import.meta.url));
const P=path.resolve(here,'../..');
const out=process.argv[2]??path.join(here,'results-v1');
const hash=f=>crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const json=f=>JSON.parse(fs.readFileSync(f,'utf8'));
const lock=json(path.join(here,'input-lock.json'));
for(const f of lock.files)assert.equal(hash(path.join(P,f.path)),f.sha256,f.path);
fs.mkdirSync(out,{recursive:true});
const save=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n');
const ids=(a,b)=>Array.from({length:Math.max(0,b-a+1)},(_,i)=>a+i);
const events=(a,b)=>ids(a+1,b);
const hull=xs=>[Math.min(...xs),Math.max(...xs)];
const meet=rs=>[Math.max(...rs.map(r=>r[0])),Math.min(...rs.map(r=>r[1]))];
const sign=x=>x>0?'Up':x<0?'Down':null;
const gcd=(a,b)=>b===0n?a:gcd(b,a%b);
function W(q){const n=1000n+102n*BigInt(q),d=10n+BigInt(q),g=gcd(n,d);return `${n/g}/${d/g}`;}
function cmp(a,b){const [an,ad]=a.split('/').map(BigInt),[bn,bd]=b.split('/').map(BigInt);const d=an*bd-bn*ad;return d>0n?1:d<0n?-1:0;}

function replay(source){
  const book=new Map([[source.initial.bid.id,{...source.initial.bid,side:'bid'}],[source.initial.ask.id,{...source.initial.ask,side:'ask'}]]);
  const states=[],snapshots=[];
  function observe(event){
    const orders=[...book.values()];
    for(const o of orders)assert.ok(Number.isSafeInteger(o.quantity)&&o.quantity>0);
    const bids=orders.filter(o=>o.side==='bid'),asks=orders.filter(o=>o.side==='ask');
    const bid=Math.max(...bids.map(o=>o.price)),ask=Math.min(...asks.map(o=>o.price));
    assert.equal(bid,100);assert.equal(ask,102);
    const q=bids.filter(o=>o.price===bid).reduce((s,o)=>s+o.quantity,0);
    const askQ=asks.filter(o=>o.price===ask).reduce((s,o)=>s+o.quantity,0);
    assert.equal(askQ,10);states.push({event,q,w:W(q)});
    snapshots.push({event,bestBid:bid,bestAsk:ask,bestBidQuantity:q,bestAskQuantity:askQ,orders:orders.map(o=>({id:o.id,side:o.side,price:o.price,quantity:o.quantity}))});
  }
  observe(0);
  for(const [i,e] of source.events.entries()){
    assert.equal(e.seq,i+1);assert.ok(Number.isSafeInteger(e.quantity)&&e.quantity>0);
    if(e.kind==='add'){assert.ok(!book.has(e.id));assert.ok(['bid','ask'].includes(e.side));book.set(e.id,{...e});}
    else {assert.equal(e.kind,'reduce');const o=book.get(e.id);assert.ok(o&&o.quantity>e.quantity);o.quantity-=e.quantity;}
    observe(e.seq);
  }
  assert.deepEqual(states,source.states);
  for(const a of states)for(const b of states)assert.equal(Math.sign(a.q-b.q),cmp(a.w,b.w));
  return {states,snapshots};
}

function rawLegs(states){
  const done=[];let start=0,direction=null;
  for(let i=1;i<states.length;i++){
    const d=sign(states[i].q-states[i-1].q);
    if(!d)continue;
    if(direction===null){direction=d;continue;}
    if(d!==direction){
      const end=i-1;
      done.push({id:done.length,start,end,direction,range:hull(states.slice(start,end+1).map(s=>s.q)),knownAt:i});
      start=end;direction=d;
    }
  }
  return {completed:done,pending:{start,end:states.length-1,direction,range:hull(states.slice(start).map(s=>s.q))}};
}

// Only raw seed / extension / leave-return stages are reconstructed. There is
// no RC49 outer-to-outer move classifier or completion Boolean in this parser.
function rawCoreSeams(states){
  const legs=rawLegs(states).completed,closed=[];let cursor=0,pending=null;
  while(cursor+2<legs.length){
    const seed=legs.slice(cursor,cursor+3),core=meet(seed.map(l=>l.range));
    if(core[0]>=core[1]){cursor++;continue;}
    const members=seed.map(l=>l.id);let j=cursor+3,exit=null;
    while(j+1<legs.length){
      const L=legs[j],R=legs[j+1];
      assert.notEqual(L.direction,R.direction);assert.equal(L.end,R.start);
      const side=R.range[0]>core[1]?'Up':R.range[1]<core[0]?'Down':null;
      if(side){
        const startsOnCoreSide=side==='Up'?states[L.start].q<=core[1]:states[L.start].q>=core[0];
        assert.ok(startsOnCoreSide);assert.equal(L.direction,side);
        exit={departure:L.id,return:R.id,returnRange:R.range,side,knownAt:R.knownAt};break;
      }
      members.push(L.id,R.id);j+=2;
    }
    const start=seed[0].start,end=legs[members.at(-1)].end;
    const base={id:`K${closed.length}`,level:0,seed:seed.map(l=>l.id),core,members,sourceStart:start,sourceEnd:end,
      outer:hull(states.slice(start,end+1).map(s=>s.q)),bornAt:seed.at(-1).knownAt,startDirection:seed[0].direction};
    if(!exit){pending=base;break;}
    const L=legs[exit.departure],R=legs[exit.return];
    const evidence={...base,sealedAt:exit.knownAt,exit,
      sourceEvents:events(start,end),memberStateSupport:ids(start,end),
      stages:{before:L.start,peak:L.end,after:R.end},
      departure:{...L,ownedEvents:events(L.start,L.end),stateSupport:ids(L.start,L.end)},
      return:{...R,ownedEvents:events(R.start,R.end),stateSupport:ids(R.start,R.end)},
      evidenceKnownAt:Math.max(...members.map(i=>legs[i].knownAt),L.knownAt,R.knownAt),
      confirmationSupport:ids(L.start,R.knownAt),
      originalCoreIdentity:null,originalLowerMoveIdentity:null,originalPCompletion:null};
    closed.push(evidence);cursor=R.id;
  }
  return {legs,closed,pending};
}

function partition(states,cores,cuts,phases,label){
  const blocks=[];
  for(let i=0;i<3;i++){
    const a=cuts[i],b=cuts[i+1];assert.ok(a<b);
    const source=states.slice(a,b+1),range=hull(source.map(s=>s.q));
    const complete=cores.filter(c=>a<=c.sourceStart&&c.sourceEnd<=b);
    const partial=cores.filter(c=>(c.sourceStart<a&&a<c.sourceEnd)||(c.sourceStart<b&&b<c.sourceEnd));
    const own=events(a,b);
    blocks.push({id:`${label}-P${i}`,start:a,end:b,ownedEvents:own,priceSupport:ids(a,b),states:source,
      actualStart:source[0],actualEnd:source.at(-1),qRange:range,wRange:range.map(W),
      ownedCoreIDs:complete.map(c=>c.id),partialCoreIDs:partial.map(c=>c.id),
      candidateLevel:0,kindIfComplete:complete.length===1?'P':null,conceptDirection:null,
      isStd:Math.min(source[0].q,source.at(-1).q)===range[0]&&Math.max(source[0].q,source.at(-1).q)===range[1],
      phase:phases[i],brsEndpointPass:b===cores[i].stages.peak,
      localEvidenceKnownAt:cores[i].evidenceKnownAt,originalCompletionEvidence:null,originalSameLevelEvidence:null});
  }
  const H=meet(blocks.map(b=>b.qRange)),strict=H[0]<H[1];
  const coverage=[...blocks.flatMap(b=>b.ownedEvents),...events(cuts[3],states.length-1)];
  assert.deepEqual(coverage,events(0,states.length-1));
  for(let i=0;i<2;i++)assert.deepEqual(blocks[i].actualEnd,blocks[i+1].actualStart);
  const ownership=blocks.every((b,i)=>b.partialCoreIDs.length===0&&b.ownedCoreIDs.length===1&&b.ownedCoreIDs[0]===`K${i}`);
  const pairSets=[];
  for(const b of blocks){
    const ps={U:[],D:[]};
    if(strict)for(let i=b.start;i<=b.end;i++)for(let j=i+1;j<=b.end;j++){
      for(const dir of ['U','D']){
        const u=dir==='U',ok=u?states[i].q<=H[0]&&states[j].q>=H[1]:states[i].q>=H[1]&&states[j].q<=H[0];
        const rw=u?cmp(states[i].w,W(H[0]))<=0&&cmp(states[j].w,W(H[1]))>=0:cmp(states[i].w,W(H[1]))>=0&&cmp(states[j].w,W(H[0]))<=0;
        assert.equal(ok,rw);if(ok)ps[dir].push([i,j]);
      }
    }
    pairSets.push(ps);b.rdwWitnesses=ps;b.rdwDirections=['U','D'].filter(d=>ps[d].length>0);
  }
  const words=[];
  for(const word of ['UDU','DUD'])if(strict&&blocks.every((b,i)=>b.rdwDirections.includes(word[i]))){
    const tuples=[];
    for(const a of pairSets[0][word[0]])for(const b of pairSets[1][word[1]])for(const c of pairSets[2][word[2]])tuples.push([a,b,c]);
    words.push({word,witnessTupleCount:tuples.length,witnessTuples:tuples});
  }
  return {label,cuts,phases,blocks,wholeCoreOwnership:ownership,qNumericalIntersection:H,strictIntersection:strict,
    qParentRange:hull(states.slice(0,cuts[3]+1).map(s=>s.q)),rdwWords:words,
    matchesBRS:blocks.every(b=>b.brsEndpointPass),sourceRangesReadableAt:cuts[3],
    localEvidenceReadableAt:Math.max(...blocks.map(b=>b.localEvidenceKnownAt)),
    originalZ3Evidence:null,originalParentCore:null,originalCompletionEvidence:null,
    sameLevelSourceEndpointsConnect:true,originalSameLevelConsecutive:null,
    tail:{start:cuts[3],end:states.length-1,ownedEvents:events(cuts[3],states.length-1),states:states.slice(cuts[3]),
      qRange:hull(states.slice(cuts[3]).map(s=>s.q))},semanticEligibleAt:null};
}

function online(states){
  const scan=rawCoreSeams(states),proposals=[];let start=0;
  for(const c of scan.closed){
    const end=c.stages.peak;
    // Whole core belongs to this interval; raw boundary joins actual sources.
    assert.ok(start<=c.sourceStart&&c.sourceEnd<=end);
    proposals.push({core:c.id,start,end,sourceEvents:events(start,end),qRange:hull(states.slice(start,end+1).map(s=>s.q)),
      evidenceKnownAt:c.evidenceKnownAt,relationshipPublishedAt:c.evidenceKnownAt,semanticEligibleAt:null});
    start=end;
  }
  assert.deepEqual([...proposals.flatMap(p=>p.sourceEvents),...events(start,states.length-1)],events(0,states.length-1));
  return {proposals,tail:{start,end:states.length-1,sourceEvents:events(start,states.length-1)}};
}

function localView(states,c){
  const seed=c.seed.map(i=>rawLegs(states).completed[i]);
  const points=[seed[0].start,seed[0].end,seed[1].end,seed[2].end,c.stages.peak,c.stages.after,c.evidenceKnownAt];
  const qs=points.map(i=>states[i].q),values=[...new Set(qs)].sort((a,b)=>a-b),ranks=qs.map(q=>values.indexOf(q));
  return {core:c.id,eventIndices:points,q:qs,w:qs.map(W),ranks,
    canonicalPhasePositions:{before:3,peak:4,after:5},seedDirection:c.startDirection,exitSide:c.exit.side,
    rawLocalEvents:events(seed[0].start,c.evidenceKnownAt),
    discardedEqualInteriorEvents:ids(seed[0].start+1,seed[0].end-1).filter(i=>states[i].q===states[i-1].q),
    scope:'ordinal local skeleton only; earlier context and absolute ID are deliberately not inputs'};
}

const summaries=[];let totalEvents=0,totalStates=0,totalPartitions=0,totalPrefixes=0;
for(const name of ['same_up_counterexample','same_down_mirror']){
  const sourceFile=`stage49/candidate-evidence/results-v1/${name}.json`;
  const old=json(path.join(P,sourceFile)),stage50=json(path.join(P,`stage50/candidate-evidence/results-v1/${name}.json`));
  assert.deepEqual(stage50.source,old.source);
  const {states,snapshots}=replay(old.source),scan=rawCoreSeams(states);
  totalEvents+=old.source.events.length;totalStates+=states.length;
  assert.deepEqual(scan.legs,old.result.legs.completed);
  assert.equal(scan.closed.length,old.result.coreScan.completed.length);
  for(const [i,c] of scan.closed.entries()){
    const frozen=old.result.coreScan.completed[i];
    for(const key of ['id','level','seed','core','members','sourceStart','sourceEnd','outer','bornAt','startDirection','sealedAt'])assert.deepEqual(c[key],frozen[key],`${name}:${c.id}:${key}`);
    for(const key of ['departure','return','returnRange','side','knownAt'])assert.deepEqual(c.exit[key],frozen.exit[key]);
  }
  const phases=['before','peak','after'],attempts=[];
  for(const a of phases)for(const b of phases)for(const c of phases){
    const ps=[a,b,c],cuts=[0,...ps.map((p,i)=>scan.closed[i].stages[p])];
    attempts.push(partition(states,scan.closed,cuts,ps,ps.join('-')));totalPartitions++;
  }
  const uniform=phases.map(p=>attempts.find(a=>a.phases.every(v=>v===p)));
  const fixed=attempts.find(a=>a.cuts.join(',')==='0,5,8,12');assert.ok(fixed);
  assert.deepEqual(fixed.phases,['peak','before','before']);assert.equal(fixed.matchesBRS,false);
  const peak=uniform.find(p=>p.phases[0]==='peak');assert.deepEqual(peak.cuts,[0,5,9,13]);
  assert.ok(peak.wholeCoreOwnership&&peak.strictIntersection&&peak.matchesBRS);
  assert.equal(uniform.find(p=>p.phases[0]==='before').strictIntersection,false);
  assert.equal(uniform.find(p=>p.phases[0]==='after').wholeCoreOwnership,false);
  assert.equal(attempts.filter(a=>a.wholeCoreOwnership).length,8);
  assert.equal(attempts.filter(a=>a.wholeCoreOwnership&&a.strictIntersection).length,4);
  assert.equal(attempts.filter(a=>a.wholeCoreOwnership&&a.strictIntersection&&a.rdwWords.length).length,3);
  assert.equal(peak.blocks.filter(b=>b.isStd).length,0);
  const onlinePrefixes=[];let previous=[];
  for(let t=0;t<states.length;t++){
    const o=online(states.slice(0,t+1));
    assert.deepEqual(o.proposals.slice(0,previous.length),previous);
    for(const p of o.proposals)assert.ok(p.end<p.evidenceKnownAt&&p.evidenceKnownAt<=t);
    onlinePrefixes.push({t,...o});previous=o.proposals;totalPrefixes++;
  }
  const firstThreeAt=onlinePrefixes.find(p=>p.proposals.length>=3)?.t;
  assert.equal(firstThreeAt,15);assert.deepEqual(onlinePrefixes[15].proposals.slice(0,3).map(p=>p.end),[5,9,13]);
  peak.relationshipOnlinePartitionPublicationAt=firstThreeAt;
  fixed.relationshipOnlinePartitionPublicationAt=null;
  const l0=localView(states,scan.closed[0]),l2=localView(states,scan.closed[2]);
  assert.deepEqual(l0.ranks,l2.ranks);assert.equal(l0.seedDirection,l2.seedDirection);assert.equal(l0.exitSide,l2.exitSide);
  const map=l0.q.map((x,i)=>({from:x,to:l2.q[i]})).sort((a,b)=>a.from-b.from);
  for(let i=1;i<map.length;i++)assert.ok(map[i-1].from<map[i].from&&map[i-1].to<map[i].to);
  const equivariance={localViews:[l0,l2],strictlyIncreasingMapKnots:map,
    targetEndpointStages:['peak','before'],targetEndpointPositions:[4,3],
    contradiction:'equal ordered local skeletons require equal selected phase under a uniform ordinal local selector',
    minimality:'two compared local inputs are necessary and sufficient to witness nonfunctionality; no global event-length minimality claimed',
    doesNotExclude:'rules depending on earlier context, root boundary, absolute time, price metric, or a separately sourced seam policy'};
  const shifts=[];
  for(let e=1;e<=old.source.events.length;e++){
    const owner=(record)=>{const i=record.blocks.findIndex(b=>b.ownedEvents.includes(e));return i<0?'tail':`K${i}-piece`;};
    if(owner(fixed)!==owner(peak))shifts.push({event:e,oldOwner:owner(fixed),newOwner:owner(peak),sourceEvent:old.source.events[e-1],resultState:states[e]});
  }
  assert.deepEqual(shifts.map(s=>s.event),[9,13]);
  const record={schema:'stage52-brs-v1',name,sourceFile,sourceSHA256:hash(path.join(P,sourceFile)),
    source:old.source,replayedStates:states,orderSnapshots:snapshots,rawLegs:scan.legs,rawCoreSeams:scan.closed,
    fixedPartition:fixed,uniformPolicies:uniform,allPhasePartitions:attempts,onlinePrefixes,
    ownershipTransport:shifts,ordinalLocalObstruction:equivariance,
    scope:'raw evidence relation and conditional semantic candidate; no original completion, level or Z3 proof'};
  save(`${name}.json`,record);
  const small=p=>({cuts:p.cuts,phases:p.phases,coreOwnership:p.wholeCoreOwnership,partialCores:p.blocks.map(b=>b.partialCoreIDs),
    qRanges:p.blocks.map(b=>b.qRange),H:p.qNumericalIntersection,strict:p.strictIntersection,
    directionSets:p.blocks.map(b=>b.rdwDirections),words:p.rdwWords.map(w=>w.word),
    witnessTupleCounts:p.rdwWords.map(w=>w.witnessTupleCount),matchesBRS:p.matchesBRS,
    sourceRangesReadableAt:p.sourceRangesReadableAt,localEvidenceReadableAt:p.localEvidenceReadableAt,
    relationshipOnlinePartitionPublicationAt:p.relationshipOnlinePartitionPublicationAt??null,semanticEligibleAt:null});
  summaries.push({name,events:old.source.events.length,states:states.length,phasePartitions:attempts.length,
    wholeCoreOwnershipPartitions:8,strictWholeCorePartitions:4,strictRDWWholeCorePartitions:3,
    fixed:small(fixed),uniform:uniform.map(small),ordinalRanks:l0.ranks,
    changedEventOwners:shifts.map(s=>s.event),prefixes:onlinePrefixes.length,relationshipFirstThreeAt:firstThreeAt});
}
assert.equal(totalEvents,80);assert.equal(totalStates,82);assert.equal(totalPartitions,54);assert.equal(totalPrefixes,82);
assert.deepEqual(summaries[0].uniform.find(x=>x.phases[0]==='peak').qRanges,[[1000,1140],[1100,1140],[1135,1240]]);
assert.deepEqual(summaries[0].uniform.find(x=>x.phases[0]==='peak').directionSets,[['U'],['U','D'],['U','D']]);
assert.deepEqual(summaries[0].uniform.find(x=>x.phases[0]==='peak').words,['UDU']);
assert.deepEqual(summaries[1].uniform.find(x=>x.phases[0]==='peak').words,['DUD']);
const summary={candidate:'BRS-v1',inputLockSHA256:hash(path.join(here,'input-lock.json')),totalEvents,totalStates,totalPartitions,totalPrefixes,
  cases:summaries,assertionsPassed:true,independentReview:false,
  semanticResult:'fixed partition fails BRS; uniform peak repartition passes raw/core-ownership/RDW checks only; semantic completion remains open'};
save('summary.json',summary);console.log(JSON.stringify(summary,null,2));
