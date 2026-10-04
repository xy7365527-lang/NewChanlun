// #1467 Stage48. All event cuts, exact path counts. Candidate conditions only.
// Usage: node partition-dag.mjs [fresh-output-directory]
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const profiles = ['CORE','TREND-NORM','TREND-ONLY'];
const policies = ['BASE','CLOSED-CUT'];
const sign = x => x > 0 ? 1 : x < 0 ? -1 : 0;
const json = x => JSON.stringify(x, (_,v)=>typeof v==='bigint'?String(v):v,2)+'\n';
const same = (a,b) => JSON.stringify(a)===JSON.stringify(b);

export function catalogue(xs,initial=xs[0]??10) {
  assert(xs.every(x=>Number.isSafeInteger(x)&&x>0));
  assert(xs.length===0 || initial===xs[0]);
  const runs=[];
  xs.forEach((value,j)=>{
    if(runs.length&&runs.at(-1).value===value)runs.at(-1).end=j+1;
    else runs.push({id:runs.length,start:j+1,end:j+1,value});
  });
  if(runs.length>1)assert(runs[0].end>=3,'outside initial-anchor coordinate domain');
  const cores=runs.filter(p=>p.end-p.start+1>=3).map(p=>({
    id:`K@${p.start}`,start:p.start,end:p.end,value:p.value,birth:p.start+2,
    seed:[p.start,p.start+1,p.start+2],closed:p.id<runs.length-1,
    sealedAt:p.id<runs.length-1?p.end+1:null,
  }));
  return {n:xs.length,initial,states:[initial,...xs],runs,cores,closedThrough:runs.length?runs.at(-1).start-1:0};
}

export function edgeDecision(cat,s,e,profile,policy) {
  if(!(s>=0&&s<e&&e<=cat.n))return {reject:'not-nonempty-event-interval'};
  if(policy==='CLOSED-CUT'&&e>cat.closedThrough)return {reject:'open-source-consumed'};
  for(const k of cat.cores) {
    const overlaps=s<k.end&&e>=k.start;
    if(overlaps&&!k.closed)return {reject:'unsealed-core-consumed'};
    if(overlaps&&!(s<k.start&&k.end<=e))return {reject:'mature-core-split'};
  }
  const own=cat.cores.filter(k=>k.closed&&s<k.start&&k.end<=e);
  if(!own.length)return {reject:'no-owned-core'};
  let type='P',direction=null;
  if(own.length>=2) {
    const up=own.every((k,i)=>i===0||k.value>own[i-1].value);
    const down=own.every((k,i)=>i===0||k.value<own[i-1].value);
    if(!up&&!down)return {reject:'cores-not-strictly-same-direction'};
    type=up?'U':'D';direction=up?1:-1;
  }
  const path=cat.states.slice(s,e+1),range=[Math.min(...path),Math.max(...path)];
  if(profile!=='CORE'&&direction!==null) {
    const norm=direction===1
      ?cat.states[s]===range[0]&&cat.states[e]===range[1]&&range[0]<range[1]
      :cat.states[s]===range[1]&&cat.states[e]===range[0]&&range[0]<range[1];
    if(!norm)return {reject:'trend-endpoints-not-normal'};
  }
  if(profile==='TREND-ONLY'&&type==='P')return {reject:'p-excluded-by-profile'};
  const endRun=cat.runs.find(p=>p.start<=e&&e<=p.end);
  return {edge:{s,e,type,direction,ownCoreIDs:own.map(k=>k.id),range,
    startValue:cat.states[s],endValue:cat.states[e],sourceEnd:e,evaluatedAt:cat.n,
    catalogueFixedAt:endRun&&endRun.id<cat.runs.length-1?endRun.end+1:null,
    semanticCompletion:null}};
}

export function graph(xs,profile,policy,initial=xs[0]??10) {
  const cat=catalogue(xs,initial),edges=[],rejects={},out=Array.from({length:xs.length+1},()=>[]);
  for(let e=1;e<=cat.n;e++)for(let s=0;s<e;s++) {
    const d=edgeDecision(cat,s,e,profile,policy);
    if(d.edge){edges.push(d.edge);out[s].push(d.edge);}
    else rejects[d.reject]=(rejects[d.reject]??0)+1;
  }
  assert.equal(edges.length+Object.values(rejects).reduce((a,b)=>a+b,0),cat.n*(cat.n+1)/2);
  return {cat,profile,policy,edges,out,rejects};
}

