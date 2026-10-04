import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';

// RT-v1 author check. Output only to this agent's evidence path.
const out = '/Users/silencehan/Documents/Codex/research-evidence/issue1467/base-completion-v1/turn-boundary-results.json';
const sign = x => Math.sign(x);
const minmax = xs => [Math.min(...xs), Math.max(...xs)];
function platforms(xs) {
  const ps = [];
  for (let e = 1; e < xs.length; e++) {
    if (!ps.length || ps.at(-1).value !== xs[e])
      ps.push({id: ps.length, start: e, end: e, value: xs[e]});
    else ps.at(-1).end = e;
  }
  for (const p of ps) {p.birth = p.start + 2; p.mature = p.end >= p.birth;}
  return ps;
}
function firstFailure(xs) {
  if (xs.length > 1 && xs[0] !== xs[1]) return 1;
  for (const p of platforms(xs))
    if (p.end < xs.length - 1 && p.end - p.start + 1 < 3) return p.end + 1;
  return null;
}
function validRT(xs) {
  assert.equal(firstFailure(xs), null);
  const ps = platforms(xs), ks = ps.filter(k => k.mature), C = [];
  let a = 0, T = 0, blocked = null;
  while (a + 1 < ks.length) {
    const d = sign(ks[a+1].value - ks[a].value);
    let b = a + 1;
    while (b + 1 < ks.length && sign(ks[b+1].value - ks[b].value) === d) b++;
    if (b + 1 === ks.length) break;
    if (b - a < 2) {
      blocked = {reason: 'first_turn_leaves_one_owned_core', a, b, observedAt: ks[b+1].birth};
      break;
    }
    const end = ks[b-1].end;
    C.push({ids: ks.slice(a,b).map(k=>k.id), direction: d, events: [T+1,end],
      boundaryStates: [T,end], endpoints: [xs[T],xs[end]], range: minmax(xs.slice(T,end+1)),
      turnSupport: [b-1,b,b+1].map(i=>({core: i,value: ks[i].value,seed: [ks[i].start,ks[i].start+1,ks[i].birth]})),
      confirm: ks[b+1].birth});
    T = end; a = b;
  }
  return {C, R: T < xs.length-1 ? [T+1,xs.length-1] : null, boundaryState: T,
    tailCores: ks.slice(a).map(k=>k.id), pendingPlatforms: ps.filter(p=>!p.mature).map(p=>p.id), blocked};
}
function rt(xs) {
  const f = firstFailure(xs);
  if (f === null) return {domain: 'valid', ...validRT(xs)};
  const frozen = validRT(xs.slice(0,f));
  return {domain: 'outside_D', firstFailure: f, ...frozen,
    R: [frozen.boundaryState+1,xs.length-1], unanalyzed: [f,xs.length-1]};
}
function w(q) {
  let n = 102n*BigInt(q)+1000n, d=BigInt(q)+10n;
  let a=n,b=d; while(b) [a,b]=[b,a%b];
  n/=a; d/=a; return d===1n ? String(n) : `${n}/${d}`;
}
// Each observation is backed by a distinct, legal add/cancel event.
function eventsFor(values) {
  const orders = new Map([['S0',{price:100,qty:values[0],side:'bid'}],['A0',{price:102,qty:10,side:'ask'}]]);
  const events = [], xs=[values[0]];
  let fresh=0, q=values[0];
  function emit(op,id,price,qty) {
    const e={seq:events.length+1,op,id,price,qty,side:'bid'};
    if(op==='add') {assert.ok(!orders.has(id)); assert.ok(qty>0); orders.set(id,{price,qty,side:'bid'});}
    else {const old=orders.get(id);assert.ok(old && old.price===price && old.qty>=qty && qty>0);old.qty-=qty;if(!old.qty) orders.delete(id);}
    events.push(e);
    const actual=[...orders.values()].filter(o=>o.side==='bid'&&o.price===100).reduce((s,o)=>s+o.qty,0);
    xs.push(actual); q=actual;
    assert.equal(Math.max(...[...orders.values()].filter(o=>o.side==='bid').map(o=>o.price)),100);
    assert.deepEqual(orders.get('A0'),{price:102,qty:10,side:'ask'});
  }
  function aux(){emit('add',`U${++fresh}`,99,1);}
  for(let i=0;i<values.length;i++) {
    const target=values[i];
    if(i===0) aux();
    else if(target>q) emit('add',`S${++fresh}`,100,target-q);
    else {
      const item=[...orders.entries()].find(([,o])=>o.side==='bid'&&o.price===100&&o.qty>=q-target);
      assert.ok(item, `single event cancellation unavailable: ${values}, step ${i}`);
      emit('cancel',item[0],100,q-target);
    }
    aux();aux();assert.equal(q,target);
  }
  assert.equal(firstFailure(xs),null);
  return {B0:{bid100:values[0],ask102:10},events,xs};
}
let coordinateChecks=0, prefixChecks=0;
function check(xs) {
  let old=[];
  for(let n=0;n<xs.length;n++) {
    const prefix=xs.slice(0,n+1), o=rt(prefix);
    assert.deepEqual(o.C.slice(0,old.length),old);
    const ids=[];let end=0,priorDirection=null;
    for(const c of o.C) {
      assert.equal(c.events[0],end+1);assert.ok(c.ids.length>=2);
      assert.ok(c.confirm<=n);assert.ok(c.confirm>c.events[1]);
      assert.deepEqual(c.range,minmax(prefix.slice(end,c.events[1]+1)));
      if(priorDirection!==null) assert.equal(c.direction,-priorDirection);
      assert.equal(c.turnSupport[1].core,c.ids.at(-1)+1);
      assert.equal(c.turnSupport[2].core,c.ids.at(-1)+2);
      ids.push(...c.ids);end=c.events[1];priorDirection=c.direction;
    }
    assert.equal(o.boundaryState,end);
    assert.deepEqual([...ids,...o.tailCores],platforms(prefix).filter(p=>p.mature).map(p=>p.id));
    assert.deepEqual(o.R,end<n ? [end+1,n] : null);
    old=o.C;prefixChecks++;
  }
  coordinateChecks++;
}
const examples=[];
for(const [name,vs] of [
  ['15_alternating',[10,20,10,20,10]],
  ['18_left_absorption',[10,20,10,20,30,20]],
  ['21_old',[10,20,12,6,10,16,11]],
  ['12_early_cut',[10,20,30,25]],
  ['30_parent_witness',[10,20,30,25,15,5,10,20,30,25]],
]) {
  for(const mirror of [false,true]) {
    const values=mirror ? vs.map(x=>40-x) : vs;
    const h=eventsFor(values);check(h.xs);
    const result=rt(h.xs);
    let parent=null;
    if(result.C.length>=3) {
      const cs=result.C.slice(0,3),lo=Math.max(...cs.map(c=>c.range[0])),hi=Math.min(...cs.map(c=>c.range[1]));
      assert.ok(lo<hi);
      parent={childIds:[0,1,2],directions:cs.map(c=>c.direction),rangeIntersection:[lo,hi],
        WrangeIntersection:[w(lo),w(hi)],confirmedAt:Math.max(...cs.map(c=>c.confirm)),
        status:'RT_only_alternation_and_strict_geometry_not_original_F2_qualification'};
    }
    examples.push({name: name+(mirror?'_mirror':''),values,history:h,result,parent});
  }
}
// All adjacent-distinct coordinate words over three values, up to nine platforms.
let enumerated=0;
for(let size=1;size<=9;size++) {
  function visit(v) {
    if(v.length===size) {check([v[0],...v.flatMap(x=>[x,x,x])]);enumerated++;return;}
    for(const x of [10,20,30]) if(x!==v.at(-1))visit([...v,x]);
  }
  visit([]);
}
assert.equal(enumerated,1533);
// Long platform extensions do not change already published fields.
for(const vals of [[10],[10,20],[10,20,30,25],[10,20,30,25,15,5,10,20,30,25]])
  for(let offset=0;offset<4;offset++)check([vals[0],...vals.flatMap((x,i)=>Array(3+(i+offset)%4).fill(x))]);
// Permanent thin-turn obstruction persists without swallowing later rich runs.
const blockedLong=[10,20,10,15,25,35,30,20,5];
const bo=rt([10,...blockedLong.flatMap(x=>[x,x,x])]);
assert.equal(bo.C.length,0);assert.equal(bo.blocked.observedAt,9);assert.deepEqual(bo.R,[1,27]);
// Domain failure is explicit and does not discard the short platform.
const invalid=rt([10,10,10,10,20,20,30,30,30]);
assert.equal(invalid.domain,'outside_D');assert.equal(invalid.firstFailure,6);assert.deepEqual(invalid.R,[1,8]);
const result={candidate:'RT-v1',examples,enumeratedCoordinateWords:enumerated,
  coordinateChecks,prefixChecks,permanentBlockExample:bo,domainFailureExample:invalid,allAssertionsPassed:true};
writeFileSync(out,JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({candidate:'RT-v1',coordinateChecks,prefixChecks,enumeratedCoordinateWords:enumerated,
  allAssertionsPassed:true,examples:examples.map(e=>({name:e.name,C:e.result.C.length,blocked:e.result.blocked,parent:e.parent}))},null,2));
