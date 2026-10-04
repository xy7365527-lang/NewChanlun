import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {relation,checkInvariants,policies,owned} from './relation-reference.mjs';
import {LegStream,parsePrefix} from './streaming-prefix.mjs';
import {replay,NativeStream,observation} from './native-replay.mjs';
const here=path.dirname(fileURLToPath(import.meta.url)),P=path.resolve(here,'../..');
const output=process.argv[2]??path.join(here,'results-v1');
const sha=f=>crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const read=f=>JSON.parse(fs.readFileSync(f,'utf8'));
const save=(name,value)=>fs.writeFileSync(path.join(output,name),JSON.stringify(value,null,2)+'\n');
const lock=read(path.join(here,'input-lock.json'));
for(const f of lock.files)assert.equal(sha(path.resolve(P,f.path)),f.sha256,f.path);
fs.mkdirSync(output,{recursive:true});
const inputs=lock.inputs.map(x=>({...x,source:read(path.resolve(P,x.path))}));
const summaries=[];let oldOps=0,controlOps=0;
for(const input of inputs){
  const raw=input.source.source??input.source,source={initial:raw.initial,events:raw.events};
  const all=replay(source);if(raw.states)assert.deepEqual(all.states,raw.states);
  const native=new NativeStream(source),stream=new LegStream(native.quote()),prefixes=[],previous={};
  const perPolicy=Object.fromEntries(Object.keys(policies).map(p=>[p,{firstPublication:null,firstThreePublication:null,changes:[]}]));
  for(let t=0;t<=source.events.length;t++){
    if(t)stream.push(native.apply(source.events[t-1]));
    const batch=replay(source,t),q=batch.states.map(x=>x.q);assert.deepEqual(q,stream.values);
    const results={};
    for(const policy of Object.keys(policies)){
      const result=relation(q,policy),independent=structuredClone(stream.parse(policy));
      assert.deepEqual(result,independent,`${input.name} t=${t} ${policy} independent`);
      checkInvariants(q,result,previous[policy]);
      if(result.outputs.length&&!perPolicy[policy].firstPublication)perPolicy[policy].firstPublication=t;
      if(result.outputs.length>=3&&!perPolicy[policy].firstThreePublication)perPolicy[policy].firstThreePublication=t;
      if(!previous[policy]||JSON.stringify(result)!==JSON.stringify(previous[policy]))perPolicy[policy].changes.push({t,outputCount:result.outputs.length,terminal:result.terminal.tag});
      previous[policy]=result;results[policy]=result;
    }
    prefixes.push({prefixAt:t,states:batch.states,results});
    if(input.ledger==='new-control')controlOps+=t;else oldOps+=t;
  }
  const last=prefixes.at(-1).results;
  for(const policy of Object.keys(policies)){
    perPolicy[policy].finalCuts=[0,...last[policy].outputs.map(x=>x.end)];
    perPolicy[policy].finalTerminal=last[policy].terminal;
    perPolicy[policy].members=last[policy].outputs.map(x=>({id:x.id,seed:x.seed,members:x.members,memberEvents:x.memberEvents,memberOuter:x.memberOuter,departure:x.departure,return:x.return,departureIsMember:x.departureIsMember,core:x.core,publishedAt:x.relationshipPublishedAt}));
  }
  // The old A53 relation is only compared against the previously frozen actual
  // successful outputs. The diagnostic is intentionally not sent to old code.
  let historicalCheck=null;
  if(['same_up_counterexample','same_down_mirror'].includes(input.name)){
    const old=read(path.resolve(P,`stage52/candidate-evidence/results-v1/${input.name}.json`));
    const frozen=old.onlinePrefixes??old.prefixes;
    historicalCheck={oldFileKeys:Object.keys(old),finalCutsMatchStage52FirstThree:last.A53.outputs.slice(0,3).map(x=>x.end).join(',')==='5,9,13'};
    assert.ok(historicalCheck.finalCutsMatchStage52FirstThree);
  }
  const data={name:input.name,ledger:input.ledger,source,states:all.states,snapshots:all.snapshots,prefixes};
  save(`${input.name}.json`,data);
  summaries.push({name:input.name,ledger:input.ledger,eventReferences:source.events.length,prefixes:prefixes.length,policies:perPolicy,historicalCheck});
}
// Numeric words only. No native orders are generated for this finite search.
const enumeration={alphabet:[0,1,2,3],lengths:[1,2,3,4,5,6,7,8,9],wordCount:0,byLength:[],policies:Object.fromEntries(Object.keys(policies).map(p=>[p,{terminalCounts:{},outputWords:0,departureMemberOutputs:0,firstBadRole:null,firstBadRoleAfterTouch:null}])),firstDifferentCuts:null,firstDifferentMembers:null,firstBSeedExit:null,firstTouchThenExit:null,firstSkippedWindowBetweenBOutputs:null,bAdjacentOutputCount:0,checks:{independentImplementations:true,coverage:true,prefixStability:true,bNoBadRole:true,noMemberConflict:true,bAdjacentMemberOutersIntersect:true},witnesses:{}};
for(let length=1;length<=9;length++){
  let count=0;const total=4**length;
  for(let code=0;code<total;code++){
    const q=Array(length);let z=code;for(let i=length-1;i>=0;i--){q[i]=z%4;z=Math.floor(z/4);}
    const stream=new LegStream(q[0]);for(const x of q.slice(1))stream.push(x);
    const results={};
    for(const policy of Object.keys(policies)){
      const r=relation(q,policy),alt=stream.parse(policy);assert.deepEqual(r,alt);
      const prev=q.length>1?relation(q.slice(0,-1),policy):null;checkInvariants(q,r,prev);results[policy]=r;
      const stat=enumeration.policies[policy];stat.terminalCounts[r.terminal.tag]=(stat.terminalCounts[r.terminal.tag]??0)+1;
      if(r.outputs.length)stat.outputWords++;
      stat.departureMemberOutputs+=r.outputs.filter(x=>x.departureIsMember).length;
      if(r.terminal.tag==='BadRole'){
        if(!stat.firstBadRole)stat.firstBadRole={q,result:r};
        if(r.terminal.checks.some(x=>x.outcome==='Touch')&&!stat.firstBadRoleAfterTouch)stat.firstBadRoleAfterTouch={q,result:r};
      }
    }
    const cuts=p=>results[p].outputs.map(x=>x.end).join(',');
    if(!enumeration.firstDifferentCuts&&(cuts('A53')!==cuts('B53')||cuts('A53')!==cuts('C53')))enumeration.firstDifferentCuts={q,results};
    const members=p=>JSON.stringify(results[p].outputs.map(x=>x.members));
    if(!enumeration.firstDifferentMembers&&results.A53.outputs.length&&cuts('A53')===cuts('B53')&&members('A53')!==members('B53'))enumeration.firstDifferentMembers={q,results};
    if(!enumeration.firstBSeedExit&&results.B53.outputs.some(x=>x.departure===x.seed[2]))enumeration.firstBSeedExit={q,results};
    if(!enumeration.firstTouchThenExit&&results.B53.outputs.some(x=>x.checks.some(c=>c.outcome==='Touch')))enumeration.firstTouchThenExit={q,results};
    enumeration.bAdjacentOutputCount+=Math.max(0,results.B53.outputs.length-1);
    if(!enumeration.firstSkippedWindowBetweenBOutputs&&results.B53.outputs.slice(1).some((x,i)=>x.seed[0]===results.B53.outputs[i].return+1))enumeration.firstSkippedWindowBetweenBOutputs={q,results};
    count++;
  }
  enumeration.wordCount+=count;enumeration.byLength.push({length,count});
  process.stdout.write(`enumerated length ${length}: ${count}\n`);
}
save('enumeration.json',enumeration);
const commonNames=['p_completion_overlap','p_completion_disjoint'];
const forks=commonNames.map(n=>read(path.join(output,`${n}.json`)));
assert.deepEqual(forks[0].source.events.slice(0,7),forks[1].source.events.slice(0,7));
assert.notDeepEqual(forks[0].source.events[7],forks[1].source.events[7]);
for(const p of Object.keys(policies))assert.deepEqual(forks[0].prefixes[7].results[p].outputs,forks[1].prefixes[7].results[p].outputs);
save('summary.json',{schema:'stage53-seed-departure-v1',baseline:'6b69fd5c805ee65d84d60a8f703b40833fb85fca',inputs:summaries,counts:{frozenFive:{eventReferences:126,prefixes:131,replayEventOperationsPerBatchAlgorithm:oldOps},newReanchorControl:{eventReferences:7,prefixes:8,replayEventOperationsPerBatchAlgorithm:controlOps},scalarWords:enumeration.wordCount},forkAt:8,allChecksPassed:true,sourceSemanticsProved:false});
process.stdout.write(JSON.stringify({ok:true,oldOps,controlOps,words:enumeration.wordCount})+'\n');