// dp[k][b] counts ALL paths from start to k using exactly b edges.
export function paths(g,start=0) {
  const n=g.cat.n,dp=Array.from({length:n+1},()=>Array(n+1).fill(0n));
  const witness=Array.from({length:n+1},()=>Array.from({length:n+1},()=>[]));
  dp[start][0]=1n;witness[start][0]=[[start]];
  for(let s=start;s<=n;s++)for(const edge of g.out[s])for(let b=0;b<n;b++) {
    if(dp[s][b]===0n)continue;
    dp[edge.e][b+1]+=dp[s][b];
    for(const w of witness[s][b])if(witness[edge.e][b+1].length<2)witness[edge.e][b+1].push([...w,edge.e]);
  }
  return {dp,witness};
}

function three(g) {
  const count=Array(g.cat.n+1).fill(0n),witness=Array.from({length:g.cat.n+1},()=>[]);
  for(const a of g.out[0])for(const b of g.out[a.e])for(const c of g.out[b.e]) {
    if(a.direction===null||b.direction===null||c.direction===null)continue;
    if(a.direction!==-b.direction||b.direction!==-c.direction)continue;
    const intersection=[Math.max(a.range[0],b.range[0],c.range[0]),Math.min(a.range[1],b.range[1],c.range[1])];
    if(intersection[0]>=intersection[1])continue;
    count[c.e]++;
    if(witness[c.e].length<2)witness[c.e].push({cuts:[0,a.e,b.e,c.e],types:[a.type,b.type,c.type],intersection,ranges:[a.range,b.range,c.range]});
  }
  return {count,witness};
}

function summary(g,includeWitness=false) {
  const p=paths(g),t=three(g);
  return {profile:g.profile,policy:g.policy,n:g.cat.n,examinedPairs:g.cat.n*(g.cat.n+1)/2,
    edgeCount:g.edges.length,rejects:g.rejects,
    endpoints:p.dp.map((row,k)=>({k,total:row.reduce((a,b)=>a+b,0n),
      byBlocks:Object.fromEntries(row.flatMap((v,b)=>v?[[b,v]]:[])),
      threeTrendStrict:t.count[k],...(includeWitness?{witnesses:Object.fromEntries(p.witness[k].flatMap((w,b)=>w.length?[[b,w]]:[])),threeWitnesses:t.witness[k]}:{})})),
  };
}

// Separate direct partition oracle: every subset of cuts, no DAG traversal.
// It reads platform footprints directly and does not call edgeDecision.
function oracleEdge(cat,s,e,profile,policy) {
  if(policy==='CLOSED-CUT'&&e>cat.closedThrough)return false;
  const members=cat.cores.filter(k=>Array.from({length:k.end-k.start+1},(_,i)=>k.start+i).some(i=>s<i&&i<=e));
  if(members.some(k=>!k.closed||k.start<=s||k.end>e)||members.length===0)return false;
  let dir=0;
  if(members.length>1) {
    dir=sign(members[1].value-members[0].value);
    if(dir===0||members.slice(1).some((k,i)=>sign(k.value-members[i].value)!==dir))return false;
  }
  if(profile==='TREND-ONLY'&&dir===0)return false;
  if(profile!=='CORE'&&dir!==0) {
    const segment=cat.states.slice(s,e+1),first=segment[0],last=segment.at(-1);
    if(dir===1&&!(first<last&&segment.every(x=>first<=x&&x<=last)))return false;
    if(dir===-1&&!(first>last&&segment.every(x=>last<=x&&x<=first)))return false;
  }
  return true;
}

function oracleCounts(cat,profile,policy) {
  const counts=Array.from({length:cat.n+1},()=>Array(cat.n+1).fill(0n));counts[0][0]=1n;
  for(let k=1;k<=cat.n;k++)for(let mask=0;mask<2**(k-1);mask++) {
    const cuts=[0];for(let i=1;i<k;i++)if(mask&(2**(i-1)))cuts.push(i);cuts.push(k);
    if(cuts.slice(1).every((e,i)=>oracleEdge(cat,cuts[i],e,profile,policy)))counts[k][cuts.length-1]++;
  }
  return counts;
}

