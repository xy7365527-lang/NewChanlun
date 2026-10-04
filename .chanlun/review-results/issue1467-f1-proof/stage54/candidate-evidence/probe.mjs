import fs from 'node:fs';import path from 'node:path';import crypto from 'node:crypto';import assert from 'node:assert/strict';import {fileURLToPath} from 'node:url';
import {relation,check,compareC} from './relation-reference.mjs';
import {OnlineLegs} from './online-parser.mjs';
import {relation as historical,policies} from '../../stage53/candidate-evidence/relation-reference.mjs';
import {replay,NativeStream} from '../../stage53/candidate-evidence/native-replay.mjs';
const here=path.dirname(fileURLToPath(import.meta.url)),P=path.resolve(here,'../..');
const out=process.argv[2]??path.join(here,'results-v1');fs.mkdirSync(out,{recursive:true});
const read=p=>JSON.parse(fs.readFileSync(p,'utf8')),sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const save=(n,x)=>fs.writeFileSync(path.join(out,n),JSON.stringify(x,null,2)+'\n');
const lock=read(path.join(here,'input-lock.json'));
for(const f of lock.files){const p=path.join(P,f.path);assert.equal(sha(p),f.sha256);assert.equal(fs.statSync(p).size,f.bytes);}
const handLock=read(path.join(here,'explicit-scalar-lock.json'));assert.equal(sha(path.join(here,handLock.path)),handLock.sha256);
const summaries=[];let nativeComparisons=0,frozenComparisons=0;
function pairs(r,field){return r.outputs.slice(1).map((next,i)=>{const prev=r.outputs[i],a=prev[field],b=next[field];return {left:i,right:i+1,leftOuter:a,rightOuter:b,strictUp:a[1]<b[0],strictDown:b[1]<a[0],intersects:Math.max(a[0],b[0])<=Math.min(a[1],b[1])};});}
function compact(r){return {cuts:[0,...r.outputs.map(x=>x.end)],terminal:r.terminal.tag,outputs:r.outputs.map(x=>({id:x.id,seed:x.seed,core:x.core,coreMembers:x.coreMembers,developmentLegs:x.developmentLegs,memberEvents:x.memberEvents,memberOuter:x.memberOuter,developmentOuter:x.developmentOuter,wholeOuter:x.wholeOuter,releasedFromDevelopment:x.releasedFromDevelopment,releasedEvents:x.releasedEvents,departureWasInDevelopment:x.departureWasInDevelopment,side:x.side,publishedAt:x.relationshipPublishedAt})),memberPairs:pairs(r,'memberOuter'),wholePairs:pairs(r,'wholeOuter')};}
for(const input of lock.inputs){
  const file=read(path.join(P,input.path)),raw=file.source??file,source={initial:raw.initial,events:raw.events};
  const native=new NativeStream(source),online=new OnlineLegs(native.quote()),prefixes=[];
  let previous=null;
  const frozen=input.ledger==='stage53-frozen-six'?read(path.join(P,`stage53/candidate-evidence/results-v1/${input.name}.json`)):null;
  for(let t=0;t<=source.events.length;t++){
    if(t)online.push(native.apply(source.events[t-1]));
    const batch=replay(source,t),q=batch.states.map(s=>s.q);assert.deepEqual(q,online.values);
    const d=relation(q),second=online.parse();assert.deepEqual(d,second,`${input.name} ${t} two D54 interpreters`);check(q,d,previous);
    const controls=Object.fromEntries(Object.keys(policies).map(p=>[p,historical(q,p)]));compareC(d,controls.C53);
    if(frozen){for(const p of Object.keys(policies)){assert.deepEqual(controls[p],frozen.prefixes[t].results[p]);frozenComparisons++;}}
    prefixes.push({prefixAt:t,states:batch.states,D54:d,controls});previous=d;nativeComparisons++;
  }
  const all=replay(source),last=prefixes.at(-1),summary={name:input.name,ledger:input.ledger,eventReferences:source.events.length,prefixes:prefixes.length,batchEventOperations:source.events.length*(source.events.length+1)/2,...compact(last.D54)};
  summaries.push(summary);save(input.name+'.json',{name:input.name,ledger:input.ledger,source,states:all.states,snapshots:all.snapshots,prefixes});
}
const hand=read(path.join(here,'explicit-scalar-input.json')),handResults=[];
for(const c of hand.cases){
  const online=new OnlineLegs(c.q[0]),prefixes=[];let prev;
  for(let t=0;t<c.q.length;t++){
    if(t)online.push(c.q[t]);const q=c.q.slice(0,t+1),d=relation(q),alt=online.parse();assert.deepEqual(d,alt);check(q,d,prev);
    const controls=Object.fromEntries(Object.keys(policies).map(p=>[p,historical(q,p)]));compareC(d,controls.C53);
    prefixes.push({prefixAt:t,D54:d,controls});prev=d;
  }
  const final=prefixes.at(-1);
  if(c.name.startsWith('after_touch_strict_separation')){
    assert.equal(final.D54.outputs.length,2);assert.ok(final.D54.outputs[0].departureWasInDevelopment);
    assert.ok(pairs(final.D54,'memberOuter')[0][c.name.endsWith('up')?'strictUp':'strictDown']);
    assert.ok(pairs(final.controls.C53,'memberOuter')[0].intersects);assert.ok(pairs(final.D54,'wholeOuter')[0].intersects);
  }
  if(c.name==='first_no_core_point'){assert.equal(prefixes[4].D54.terminal.tag,'NoCore');assert.deepEqual(prefixes[4].D54.terminal.rejectedSeeds[0].intersection,[3,3]);assert.deepEqual(prefixes[5].D54.terminal.seed,[1,2,3]);}
  if(c.name==='first_no_core_empty'){assert.equal(prefixes[4].D54.terminal.tag,'NoCore');assert.deepEqual(prefixes[4].D54.terminal.rejectedSeeds[0].intersection,[4,3]);assert.deepEqual(prefixes[5].D54.terminal.seed,[1,2,3]);}
  if(c.name==='closed_boundary_contact')assert.ok(final.D54.outputs.some(x=>x.contactEvidence.some(e=>e.Rrange[1]===x.core[0]||e.Rrange[0]===x.core[1])));
  if(c.name==='bad_role_all_future_tail')for(const p of prefixes.slice(6)){assert.equal(p.D54.terminal.tag,'BadRole');assert.equal(p.D54.outputs.length,0);assert.equal(p.D54.tail.ownedEvents.length,p.prefixAt);}
  handResults.push({...c,prefixes,summary:compact(final.D54)});
}
save('explicit-scalar-results.json',{...hand,cases:handResults});
const enumeration={alphabet:[0,1,2,3],lengths:[1,2,3,4,5,6,7,8,9],wordCount:0,byLength:[],terminalCounts:{},outputWords:0,outputs:0,releasedOutputs:0,unreleasedOutputs:0,strictShrinkOutputs:0,sameOuterDifferentMembersOutputs:0,adjacentPairs:0,strictSeparatedPairs:0,firstBadRole:null,firstRelease:null,firstSameOuterDifferentMembers:null,firstNoCore:null,firstBoundaryContact:null,firstEqualityRelease:null,checks:{twoImplementations:true,coverage:true,prefixStability:true,seedRetained:true,memberSubset:true,contactHistoryRetained:true,c53CutsAndTiming:true}};
for(let length=1;length<=9;length++){
  for(let code=0;code<4**length;code++){
    const q=Array(length);let z=code;for(let i=length-1;i>=0;i--){q[i]=z%4;z=Math.floor(z/4);}
    const online=new OnlineLegs(q[0]);for(const x of q.slice(1))online.push(x);
    const d=relation(q);assert.deepEqual(d,online.parse());check(q,d,length>1?relation(q.slice(0,-1)):null);const c=historical(q,'C53');compareC(d,c);
    enumeration.wordCount++;enumeration.terminalCounts[d.terminal.tag]=(enumeration.terminalCounts[d.terminal.tag]??0)+1;
    if(d.outputs.length)enumeration.outputWords++;enumeration.outputs+=d.outputs.length;
    for(const x of d.outputs){
      if(x.departureWasInDevelopment){enumeration.releasedOutputs++;if(!enumeration.firstRelease)enumeration.firstRelease={q,D54:d,C53:c};
        if(JSON.stringify(x.memberOuter)===JSON.stringify(x.developmentOuter)){enumeration.sameOuterDifferentMembersOutputs++;if(!enumeration.firstSameOuterDifferentMembers)enumeration.firstSameOuterDifferentMembers={q,D54:d,C53:c};}
        else enumeration.strictShrinkOutputs++;
        if(q.some((v,i)=>i&&v===q[i-1])&&!enumeration.firstEqualityRelease)enumeration.firstEqualityRelease={q,D54:d};
      }else enumeration.unreleasedOutputs++;
      if(!enumeration.firstBoundaryContact&&x.contactEvidence.some(e=>e.Rrange[1]===x.core[0]||e.Rrange[0]===x.core[1]))enumeration.firstBoundaryContact={q,D54:d};
    }
    const adjacent=pairs(d,'memberOuter');enumeration.adjacentPairs+=adjacent.length;enumeration.strictSeparatedPairs+=adjacent.filter(p=>p.strictUp||p.strictDown).length;
    if(d.terminal.tag==='BadRole'&&!enumeration.firstBadRole)enumeration.firstBadRole={q,D54:d};
    if(d.terminal.tag==='NoCore'&&d.terminal.rejectedSeeds.length&&!enumeration.firstNoCore)enumeration.firstNoCore={q,D54:d};
  }
  enumeration.byLength.push({length,count:4**length});process.stdout.write(`D54 enumerated length ${length}: ${4**length}\n`);
}
assert.deepEqual(enumeration.terminalCounts,{NoCore:69078,AwaitPair:279864,BadRole:582});assert.equal(enumeration.outputWords,1608);
save('enumeration.json',enumeration);
save('summary.json',{schema:'stage54-exit-release-v1',baseline:lock.baseline,inputs:summaries,counts:{stage53FrozenSix:{eventReferences:133,prefixes:139,batchEventOperations:2109},stage54NewControl:{eventReferences:8,prefixes:9,batchEventOperations:36},total:{eventReferences:141,prefixes:148,batchEventOperations:2145,persistentLedgerOperations:141},nativeD54Comparisons:nativeComparisons,frozenStage53PolicyComparisons:frozenComparisons,scalarWords:enumeration.wordCount,explicitScalarControls:hand.cases.length},allChecksPassed:true,generalProgramRefinement:null,sourceSemanticsProved:null});
process.stdout.write(JSON.stringify({ok:true,nativeComparisons,frozenComparisons,wordCount:enumeration.wordCount,releasedOutputs:enumeration.releasedOutputs,strictShrinkOutputs:enumeration.strictShrinkOutputs,sameOuterDifferentMembersOutputs:enumeration.sameOuterDifferentMembersOutputs,adjacentPairs:enumeration.adjacentPairs})+'\n');
