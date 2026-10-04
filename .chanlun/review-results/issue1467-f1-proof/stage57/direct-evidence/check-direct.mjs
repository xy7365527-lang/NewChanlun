import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {relation as scalarD54} from '../../stage54/candidate-evidence/relation-reference.mjs';
const dir=fileURLToPath(new URL('.',import.meta.url));
const controls=JSON.parse(readFileSync(dir+'controls-v1.json')).controls;
const copy=x=>structuredClone(x), indices=(a,b)=>Array.from({length:Math.max(0,b-a+1)},(_,i)=>a+i);
const pressure=e=>e.side==='bid'?(e.action==='add'?1:-1):(e.action==='add'?-1:1);
const hull=rs=>[Math.min(...rs.map(x=>x[0])),Math.max(...rs.map(x=>x[1]))];
const quote=b=>b.bids.length&&b.asks.length?{bid:Math.max(...b.bids.map(o=>o.price)),ask:Math.min(...b.asks.map(o=>o.price))}:null;
function replay(control,n){
 const events=control.events.slice(0,n),states=[copy(control.initial)],steps=[];
 for(const e of events){
  const before=states.at(-1),after=copy(before),key=e.side==='bid'?'bids':'asks',other=e.side==='bid'?'asks':'bids';
  assert.ok([...before.bids,...before.asks].every(o=>o.id!==e.orderId)||e.action!=='add');
  let deletion=null;
  if(e.action==='add')after[key].unshift({id:e.orderId,price:e.price});
  else {const k=after[key].findIndex(o=>o.id===e.orderId);assert.ok(k>=0);assert.equal(after[key][k].price,e.price);
   deletion={pre:before[key].slice(0,k).map(o=>o.price),post:before[key].slice(k+1).map(o=>o.price)};after[key].splice(k,1);}
  assert.deepEqual(before[other],after[other]);
  const q=quote(after);if(q)assert.ok(q.bid<q.ask);
  steps.push({eventId:e.id,amount:1,pressure:pressure(e),before:states.length-1,after:states.length,updateWitness:deletion??'prepend one price',validBook:true});states.push(after);
 }
 const quotes=states.map(quote),outs=[];
 for(let i=0;i<events.length;){let end=i+1;while(end<events.length&&pressure(events[end])===pressure(events[i]))end++;
  const qs=quotes.slice(i,end+1),pos=pressure(events[i]),q=qs[0],r=qs.at(-1),ready=qs.every(x=>x&&x.bid<x.ask);
  const geometry=ready?{direction:pos,startValue:pos===1?q.bid:q.ask,endValue:pos===1?r.ask:r.bid}:null;
  if(geometry){geometry.range=[Math.min(geometry.startValue,geometry.endValue),Math.max(geometry.startValue,geometry.endValue)];
   assert.equal(Math.sign(geometry.endValue-geometry.startValue),pos);
   assert.deepEqual(geometry.range,hull(qs.map(q=>[q.bid,q.ask])));
   for(let k=1;k<qs.length;k++){assert.ok(pos*(qs[k].bid-qs[k-1].bid)>=0);assert.ok(pos*(qs[k].ask-qs[k-1].ask)>=0);}}
  outs.push({flowId:`${control.id}:F${outs.length}`,start:i,end,firstKnown:i+1,knownAt:end<n?end+1:null,pressure:pos,eventIds:events.slice(i,end).map(e=>e.id),geometry});i=end;
 }
 assert.deepEqual(outs.flatMap(o=>o.eventIds),events.map(e=>e.id));
 for(let k=1;k<outs.length;k++){const a=outs[k-1],b=outs[k];assert.equal(a.end,b.start);assert.equal(a.pressure,-b.pressure);
  if(a.geometry&&b.geometry)assert.equal(a.geometry.endValue,b.geometry.startValue);}
 return {controlId:control.id,n,steps,states,quotes,outputs:outs};
}
// Source-aware lift of the frozen D54 relation. It never filters None and never
// equates a Flow's scalar readout with the entire order book.
function directD54(run){
 const all=run.outputs,firstNone=all.findIndex(o=>!o.geometry);
 if(firstNone>=0)return {domain:'outside-all-quotes-ready',firstNone,outputs:[],tail:{start:0,end:run.n,eventIds:all.flatMap(o=>o.eventIds)},rawOutputs:all};
 const legs=all.filter(o=>o.knownAt!==null),out=[];let c=0,b=0;
 for(;;){let s=c,C=null;
  for(;s+2<legs.length;s++){const q=legs.slice(s,s+3).map(x=>x.geometry.range),lo=Math.max(...q.map(x=>x[0])),hi=Math.min(...q.map(x=>x[1]));if(lo<hi){C=[lo,hi];break;}}
  const stop=tag=>({domain:'all-quotes-ready',outputs:out,terminal:{tag,c,s,core:C},tail:{start:b,end:run.n,eventIds:run.steps.slice(b).map(e=>e.eventId)}});
  if(!C)return stop('NoCore');let j=s+3,H=indices(s,s+2),contact=[];
  for(;;){if(j+1>=legs.length)return stop('AwaitPair');const L=legs[j],R=legs[j+1],g=L.geometry,r=R.geometry;
   const side=r.range[0]>C[1]?1:r.range[1]<C[0]?-1:0;
   if(!side){contact.push({L:j,R:j+1,knownAt:R.knownAt});H=indices(s,j+1);j++;continue;}
   const good=side===1?g.startValue<=C[1]&&C[1]<g.endValue&&L.pressure===1&&R.pressure===-1:g.startValue>=C[0]&&C[0]>g.endValue&&L.pressure===-1&&R.pressure===1;
   if(!good)return stop('BadRole');const M=H.filter(k=>k!==j),wholeLegs=indices(c,j),wholeRange=hull(wholeLegs.map(k=>legs[k].geometry.range));
   const o={id:`${run.controlId}:K${out.length}`,instanceKey:{version:'D54-direct-v1',sourceNamespace:run.controlId,cycle:out.length,seedFlowIds:indices(s,s+2).map(k=>legs[k].flowId)},start:b,end:L.end,seed:indices(s,s+2),core:C,H,M,contact,L:j,R:j+1,side,releasedFromDevelopment:H.includes(j)?[j]:[],departureWasInDevelopment:H.includes(j),departureIsCoreMember:false,departureEvents:L.eventIds,seedEnd:legs[s+2].end,seedKnownAt:legs[s+2].knownAt,relationshipPublishedAt:R.knownAt,
    memberEvents:M.flatMap(k=>legs[k].eventIds),memberOuter:hull(M.map(k=>legs[k].geometry.range)),developmentOuter:hull(H.map(k=>legs[k].geometry.range)),wholeOuter:wholeRange,
    ownedEvents:run.steps.slice(b,L.end).map(e=>e.eventId),confirmationEvent:run.steps[R.knownAt-1].eventId,RownedEvents:R.eventIds,sourceCut:L.end,
    modelKind:'Closed0/P0',originalP:null,originalBreak:null,originalLevel:null,wholeF1:null,fixedF2:null,connectorIdentity:null,semanticEligibleAt:null,L56windows:[]};
   assert.deepEqual(wholeRange,hull(run.quotes.slice(b,L.end+1).map(q=>[q.bid,q.ask])));
   for(let k=c;k<=j-2;k++){const r=legs.slice(k,k+3).map(x=>x.geometry.range),J=[Math.max(...r.map(x=>x[0])),Math.min(...r.map(x=>x[1]))];
    const hit=Math.max(J[0],C[0])<=Math.min(J[1],C[1]);if(J[0]<J[1])assert.ok(hit);o.L56windows.push({legs:indices(k,k+2),J,positive:J[0]<J[1],touchesCore:hit});}
   out.push(o);c=j+1;b=L.end;break;
  }
 }
}
const results=[];
for(const control of controls){const prefixes=[];let prior=[];
 for(let n=0;n<=control.events.length;n++){const r=replay(control,n),d=directD54(r);assert.deepEqual(d.outputs.slice(0,prior.length),prior);prior=d.outputs;
  const scalarCrosscheck=n>0&&r.outputs.every(o=>o.geometry)?(()=>{
   const endpointSkeleton=[r.outputs[0].geometry.startValue,...r.outputs.map(o=>o.geometry.endValue)],scalar=scalarD54(endpointSkeleton);
   assert.equal(scalar.outputs.length,d.outputs.length);
   for(let k=0;k<scalar.outputs.length;k++){const a=scalar.outputs[k],b=d.outputs[k];assert.deepEqual(a.core,b.core);assert.deepEqual(a.coreMembers,b.M);assert.deepEqual(a.wholeOuter,b.wholeOuter);assert.equal(a.end,b.L+1);assert.equal(r.outputs[a.end-1].end,b.end);assert.equal(r.outputs[a.relationshipPublishedAt-1].firstKnown,b.relationshipPublishedAt);}
   const onlyConfirmed=[r.outputs[0].geometry.startValue,...r.outputs.filter(o=>o.knownAt!==null).map(o=>o.geometry.endValue)];
   return {endpointSkeleton,scalarOutputs:scalar.outputs,onlyConfirmedEndpointD54:scalarD54(onlyConfirmed),clockMap:'scalar confirmation index k maps to firstKnown of original Flow k-1; not its current end'};
  })():null;
  prefixes.push({...r,direct:d,scalarCrosscheck});}
 results.push({controlId:control.id,prefixes});}
const main=results[0].prefixes.at(-1),o=main.direct.outputs[0];assert.equal(main.direct.outputs.length,1);assert.equal(o.end,6);assert.equal(o.relationshipPublishedAt,8);assert.deepEqual(o.core,[100,102]);assert.deepEqual(o.wholeOuter,[1,102]);assert.equal(main.scalarCrosscheck.onlyConfirmedEndpointD54.outputs.length,0);
const nr=results[1].prefixes.at(-1);assert.deepEqual(nr.outputs.map(o=>!!o.geometry),[true,false,false,true,true]);assert.equal(nr.outputs[0].end,1);assert.equal(nr.outputs[3].start,5);
writeFileSync(dir+'results-v1.json',JSON.stringify({scope:{controls:2,events:15,prefixes:17,marketSamples:0,independentReview:false},results},null,2)+'\n');
console.log(JSON.stringify({status:'author-check-pass',controls:2,events:15,prefixes:17,main:{core:o.core,end:o.end,published:o.relationshipPublishedAt,wholeOuter:o.wholeOuter},noneGeometry:nr.outputs.map(o=>!!o.geometry)},null,2));
