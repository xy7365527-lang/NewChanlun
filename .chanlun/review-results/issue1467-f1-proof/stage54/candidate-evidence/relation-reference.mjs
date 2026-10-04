// D54 batch interpreter. Stage53 supplies only the frozen scalar leg constructor.
import assert from 'node:assert/strict';
import {batchLegs} from '../../stage53/candidate-evidence/relation-reference.mjs';
export const indices=(a,b)=>Array.from({length:Math.max(0,b-a+1)},(_,i)=>a+i);
const events=(a,b)=>indices(a+1,b);
const hull=q=>[Math.min(...q),Math.max(...q)];
export function relation(q,legs=batchLegs(q)){
  const outputs=[],attempts=[];let cursor=0,cut=0;
  const finish=terminal=>({policy:'D54',legs,outputs,attempts,terminal,tail:{start:cut,end:q.length-1,ownedEvents:events(cut,q.length-1)}});
  for(;;){
    let s=cursor,core;const rejectedSeeds=[];
    for(;s+2<legs.length;s++){
      const triple=legs.slice(s,s+3),low=Math.max(...triple.map(l=>l.low)),high=Math.min(...triple.map(l=>l.high));
      if(low<high){core=[low,high];break;}
      rejectedSeeds.push({seedStart:s,intersection:[low,high]});
    }
    if(!core)return finish({tag:'NoCore',cursor,rejectedSeeds});
    const seed=indices(s,s+2),checks=[],contactEvidence=[];
    let H=seed.slice(),j=s+3;
    for(;;){
      const stop=tag=>finish({tag,cursor,seed,core,developmentLegs:H,nextL:j,rejectedSeeds,checks,contactEvidence});
      if(j+1>=legs.length)return stop('AwaitPair');
      const L=legs[j],R=legs[j+1];
      const side=R.low>core[1]?1:R.high<core[0]?-1:0;
      if(side===0){
        const added=indices(s,j+1).filter(i=>!H.includes(i));
        checks.push({L:j,R:j+1,outcome:'Touch',addedDevelopmentLegs:added});
        contactEvidence.push({L:j,R:j+1,Lsource:events(L.start,L.end),Rsource:events(R.start,R.end),Rrange:[R.low,R.high],knownAt:R.knownAt});
        H=indices(s,j+1);j++;continue;
      }
      const good=side===1?q[L.start]<=core[1]&&q[L.end]>core[1]&&L.direction===1&&R.direction===-1
        :q[L.start]>=core[0]&&q[L.end]<core[0]&&L.direction===-1&&R.direction===1;
      checks.push({L:j,R:j+1,outcome:good?'Exit':'BadRole',side});
      if(!good)return stop('BadRole');
      const releasedFromDevelopment=H.includes(j)&&!seed.includes(j)?[j]:[];
      const coreMembers=H.filter(i=>!releasedFromDevelopment.includes(i));
      const memberStart=legs[coreMembers[0]].start,memberEnd=legs[coreMembers.at(-1)].end;
      if(!(cut<=memberStart&&memberEnd<=L.end&&cut<L.end))return stop('MemberConflict');
      const developmentStart=legs[H[0]].start,developmentEnd=legs[H.at(-1)].end;
      outputs.push({id:outputs.length,start:cut,end:L.end,seed,core,coreMembers,developmentLegs:H,
        memberStart,memberEnd,memberEvents:events(memberStart,memberEnd),memberOuter:hull(q.slice(memberStart,memberEnd+1)),
        developmentStart,developmentEnd,developmentEvents:events(developmentStart,developmentEnd),developmentOuter:hull(q.slice(developmentStart,developmentEnd+1)),
        releasedFromDevelopment,releasedEvents:releasedFromDevelopment.flatMap(k=>events(legs[k].start,legs[k].end)),
        departureCandidate:j,returnCandidate:j+1,departureEvents:events(L.start,L.end),departureWasInDevelopment:H.includes(j),departureIsCoreMember:coreMembers.includes(j),side,
        seedEndpointAt:legs[s+2].end,seedKnownAt:legs[s+2].knownAt,relationshipPublishedAt:R.knownAt,
        ownedEvents:events(cut,L.end),wholeOuter:hull(q.slice(cut,L.end+1)),checks,contactEvidence,
        originalP:null,originalBreak:null,originalLevel:null,wholeF1:null,fixedF2:null,connectorIdentity:null,semanticEligibleAt:null});
      attempts.push({cursor,seedStart:s,rejectedSeeds});cursor=j+1;cut=L.end;break;
    }
  }
}

