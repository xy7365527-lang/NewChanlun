#!/usr/bin/env node
// #1467 研究探针：仅检验声明的 F1 平台候选，不接生产 F2。
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const gcd = (a, b) => {
  a = a < 0n ? -a : a;
  while (b !== 0n) [a, b] = [b, a % b];
  return a;
};
class Rat {
  constructor(n, d = 1n) {
    n = BigInt(n); d = BigInt(d);
    assert.notEqual(d, 0n);
    if (d < 0n) { n = -n; d = -d; }
    const g = gcd(n, d);
    this.n = n / g; this.d = d / g;
    Object.freeze(this);
  }
  static of(x) {
    if (x instanceof Rat) return x;
    const [n, d = '1'] = String(x).split('/');
    return new Rat(n, d);
  }
  add(y) { y = Rat.of(y); return new Rat(this.n*y.d + y.n*this.d, this.d*y.d); }
  sub(y) { y = Rat.of(y); return new Rat(this.n*y.d - y.n*this.d, this.d*y.d); }
  mul(y) { y = Rat.of(y); return new Rat(this.n*y.n, this.d*y.d); }
  div(y) { y = Rat.of(y); return new Rat(this.n*y.d, this.d*y.n); }
  cmp(y) {
    y = Rat.of(y);
    const delta = this.n*y.d - y.n*this.d;
    return delta < 0n ? -1 : delta > 0n ? 1 : 0;
  }
  toString() { return this.d === 1n ? String(this.n) : `${this.n}/${this.d}`; }
}
const R = Rat.of;
const W = q => R(100).add(R(2).mul(q).div(R(q).add(10)));
const seqs = (a, b) => a > b ? [] : Array.from({length:b-a+1}, (_,i)=>a+i);
const minR = xs => xs.reduce((a,b)=>a.cmp(b)<0?a:b);
const maxR = xs => xs.reduce((a,b)=>a.cmp(b)>0?a:b);
const range = xs => [String(minR(xs)), String(maxR(xs))];
const exactJSON = x => JSON.stringify(x, (_,v)=>typeof v==='bigint'?String(v):v, 2)+'\n';
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

// 平台关系的直接表示。区间只引用实际事件后状态，不把初态算作观测。
function platforms(xs, n) {
  const ps = [];
  for (let i=1; i<=n; i++) {
    const last = ps.at(-1);
    if (last && R(last.value).cmp(xs[i])===0) last.endObserved=i;
    else ps.push({id:ps.length, start:i, endObserved:i, value:String(xs[i])});
  }
  return ps.map((p,i)=>({
    ...p, count:p.endObserved-p.start+1, closed:i<ps.length-1,
    mature:p.endObserved-p.start+1>=3,
    birthAt:p.endObserved-p.start+1>=3?p.start+2:null,
  }));
}

function firstDomainFailure(xs) {
  const n=xs.length-1;
  if (n && xs[0].cmp(xs[1])!==0)
    return {at:1, reason:'initial_observation_differs', initial:String(xs[0]), first:String(xs[1])};
  const ps=platforms(xs,n);
  for (const p of ps) {
    if (p.closed && p.count<3)
      return {at:p.endObserved+1, reason:'closed_short_platform', platform:p};
  }
  return null;
}

