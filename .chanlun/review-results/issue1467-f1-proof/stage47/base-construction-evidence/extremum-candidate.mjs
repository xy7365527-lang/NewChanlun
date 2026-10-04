// #1467 Stage47: EV-P-v1 research probe. No production or doctrine imports.
// Run: node extremum-candidate.mjs [new-output-directory]
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const sign = x => x > 0 ? 1 : x < 0 ? -1 : 0;
const span = (a, b) => Array.from({length: Math.max(0, b - a + 1)}, (_, j) => a + j);
const eq = (a, b) => JSON.stringify(a) === JSON.stringify(b);
function runs(xs) {
  const r = [];
  xs.forEach((value, i) => {
    if (r.length && r.at(-1).value === value) r.at(-1).end = i + 1;
    else r.push({id: r.length, start: i + 1, end: i + 1, value});
  });
  return r;
}

// Independent finite-prefix relation in the report: ALL raw sign changes,
// with exact event membership, NOT an externally supplied completion flag.
export function relation(xs) {
  assert(xs.every(x => Number.isSafeInteger(x) && x > 0));
  const raw = runs(xs), n = xs.length;
  if (raw.length > 1) assert(raw[0].end >= 3, 'outside initial-anchor domain');
  const cores = raw.filter(p => p.end - p.start + 1 >= 3).map(p => ({
    id: `K@${p.start}`, plateau: p.id, start: p.start, end: p.end,
    value: p.value, birth: p.start + 2, seed: span(p.start, p.start + 2),
    memberEvents: span(p.start, p.end), closed: p.id < raw.length - 1,
    extent: [p.value, p.value],
  }));
  const pivots = [];
  for (let i = 1; i + 1 < raw.length; i++) {
    if (sign(raw[i].value - raw[i-1].value) !== sign(raw[i+1].value - raw[i].value)) pivots.push(i);
  }
  let prior = 0;
  const completed = pivots.map((i, j) => {
    const p = raw[i], end = p.end, from = prior;
    const own = cores.filter(k => k.start > from && k.end <= end);
    const path = [from === 0 ? xs[0] : xs[from - 1], ...xs.slice(from, end)];
    const dir = sign(p.value - raw[i-1].value);
    const type = own.length === 0 ? 'NoCore' : own.length === 1 ? 'P' : dir === 1 ? 'U' : 'D';
    const m = {
      id: `M${j+1}`, startBoundary: from, end, confirm: end + 1,
      memberEvents: span(from + 1, end), ownCoreIDs: own.map(k => k.id),
      type, semanticDirection: type === 'U' ? 1 : type === 'D' ? -1 : null,
      pathDirection: dir, startValue: path[0], endValue: p.value,
      range: [Math.min(...path), Math.max(...path)],
      support: {
        leftRun: {start: raw[i-1].start, end: raw[i-1].end, value: raw[i-1].value},
        pivotRun: {start: p.start, end: p.end, value: p.value},
        oppositeFirstEvent: raw[i+1].start, oppositeValue: raw[i+1].value,
      },
    };
    prior = end;
    return m;
  });
  return {n, raw, cores, completed, tail: {startBoundary: prior, memberEvents: span(prior + 1, n)}};
}

function check(xs) {
  const o = relation(xs);
  assert.deepEqual([...o.completed.flatMap(m => m.memberEvents), ...o.tail.memberEvents], span(1, xs.length));
  const own = o.completed.flatMap(m => m.ownCoreIDs);
  assert.equal(new Set(own).size, own.length);
  for (const m of o.completed) {
    assert(m.confirm <= xs.length && m.confirm > m.end);
    assert.deepEqual(m, relation(xs.slice(0, m.confirm)).completed.find(x => x.id === m.id));
    assert.equal(m.startValue, m.pathDirection === 1 ? m.range[0] : m.range[1]);
    assert.equal(m.endValue, m.pathDirection === 1 ? m.range[1] : m.range[0]);
    const k = o.cores.filter(k => m.ownCoreIDs.includes(k.id));
    assert(k.every(k => k.start > m.startBoundary && k.end <= m.end && k.closed));
    for (let i = 1; i < k.length; i++) assert.equal(sign(k[i].value - k[i-1].value), m.pathDirection);
    assert.equal(m.type === 'NoCore', k.length === 0);
    assert.equal(m.type === 'P', k.length === 1);
  }
  for (let i = 1; i < o.completed.length; i++) {
    assert.equal(o.completed[i-1].end, o.completed[i].startBoundary);
    assert.equal(o.completed[i-1].endValue, o.completed[i].startValue);
    assert.equal(o.completed[i-1].pathDirection, -o.completed[i].pathDirection);
  }
  return o;
}

