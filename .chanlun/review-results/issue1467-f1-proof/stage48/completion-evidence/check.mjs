// #1467 Stage48: conditional trend-stop certificates. Research only.
// node check.mjs /new/output/directory
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const span = (a,b) => Array.from({length:Math.max(0,b-a+1)},(_,i)=>a+i);
const rep = (...v) => v.flatMap(x=>[x,x,x]);
const eq = (a,b) => JSON.stringify(a)===JSON.stringify(b);
const sign = x => x>0?1:x<0?-1:0;
const state = (xs,i) => i===0?xs[0]:xs[i-1];
const path = (xs,s,e) => span(s,e).map(i=>state(xs,i));
const range = ys => [Math.min(...ys),Math.max(...ys)];

export function catalog(xs) {
  assert(xs.every(x=>Number.isSafeInteger(x)&&x>0));
  const runs=[];
  xs.forEach((value,j)=>{
    if(runs.length&&runs.at(-1).value===value) runs.at(-1).end=j+1;
    else runs.push({start:j+1,end:j+1,value});
  });
  assert(runs.length<2||runs[0].end>=3,'outside H_EV initialization');
  return runs.filter(r=>r.end-r.start+1>=3).map(r=>({
    id:`K@${r.start}`, ...r, birth:r.start+2, closed:r.end<xs.length,
  }));
}

export function trendStop(xs,s=0) {
  if(!xs.length) return {status:'waiting'};
  const ks=catalog(xs);
  if(ks.some(k=>k.start<=s&&s<k.end)) return {status:'split-core-start'};
  const after=ks.filter(k=>k.start>s);
  if(after.length<2) return {status:'waiting'};
  const d=sign(after[1].value-after[0].value);
  if(!d) return {status:'no-trend-seed'};
  let j=1;
  while(j+1<after.length&&sign(after[j+1].value-after[j].value)===d) j++;
  if(j+1===after.length) return {status:'waiting'};
  const own=after.slice(0,j+1), last=own.at(-1), t=after[j+1];
  assert(own.every(k=>k.closed));
  const b=t.start,C=t.birth,knownPath=path(xs,s,b-1);
  const extreme=d===1?Math.max(...knownPath):Math.min(...knownPath);
  const rawExtremePositions=span(s,b-1).filter(i=>state(xs,i)===extreme);
  const eligibleEnds=span(last.end,b-1).filter(e=>{
    const [lo,hi]=range(path(xs,s,e));
    return state(xs,e)===extreme&&state(xs,s)===(d===1?lo:hi)&&
      state(xs,e)===(d===1?hi:lo)&&state(xs,s)!==state(xs,e);
  });
  // Trigger's current end is deliberately not a frozen certificate field.
  return {
    status:eligibleEnds.length?'certified-candidate':'no-normal-endpoint',
    start:s,type:d===1?'U':'D',direction:d,confirm:C,
    ownCores:own, trigger:{id:t.id,start:b,birth:C,value:t.value,seed:span(b,C)},
    evidenceWindow:[s,C],extreme,rawExtremePositions,eligibleEnds,
  };
}

export function select(xs,s,mode) {
  const c=trendStop(xs,s);
  if(c.status!=='certified-candidate') return {status:c.status,certificate:c};
  const e=mode==='first'?c.eligibleEnds[0]:c.eligibleEnds.at(-1);
  return {status:'certified-candidate',certificate:c,object:{
    start:s,end:e,confirm:c.confirm,type:c.type,
    memberEvents:span(s+1,e),ownCoreIDs:c.ownCores.map(k=>k.id),
    startValue:state(xs,s),endValue:state(xs,e),range:range(path(xs,s,e)),
  }};
}

function stream(xs,mode) {
  let s=0; const certificates=[],objects=[];
  while(s<xs.length) {
    const z=select(xs,s,mode);
    if(z.status!=='certified-candidate') break;
    certificates.push(z.certificate);objects.push(z.object);s=z.object.end;
  }
  const tail={start:s,memberEvents:span(s+1,xs.length),semanticStatus:'unresolved-tail'};
  assert.deepEqual([...objects.flatMap(m=>m.memberEvents),...tail.memberEvents],span(1,xs.length));
  const coreIDs=objects.flatMap(m=>m.ownCoreIDs);
  assert.equal(new Set(coreIDs).size,coreIDs.length);
  return {objects,certificates,tail};
}

const prefixChecks={words:0,prefixes:0,confirmedComparisons:0};
function checkPrefixes(xs) {
  prefixChecks.words++;
  for(const mode of ['first','last']) {
    let prev={objects:[],certificates:[]};
    for(let n=0;n<=xs.length;n++) {
      const now=stream(xs.slice(0,n),mode);prefixChecks.prefixes++;
      assert.deepEqual(now.objects.slice(0,prev.objects.length),prev.objects);
      assert.deepEqual(now.certificates.slice(0,prev.certificates.length),prev.certificates);
      prefixChecks.confirmedComparisons+=prev.objects.length;
      prev=now;
    }
  }
}

