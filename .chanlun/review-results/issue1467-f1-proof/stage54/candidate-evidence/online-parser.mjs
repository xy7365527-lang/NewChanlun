// D54 second interpreter: separate state machine and range accumulation.
// No import of the batch D54 interpreter or its helpers.
export class OnlineLegs{
  constructor(x){this.values=[x];this.legs=[];this.start=0;this.sign=0;}
  push(x){
    const t=this.values.length,d=Math.sign(x-this.values[t-1]);this.values.push(x);
    if(d===0)return;
    if(this.sign&&this.sign!==d){
      const end=t-1,a=this.values[this.start],b=this.values[end];
      this.legs.push({id:this.legs.length,start:this.start,end,direction:this.sign,low:Math.min(a,b),high:Math.max(a,b),knownAt:t});
      this.start=end;
    }
    this.sign=d;
  }
  parse(){return parse(this.values,this.legs);}
}
function seq(a,b){const z=[];for(let t=a;t<=b;t++)z.push(t);return z;}
function span(q,a,b){let lo=q[a],hi=q[a];for(let t=a+1;t<=b;t++){lo=Math.min(lo,q[t]);hi=Math.max(hi,q[t]);}return [lo,hi];}
function parse(q,legs){
  const outputs=[],attempts=[];let cursor=0,cut=0;
  const finish=terminal=>({policy:'D54',legs,outputs,attempts,terminal,tail:{start:cut,end:q.length-1,ownedEvents:seq(cut+1,q.length-1)}});
  while(true){
    let first=-1,lo,hi;const rejectedSeeds=[];
    for(let k=cursor;k+2<legs.length;k++){
      lo=Math.max(legs[k].low,legs[k+1].low,legs[k+2].low);hi=Math.min(legs[k].high,legs[k+1].high,legs[k+2].high);
      if(lo<hi){first=k;break;}
      rejectedSeeds.push({seedStart:k,intersection:[lo,hi]});
    }
    if(first<0)return finish({tag:'NoCore',cursor,rejectedSeeds});
    const seed=[first,first+1,first+2],core=[lo,hi],checks=[],contactEvidence=[];
    let lastDevelopment=first+2,pair=first+3,emitted=false;
    const stop=tag=>finish({tag,cursor,seed,core,developmentLegs:seq(first,lastDevelopment),nextL:pair,rejectedSeeds,checks,contactEvidence});
    while(pair+1<legs.length){
      const left=legs[pair],right=legs[pair+1];
      let side=0;if(right.low>hi)side=1;else if(right.high<lo)side=-1;
      if(side===0){
        checks.push({L:pair,R:pair+1,outcome:'Touch',addedDevelopmentLegs:seq(lastDevelopment+1,pair+1)});
        contactEvidence.push({L:pair,R:pair+1,Lsource:seq(left.start+1,left.end),Rsource:seq(right.start+1,right.end),Rrange:[right.low,right.high],knownAt:right.knownAt});
        lastDevelopment=pair+1;pair++;continue;
      }
      const valid=side>0?left.direction===1&&right.direction===-1&&q[left.start]<=hi&&hi<q[left.end]
        :left.direction===-1&&right.direction===1&&q[left.start]>=lo&&lo>q[left.end];
      checks.push({L:pair,R:pair+1,outcome:valid?'Exit':'BadRole',side});
      if(!valid)return stop('BadRole');
      const departureWasInDevelopment=lastDevelopment===pair;
      const finalLast=departureWasInDevelopment?lastDevelopment-1:lastDevelopment;
      const memberStart=legs[first].start,memberEnd=legs[finalLast].end;
      if(memberStart<cut||memberEnd>left.end||cut>=left.end)return stop('MemberConflict');
      const developmentStart=legs[first].start,developmentEnd=legs[lastDevelopment].end;
      outputs.push({id:outputs.length,start:cut,end:left.end,seed,core,coreMembers:seq(first,finalLast),developmentLegs:seq(first,lastDevelopment),
        memberStart,memberEnd,memberEvents:seq(memberStart+1,memberEnd),memberOuter:span(q,memberStart,memberEnd),
        developmentStart,developmentEnd,developmentEvents:seq(developmentStart+1,developmentEnd),developmentOuter:span(q,developmentStart,developmentEnd),
        releasedFromDevelopment:departureWasInDevelopment?[pair]:[],releasedEvents:departureWasInDevelopment?seq(left.start+1,left.end):[],
        departureCandidate:pair,returnCandidate:pair+1,departureEvents:seq(left.start+1,left.end),departureWasInDevelopment,departureIsCoreMember:false,side,
        seedEndpointAt:legs[first+2].end,seedKnownAt:legs[first+2].knownAt,relationshipPublishedAt:right.knownAt,
        ownedEvents:seq(cut+1,left.end),wholeOuter:span(q,cut,left.end),checks,contactEvidence,
        originalP:null,originalBreak:null,originalLevel:null,wholeF1:null,fixedF2:null,connectorIdentity:null,semanticEligibleAt:null});
      attempts.push({cursor,seedStart:first,rejectedSeeds});cursor=pair+1;cut=left.end;emitted=true;break;
    }
    if(!emitted)return stop('AwaitPair');
  }
}