function gcd(a, b) { while (b) [a,b] = [b,a%b]; return a; }
function w(q) {
  let a = 102n * BigInt(q) + 1000n, b = BigInt(q) + 10n;
  const g = gcd(a,b); return `${a/g}/${b/g}`;
}

// Exact legal add/partial-cancel histories for the NAMED examples only.
// The exhaustive coordinate words below are not claimed to be native histories.
function replayExample(xs) {
  const orders = new Map([
    ['S0', {side:'bid', price:100, quantity:xs[0]}],
    ['A0', {side:'ask', price:102, quantity:10}],
  ]);
  const initial = Array.from(orders, ([id,x]) => ({id,...x}));
  const events = [], states = [{index:0, q:xs[0], w:w(xs[0]), orders:structuredClone(initial)}];
  let q = xs[0], dummy = null;
  for (let j = 0; j < xs.length; j++) {
    const target = xs[j], i = j + 1;
    let e;
    if (target > q) e = {op:'add', id:`Q${i}`, side:'bid', price:100, quantity:target-q};
    else if (target < q) {
      const hit = [...orders].find(([,o]) => o.side === 'bid' && o.price === 100 && o.quantity >= q-target);
      assert(hit, 'named history needs a multi-message decrease; choose explicit native events');
      e = {op:'cancel', id:hit[0], side:'bid', price:100, quantity:q-target};
    } else if (dummy === null) {
      dummy = `D${i}`; e = {op:'add', id:dummy, side:'bid', price:99, quantity:1};
    } else {
      e = {op:'cancel', id:dummy, side:'bid', price:99, quantity:1}; dummy = null;
    }
    e = {seq:i, occurredNs:String(i*1000000), receivedNs:String(i*1000000+500), ...e};
    if (e.op === 'add') {
      assert(!orders.has(e.id) && e.quantity > 0);
      orders.set(e.id, {side:e.side,price:e.price,quantity:e.quantity});
    } else {
      const old = orders.get(e.id);
      assert(old && old.side === e.side && old.price === e.price && old.quantity >= e.quantity && e.quantity > 0);
      old.quantity -= e.quantity; if (!old.quantity) orders.delete(e.id);
    }
    q = [...orders.values()].filter(x => x.side === 'bid' && x.price === 100).reduce((a,x) => a+x.quantity,0);
    assert.equal(q,target);
    assert.equal(Math.max(...[...orders.values()].filter(x=>x.side==='bid').map(x=>x.price)),100);
    assert.equal(Math.min(...[...orders.values()].filter(x=>x.side==='ask').map(x=>x.price)),102);
    assert.equal([...orders.values()].filter(x=>x.side==='ask'&&x.price===102).reduce((a,x)=>a+x.quantity,0),10);
    events.push(e);
    states.push({index:i,q,w:w(q),orders:[...orders].map(([id,x])=>({id,...x}))});
  }
  const output = check(xs);
  return {initial, events, states, output, clocks: output.completed.map(m=>({
    id:m.id,endEvent:m.end,occurrenceNs:events[m.end-1].occurredNs,
    knowEvent:m.confirm,knownNs:events[m.confirm-1].receivedNs,
    confirmationNs:events[m.confirm-1].receivedNs,publishNs:events[m.confirm-1].receivedNs,
  }))};
}

const repeat = (a,n=3) => a.flatMap(x=>Array(n).fill(x));
const cases = {
  minimal6:[10,10,10,20,10,20],
  priorTrend8:[10,10,10,20,20,20,10,20],
  delayed10:[10,10,10,20,20,20,10,20,20,20],
  alternating15:repeat([10,20,10,20,10]),
  geometric30:repeat([10,20,30,25,15,5,10,20,30,25]),
  sameDirectionReturn13:[...repeat([10,20]),15,...repeat([30,25])],
  forwardMerge16:[...repeat([10,20]),10,...repeat([20,10,20])],
};
const examples = Object.fromEntries(Object.entries(cases).map(([name,xs])=>[name,replayExample(xs)]));
assert.deepEqual(examples.minimal6.output.completed.map(m=>m.type),['P','NoCore']);
assert.deepEqual(examples.priorTrend8.output.completed.map(m=>m.type),['U','NoCore']);
assert.deepEqual(examples.alternating15.output.completed.map(m=>m.type),['U','P','P']);
const up = examples.geometric30.output.completed;
assert.deepEqual(up.map(m=>m.type),['U','D','U']);
assert.deepEqual(up.map(m=>m.end),[9,18,27]);
assert.deepEqual(up.map(m=>m.confirm),[10,19,28]);
const candidateParentSeed = {
  memberIDs:up.map(m=>m.id), memberEvents:up.flatMap(m=>m.memberEvents),
  directions:up.map(m=>m.semanticDirection),
  core:[Math.max(...up.map(m=>m.range[0])),Math.min(...up.map(m=>m.range[1]))],
  extent:[Math.min(...up.map(m=>m.range[0])),Math.max(...up.map(m=>m.range[1]))],
  confirm:Math.max(...up.map(m=>m.confirm)),status:'numeric-three-completed-candidate-window-not-qualified-F2',
};
assert(candidateParentSeed.core[0]<candidateParentSeed.core[1]);