function gcd(a,b) { while(b) [a,b]=[b,a%b]; return a; }
function W(q) {
  const a=102n*BigInt(q)+1000n,b=BigInt(q)+10n,g=gcd(a,b);
  return {numerator:String(a/g),denominator:String(b/g)};
}
function cmpW(a,b) {
  const d=BigInt(a.numerator)*BigInt(b.denominator)-BigInt(b.numerator)*BigInt(a.denominator);
  return d>0n?1:d<0n?-1:0;
}

// Every finite positive integer word can be translated into a SAME-LENGTH
// legal single-ID add/cancel history. Translation is a new numeric instance.
export function translatedOrders(xs,offset=100,policy='reserve') {
  const totalDecrease=xs.slice(1).reduce((d,x,i)=>d+Math.max(0,xs[i]-x),0);
  if(policy==='reserve') assert(xs[0]+offset>totalDecrease);
  const targets=xs.map(x=>x+offset);
  const orders=new Map([
    ['S0',{side:'bid',price:100,quantity:targets[0]}],
    ['A0',{side:'ask',price:102,quantity:10}],
  ]);
  const snapshot=()=>[...orders].map(([id,o])=>({id,...o}));
  const initial=snapshot(),events=[],states=[{index:0,q:targets[0],w:W(targets[0]),orders:snapshot()}];
  let q=targets[0],dummy=null;
  for(let j=0;j<xs.length;j++) {
    const i=j+1,target=targets[j]; let e;
    if(target>q) e={op:'add',id:`Q${i}`,side:'bid',price:100,quantity:target-q};
    else if(target<q) {
      const id=policy==='reserve'?'S0':[...orders].find(([,o])=>o.side==='bid'&&o.price===100&&o.quantity>=q-target)?.[0];
      assert(id,'single-ID decrease unavailable');
      e={op:'cancel',id,side:'bid',price:100,quantity:q-target};
    }
    else if(dummy===null) { dummy=`D${i}`; e={op:'add',id:dummy,side:'bid',price:99,quantity:1}; }
    else { e={op:'cancel',id:dummy,side:'bid',price:99,quantity:1}; dummy=null; }
    e={seq:i,occurredNs:String(i*1000000),receivedNs:String(i*1000000+500),...e};
    assert(e.quantity>0);
    if(e.op==='add') { assert(!orders.has(e.id));orders.set(e.id,{side:e.side,price:e.price,quantity:e.quantity}); }
    else {
      const old=orders.get(e.id);assert(old&&old.side===e.side&&old.price===e.price&&old.quantity>=e.quantity);
      old.quantity-=e.quantity;if(!old.quantity)orders.delete(e.id);
    }
    if(policy==='reserve') assert(orders.get('S0').quantity>0);
    const bids=[...orders.values()].filter(o=>o.side==='bid'),asks=[...orders.values()].filter(o=>o.side==='ask');
    assert.equal(Math.max(...bids.map(o=>o.price)),100);assert.equal(Math.min(...asks.map(o=>o.price)),102);
    q=bids.filter(o=>o.price===100).reduce((n,o)=>n+o.quantity,0);
    assert.equal(q,target);assert.equal(asks.filter(o=>o.price===102).reduce((n,o)=>n+o.quantity,0),10);
    events.push(e);states.push({index:i,q,w:W(q),orders:snapshot()});
  }
  assert.equal(states[0].q,states[1].q);
  for(let i=0;i<targets.length;i++) for(let j=0;j<targets.length;j++) {
    assert.equal(sign(xs[i]-xs[j]),sign(targets[i]-targets[j]));
    assert.equal(sign(targets[i]-targets[j]),cmpW(W(targets[i]),W(targets[j])));
  }
  const selections=Object.fromEntries(['first','last'].map(mode=>[mode,stream(targets,mode)]));
  for(const mode of ['first','last']) {
    const original=stream(xs,mode);
    assert.deepEqual(selections[mode].objects.map(m=>[m.start,m.end,m.confirm,m.type,m.ownCoreIDs]),
      original.objects.map(m=>[m.start,m.end,m.confirm,m.type,m.ownCoreIDs]));
    for(let i=0;i<original.objects.length;i++) assert.deepEqual(selections[mode].objects[i].range,original.objects[i].range.map(x=>x+offset));
  }
  return {kind:'synthetic-legal-not-market-capture',policy,originalCoordinates:xs,offset,totalDecrease,
    actualQ:targets,initial,events,states,selections};
}

