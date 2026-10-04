import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';

const here=path.dirname(fileURLToPath(import.meta.url)),P=path.resolve(here,'../..');
const out=process.argv[2]??path.join(here,'results-v1');
const json=f=>JSON.parse(fs.readFileSync(f,'utf8'));
const hash=f=>crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const lock=json(path.join(here,'additional-input-lock.json'));
for(const f of lock.files)assert.equal(hash(path.join(P,f.path)),f.sha256,f.path);
fs.mkdirSync(out,{recursive:true});
const save=(n,x)=>fs.writeFileSync(path.join(out,n),JSON.stringify(x,null,2)+'\n');
const hull=a=>[Math.min(...a),Math.max(...a)];
const intersect=a=>[Math.max(...a.map(v=>v[0])),Math.min(...a.map(v=>v[1]))];
const events=(a,b)=>Array.from({length:b-a},(_,i)=>a+i+1);

// Reconstruct every prefix from the actual event IDs and starting order book.
// This function never receives final core metadata or final replayed states.
function prefixBook(initial,inputEvents){
  const orders=Object.fromEntries([initial.bid,initial.ask].map((o,i)=>[o.id,{...o,side:i?'ask':'bid'}]));
  const q=[];
  function observe(){
    const all=Object.values(orders);for(const x of all)assert.ok(x.quantity>0&&Number.isSafeInteger(x.quantity));
    const bids=all.filter(o=>o.side==='bid'),asks=all.filter(o=>o.side==='ask');
    assert.equal(Math.max(...bids.map(o=>o.price)),100);assert.equal(Math.min(...asks.map(o=>o.price)),102);
    assert.equal(asks.filter(o=>o.price===102).reduce((n,o)=>n+o.quantity,0),10);
    q.push(bids.filter(o=>o.price===100).reduce((n,o)=>n+o.quantity,0));
  }
  observe();
  for(let i=0;i<inputEvents.length;i++){
    const e=inputEvents[i];assert.equal(e.seq,i+1);
    if(e.kind==='add'){assert.ok(!(e.id in orders));orders[e.id]={...e};}
    else {assert.equal(e.kind,'reduce');assert.ok(orders[e.id]);assert.ok(orders[e.id].quantity>e.quantity);orders[e.id].quantity-=e.quantity;}
    observe();
  }
  return q;
}

// Independent grouping of nonzero edge signs into runs; flat events survive in
// run endpoints and therefore in event ownership, but do not create new turns.
function signedRuns(q){
  const moves=[];
  for(let i=1;i<q.length;i++)if(q[i]!==q[i-1])moves.push({i,d:Math.sign(q[i]-q[i-1])});
  const changes=moves.filter((m,j)=>j>0&&m.d!==moves[j-1].d);
  const ends=changes.map(m=>m.i-1),starts=[0,...ends.slice(0,-1)];
  return ends.map((b,j)=>({a:starts[j],b,lo:Math.min(...q.slice(starts[j],b+1)),hi:Math.max(...q.slice(starts[j],b+1)),known:changes[j].i}));
}

function parse(q){
  const ls=signedRuns(q),emitted=[];let cursor=0,left=0;
  while(cursor+2<ls.length){
    const first=ls.slice(cursor,cursor+3),lo=Math.max(...first.map(x=>x.lo)),hi=Math.min(...first.map(x=>x.hi));
    if(lo>=hi){cursor++;continue;}
    let k=cursor+3,found=null;
    for(;k+1<ls.length;k+=2){
      const r=ls[k+1];
      if(r.lo>hi||r.hi<lo){found={l:ls[k],r,returnIndex:k+1};break;}
    }
    if(!found)break;
    const end=found.l.b;
    emitted.push({core:`K${emitted.length}`,start:left,end,sourceEvents:events(left,end),qRange:hull(q.slice(left,end+1)),
      evidenceKnownAt:found.r.known,relationshipPublishedAt:found.r.known,semanticEligibleAt:null});
    left=end;cursor=found.returnIndex;
  }
  return emitted;
}

const names=['same_up_counterexample','same_down_mirror','p_completion_overlap','p_completion_disjoint'];
const prefixReports=[],cases={};let sourceEvents=0,prefixCount=0,replayedPrefixEventOccurrences=0;
for(const name of names){
  const file=path.join(P,`stage49/candidate-evidence/results-v1/${name}.json`),old=json(file);
  sourceEvents+=old.source.events.length;
  const observations=[];let prior=[];
  for(let t=0;t<=old.source.events.length;t++){
    const q=prefixBook(old.source.initial,old.source.events.slice(0,t)),p=parse(q);
    assert.deepEqual(q,old.source.states.slice(0,t+1).map(s=>s.q));
    assert.deepEqual(p.slice(0,prior.length),prior);
    for(const e of p)assert.ok(e.evidenceKnownAt<=t);
    observations.push({t,proposals:p});prior=p;prefixCount++;replayedPrefixEventOccurrences+=t;
  }
  if(name.startsWith('same_')){
    const main=json(path.join(here,`results-v1/${name}.json`));
    for(const x of observations)assert.deepEqual(x.proposals,main.onlinePrefixes[x.t].proposals);
    assert.equal(observations.find(x=>x.proposals.length>=3).t,15);
  }
  cases[name]={old,observations};
  prefixReports.push({name,sourceEvents:old.source.events.length,prefixes:observations.length,
    firstEmissionAt:observations.find(x=>x.proposals.length)?.t??null,
    firstThreeAt:observations.find(x=>x.proposals.length>=3)?.t??null,
    finalProposals:observations.at(-1).proposals});
}
const a=cases.p_completion_overlap,b=cases.p_completion_disjoint;
assert.deepEqual(a.old.source.initial,b.old.source.initial);
assert.deepEqual(a.old.source.events.slice(0,7),b.old.source.events.slice(0,7));
assert.notDeepEqual(a.old.source.events[7],b.old.source.events[7]);
assert.deepEqual(a.observations[7].proposals,b.observations[7].proposals);
const common=a.observations[7].proposals;
assert.equal(common.length,1);assert.equal(common[0].end,5);assert.equal(common[0].evidenceKnownAt,7);
for(const c of [a,b])for(const x of c.observations.filter(x=>x.t>=7))assert.deepEqual(x.proposals.slice(0,1),common);
const branches={sharedEventsThrough:7,firstDifferentEvent:8,commonEvidence:common,
  sourceNames:['p_completion_overlap','p_completion_disjoint'],
  divergentEvents:[a.old.source.events[7],b.old.source.events[7]],
  successorFrozenOuters:[a.old.result.coreScan.completed[1].outer,b.old.result.coreScan.completed[1].outer],
  sharedEvidenceStableThrough:[a.old.source.events.length,b.old.source.events.length],
  scope:'BRS raw relation only; same prefix certificate is compatible with two different successor outer branches'};
