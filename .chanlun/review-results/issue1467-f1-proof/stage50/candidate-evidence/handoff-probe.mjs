import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const P=path.resolve(here,'../..');
const inputDir=path.join(P,'stage49/candidate-evidence/results-v1');
const out=process.argv[2]??path.join(here,'results-v1');
fs.mkdirSync(out,{recursive:true});
const hull=xs=>[Math.min(...xs),Math.max(...xs)];
const inter=rs=>[Math.max(...rs.map(r=>r[0])),Math.min(...rs.map(r=>r[1]))];
const range=(a,b)=>Array.from({length:b-a},(_,i)=>a+i+1);
const rel=(a,b)=>b[0]>a[1]?'Up':b[1]<a[0]?'Down':'Overlap';
const sha=f=>crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
function W(q) {let a=1000n+102n*BigInt(q),b=10n+BigInt(q),x=a,y=b;while(y)[x,y]=[y,x%y];return `${a/x}/${b/x}`;}
const wr=r=>r.map(W);
const sum=[];
let cutPairsTested=0,jointEventEdgesTested=0;

function replay(s) {
  const book=new Map([[s.initial.bid.id,{...s.initial.bid,side:'bid'}],[s.initial.ask.id,{...s.initial.ask,side:'ask'}]]);
  const qs=[s.initial.bid.quantity];
  for(const e of s.events) {
    if(e.kind==='add') {assert.ok(!book.has(e.id));book.set(e.id,{...e});}
    else {const o=book.get(e.id);assert.equal(e.kind,'reduce');assert.ok(o.quantity>e.quantity);o.quantity-=e.quantity;}
    const bids=[...book.values()].filter(o=>o.side==='bid'),asks=[...book.values()].filter(o=>o.side==='ask');
    assert.equal(Math.max(...bids.map(o=>o.price)),100);assert.equal(Math.min(...asks.map(o=>o.price)),102);
    qs.push(bids.filter(o=>o.price===100).reduce((a,o)=>a+o.quantity,0));
  }
  assert.deepEqual(qs,s.states.map(x=>x.q));
  assert.deepEqual(qs.map(W),s.states.map(x=>x.w));
  return qs;
}