const cases={
  doublePeak15:[...rep(10,20),30,25,30,...rep(10,20)],
  doublePeak18:[...rep(10,20),30,25,30,...rep(10,5,15)],
  doublePeak24:[...rep(10,20),30,25,30,...rep(10,5,15,25,10)],
  earliestSplitsCore9:rep(10,20,10),
  equalityTrigger10:[...rep(10,20),15,...rep(20)],
  earlierSpike10:[...rep(10),30,...rep(20,15)],
  firstBlocksRight18:[...rep(5,20),30,10,30,...rep(15,12,20)],
  laterOvershoot19:[...rep(10,20),30,25,30,...rep(10),35,...rep(5,20)],
  triggerContinues14:[...rep(10,20),30,25,30,10,10,10,10,10],
  laterEqualPeak13:[...rep(10,20),30,25,30,...rep(10),30],
};
const old=JSON.parse(readFileSync(resolve(dirname(fileURLToPath(import.meta.url)),
  '../../stage47/base-construction-evidence/results/examples.json'),'utf8'));
for(const [name,entry] of Object.entries(old)) cases[`stage47_${name}`]=entry.states.slice(1).map(s=>s.q);
const coordinateResults={};
for(const [name,xs] of Object.entries(cases)) {
  checkPrefixes(xs);
  coordinateResults[name]={xs,catalog:catalog(xs),first:stream(xs,'first'),last:stream(xs,'last')};
}
const dp=coordinateResults.doublePeak24;
assert.deepEqual(dp.first.objects.map(m=>[m.end,m.confirm]),[[7,12],[15,18],[21,24]]);
assert.deepEqual(dp.last.objects.map(m=>[m.end,m.confirm]),[[9,12],[15,18],[21,24]]);
const intersect=os=>[Math.max(...os.map(m=>m.range[0])),Math.min(...os.map(m=>m.range[1]))];
assert.deepEqual(intersect(dp.first.objects),[10,25]);assert.deepEqual(intersect(dp.last.objects),[10,25]);
assert.deepEqual(dp.first.certificates[0].eligibleEnds,[7,9]);
assert.equal(coordinateResults.earliestSplitsCore9.first.certificates[0].rawExtremePositions[0],4);
assert.deepEqual(coordinateResults.earliestSplitsCore9.first.certificates[0].eligibleEnds,[6]);
assert.equal(trendStop(cases.earlierSpike10).status,'no-normal-endpoint');
assert.equal(trendStop(cases.firstBlocksRight18,7).status,'no-normal-endpoint');
assert.deepEqual(trendStop(cases.firstBlocksRight18,9).eligibleEnds,[15]);
assert.equal(trendStop(cases.laterOvershoot19,7).status,'no-normal-endpoint');
assert.equal(trendStop(cases.laterOvershoot19,9).status,'no-normal-endpoint');
const firstTriggerEnd=catalog(cases.triggerContinues14.slice(0,12)).at(-1).end;
const extendedTriggerEnd=catalog(cases.triggerContinues14).at(-1).end;
assert.deepEqual([firstTriggerEnd,extendedTriggerEnd],[12,14]);
assert.deepEqual(select(cases.triggerContinues14.slice(0,12),0,'last'),select(cases.triggerContinues14,0,'last'));

// Independent bounded coordinate check: all ternary words, initial three equal,
// length 3..10. This is finite computational evidence, not an all-history proof.
let enumerated=0,withCertificate=0,withNoNormalEndpoint=0;
for(let len=3;len<=10;len++) {
  const visit=xs=>{
    if(xs.length<len) {for(const x of [10,20,30])visit([...xs,x]);return;}
    enumerated++;checkPrefixes(xs);const t=trendStop(xs);
    if(t.status==='certified-candidate')withCertificate++;
    if(t.status==='no-normal-endpoint')withNoNormalEndpoint++;
  };
  for(const x of [10,20,30])visit([x,x,x]);
}
assert.equal(enumerated,9840);
const native=Object.fromEntries(Object.entries(cases).filter(([n])=>['doublePeak15','doublePeak18','doublePeak24'].includes(n)).map(([n,xs])=>[n,translatedOrders(xs)]));
native.literalEarlierSpike10=translatedOrders(cases.earlierSpike10,0,'available-order');
const out=resolve(process.argv[2]??resolve(dirname(fileURLToPath(import.meta.url)),'results'));
mkdirSync(out,{recursive:true});
const summary={namedCoordinateCases:Object.keys(cases).length,enumerated,withCertificate,withNoNormalEndpoint,
  prefixChecks,legalSyntheticCases:Object.keys(native).length,legalSyntheticEvents:Object.values(native).reduce((n,e)=>n+e.events.length,0),
  original24Ranges:dp.first.objects.map(m=>m.range),original24Intersection:intersect(dp.first.objects),
  translated24Ranges:native.doublePeak24.selections.first.objects.map(m=>m.range),
  translated24Intersection:intersect(native.doublePeak24.selections.first.objects),
  status:'author-development-check-not-independent-review'};
for(const [name,value] of Object.entries({coordinateResults,native,summary}))writeFileSync(resolve(out,`${name}.json`),JSON.stringify(value,null,2)+'\n');
console.log(JSON.stringify({out,...summary}));