function replay(record) {
  const orders=new Map(record.initial.map(x=>[x.id,{...x}]));
  const quantity=()=>[...orders.values()].filter(x=>x.side==='bid'&&x.price===100).reduce((a,x)=>a+x.quantity,0);
  const initial=quantity(),xs=[];
  record.events.forEach((e,i)=>{
    assert.equal(e.seq,i+1);assert(e.quantity>0);
    if(e.op==='add') {assert(!orders.has(e.id));orders.set(e.id,{id:e.id,side:e.side,price:e.price,quantity:e.quantity});}
    else {assert.equal(e.op,'cancel');const o=orders.get(e.id);assert(o&&o.side===e.side&&o.price===e.price&&o.quantity>=e.quantity);o.quantity-=e.quantity;if(!o.quantity)orders.delete(e.id);}
    assert.equal(Math.max(...[...orders.values()].filter(x=>x.side==='bid').map(x=>x.price)),100);
    assert.equal(Math.min(...[...orders.values()].filter(x=>x.side==='ask').map(x=>x.price)),102);
    assert.equal([...orders.values()].filter(x=>x.side==='ask'&&x.price===102).reduce((a,x)=>a+x.quantity,0),10);
    xs.push(quantity());
  });
  assert(initial>0&&xs.every(x=>x>0));return {initial,xs};
}

function makeNative(xs) {
  const initial=[{id:'S0',side:'bid',price:100,quantity:xs[0]},{id:'A0',side:'ask',price:102,quantity:10}];
  const orders=new Map(initial.map(o=>[o.id,{...o}])),events=[];let q=xs[0],dummy=null;
  for(let j=0;j<xs.length;j++) {
    const i=j+1,delta=xs[j]-q;let e;
    if(delta>0)e={op:'add',id:`Q${i}`,side:'bid',price:100,quantity:delta};
    else if(delta<0) {
      const hit=[...orders.values()].find(o=>o.side==='bid'&&o.price===100&&o.quantity>=-delta);
      if(!hit)return {failure:{event:i,from:q,to:xs[j],requestedCancel:-delta,largestAvailable:Math.max(...[...orders.values()].filter(o=>o.side==='bid'&&o.price===100).map(o=>o.quantity))}};
      e={op:'cancel',id:hit.id,side:'bid',price:100,quantity:-delta};
    }else if(dummy===null){dummy=`D${i}`;e={op:'add',id:dummy,side:'bid',price:99,quantity:1};}
    else{e={op:'cancel',id:dummy,side:'bid',price:99,quantity:1};dummy=null;}
    if(e.op==='add')orders.set(e.id,{...e});else{const o=orders.get(e.id);o.quantity-=e.quantity;if(o.quantity===0)orders.delete(e.id);}
    events.push({seq:i,occurredNs:String(i*1000000),receivedNs:String(i*1000000+500),...e});q=xs[j];
  }
  const record={initial,events};assert.deepEqual(replay(record).xs,xs);return {record};
}

const old=JSON.parse(readFileSync(resolve(here,'../../stage47/base-construction-evidence/results/examples.json'),'utf8'));
const input=[];
for(const [name,record]of Object.entries(old)) {
  const r=replay(record);assert.deepEqual(r.xs,record.states.slice(1).map(s=>s.q));
  input.push({name,kind:'stage47-native-replay',...r,native:{initial:record.initial,events:record.events}});
}
const rep=a=>a.flatMap(x=>[x,x,x]);
const added={
  repeated15:[...rep([10,20]),30,25,30,...rep([10,20])],
  repeated18:[...rep([10,20]),30,25,30,...rep([10,5,15])],
  repeated24:[...rep([10,20]),30,25,30,...rep([10,5,15,25,10])],
};
const reachability={};
for(const[name,xs]of Object.entries(added)) {
  const unshifted=makeNative(xs);assert(unshifted.failure);reachability[name]=unshifted;
  input.push({name,kind:'coordinate-only-new-diagnostic',initial:xs[0],xs});
  const shifted=xs.map(x=>x+100),native=makeNative(shifted);assert(native.record);
  input.push({name:`${name}Shift100`,kind:'new-native-shifted-witness',initial:shifted[0],xs:shifted,native:native.record});
}
const fixedOwnedXs=[...rep([10]),30,...rep([20,15])];
const fixedOwnedNative=makeNative(fixedOwnedXs);assert(fixedOwnedNative.record);
input.push({name:'fixedOwned10',kind:'new-native-fixed-owned-diagnostic',initial:10,xs:fixedOwnedXs,native:fixedOwnedNative.record});

