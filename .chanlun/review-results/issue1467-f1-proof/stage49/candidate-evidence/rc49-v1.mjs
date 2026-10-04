import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const out = process.argv[2] ?? path.join(here, 'results-v1');
fs.mkdirSync(out, {recursive: true});
const hull = xs => [Math.min(...xs), Math.max(...xs)];
const intersection = xs => [Math.max(...xs.map(x=>x[0])), Math.min(...xs.map(x=>x[1]))];
const touch = (a,b) => Math.max(a[0],b[0]) <= Math.min(a[1],b[1]);
const rel = (a,b) => b[0]>a[1] ? 'Up' : b[1]<a[0] ? 'Down' : 'Overlap';
function gcd(a,b) { while(b) [a,b]=[b,a%b]; return a; }
function W(q) {
  const a=1000n+102n*BigInt(q), b=10n+BigInt(q), g=gcd(a,b);
  return `${a/g}/${b/g}`;
}
const wrange = r => r.map(W);
function priceOfQuantity(a,b) {
  return (1000n+102n*BigInt(a))*(10n+BigInt(b)) <
    (1000n+102n*BigInt(b))*(10n+BigInt(a));
}

// Every event is a real order mutation, including unchanged best quantity.
function source(word, shift=1000) {
  assert.ok(word.length>0 && word.every(Number.isSafeInteger));
  const targets=word.map(q=>q+shift);
  assert.ok(targets.every(q=>q>0));
  const initial={bid:{id:'reservoir',price:100,quantity:targets[0]},
    ask:{id:'ask',price:102,quantity:10}};
  const orders=new Map([['reservoir',{...initial.bid}], ['ask',{...initial.ask}]]);
  let reserve=targets[0], previous=targets[0];
  const events=[], states=[{event:0,q:previous,w:W(previous)}];
  for(let i=0;i<targets.length;i++) {
    const wanted=targets[i], d=wanted-previous, seq=i+1;
    let event;
    if(d>0) { event={seq,kind:'add',id:`up-${seq}`,side:'bid',price:100,quantity:d};
      orders.set(event.id,{price:100,quantity:d}); }
    else if(d<0) {
      assert.ok(reserve+d>0, 'finite reservoir is sufficient');
      event={seq,kind:'reduce',id:'reservoir',quantity:-d};
      reserve+=d; orders.get('reservoir').quantity=reserve;
    } else {
      event={seq,kind:'add',id:`deep-${seq}`,side:'bid',price:99,quantity:1};
      orders.set(event.id,{price:99,quantity:1});
    }
    const actual=[...orders.values()].filter(o=>o.price===100).reduce((s,o)=>s+o.quantity,0);
    assert.equal(actual,wanted);
    assert.ok([...orders.values()].every(o=>o.quantity>0));
    events.push(event); states.push({event:seq,q:actual,w:W(actual)}); previous=actual;
  }
  return {epoch:'synthetic-rc49-v1',initial,events,states,coordinates:word,shift};
}

function legs(q) {
  const completed=[];
  let start=0, direction=0;
  for(let i=1;i<q.length;i++) {
    const s=Math.sign(q[i]-q[i-1]);
    if(s===0) continue;
    if(direction===0) {direction=s;continue;}
    if(s!==direction) {
      const end=i-1;
      completed.push({id:completed.length,start,end,direction:direction>0?'Up':'Down',
        range:hull(q.slice(start,end+1)),knownAt:i});
      start=end;direction=s;
    }
  }
  return {completed, active:{start,end:q.length-1,direction:direction===0?null:direction>0?'Up':'Down',
    range:hull(q.slice(start)),knownAt:null}};
}

function cores(ls) {
  const completed=[],skipped=[];
  let cursor=0, active=null;
  while(cursor+2<ls.length) {
    const seed=ls.slice(cursor,cursor+3), z=intersection(seed.map(l=>l.range));
    if(!(z[0]<z[1])) { skipped.push(cursor++);continue; }
    assert.notEqual(seed[0].direction,seed[1].direction);
    assert.notEqual(seed[1].direction,seed[2].direction);
    let last=cursor+2;
    const make = () => {
      const members=ls.slice(cursor,last+1);
      return {id:`K${completed.length}`,level:0,seed:seed.map(l=>l.id),core:z,
        members:members.map(l=>l.id),sourceStart:members[0].start,sourceEnd:members.at(-1).end,
        outer:hull(members.flatMap(l=>l.range)),bornAt:seed.at(-1).knownAt,
        startDirection:seed[0].direction};
    };
    let closed=false;
    while(last+2<ls.length) {
      const L=ls[last+1],R=ls[last+2];
      if(touch(R.range,z)) {last+=2;continue;}
      const c=make();
      c.sealedAt=Math.max(L.knownAt,R.knownAt);
      c.exit={departure:L.id,return:R.id,returnRange:R.range,
        side:R.range[0]>z[1]?'Up':'Down',knownAt:c.sealedAt,
        scope:'RC49 initial stopping law; not original sublevel-move certificate'};
      completed.push(c);cursor=last+2;closed=true;break;
    }
    if(!closed) {active=make();break;}
  }
  return {completed,active,skipped,scanCursor:cursor};
}