function validRelation(xs) {
  const n=xs.length-1;
  assert.equal(firstDomainFailure(xs),null);
  const ps=platforms(xs,n);
  const cores=ps.filter(p=>p.mature);
  const runs=[];
  for (let i=1;i<cores.length;i++) {
    const direction=R(cores[i].value).cmp(cores[i-1].value);
    assert.notEqual(direction,0);
    if (runs.at(-1)?.direction===direction) runs.at(-1).lastEdge=i;
    else runs.push({firstEdge:i,lastEdge:i,direction});
  }
  const owned = run => seqs(run.firstEdge===1?0:run.firstEdge,run.lastEdge);
  const completed=runs.slice(0,-1).map((run,index)=>{
    const b=run.lastEdge;
    const after=index===0?0:cores[run.firstEdge-1].endObserved;
    const end=cores[b].endObserved;
    const witness=cores[b+1];
    const centreIds=owned(run);
    return {
      id:index, firstEdge:run.firstEdge, lastEdge:b, direction:run.direction,
      kind:centreIds.length===1?'consolidation':run.direction===1?'uptrend':'downtrend',
      centreIds, raw:{after,end,events:seqs(after+1,end)},
      start:String(xs[after]), end:String(xs[end]), range:range(xs.slice(after,end+1)),
      completion:{
        witnessCentreId:witness.id, witnessValue:witness.value,
        seedEvents:seqs(witness.start,witness.start+2), knownAt:witness.start+2,
        followingDirection:R(witness.value).cmp(cores[b].value),
      },
    };
  });
  const last=runs.at(-1);
  const matureEdgeTail=last?{
    ...last, anchorCentreId:last.firstEdge-1, centreIds:owned(last),
    lastMaturePlatformEndObserved:cores[last.lastEdge].endObserved,
    lastMaturePlatformClosed:cores[last.lastEdge].closed,
  }:null;
  const after=completed.at(-1)?.raw.end??0;
  return {
    status:'valid_D', sourceEventCount:n, analysedThrough:n, failure:null,
    completed, unconsumed:{after,startStateAt:after,events:seqs(after+1,n)},
    matureEdgeTail, unorientedSeedCentre:cores.length===1?cores[0].id:null,
    pendingPlatform:ps.at(-1)&&!ps.at(-1).mature?ps.at(-1):null,
    coreCatalog:cores, unanalysedEvents:[],
  };
}

// 域外诊断并非 F1 在域外的延拓：停在最早失败之前，不删除或拼接。
function observePrefix(xs) {
  const failure=firstDomainFailure(xs);
  if (!failure) return validRelation(xs);
  const through=failure.at-1;
  const result=validRelation(xs.slice(0,through+1));
  return {
    ...result, status:'outside_D', sourceEventCount:xs.length-1,
    analysedThrough:through, failure,
    unconsumed:{...result.unconsumed,events:seqs(result.unconsumed.after+1,xs.length-1)},
    unanalysedEvents:seqs(failure.at,xs.length-1),
  };
}

function verifyResult(xs,result) {
  const n=xs.length-1;
  assert.equal(result.sourceEventCount,n);
  const ownership=result.completed.flatMap(m=>m.raw.events).concat(result.unconsumed.events);
  assert.deepEqual(ownership,seqs(1,n),'原始事件须恰好归属一次');
  assert.equal(new Set(ownership).size,n);
  let previousEnd=0;
  for (const m of result.completed) {
    assert.equal(m.raw.after,previousEnd);
    assert.equal(m.raw.end,m.raw.events.at(-1));
    assert.ok(m.completion.knownAt<=result.analysedThrough);
    assert.ok(m.raw.end<m.completion.seedEvents[0]);
    assert.equal(m.completion.followingDirection,-m.direction);
    assert.deepEqual(m.range,range(xs.slice(m.raw.after,m.raw.end+1)));
    for (const i of m.completion.seedEvents) assert.equal(String(xs[i]),m.completion.witnessValue);
    for (const id of m.centreIds) {
      const p=result.coreCatalog[id];
      assert.ok(p.mature&&p.closed&&p.count>=3);
      assert.ok(m.raw.after<p.start&&p.endObserved<=m.raw.end);
    }
    previousEnd=m.raw.end;
  }
  assert.equal(result.unconsumed.after,previousEnd);
  const ownedCentres=result.completed.flatMap(m=>m.centreIds);
  const activeCentres=result.matureEdgeTail?.centreIds??
    (result.unorientedSeedCentre===null?[]:[result.unorientedSeedCentre]);
  assert.deepEqual(ownedCentres.concat(activeCentres),result.coreCatalog.map(p=>p.id));
  if (result.pendingPlatform) {
    assert.ok(result.pendingPlatform.count>=1&&result.pendingPlatform.count<=2);
    assert.ok(result.pendingPlatform.start>result.unconsumed.after);
    assert.equal(result.pendingPlatform.endObserved,result.analysedThrough);
  }
  if (result.status==='outside_D') {
    assert.equal(result.analysedThrough,result.failure.at-1);
    assert.deepEqual(result.unanalysedEvents,seqs(result.failure.at,n));
  }
}