const output=resolve(process.argv[2]??resolve(here,'results'));mkdirSync(output,{recursive:true});
const put=(name,value)=>writeFileSync(resolve(output,`${name}.json`),json(value));
put('inputs',{inputs:input,reachability});
const finalGraphs=new Map(),allPrefixResults=[];
let graphCount=0,pairCount=0,closedCutPersistenceComparisons=0,baseLostEdgeCount=0;
for(const h of input) {
  const prefixes=[],previous=new Map();
  for(let n=0;n<=h.xs.length;n++) {
    const rows=[];
    for(const profile of profiles)for(const policy of policies) {
      const g=graph(h.xs.slice(0,n),profile,policy,h.initial);graphCount++;pairCount+=n*(n+1)/2;
      const key=`${profile}/${policy}`,past=previous.get(key);
      if(past) {
        const now=new Map(g.edges.map(e=>[`${e.s},${e.e}`,e]));
        for(const oldEdge of past.edges) {
          if(policy==='CLOSED-CUT'){
            const fresh=now.get(`${oldEdge.s},${oldEdge.e}`);assert(fresh);
            const stable=e=>({s:e.s,e:e.e,type:e.type,direction:e.direction,ownCoreIDs:e.ownCoreIDs,range:e.range,startValue:e.startValue,endValue:e.endValue,catalogueFixedAt:e.catalogueFixedAt});
            assert.deepEqual(stable(oldEdge),stable(fresh));closedCutPersistenceComparisons++;
          }
          else if(!now.has(`${oldEdge.s},${oldEdge.e}`))baseLostEdgeCount++;
        }
      }
      previous.set(key,g);rows.push(summary(g,false));
      if(n===h.xs.length)finalGraphs.set(`${h.name}/${key}`,g);
    }
    prefixes.push({n,rows});
  }
  allPrefixResults.push({name:h.name,kind:h.kind,length:h.xs.length,prefixes});
}
put('all-prefix-counts',allPrefixResults);
put('final-graphs',Object.fromEntries([...finalGraphs].map(([key,g])=>[key,{cat:g.cat,edges:g.edges,summary:summary(g,true)}])));

// Explicit fixed endpoint queries; no sums across unrelated consumed prefixes.
const querySpecs=[['minimal6',5,1],['minimal6',5,2],['priorTrend8',7,1],['priorTrend8',7,2],
 ['forwardMerge16',13,2],['geometric30',27,3],['repeated15',12,2],['repeated18',15,2],['repeated24',21,3],
 ['repeated15Shift100',12,2],['repeated18Shift100',15,2],['repeated24Shift100',21,3],['fixedOwned10',7,1],['fixedOwned10',7,2]];
const fixedQueries=querySpecs.map(([name,k,b])=>({name,k,b,rows:profiles.flatMap(profile=>policies.map(policy=>{
  const g=finalGraphs.get(`${name}/${profile}/${policy}`),p=paths(g),t=three(g);
  return {profile,policy,count:p.dp[k][b],witnesses:p.witness[k][b],threeTrendStrict:t.count[k],threeWitnesses:t.witness[k]};
}))}));
put('fixed-queries',fixedQueries);

const longEdgeRows=profiles.flatMap(profile=>policies.map(policy=>{
  const g=finalGraphs.get(`forwardMerge16/${profile}/${policy}`),p=paths(g),suffix=paths(g,6);
  let lockedCount=0n;for(let b=0;b<=2;b++)lockedCount+=p.dp[6][b]*suffix.dp[13][2-b];
  return {profile,policy,short:edgeDecision(g.cat,6,7,profile,policy),long:edgeDecision(g.cat,6,13,profile,policy),fixedK:13,blocks:2,lockAt:6,lockedCount};
}));
assert(longEdgeRows.every(r=>r.long.edge?.type==='D'&&r.short.reject==='no-owned-core'));
put('long-edge-and-lock',longEdgeRows);