export function check(q,r,previous){
  assert.deepEqual([...r.outputs.flatMap(x=>x.ownedEvents),...r.tail.ownedEvents],events(0,q.length-1));
  if(previous)assert.deepEqual(r.outputs.slice(0,previous.outputs.length),previous.outputs);
  let boundary=0,lastMemberEnd=0;
  for(const o of r.outputs){
    const L=r.legs[o.departureCandidate],R=r.legs[o.returnCandidate];
    assert.equal(o.start,boundary);assert.ok(o.end>boundary);assert.equal(o.end,L.end);assert.equal(L.end,R.start);
    assert.deepEqual(o.coreMembers,indices(o.seed[0],o.departureCandidate-1));
    assert.ok(o.seed.every(k=>o.coreMembers.includes(k)));assert.ok(!o.coreMembers.includes(o.departureCandidate));
    assert.equal(o.memberEnd,L.start);assert.ok(o.memberStart>=boundary&&o.memberStart>=lastMemberEnd);
    assert.equal(o.relationshipPublishedAt,R.knownAt);assert.ok(R.knownAt<q.length);
    assert.ok(o.memberEvents.every(e=>o.ownedEvents.includes(e)));
    assert.ok(o.releasedEvents.every(e=>o.ownedEvents.includes(e)&&!o.memberEvents.includes(e)));
    assert.deepEqual(o.contactEvidence.flatMap(c=>[c.knownAt]).every(t=>t<o.relationshipPublishedAt),true);
    assert.ok(o.memberOuter[0]>=o.developmentOuter[0]&&o.memberOuter[1]<=o.developmentOuter[1]);
    assert.equal(o.departureWasInDevelopment,o.contactEvidence.length>0);
    if(o.departureWasInDevelopment)assert.deepEqual(o.releasedFromDevelopment,[o.departureCandidate]);
    else assert.deepEqual(o.releasedFromDevelopment,[]);
    for(const k of ['originalP','originalBreak','originalLevel','wholeF1','fixedF2','connectorIdentity','semanticEligibleAt'])assert.equal(o[k],null);
    boundary=o.end;lastMemberEnd=o.memberEnd;
  }
  assert.notEqual(r.terminal.tag,'MemberConflict');
  if(r.terminal.tag==='BadRole')assert.equal(r.terminal.contactEvidence.length,0);
}

export function compareC(d,c){
  assert.deepEqual(d.legs,c.legs);assert.deepEqual(d.tail,c.tail);assert.deepEqual(d.attempts,c.attempts);
  assert.equal(d.terminal.tag,c.terminal.tag);assert.equal(d.outputs.length,c.outputs.length);
  for(let i=0;i<d.outputs.length;i++){
    const x=d.outputs[i],y=c.outputs[i];
    for(const key of ['start','end','seed','core','ownedEvents','relationshipPublishedAt','side'])assert.deepEqual(x[key],y[key]);
    assert.deepEqual(x.developmentLegs,y.members);
    assert.deepEqual(x.coreMembers,y.members.filter(k=>k!==y.departure));
    assert.deepEqual(x.developmentOuter,y.memberOuter);assert.deepEqual(x.wholeOuter,y.fullRange);
    assert.equal(x.departureCandidate,y.departure);assert.equal(x.returnCandidate,y.return);
  }
}
