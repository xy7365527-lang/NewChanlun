// Independent online leg builder and parser. Does not import relation-reference.
export class LegStream{
  constructor(initial){this.values=[initial];this.sealed=[];this.anchor=0;this.direction=0;}
  push(value){
    const t=this.values.length,delta=Math.sign(value-this.values[t-1]);this.values.push(value);
    if(delta===0)return;
    if(this.direction&&delta!==this.direction){
      const end=t-1,a=this.values[this.anchor],b=this.values[end];
      this.sealed.push({id:this.sealed.length,start:this.anchor,end,direction:this.direction,low:Math.min(a,b),high:Math.max(a,b),knownAt:t});
      this.anchor=end;
    }
    this.direction=delta;
  }
  parse(name){return parsePrefix(this.values,this.sealed,name);}
}
const interval=(a,b)=>{const z=[];for(let i=a;i<=b;i++)z.push(i);return z;};
export function parsePrefix(values,sealed,name){
  let begin=0,lastCut=0;
  const outputs=[],attempts=[];
  for(;;){
    let first=-1,lo,hi;const misses=[];
    for(let i=begin;i+2<sealed.length;i++){
      lo=Math.max(sealed[i].low,sealed[i+1].low,sealed[i+2].low);
      hi=Math.min(sealed[i].high,sealed[i+1].high,sealed[i+2].high);
      if(lo<hi){first=i;break;}
      misses.push({seedStart:i,intersection:[lo,hi]});
    }
    if(first<0)return finish({tag:'NoCore',cursor:begin,rejectedSeeds:misses});
    const seed=[first,first+1,first+2],core=[lo,hi],checks=[];
    let endMember=first+2,pair=first+(name==='B53'?2:3),emitted=false;
    while(pair+1<sealed.length){
      const a=sealed[pair],b=sealed[pair+1];
      let side=0;if(b.low>hi)side=1;else if(b.high<lo)side=-1;
      if(!side){
        checks.push({L:pair,R:pair+1,outcome:'Touch',addedMembers:interval(endMember+1,pair+1)});
        endMember=pair+1;pair+=name==='A53'?2:1;continue;
      }
      const good=side===1?a.direction===1&&b.direction===-1&&values[a.start]<=hi&&hi<values[a.end]
        :a.direction===-1&&b.direction===1&&values[a.start]>=lo&&lo>values[a.end];
      checks.push({L:pair,R:pair+1,outcome:good?'Exit':'BadRole',side});
      const members=interval(first,endMember);
      if(!good)return finish({tag:'BadRole',cursor:begin,seed,core,members,nextL:pair,rejectedSeeds:misses,checks});
      const memberStart=sealed[first].start,memberEnd=sealed[endMember].end;
      if(lastCut>memberStart||memberEnd>a.end||lastCut>=a.end)return finish({tag:'MemberConflict',cursor:begin,seed,core,members,nextL:pair,rejectedSeeds:misses,checks});
      let min=Infinity,max=-Infinity;
      for(let i=lastCut;i<=a.end;i++){min=Math.min(min,values[i]);max=Math.max(max,values[i]);}
      let memberLow=Infinity,memberHigh=-Infinity;
      for(let i=memberStart;i<=memberEnd;i++){memberLow=Math.min(memberLow,values[i]);memberHigh=Math.max(memberHigh,values[i]);}
      outputs.push({id:outputs.length,start:lastCut,end:a.end,seed,core,members,memberStart,memberEnd,
        memberEvents:interval(memberStart+1,memberEnd),memberOuter:[memberLow,memberHigh],departure:pair,return:pair+1,side,
        departureIsMember:pair<=endMember,seedEndpointAt:sealed[first+2].end,seedKnownAt:sealed[first+2].knownAt,
        relationshipPublishedAt:b.knownAt,ownedEvents:interval(lastCut+1,a.end),fullRange:[min,max],checks,
        semanticEligibleAt:null,originalBreakIdentity:null,originalPIdentity:null});
      attempts.push({cursor:begin,seedStart:first,rejectedSeeds:misses});
      begin=pair+1;lastCut=a.end;emitted=true;break;
    }
    if(!emitted)return finish({tag:'AwaitPair',cursor:begin,seed,core,members:interval(first,endMember),nextL:pair,rejectedSeeds:misses,checks});
  }
  function finish(terminal){return {policy:name,legs:sealed,outputs,attempts,terminal,tail:{start:lastCut,end:values.length-1,ownedEvents:interval(lastCut+1,values.length-1)}};}
}