const repeated=input.find(h=>h.name==='repeated15');
const repeatG=graph(repeated.xs.slice(0,12),'TREND-NORM','BASE',repeated.initial);
const fixedOwned=repeatG.edges.filter(e=>e.s===0&&e.type==='U'&&same(e.ownCoreIDs,['K@1','K@4']));
const nextCore=repeatG.cat.cores.find(k=>k.id==='K@10');assert(nextCore&&!nextCore.closed&&nextCore.birth===12);
const witnessHull=[Math.min(...repeatG.cat.states.slice(0,nextCore.birth+1)),Math.max(...repeatG.cat.states.slice(0,nextCore.birth+1))];
const repeatDiagnostic={evaluatedAt:12,profile:'TREND-NORM',policy:'BASE',fixedOwned:['K@1','K@4'],nextMatureCore:nextCore,
  allFirstUEnds:fixedOwned.map(e=>e.e),edges:fixedOwned,witnessHull,
  withNamedWitnessHullExtremum:fixedOwned.filter(e=>e.endValue===witnessHull[1]).map(e=>e.e),semanticCompletion:null};
assert.deepEqual(repeatDiagnostic.allFirstUEnds,[6,7,9]);
assert.deepEqual(repeatDiagnostic.withNamedWitnessHullExtremum,[7,9]);
put('repeated-peak',repeatDiagnostic);

const fixedCat=catalogue(fixedOwnedXs),k20=fixedCat.cores.find(k=>k.id==='K@5');
const allEnds=Array.from({length:fixedCat.n},(_,i)=>i+1);
const wholeK20Ends=allEnds.filter(e=>e>=k20.end);
const normalUpEnds=allEnds.filter(e=>{
  const a=fixedCat.states.slice(0,e+1),lo=Math.min(...a),hi=Math.max(...a);
  return fixedCat.states[0]===lo&&fixedCat.states[e]===hi&&lo<hi;
});
assert.deepEqual(wholeK20Ends,[7,8,9,10]);assert.deepEqual(normalUpEnds,[4]);
const fixedOwnedDiagnostic={name:'fixedOwned10',n:10,states:fixedCat.states,ownCoreIDs:['K@1','K@5'],
  nextMatureCore:fixedCat.cores.find(k=>k.id==='K@8'),wholeK20Ends,normalUpEnds,
  intersection:wholeK20Ends.filter(e=>normalUpEnds.includes(e)),
  rows:profiles.flatMap(profile=>policies.map(policy=>{
    const g=finalGraphs.get(`fixedOwned10/${profile}/${policy}`);
    return {profile,policy,firstEdgesWithFixedCores:g.edges.filter(e=>e.s===0&&same(e.ownCoreIDs,['K@1','K@5']))};
  })),
  scope:'two-subcondition minimal incompatibility for fixed history/start; not global impossibility of P partitions'};
put('fixed-owned-conflict',fixedOwnedDiagnostic);

function fractionW(q){let a=102n*BigInt(q)+1000n,b=BigInt(q)+10n;let x=a,y=b;while(y)[x,y]=[y,x%y];return `${a/x}/${b/x}`;}
const doublePartitions={};
for(const name of ['repeated18','repeated24','repeated18Shift100','repeated24Shift100']) {
  const g=finalGraphs.get(`${name}/TREND-ONLY/CLOSED-CUT`),p=paths(g),isThree=name.includes('24'),k=isThree?21:15,b=isThree?3:2;
  assert.equal(p.dp[k][b],2n);
  doublePartitions[name]={profile:'TREND-ONLY',policy:'CLOSED-CUT',k,blocks:b,count:p.dp[k][b],
    witnesses:p.witness[k][b].map(cuts=>({cuts,tail:[k+1,g.cat.n],
      edges:cuts.slice(1).map((e,i)=>{
        const edge=g.out[cuts[i]].find(x=>x.e===e),last=g.cat.cores.find(x=>x.id===edge.ownCoreIDs.at(-1));
        const next=g.cat.cores.find(x=>x.start>last.end);assert(next);
        const nonAlong=edge.direction===1?next.value<=last.value:next.value>=last.value;assert(nonAlong);
        return {...edge,wRange:edge.range.map(fractionW),nextMatureWitness:{id:next.id,value:next.value,birth:next.birth,seed:next.seed,nonAlong},semanticCompletion:null};
      }),
    }))};
}
put('double-partitions',doublePartitions);

