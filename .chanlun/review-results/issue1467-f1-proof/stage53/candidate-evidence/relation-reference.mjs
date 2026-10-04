// Stage53 relation interpreter. Input: exact scalar states and batch-derived legs.
// No historical parser, completion field, or final core catalogue is imported.
import assert from 'node:assert/strict';
export const policies={A53:{offset:3,stride:2},B53:{offset:2,stride:1},C53:{offset:3,stride:1}};
export const range=(a,b)=>Array.from({length:Math.max(0,b-a+1)},(_,i)=>a+i);
export const owned=(a,b)=>range(a+1,b);
export const hull=a=>[Math.min(...a),Math.max(...a)];

// Equal-value blocks and their direction changes determine maximal sealed legs.
// This differs from the incremental reversal detector in streaming-prefix.mjs.
export function batchLegs(q){
  const blocks=[];
  for(let t=0;t<q.length;t++){
    if(blocks.length&&blocks.at(-1).value===q[t])blocks.at(-1).end=t;
    else blocks.push({start:t,end:t,value:q[t]});
  }
  const legs=[];let start=0;
  for(let k=1;k+1<blocks.length;k++){
    const left=Math.sign(blocks[k].value-blocks[k-1].value);
    const right=Math.sign(blocks[k+1].value-blocks[k].value);
    if(left!==right){
      const end=blocks[k].end;
      legs.push({id:legs.length,start,end,direction:left,low:Math.min(q[start],q[end]),high:Math.max(q[start],q[end]),knownAt:blocks[k+1].start});
      start=end;
    }
  }
  return legs;
}

export function relation(q,policy,legs=batchLegs(q)){
  const {offset,stride}=policies[policy];
  const outputs=[],attempts=[];let cursor=0,cut=0,terminal;
  while(true){
    const rejected=[];let s=cursor,core;
    for(;s+2<legs.length;s++){
      const window=legs.slice(s,s+3);
      const low=Math.max(...window.map(l=>l.low)),high=Math.min(...window.map(l=>l.high));
      if(low<high){core=[low,high];break;}
      rejected.push({seedStart:s,intersection:[low,high]});
    }
    if(!core){terminal={tag:'NoCore',cursor,rejectedSeeds:rejected};break;}
    const seed=range(s,s+2),checks=[];let members=seed.slice(),j=s+offset,exit;
    while(true){
      if(j+1>=legs.length){terminal={tag:'AwaitPair',cursor,seed,core,members,nextL:j,rejectedSeeds:rejected,checks};break;}
      const L=legs[j],R=legs[j+1];
      const side=R.low>core[1]?1:R.high<core[0]?-1:0;
      if(side===0){
        const added=range(s,j+1).filter(k=>!members.includes(k));
        checks.push({L:j,R:j+1,outcome:'Touch',addedMembers:added});
        members=range(s,j+1);j+=stride;continue;
      }
      const role=side===1?q[L.start]<=core[1]&&q[L.end]>core[1]&&L.direction===1&&R.direction===-1
        :q[L.start]>=core[0]&&q[L.end]<core[0]&&L.direction===-1&&R.direction===1;
      const step={L:j,R:j+1,outcome:role?'Exit':'BadRole',side};checks.push(step);
      if(!role){terminal={tag:'BadRole',cursor,seed,core,members,nextL:j,rejectedSeeds:rejected,checks};break;}
      const memberStart=legs[members[0]].start,memberEnd=legs[members.at(-1)].end;
      if(!(cut<=memberStart&&memberEnd<=L.end&&cut<L.end)){
        terminal={tag:'MemberConflict',cursor,seed,core,members,nextL:j,rejectedSeeds:rejected,checks};break;
      }
      exit={id:outputs.length,start:cut,end:L.end,seed,core,members,memberStart,memberEnd,
        memberEvents:owned(memberStart,memberEnd),memberOuter:hull(q.slice(memberStart,memberEnd+1)),departure:j,return:j+1,side,
        departureIsMember:members.includes(j),seedEndpointAt:legs[s+2].end,seedKnownAt:legs[s+2].knownAt,
        relationshipPublishedAt:Math.max(...members.map(k=>legs[k].knownAt),L.knownAt,R.knownAt),
        ownedEvents:owned(cut,L.end),fullRange:hull(q.slice(cut,L.end+1)),checks,
        semanticEligibleAt:null,originalBreakIdentity:null,originalPIdentity:null};
      outputs.push(exit);attempts.push({cursor,seedStart:s,rejectedSeeds:rejected});
      cursor=j+1;cut=L.end;break;
    }
    if(!exit)break;
  }
  return {policy,legs,outputs,attempts,terminal,tail:{start:cut,end:q.length-1,ownedEvents:owned(cut,q.length-1)}};
}

export function checkInvariants(q,r,previous){
  assert.deepEqual([...r.outputs.flatMap(x=>x.ownedEvents),...r.tail.ownedEvents],owned(0,q.length-1),'exact ownership coverage');
  if(previous)assert.deepEqual(r.outputs.slice(0,previous.outputs.length),previous.outputs,'published certificate prefix stability');
  let edge=0,memberEnd=0;
  for(const x of r.outputs){
    assert.equal(x.start,edge);assert.ok(x.end>x.start);assert.ok(x.memberStart>=edge&&x.memberEnd<=x.end);
    assert.ok(x.memberStart>=memberEnd);assert.ok(x.relationshipPublishedAt<q.length);
    const L=r.legs[x.departure],R=r.legs[x.return];
    assert.equal(L.end,R.start);assert.equal(x.end,R.start);
    assert.equal(x.relationshipPublishedAt,R.knownAt);
    assert.ok(x.memberEvents.every(e=>x.ownedEvents.includes(e)));
    edge=x.end;memberEnd=x.memberEnd;
  }
  if(r.policy==='B53')assert.notEqual(r.terminal.tag,'BadRole','B contact invariant');
  if(r.policy==='B53')for(let i=1;i<r.outputs.length;i++){
    const old=r.outputs[i-1],next=r.outputs[i],b=q[old.end];
    assert.ok(old.memberOuter[0]<=b&&b<=old.memberOuter[1]);
    assert.ok(next.memberOuter[0]<=b&&b<=next.memberOuter[1],'B adjacent member outers share prior cut value');
    assert.ok(next.seed[0]===old.return||next.seed[0]===old.return+1,'four-leg seed bound');
  }
  assert.notEqual(r.terminal.tag,'MemberConflict','membership containment');
}