function checkEveryPrefix(xs) {
  const outputs=[];
  for (let n=0;n<xs.length;n++) {
    const prefix=xs.slice(0,n+1), current=observePrefix(prefix);
    verifyResult(prefix,current);
    if (outputs.length) {
      const previous=outputs.at(-1);
      assert.deepEqual(current.completed.slice(0,previous.completed.length),previous.completed,
        '已完成对象不能被有效后续重写；域外诊断也不得增删既有对象');
      if (previous.status==='outside_D') {
        assert.equal(current.failure.at,previous.failure.at);
        assert.deepEqual(current.completed,previous.completed);
      }
    }
    outputs.push(current);
  }
  return outputs;
}

const add=(id,p,q)=>({op:'add',id,side:'bid',p:BigInt(p),q:BigInt(q)});
const cancel=(id,p,q)=>({op:'cancel',id,side:'bid',p:BigInt(p),q:BigInt(q)});
const numbered=es=>es.map((e,i)=>({seq:i+1,...e}));
function originalEvents() {
  return numbered([
    add('U1',99,1),cancel('U1',99,1),add('U2',99,1),
    add('S1',100,10),cancel('U2',99,1),add('U3',99,1),
    cancel('S1',100,8),cancel('U3',99,1),add('U4',99,1),
    cancel('S0',100,6),cancel('U4',99,1),add('U5',99,1),
    add('S2',100,4),cancel('U5',99,1),add('U6',99,1),
    add('S3',100,6),cancel('U6',99,1),add('U7',99,1),
    cancel('S3',100,5),cancel('U7',99,1),add('U8',99,1),
  ]);
}
function shortEvents() {
  return numbered([
    add('U1',99,1),cancel('U1',99,1),add('U2',99,1),
    add('S1',100,10),cancel('U2',99,1),add('U3',99,1),
    cancel('S1',100,10),cancel('U3',99,1),add('U4',99,1),
    add('S2',100,10),cancel('U4',99,1),add('U5',99,1),
    cancel('S2',100,10),cancel('U5',99,1),add('U6',99,1),
  ]);
}
function replayBook(events) {
  const orders=new Map([
    ['S0',{side:'bid',p:100n,q:10n}],['A0',{side:'ask',p:102n,q:10n}],
  ]);
  const qTrace=[], wTrace=[], states=[];
  const snapshot=seq=>{
    const all=[...orders.values()];
    const bids=all.filter(o=>o.side==='bid'), asks=all.filter(o=>o.side==='ask');
    const bid=bids.reduce((p,o)=>o.p>p?o.p:p,bids[0].p);
    const ask=asks.reduce((p,o)=>o.p<p?o.p:p,asks[0].p);
    const q=bids.filter(o=>o.p===bid).reduce((s,o)=>s+o.q,0n);
    const k=asks.filter(o=>o.p===ask).reduce((s,o)=>s+o.q,0n);
    assert.equal(bid,100n); assert.equal(ask,102n); assert.equal(k,10n); assert.ok(q>0n);
    const w=new Rat(ask*q+bid*k,q+k);
    assert.equal(String(w),String(W(new Rat(q))));
    qTrace.push(new Rat(q));wTrace.push(w);
    states.push({seq,bid,ask,q,k,w:String(w),orders:[...orders].map(([id,o])=>({id,...o}))});
  };
  snapshot(0);
  events.forEach((e,i)=>{
    assert.equal(e.seq,i+1);assert.ok(e.q>0n);
    if (e.op==='add') {
      assert.ok(!orders.has(e.id));orders.set(e.id,{side:e.side,p:e.p,q:e.q});
    } else {
      const o=orders.get(e.id);
      assert.ok(o);assert.equal(o.side,e.side);assert.equal(o.p,e.p);assert.ok(o.q>=e.q);
      o.q-=e.q;if(o.q===0n)orders.delete(e.id);
    }
    snapshot(i+1);
  });
  return {events,states,qTrace,wTrace};
}