for(const name of Object.keys(added))for(const profile of profiles)for(const policy of policies) {
  const a=finalGraphs.get(`${name}/${profile}/${policy}`),b=finalGraphs.get(`${name}Shift100/${profile}/${policy}`);
  const shape=e=>[e.s,e.e,e.type,e.direction,e.ownCoreIDs];
  assert.deepEqual(a.edges.map(shape),b.edges.map(shape));
  assert.deepEqual(paths(a).dp,paths(b).dp);
}

const openness={
  at4:summary(graph([10,10,10,20],'CORE','BASE'),true),
  at6:summary(graph([10,10,10,20,20,20],'CORE','BASE'),true),
  closedCutAt4:summary(graph([10,10,10,20],'CORE','CLOSED-CUT'),true),
  closedCutAt6:summary(graph([10,10,10,20,20,20],'CORE','CLOSED-CUT'),true),
};
assert.equal(openness.at4.endpoints[4].byBlocks[1],1n);assert.equal(openness.at6.endpoints[4].total,0n);
put('open-platform',openness);

// Deliberately exceed Number's exact-integer range without enumerating paths.
const increasing60=Array.from({length:60},(_,i)=>i+1).flatMap(x=>[x,x,x]);
const bigGraph=graph([...increasing60,61],'CORE','CLOSED-CUT'),bigPaths=paths(bigGraph);
const choose=(n,k)=>{let v=1n;for(let i=1;i<=k;i++)v=v*BigInt(n-i+1)/BigInt(i);return v;};
for(let b=1;b<=60;b++)assert.equal(bigPaths.dp[180][b],choose(59,b-1));
const bigTotal=bigPaths.dp[180].reduce((a,b)=>a+b,0n);assert.equal(bigTotal,2n**59n);
put('bigint-stress',{domain:'coordinate-only monotone 60 closed plateau cores plus one sealing observation',n:181,k:180,profile:'CORE',policy:'CLOSED-CUT',
  total:bigTotal,formula:'2^59',exceedsNumberSafeInteger:bigTotal>BigInt(Number.MAX_SAFE_INTEGER),
  byBlocks:Object.fromEntries(bigPaths.dp[180].flatMap((v,b)=>v?[[b,v]]:[]))});

let coordinateWords=0,oracleGraphs=0,oracleCells=0;
const histogram={};
for(let n=3;n<=8;n++) {
  let words=0;
  function visit(xs) {
    if(xs.length<n){for(const x of[10,20,30])visit([...xs,x]);return;}
    coordinateWords++;words++;
    for(const profile of profiles)for(const policy of policies) {
      const g=graph(xs,profile,policy),p=paths(g),oracle=oracleCounts(g.cat,profile,policy);
      assert.deepEqual(p.dp,oracle);oracleGraphs++;oracleCells+=(n+1)**2;
    }
  }
  for(const x of[10,20,30])visit([x,x,x]);histogram[n]=words;
}
const counts={namedHistories:input.length,stage47NativeHistories:7,stage47NativeEvents:Object.values(old).reduce((n,h)=>n+h.events.length,0),
  newCoordinateDiagnostics:3,newShiftedNativeHistories:3,newShiftedNativeEvents:Object.values(added).reduce((n,x)=>n+x.length,0),
  newFixedOwnedNativeHistories:1,newFixedOwnedNativeEvents:10,
  namedPrefixRows:input.reduce((n,h)=>n+h.xs.length+1,0),graphCount,pairCount,closedCutPersistenceComparisons,baseLostEdgeCount,
  coordinateWords,oracleGraphs,oracleCells,histogram,verdict:'finite-author-checks-not-independent-review'};
put('checks',counts);console.log(json(counts));