function construct(q) {
  const ls=legs(q), cs=cores(ls.completed), outputs=[];
  let boundary=0;
  const catalog=cs.completed;
  let current=catalog.length?[catalog[0]]:[],direction=null;
  function emit(witness,relation) {
    const first=current[0],last=current.at(-1),end=last.sourceEnd;
    const range=hull(q.slice(boundary,end+1));
    const type=current.length===1?'P':direction==='Up'?'U':'D';
    const m={id:`M${outputs.length}`,epoch:'synthetic-rc49-v1',level:0,type,
      conceptDirection:type==='P'?null:direction,breakDirection:last.exit.side,
      start:boundary,end,sourceEvents:Array.from({length:end-boundary},(_,i)=>boundary+i+1),
      startValue:q[boundary],endValue:q[end],range,wRange:wrange(range),
      ownedCoreIDs:current.map(c=>c.id),cores:current,
      bornAt:current.length===1?first.bornAt:current[1].bornAt,
      confirmedAt:witness.sealedAt,publishedAt:witness.sealedAt,
      completion:{kind:current.length===1?'overlapping-sealed-successor':'noncontinuing-sealed-successor',
        witnessCore:witness.id,relation,knownAt:witness.sealedAt,
        scope:'candidate completion rule, to be checked against original semantics'}};
    assert.ok(m.end<m.confirmedAt);
    outputs.push(m);boundary=end;
  }
  for(let i=1;i<catalog.length;i++) {
    const c=catalog[i],relation=rel(current.at(-1).outer,c.outer);
    if(current.length===1) {
      if(relation==='Overlap') {emit(c,relation);current=[c];direction=null;}
      else {current.push(c);direction=relation;}
    } else if(relation===direction) current.push(c);
    else {emit(c,relation);current=[c];direction=null;}
  }
  const tail={start:boundary,end:q.length-1,
    sourceEvents:Array.from({length:q.length-1-boundary},(_,i)=>boundary+i+1),
    range:hull(q.slice(boundary)),wRange:wrange(hull(q.slice(boundary))),
    sealedCoreIDs:current.map(c=>c.id),activeCore:cs.active,activeLeg:ls.active,
    status:cs.active||current.length?'CoreActive':'NoCore'};
  return {legs:ls,coreScan:cs,outputs,tail};
}

function check(q, r) {
  assert.deepEqual([...r.outputs.flatMap(m=>m.sourceEvents),...r.tail.sourceEvents],
    Array.from({length:q.length-1},(_,i)=>i+1));
  const ids=r.outputs.flatMap(m=>m.ownedCoreIDs);
  assert.equal(new Set(ids).size,ids.length);
  const ownedLegs=r.coreScan.completed.flatMap(c=>c.members);
  assert.equal(new Set(ownedLegs).size,ownedLegs.length);
  for(const c of r.coreScan.completed) {
    assert.deepEqual(c.core,intersection(c.seed.map(id=>r.legs.completed[id].range)));
    assert.deepEqual(c.outer,hull(c.members.flatMap(id=>r.legs.completed[id].range)));
    assert.ok(c.core[0]<c.core[1]);
    assert.ok(!touch(c.exit.returnRange,c.core));
    assert.ok(!c.members.includes(c.exit.departure)&&!c.members.includes(c.exit.return));
  }
  for(const m of r.outputs) {
    assert.deepEqual(m.range,hull(q.slice(m.start,m.end+1)));
    assert.deepEqual(m.wRange,wrange(m.range));
    assert.equal(m.ownedCoreIDs.length,m.cores.length);
    if(m.type==='P') {assert.equal(m.cores.length,1);assert.equal(m.conceptDirection,null);}
    else {
      assert.ok(m.cores.length>=2);
      for(let j=1;j<m.cores.length;j++) assert.equal(rel(m.cores[j-1].outer,m.cores[j].outer),m.conceptDirection);
    }
  }
  for(let j=1;j<r.outputs.length;j++) {
    assert.equal(r.outputs[j-1].end,r.outputs[j].start);
    assert.equal(r.outputs[j-1].endValue,r.outputs[j].startValue);
  }
}