function coreOfThree(moves) {
  assert.equal(moves.length,3);
  const lo=maxR(moves.map(m=>R(m.range[0]))),hi=minR(moves.map(m=>R(m.range[1])));
  assert.ok(lo.cmp(hi)<0);return [String(lo),String(hi)];
}
function checkMapping(qOutputs,wOutputs) {
  assert.equal(qOutputs.length,wOutputs.length);
  for(let i=0;i<qOutputs.length;i++) {
    const q=qOutputs[i],w=wOutputs[i];
    assert.equal(q.status,w.status);assert.deepEqual(q.unconsumed,w.unconsumed);
    assert.deepEqual(q.matureEdgeTail,w.matureEdgeTail);
    assert.equal(q.unorientedSeedCentre,w.unorientedSeedCentre);
    assert.deepEqual(q.completed.map(m=>m.raw),w.completed.map(m=>m.raw));
    for(let j=0;j<q.completed.length;j++) {
      assert.deepEqual(q.completed[j].centreIds,w.completed[j].centreIds);
      assert.equal(q.completed[j].direction,w.completed[j].direction);
      assert.equal(q.completed[j].completion.knownAt,w.completed[j].completion.knownAt);
      assert.deepEqual(q.completed[j].range.map(v=>String(W(R(v)))),w.completed[j].range);
    }
  }
}

function* words(alphabet,maxLength,prefix=[]) {
  yield prefix;
  if(prefix.length<maxLength)for(const v of alphabet)yield* words(alphabet,maxLength,[...prefix,v]);
}