let wordCount=0,prefixCount=0,zeroCoreCount=0,firstZero=null,firstAfterTrend=null;
const histogram={};
for (let length=3;length<=10;length++) {
  let atLength=0;
  function visit(xs) {
    if (xs.length<length) { for (const x of [10,20,30]) visit([...xs,x]); return; }
    wordCount++;atLength++;
    let prev=[];
    for (let n=0;n<=xs.length;n++) {
      const p=check(xs.slice(0,n));prefixCount++;
      assert(eq(prev,p.completed.slice(0,prev.length)));prev=p.completed;
    }
    const o=relation(xs), z=o.completed.findIndex(m=>m.type==='NoCore');
    if(z>=0) {
      zeroCoreCount++;
      firstZero ??= {length,xs,object:o.completed[z]};
      if(o.completed.slice(0,z).some(m=>m.type==='U'||m.type==='D')) firstAfterTrend ??= {length,xs,object:o.completed[z]};
    }
  }
  for(const x of [10,20,30])visit([x,x,x]);
  histogram[length]=atLength;
}
assert.equal(firstZero.length,6);assert.equal(firstAfterTrend.length,8);

// FIXED membership is what makes delayed confirmation ineffective.
const noCore = examples.priorTrend8.output.completed[1];
const delayed = relation(cases.delayed10);
assert.equal(delayed.cores.filter(k=>k.start>noCore.startBoundary&&k.end<=noCore.end).length,0);
const oldUp = examples.sameDirectionReturn13.output.completed[0];
const allCore = examples.sameDirectionReturn13.output.cores;
const conditionalExtensionCounterexample = {
  oldObject:oldUp,orderedCoreValues:allCore.map(k=>k.value),
  nextCore:allCore.find(k=>k.start>oldUp.end),
  conditions:'same P point extent/core identity; independent semantic J; EXT-own exact identity',
};
assert.equal(oldUp.type,'U');assert.equal(conditionalExtensionCounterexample.nextCore.value,30);

const fm=examples.forwardMerge16.output;
const mergedCore=(start,end)=>fm.cores.filter(k=>k.start>start&&k.end<=end).map(k=>k.value);
const merges = {
  backward:{oldEnd:6,newEnd:7,oldConfirmation:7,newDecisionAt:8,withdrawsPreviouslyConfirmedBoundary:true},
  forwardTwo:{events:[7,10],path:cases.forwardMerge16.slice(5,10),cores:mergedCore(6,10),start:20,end:20,range:[10,20]},
  forwardThree:{events:[7,13],path:cases.forwardMerge16.slice(5,13),cores:mergedCore(6,13),start:20,end:10,range:[10,20],confirm:14},
};
assert.deepEqual(merges.forwardTwo.cores,[20]);assert.deepEqual(merges.forwardThree.cores,[20,10]);

const out=resolve(process.argv[2]??resolve(dirname(fileURLToPath(import.meta.url)),'results'));
mkdirSync(out,{recursive:true});
for(const [name,value] of Object.entries({examples,candidateParentSeed,enumeration:{wordCount,prefixCount,zeroCoreCount,histogram,firstZero,firstAfterTrend},conditionalExtensionCounterexample,merges}))
  writeFileSync(resolve(out,`${name}.json`),JSON.stringify(value,null,2)+'\n');
console.log(JSON.stringify({out,wordCount,prefixCount,zeroCoreCount,minimal:firstZero.length,minimalAfterTrend:firstAfterTrend.length,candidateParentSeed:{core:candidateParentSeed.core,confirm:candidateParentSeed.confirm},nativeExamples:Object.keys(examples).length,nativeEvents:Object.values(examples).reduce((n,e)=>n+e.events.length,0)}));