for(const name of ['same_up_counterexample','same_down_mirror']) {
  const file=path.join(inputDir,name+'.json'),old=JSON.parse(fs.readFileSync(file,'utf8'));
  const q=replay(old.source),all=old.result.coreScan.completed,legs=old.result.legs.completed;
  const core=c=>({...c,qCore:c.core,wCore:wr(c.core),qOuter:c.outer,wOuter:wr(c.outer),
    sourceEvents:range(c.sourceStart,c.sourceEnd),actualStart:q[c.sourceStart],actualEnd:q[c.sourceEnd],
    initialIdentityScope:'frozen RC49 instance, original center qualification not assumed proved'});
  const decorated=all.map(core);
  function known(t) {
    const cs=decorated.filter(c=>c.sealedAt<=t);
    for(const c of cs) {
      assert.ok(c.sourceEnd<=t);
      for(const j of [...c.members,c.exit.departure,c.exit.return]) assert.ok(legs[j].knownAt<=c.sealedAt);
      assert.deepEqual(c.outer,hull(q.slice(c.sourceStart,c.sourceEnd+1)));
    }
    return cs;
  }
  function block(a,b,cs) {
    const partial=cs.filter(c=>(c.sourceStart<a&&a<c.sourceEnd)||(c.sourceStart<b&&b<c.sourceEnd));
    const own=cs.filter(c=>a<=c.sourceStart&&c.sourceEnd<=b);
    const r=hull(q.slice(a,b+1));
    let type=null;
    if(own.length===1)type='P';
    if(own.length>1) {
      const d=rel(own[0].outer,own[1].outer);
      if(d!=='Overlap'&&own.slice(1).every((c,i)=>rel(own[i].outer,c.outer)===d))type=d==='Up'?'U':'D';
    }
    return {level:0,start:a,end:b,sourceEvents:range(a,b),actualStart:q[a],actualEnd:q[b],
      qRange:r,wRange:wr(r),priceSupportEvents:Array.from({length:b-a+1},(_,i)=>a+i),
      ownedCoreIDs:own.map(c=>c.id),coreEvidence:own,type,
      conceptDirection:type==='U'?'Up':type==='D'?'Down':null,
      endpointDirectionDiagnostic:q[b]>q[a]?'Up':q[b]<q[a]?'Down':null,
      partialCoreIDs:partial.map(c=>c.id),
      necessaryCoreClassValid:partial.length===0&&own.length>0&&type!==null,
      originalCompletionEvidence:null,dirForZ3Evidence:null};
  }
  function three(a,b,t) {
    const cs=known(t),regional=cs.filter(c=>a<=c.sourceStart&&c.sourceEnd<=b);
    const attempts=[],necessary=[];
    for(let x=a+1;x<b;x++)for(let y=x+1;y<b;y++) {
      cutPairsTested++;
      const ms=[block(a,x,cs),block(x,y,cs),block(y,b,cs)],ids=ms.flatMap(m=>m.ownedCoreIDs);
      const coreValid=ms.every(m=>m.necessaryCoreClassValid)&&new Set(ids).size===ids.length&&
        ids.join(',')===regional.map(c=>c.id).join(',');
      const z=inter(ms.map(m=>m.qRange)),strict=z[0]<z[1];
      attempts.push({cuts:[a,x,y,b],coreCounts:ms.map(m=>m.ownedCoreIDs.length),
        straddled:ms.map(m=>m.partialCoreIDs),types:ms.map(m=>m.type),coreValid,strictIntersection:strict});
      if(coreValid) {
        const ds=ms.map(m=>m.endpointDirectionDiagnostic);
        necessary.push({cuts:[a,x,y,b],children:ms,parentLevel:1,qParentCore:z,wParentCore:strict?wr(z):null,
          qParentFullRange:hull(q.slice(a,b+1)),wParentFullRange:wr(hull(q.slice(a,b+1))),
          strictCore:strict,endpointDiagnosticAlternation:ds.every(d=>d!==null)&&ds[0]!==ds[1]&&ds[1]!==ds[2],
          originalF2Qualification:'unproved: children completion and P direction bridge absent'});
      }
    }
    return {start:a,end:b,knownAt:t,sourceEvents:range(a,b),regionalCoreIDs:regional.map(c=>c.id),
      arbitraryCutPairs:attempts.length,attempts,necessary,strictNecessary:necessary.filter(x=>x.strictCore)};
  }
  function joint(end,t,enforceSuccession) {
    const cs=known(t),edges=Array.from({length:end+1},()=>[]),rejections=[];
    for(let a=0;a<end;a++)for(let b=a+1;b<=end;b++) {
      jointEventEdgesTested++;
      const m=block(a,b,cs);
      if(!m.necessaryCoreClassValid)continue;
      const last=cs.findIndex(c=>c.id===m.ownedCoreIDs.at(-1)),witness=cs[last+1];
      const relation=witness?rel(cs[last].outer,witness.outer):null;
      const gate=witness&&(m.type==='P'?relation==='Overlap':relation!==m.conceptDirection);
      if(!gate) {rejections.push({start:a,end:b,type:m.type,coreIDs:m.ownedCoreIDs,relation,
        reason:witness?'RC49 local end gate fails':'no sealed successor'});continue;}
      m.endGate={witnessCore:witness.id,relation,knownAt:witness.sealedAt,
        scope:'necessary numerical gate only; no original completion or arbitrary endpoint certificate'};
      edges[a].push(m);
    }
    const paths=[];
    function dfs(a,acc) {
      if(a===end){paths.push(acc);return;}
      for(const e of edges[a]) {
        const prev=acc.at(-1);
        if(enforceSuccession&&prev&&prev.type===e.type&&(e.type==='U'||e.type==='D'))continue;
        dfs(e.end,[...acc,e]);
      }
    }
    dfs(0,[]);
    for(const p of paths) {
      assert.deepEqual(p.flatMap(m=>m.sourceEvents),range(0,end));
      assert.equal(new Set(p.flatMap(m=>m.ownedCoreIDs)).size,p.reduce((n,m)=>n+m.ownedCoreIDs.length,0));
      assert.deepEqual(p.flatMap(m=>m.ownedCoreIDs),cs.filter(c=>c.sourceEnd<=end).map(c=>c.id));
    }
    return {knownAt:t,consumeThrough:end,enforceSuccession,pathCount:paths.length,
      paths:paths.map(p=>({cuts:[0,...p.map(m=>m.end)],types:p.map(m=>m.type),objects:p})),
      allNumericallyEligibleEdges:edges.flat(),localGateRejected:rejections,
      remainingRawTail:{start:end,end:t,sourceEvents:range(end,t),qRange:hull(q.slice(end,t+1)),
        wRange:wr(hull(q.slice(end,t+1))),rawStates:old.source.states.slice(end,t+1),
        remainingSealedCoreIDs:cs.filter(c=>c.sourceStart>=end).map(c=>c.id)}};
  }
  const snapshots=[];
  for(const [t,i,j,consume] of [[15,1,2,8],[23,3,4,16],[31,5,6,24]]) {
    const cs=known(t),left=cs[i],right=cs[j];
    assert.ok(right.core[0]>left.core[1]||right.core[1]<left.core[0]);
    assert.equal(rel(left.outer,right.outer),'Overlap');
    const a=left.sourceStart,b=right.sourceEnd;
    const local=three(a,b,t),widened=three(i===1?0:cs[i-1].sourceStart,b,t);
    assert.deepEqual(local.regionalCoreIDs,[left.id,right.id]);
    assert.equal(local.necessary.length,0);
    assert.ok(widened.strictNecessary.length>0);
    const free=joint(consume,t,false),strict=joint(consume,t,true);
    assert.equal(strict.pathCount,t===15?1:0);
    assert.equal(free.pathCount,t===15?1:t===23?2:4);
    const proposal={id:`EHL-${left.id}-${right.id}`,level:1,formedDiagnosticAt:t,
      sourceStart:a,sourceEnd:b,sourceEvents:range(a,b),actualStart:q[a],actualEnd:q[b],
      qFullRange:hull(q.slice(a,b+1)),wFullRange:wr(hull(q.slice(a,b+1))),
      pairCoreIDs:[left.id,right.id],pairInstanceEvidence:[left,right],
      qPairOuterIntersection:inter([left.outer,right.outer]),wPairOuterIntersection:wr(inter([left.outer,right.outer])),
      fixedF2Core:null,qualifiedChildren:[],completionEvidence:null,
      result:'local exact-pair-hull bridge impossible under frozen complete level0 catalogue',
      obstruction:{requiredDistinctOwnedLevel0Cores:3,availableDistinctOwnedLevel0Cores:2},
      outsideLocalView:{leftEvents:range(0,a),rightEvents:range(b,t)}};
    snapshots.push({t,availableCoreIDs:cs.map(c=>c.id),localLift:proposal,localAllCuts:local,
      leftWidenedNecessary:widened,jointWithoutSuccession:free,jointWithSuccession:strict});
  }
  const partial=joint(8,23,true);
  assert.equal(partial.pathCount,1);
  assert.deepEqual(partial.paths[0].types,[name==='same_up_counterexample'?'U':'D']);
  const record={schema:'stage50-handoff-bridges/v1',name,sourceFile:path.relative(P,file),sourceSHA256:sha(file),
    source:old.source,scope:'frozen instances reused; no original Complete bool supplied',snapshots,
    partialAt23:partial,originalCompletedOutputsIssued:[],
    status:'two explicit bridge classes refuted; left-expanded geometric alternatives remain unqualified'};
  fs.writeFileSync(path.join(out,name+'.json'),JSON.stringify(record,null,2)+'\n');
  sum.push({name,events:old.source.events.length,snapshots:snapshots.map(s=>({knownAt:s.t,
    localPair:s.localLift.pairCoreIDs,localSource:[s.localLift.sourceStart,s.localLift.sourceEnd],
    localCutPairs:s.localAllCuts.arbitraryCutPairs,localNecessaryTriples:s.localAllCuts.necessary.length,
    widenedSource:[s.leftWidenedNecessary.start,s.leftWidenedNecessary.end],
    widenedNecessaryTriples:s.leftWidenedNecessary.necessary.length,widenedStrictTriples:s.leftWidenedNecessary.strictNecessary.length,
    jointConsumeThrough:s.jointWithSuccession.consumeThrough,jointWithoutB4:s.jointWithoutSuccession.pathCount,
    jointWithB4:s.jointWithSuccession.pathCount})),partialAt23:{consumeThrough:8,solutions:partial.pathCount}});
}
const summary={candidateClasses:['EHL50-v1','JH50-v1'],sourceEventsReplayed:80,
  cutPairsTested,jointEventEdgesTested,cases:sum,
  assertionsPassed:true,independentReview:false};
fs.writeFileSync(path.join(out,'summary.json'),JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary,null,2));