assert.equal(sourceEvents,120);assert.equal(prefixCount,124);assert.equal(replayedPrefixEventOccurrences,2060);
save('prefix-check.json',{inputLockSHA256:hash(path.join(here,'additional-input-lock.json')),sourceEvents,prefixCount,
  replayedPrefixEventOccurrences,prefixReports,branches,assertionsPassed:true,independentReview:false});

const source039=[];
for(const name of names.slice(0,2)){
  const c=cases[name],q=prefixBook(c.old.source.initial,c.old.source.events),ps=parse(q),down=name==='same_down_mirror';
  const blocks=ps.map(p=>({...p,actualStart:q[p.start],actualEnd:q[p.end],states:c.old.source.states.slice(p.start,p.end+1),
    qRange:hull(q.slice(p.start,p.end+1)),kindIfCompleted:'P',originalCompletion:null,originalDirection:null}));
  const windows=[];
  for(let i=0;i+3<blocks.length;i++){
    const [a,A1,A2,A3]=blocks.slice(i,i+4),H=intersect([A1,A2,A3].map(x=>x.qRange));
    const holdFirst=down?A1.qRange[1]<=a.qRange[1]:A1.qRange[0]>=a.qRange[0];
    const breakSecond=down?A2.qRange[0]<a.qRange[0]:A2.qRange[1]>a.qRange[1];
    const boundary=down?a.qRange[0]:a.qRange[1],third=down?A3.qRange[1]:A3.qRange[0];
    const back=down?third>boundary:third<boundary,noBack=down?third<boundary:third>boundary;
    windows.push({coreRoles:{a:a.core,A1:A1.core,A2:A2.core,A3:A3.core},
      eventCuts:[a.start,a.end,A1.end,A2.end,A3.end],qRanges:[a,A1,A2,A3].map(x=>x.qRange),
      originalRoleQualification:{aCompletion:null,AiCompletion:null,sameLevel:null,aDirection:null,AiDirections:null},
      numericalConditions:{branch:down?'downward mirror':'upward',firstDoesNotBreakLeadOppositeExtreme:holdFirst,
        secondBreaksLeadExtreme:breakSecond,thirdCrossesBackLeadExtreme:back,thirdStrictlyDoesNotReturn:noBack,thirdTouchesExactly:third===boundary},
      sourceBranchIfAllRolesQualified:holdFirst&&breakSecond&&noBack?'conditional recombination a-prime; not the cited returning branch':back?'returning numerical comparison; full source roles still required':'unresolved',
      AiNumericalIntersection:H,AiStrictIntersection:H[0]<H[1],
      conditionalRecombination:{sourceEvents:events(a.start,A3.end),qRange:hull(q.slice(a.start,A3.end+1)),
        ownFrozenCoreIDs:[a,A1,A2,A3].map(p=>p.core),semanticIssued:false},
      knownAt:Math.max(...[a,A1,A2,A3].map(x=>x.evidenceKnownAt))});
  }
  assert.ok(windows.length>0);assert.ok(windows.every(w=>w.numericalConditions.firstDoesNotBreakLeadOppositeExtreme&&w.numericalConditions.secondBreaksLeadExtreme&&w.numericalConditions.thirdStrictlyDoesNotReturn));
  source039.push({name,proposals:blocks.length,windows,
    firstThreeAsA1A2A3:{eventCuts:[0,...blocks.slice(0,3).map(p=>p.end)],leadingA:null,
      reason:'no native events before B0; cannot invent a completed same-level leading object'},
    qualified039Applications:0,scope:'necessary numerical role check only; no argument from RDW or parser labels to original completion'});
}
save('source039-role-check.json',{source:'docs/chanlun/text/blog/039-第39课.md:26-30',cases:source039,assertionsPassed:true,independentReview:false});
console.log(JSON.stringify({sourceEvents,prefixCount,replayedPrefixEventOccurrences,branches,
  source039:source039.map(c=>({name:c.name,proposals:c.proposals,windows:c.windows.length,
    nonreturn:c.windows.filter(w=>w.numericalConditions.thirdStrictlyDoesNotReturn).length,
    strictAiNumerical:c.windows.filter(w=>w.AiStrictIntersection).length,qualifiedApplications:0})),assertionsPassed:true},null,2));
