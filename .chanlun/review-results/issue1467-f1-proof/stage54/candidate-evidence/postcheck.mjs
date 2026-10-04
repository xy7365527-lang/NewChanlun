import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import crypto from 'node:crypto';import {fileURLToPath} from 'node:url';
import {relation,check,compareC} from './relation-reference.mjs';import {OnlineLegs} from './online-parser.mjs';
import {relation as historical} from '../../stage53/candidate-evidence/relation-reference.mjs';
const here=path.dirname(fileURLToPath(import.meta.url)),out=process.argv[2]??path.join(here,'results-v1');
const read=p=>JSON.parse(fs.readFileSync(p)),save=(n,v)=>fs.writeFileSync(path.join(out,n),JSON.stringify(v,null,2)+'\n');
const sourceInput=path.join(here,'source-conditioned-control.json'),b=fs.readFileSync(sourceInput),c=JSON.parse(b);
const online=new OnlineLegs(c.q[0]);let prev;const prefixes=[];
for(let t=0;t<c.q.length;t++){
  if(t)online.push(c.q[t]);const q=c.q.slice(0,t+1),d=relation(q);assert.deepEqual(d,online.parse());check(q,d,prev);compareC(d,historical(q,'C53'));prefixes.push({t,D54:d});prev=d;
}
const result=prefixes.at(-1).D54,x=result.outputs[0];assert.deepEqual(x.core,[1,2]);assert.deepEqual(x.coreMembers,[0,1,2,3]);assert.deepEqual(x.releasedFromDevelopment,[4]);
const values=c.assumedZLegsZeroBased.flatMap(k=>{const l=result.legs[k];return c.q.slice(l.start,l.end+1);});
const conditionalZnHull=[Math.min(...values),Math.max(...values)];assert.deepEqual(conditionalZnHull,[0,3]);assert.deepEqual(x.memberOuter,[-1,3]);assert.deepEqual(x.wholeOuter,[-1,4]);
save('source-field-comparison.json',{...c,inputSha256:crypto.createHash('sha256').update(b).digest('hex'),inputBytes:b.length,prefixes,comparison:{core:x.core,finalCoreMemberHull:x.memberOuter,conditionalZnHull,wholeXHull:x.wholeOuter,originalZnIdentity:null}});
const summary=read(path.join(out,'summary.json'));let releaseChecks=0,sourceChecks=0;
for(const input of summary.inputs){
  const data=read(path.join(out,input.name+'.json'));
  for(const prefix of data.prefixes)for(const o of prefix.D54.outputs){
    const q=prefix.states.map(s=>s.q);
    assert.deepEqual(o.memberEvents,o.coreMembers.flatMap(k=>{const l=prefix.D54.legs[k];return Array.from({length:l.end-l.start},(_,i)=>l.start+i+1);}));
    assert.deepEqual(o.developmentEvents,o.developmentLegs.flatMap(k=>{const l=prefix.D54.legs[k];return Array.from({length:l.end-l.start},(_,i)=>l.start+i+1);}));
    if(o.releasedFromDevelopment.length){
      const evidence=o.contactEvidence.at(-1);assert.equal(evidence.R,o.departureCandidate);assert.deepEqual(evidence.Rsource,o.releasedEvents);
      const v=q[o.end],a=o.memberOuter;assert.deepEqual(o.developmentOuter,[Math.min(a[0],v),Math.max(a[1],v)]);releaseChecks++;
    }
    sourceChecks++;
  }
}
const forks=['p_completion_overlap','p_completion_disjoint'].map(n=>read(path.join(out,n+'.json')));
assert.deepEqual(forks[0].source.events.slice(0,7),forks[1].source.events.slice(0,7));assert.notDeepEqual(forks[0].source.events[7],forks[1].source.events[7]);
assert.deepEqual(forks[0].prefixes[7].D54.outputs,forks[1].prefixes[7].D54.outputs);
for(const f of forks)assert.deepEqual(f.prefixes[7].D54.outputs,f.prefixes.at(-1).D54.outputs.slice(0,1));
save('postcheck.json',{ok:true,nativePrefixOutputSourceChecks:sourceChecks,nativeReleasedOutputOccurrenceChecks:releaseChecks,branchSharedThroughEvent:7,branchDivergenceAtEvent:8,conditionalScalarControls:1,conditionalScalarPrefixes:prefixes.length,conditionalScalarNativeEvents:0,conditionalScalarAddedEnumerationWords:0});
console.log(JSON.stringify({ok:true,sourceChecks,releaseChecks,conditionalScalarPrefixes:prefixes.length}));