function main() {
  assert.equal(process.argv[2],'--out','用 --out 指定仓外结果目录');
  assert.equal(process.argv.length,4);
  const out=resolve(process.argv[3]);
  const script=fileURLToPath(import.meta.url);
  const document=resolve(dirname(script),'PlateauBaseCandidate-v2.md');
  const v1='/Users/silencehan/Documents/Codex/research-evidence/issue1467/interface-review-v1/fresh-construction.md';
  const v1Hash=sha256(readFileSync(v1));
  assert.equal(v1Hash,'f32688769d404423778f6d2b62b4ee4433a48cdf74acca50f8ba045d85ebd123');

  const long=replayBook(originalEvents()),short=replayBook(shortEvents());
  const lq=checkEveryPrefix(long.qTrace),lw=checkEveryPrefix(long.wTrace);
  const sq=checkEveryPrefix(short.qTrace),sw=checkEveryPrefix(short.wTrace);
  checkMapping(lq,lw);checkMapping(sq,sw);
  for(let n=0;n<=21;n++)assert.equal(lq[n].completed.length,[9,15,21].filter(t=>t<=n).length);
  assert.deepEqual(lq[0].unconsumed.events,[]);assert.equal(lq[0].pendingPlatform,null);
  for(const n of [1,2]) {
    assert.equal(lq[n].matureEdgeTail,null);assert.equal(lq[n].unorientedSeedCentre,null);
    assert.equal(lq[n].pendingPlatform.count,n);
  }
  assert.equal(lq[3].unorientedSeedCentre,0);assert.equal(lq[3].matureEdgeTail,null);
  for(const n of [7,8]) {
    assert.equal(lq[n].completed.length,0);assert.deepEqual(lq[n].unconsumed.events,seqs(1,n));
    assert.deepEqual(lq[n].matureEdgeTail.centreIds,[0,1]);
    assert.deepEqual([lq[n].pendingPlatform.start,lq[n].pendingPlatform.endObserved],[7,n]);
  }
  assert.deepEqual(coreOfThree(lq[21].completed),['10','16']);
  assert.deepEqual(coreOfThree(lw[21].completed),['101','1316/13']);
  assert.deepEqual(sq[15].completed.map(m=>[m.kind,m.direction,m.completion.knownAt]),[
    ['uptrend',1,9],['consolidation',-1,12],['consolidation',1,15],
  ]);

  const invalidRaw=[10,10,10,20,10,10,10,20,20,20,10,10,10];
  const invalid=checkEveryPrefix([R(10),...invalidRaw.map(R)]);
  assert.equal(invalid.at(-1).failure.at,5);
  assert.equal(invalid.at(-1).analysedThrough,4);
  assert.equal(invalid.at(-1).completed.length,0);
  assert.deepEqual(invalid.at(-1).unconsumed.events,seqs(1,13));
  const afterCompletion=checkEveryPrefix([...long.qTrace,...[12,13,13,13,10,10,10].map(R)]);
  assert.equal(afterCompletion.at(-1).failure.at,23);
  assert.deepEqual(afterCompletion.at(-1).completed,lq[21].completed);

  let rawWords=0,validRawWords=0,platformWords=0,enumeratedPrefixChecks=0;
  for(const word of words([1,2],12)) {
    const xs=[R(word[0]??1),...word.map(R)];
    const checks=checkEveryPrefix(xs);rawWords++;enumeratedPrefixChecks+=checks.length;
    if(checks.at(-1).status==='valid_D')validRawWords++;
  }
  for(const word of words([1,2,3],7)) {
    const raw=word.flatMap(x=>[x,x,x]);
    const xs=[R(word[0]??1),...raw.map(R)];
    const checks=checkEveryPrefix(xs);platformWords++;enumeratedPrefixChecks+=checks.length;
    assert.equal(checks.at(-1).status,'valid_D');
  }

  const speed={
    qA:R(3).sub(1),qB:R(13).sub(10),wA:W(R(3)).sub(W(R(1))),wB:W(R(13)).sub(W(R(10))),
  };
  assert.ok(speed.qA.cmp(speed.qB)<0&&speed.wA.cmp(speed.wB)>0);
  const force=points=>{
    const p=points.map(R),w=p.map(W);
    return {q:p[3].sub(p[2]).sub(p[1].sub(p[0])),w:w[3].sub(w[2]).sub(w[1].sub(w[0]))};
  };
  const A=force(['1','2','3/2','4']),B=force(['10','11','21/2','14']);
  assert.equal(String(A.q),'3/2');assert.equal(String(B.q),'5/2');
  assert.ok(A.q.cmp(B.q)<0&&A.w.cmp(B.w)>0);

  const result={
    verdict:'PASS_FOR_DECLARED_FINITE_PROBE_ONLY',
    original21:{events:21,prefixesPerAxis:22,confirmationTimes:[9,15,21],qCore:coreOfThree(lq[21].completed),wCore:coreOfThree(lw[21].completed)},
    short15:{events:15,prefixesPerAxis:16,completed:sq[15].completed.map(m=>({kind:m.kind,direction:m.direction,knownAt:m.completion.knownAt}))},
    outsideDomain:{firstFailure:5,analysedThrough:4,all13EventsRetained:true,postCompletionFailure:23,completedResultsFrozen:true},
    boundedEnumeration:{rawAlphabet:[1,2],maxRawEvents:12,rawWords,validRawWords,platformAlphabet:[1,2,3],maxPlatformTokens:7,observationsPerToken:3,platformWords,enumeratedPrefixChecks},
    speedComparison:{qA:String(speed.qA),qB:String(speed.qB),wA:String(speed.wA),wB:String(speed.wB),scope:'等时长速度比较，不是单凭这组数就宣称 L 反例'},
    coordinateLExample:{edgeDuration:1,A:['1','2','3/2','4'],B:['10','11','21/2','14'],qLA:String(A.q),qLB:String(B.q),wLA:String(A.w),wLB:String(B.w),scope:'仅三边数学坐标例，不赋予原文笔或背驰资格'},
    sourceFiles:{script:{path:script,sha256:sha256(readFileSync(script))},document:{path:document,sha256:sha256(readFileSync(document))},v1:{path:v1,sha256:v1Hash,preserved:true}},
  };
  mkdirSync(out,{recursive:true});
  const files={
    'probe-result.json':result,
    'original-events-and-states.json':{events:long.events,states:long.states},
    'prefixes-long-Q.json':lq,'prefixes-long-W.json':lw,
    'prefixes-short-Q.json':sq,'prefixes-short-W.json':sw,
    'prefixes-outside-domain.json':invalid,
    'prefixes-domain-failure-after-completion.json':afterCompletion,
  };
  const hashes=[];
  for(const [name,data] of Object.entries(files)) {
    const bytes=exactJSON(data);writeFileSync(resolve(out,name),bytes);hashes.push(`${sha256(bytes)}  ${name}`);
  }
  writeFileSync(resolve(out,'output-hashes.sha256'),hashes.join('\n')+'\n');
  assert.equal(sha256(readFileSync(v1)),v1Hash,'v1 原字节必须保留');
  process.stdout.write(exactJSON(result));
}
main();