const rep=(q,k=3)=>Array(k).fill(q);
const cases={
  same_up_counterexample:[40,0,30,10,140,100,130,110,140,135,138,136,
    240,200,230,210,240,235,238,236,340,300,330,310,340,335,338,336,
    440,400,430,410,540,500,530,510,640,600,630,610],
  same_down_mirror:[],
  p_completion_overlap:[140,100,120,80,200,160,220,130,240,230,238,232,
    340,300,330,310,440,400,430,410],
  p_completion_disjoint:[140,100,120,80,200,160,220,180,240,230,238,232,
    340,300,330,310,440,400,430,410],
  adverse_10x3_30_20x3_15x3:[...rep(10),30,...rep(20),...rep(15)],
  short_platform:[...rep(10),20,10,20],
  short_platform_after_trend:[...rep(10),...rep(20),10,20],
  long_10_20:Array.from({length:80},(_,i)=>i%2?20:10),
  flat_10:Array(80).fill(10)
};
cases.same_down_mirror=cases.same_up_counterexample.map(x=>700-x);
const summaries=[];
let totalPrefixes=0,totalSourceEvents=0,totalSourceStates=0;
for(const [name,word] of Object.entries(cases)) {
  const src=source(word),q=src.states.map(s=>s.q),r=construct(q);
  check(q,r);
  for(let n=1;n<=q.length;n++) {
    const partial=construct(q.slice(0,n));check(q.slice(0,n),partial);totalPrefixes++;
    for(const old of partial.outputs) {
      assert.deepEqual(old,r.outputs[Number(old.id.slice(1))], 'append-stable completed candidate');
      assert.ok(old.confirmedAt<n);
    }
  }
  for(let a=0;a<q.length;a++) for(let b=0;b<q.length;b++)
    assert.equal(priceOfQuantity(q[a],q[b]),q[a]<q[b]);
  const transitions=r.outputs.slice(1).map((m,i)=>`${r.outputs[i].type}->${m.type}`);
  const bad=transitions.filter(t=>t==='U->U'||t==='D->D');
  const triples=[];
  for(let i=0;i+2<r.outputs.length;i++) {
    const ms=r.outputs.slice(i,i+3),z=intersection(ms.map(m=>m.range));
    const dirs=ms.map(m=>m.conceptDirection);
    triples.push({members:ms.map(m=>m.id),ranges:ms.map(m=>m.range),intersection:z,
      strict:z[0]<z[1],directions:dirs,
      directionAlternation:dirs.every(d=>d!==null)&&dirs[0]!==dirs[1]&&dirs[1]!==dirs[2],
      fullSemanticAdmissibility:false,
      reason:bad.length?'candidate has forbidden completed-type succession':'candidate completion/P direction not certified'});
  }
  const result={name,source:src,result:r,transitions,violations:bad,parentRequests:triples};
  fs.writeFileSync(path.join(out,`${name}.json`),JSON.stringify(result,null,2)+'\n');
  summaries.push({name,events:src.events.length,closedLegs:r.legs.completed.length,
    sealedCores:r.coreScan.completed.length,activeCore:r.coreScan.active?.id??null,
    outputs:r.outputs.map(m=>({id:m.id,type:m.type,start:m.start,end:m.end,
      confirmedAt:m.confirmedAt,coreIDs:m.ownedCoreIDs,range:m.range})),
    tail:r.tail.status,transitions,violations:bad,threeCandidateRequests:triples.length});
  totalSourceEvents+=src.events.length;totalSourceStates+=src.states.length;
}
assert.ok(summaries.find(s=>s.name==='same_up_counterexample').violations.includes('U->U'));
assert.ok(summaries.find(s=>s.name==='same_up_counterexample').outputs.length>=3);
assert.ok(summaries.find(s=>s.name==='same_down_mirror').violations.includes('D->D'));
assert.equal(summaries.find(s=>s.name==='long_10_20').sealedCores,0);
assert.equal(summaries.find(s=>s.name==='long_10_20').activeCore,'K0');
assert.equal(summaries.find(s=>s.name==='flat_10').activeCore,null);
const summary={version:'RC49-v1',scope:'author finite development checks, not independent review',
  totalSourceEvents,totalSourceStates,totalPrefixes,cases:summaries};
fs.writeFileSync(path.join(out,'summary.json'),JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary,null,2));
