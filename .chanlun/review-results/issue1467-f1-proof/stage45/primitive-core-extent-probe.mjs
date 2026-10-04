import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const tree = '/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const hull = xs => [Math.min(...xs), Math.max(...xs)];
const gcd = (a,b) => b ? gcd(b,a%b) : a;
const w = q => { const n=102*q+1000,d=q+10,g=gcd(n,d); return `${n/g}/${d/g}`; };
function relation(a,b) {
  if (b.outer[0] > a.outer[1]) return 'up';
  if (b.outer[1] < a.outer[0]) return 'down';
  if (b.core[0] > a.core[1] || b.core[1] < a.core[0]) return 'expansion-predicate';
  return 'cores-not-separated';
}
function domains(cs) {
  return {
    P: cs.map((c,i) => ({id:i,core:[c,c],outer:[c,c]})),
    E: cs.map((c,i) => ({id:i,core:[c,c],outer:hull([i ? cs[i-1] : c,c])})),
  };
}
function legalHistory(cs) {
  const orders = new Map([['S0',{side:'buy',price:100,quantity:cs[0]}],['A0',{side:'sell',price:102,quantity:10}]]);
  let aux=0,buy=0;
  const snapshot = () => Object.fromEntries([...orders].map(([id,o])=>[id,{...o}]));
  const initialOrders=snapshot(), states=[{after:0,q:cs[0],orders:snapshot()}],events=[];
  function add(id,price,quantity) { assert(!orders.has(id)); orders.set(id,{side:'buy',price,quantity}); return {op:'add',id,side:'buy',price,quantity}; }
  for(let i=0;i<cs.length;i++) for(let j=0;j<3;j++) {
    let e;
    if(i>0 && j===0) {
      const delta=cs[i]-cs[i-1];
      if(delta>0) e=add(`S${++buy}`,100,delta);
      else {
        const candidates=[...orders].filter(([,o])=>o.side==='buy'&&o.price===100&&o.quantity>=-delta);
        assert(candidates.length,'one legal cancel must suffice for these named histories');
        const [id,o]=candidates[0];e={op:'cancel',id,side:'buy',price:100,quantity:-delta};
        o.quantity+=delta;if(o.quantity===0) orders.delete(id);
      }
    } else e=add(`U${++aux}`,99,1);
    events.push({sequence:events.length+1,...e});
    const q=[...orders.values()].filter(o=>o.side==='buy'&&o.price===100).reduce((s,o)=>s+o.quantity,0);
    assert.equal(q,cs[i]);assert(q>0);assert.deepEqual(orders.get('A0'),{side:'sell',price:102,quantity:10});
    states.push({after:events.length,q,orders:snapshot()});
  }
  return {initialOrders,events,states};
}
function moves(cs,kind) {
  const out=[];let previousEnd=0;
  function emit(a,b,confirmCore) {
    const start=previousEnd,end=3*(b+1);
    const path=[start===0?cs[0]:cs[start/3-1],...cs.slice(a,b+1)];
    const {P,E}=domains(cs);
    const fields={ownedCoreIndices:Array.from({length:b-a+1},(_,j)=>a+j),ownedValues:cs.slice(a,b+1),
      events:[start+1,end],confirm:3*(confirmCore+1),range:hull(path),path,
      candidateType:b===a?'P':cs[b]>cs[a]?'U':'D',
      P_internalRelations:P.slice(a,b).map((x,j)=>relation(x,P[a+j+1])),
      E_internalRelations:E.slice(a,b).map((x,j)=>relation(x,E[a+j+1]))};
    out.push(fields);previousEnd=end;
  }
  if(kind==='RT') {
    let prevTurn=0;
    for(let t=1;t<cs.length-1;t++) if(Math.sign(cs[t]-cs[t-1])!==Math.sign(cs[t+1]-cs[t])) {
      if(t-prevTurn<2) return {out,blockedAt:3*(t+2)};
      emit(prevTurn,t-1,t+1);prevTurn=t;
    }
  } else if(kind==='v2') {
    let a=0;
    for(let t=1;t<cs.length-1;t++) if(Math.sign(cs[t]-cs[t-1])!==Math.sign(cs[t+1]-cs[t])) {
      emit(a,t,t+1);a=t+1;
    }
  } else if(kind==='OC') {
    let a=0;
    while(a+1<cs.length) {
      let b=a+1,d=Math.sign(cs[b]-cs[a]);
      while(b+1<cs.length&&Math.sign(cs[b+1]-cs[b])===d)b++;
      if(b+1===cs.length) break;
      emit(a,b,b+1);a=b+1;
    }
  }
  return {out,blockedAt:null};
}
const words={four:[10,20,30,25],alternating12:[10,20,10,20],alternating15:[10,20,10,20,10],RT30:[10,20,30,25,15,5,10,20,30,25]};
const named={};
for(const [name,cs] of Object.entries(words)) {
  const history=legalHistory(cs), {P,E}=domains(cs);
  for(let i=0;i<cs.length;i++) {
    const seed=history.states.slice(3*i+1,3*i+4).map(s=>s.q);
    const cells=history.states.slice(3*i,3*i+4).map(s=>s.q);
    assert.deepEqual(hull(seed),P[i].outer);assert.deepEqual(hull(cells),E[i].outer);
    const cellIntervals=Array.from({length:3},(_,j)=>hull(cells.slice(j,j+2)));
    assert.deepEqual([Math.max(...cellIntervals.map(x=>x[0])),Math.min(...cellIntervals.map(x=>x[1]))],E[i].core);
  }
  named[name]={values:cs,history,P,E,P_pairs:P.slice(1).map((b,i)=>relation(P[i],b)),E_pairs:E.slice(1).map((b,i)=>relation(E[i],b)),
    v2:moves(cs,'v2'),OC:moves(cs,'OC'),RT:moves(cs,'RT')};
}
assert.deepEqual(named.four.RT.out[0].ownedValues,[10,20]);
assert.equal(named.four.RT.out[0].confirm,12);
assert.equal(named.alternating12.RT.blockedAt,9);
assert.deepEqual(named.RT30.RT.out.map(m=>m.ownedValues),[[10,20],[30,25,15],[5,10,20]]);
assert.deepEqual(named.RT30.RT.out.map(m=>m.range),[[10,20],[15,30],[5,20]]);
const parentCore=[Math.max(...named.RT30.RT.out.map(m=>m.range[0])),Math.min(...named.RT30.RT.out.map(m=>m.range[1]))];
assert.deepEqual(parentCore,[15,20]);
let wordCount=0,pairCount=0,pUp=0,pDown=0,eExpansion=0,constantExtensions=0;
function enumerate(prefix,n) {
  if(prefix.length===n) {
    wordCount++;const {P,E}=domains(prefix);
    for(let i=1;i<n;i++) {
      pairCount++;const rp=relation(P[i-1],P[i]);rp==='up'?pUp++:pDown++;
      assert.equal(relation(E[i-1],E[i]),'expansion-predicate');eExpansion++;
      assert(E[i-1].outer[0]<=prefix[i-1]&&E[i-1].outer[1]>=prefix[i-1]);
      assert(E[i].outer[0]<=prefix[i-1]&&E[i].outer[1]>=prefix[i-1]);
    }
    for(let i=0;i<n;i++) for(let len=3;len<=9;len++) {
      const c=prefix[i],prev=i?prefix[i-1]:c;
      assert.deepEqual(hull(Array(len).fill(c)),P[i].outer);
      assert.deepEqual(hull([prev,...Array(len).fill(c)]),E[i].outer);constantExtensions++;
    }
    return;
  }
  for(const c of [10,20,30])if(!prefix.length||prefix.at(-1)!==c)enumerate([...prefix,c],n);
}
for(let n=2;n<=8;n++)enumerate([],n);
const paths=['docs/chanlun/text/blog/017-第17课.md','docs/chanlun/text/blog/018-第18课.md','docs/chanlun/text/blog/020-第20课.md','docs/chanlun/text/blog/035-第35课.md','docs/chanlun/text/blog/037-第37课.md','docs/chanlun/text/blog/083-第83课.md','docs/chanlun/text/blog/084-第84课.md','.chanlun/definitions/zhongshu.md','.chanlun/definitions/qushi.md','.chanlun/definitions/level_recursion.md','.chanlun/review-results/issue1467-f1-proof/stage42/classification-bridge.md','.chanlun/review-results/issue1467-f1-proof/stage42/axis-bridge.md','.chanlun/review-results/issue1467-f1-proof/stage44/turn-boundary-candidate.md','.chanlun/review-results/issue1467-f1-proof/stage44/f1-completion-scope.md','.chanlun/review-results/issue1467-f1-proof/stage44/ScopeCorrections-v2.md','.chanlun/review-results/issue1467-f1-proof/GoalReframe-v4.md','.chanlun/review-results/issue1467-f1-proof/PlateauBaseCandidate-v2.md','.chanlun/review-results/issue1467-f1-proof/stage43/owned-core-candidate.md'];
const sourceHashes=Object.fromEntries(paths.map(p=>[p,createHash('sha256').update(readFileSync(join(tree,p))).digest('hex')]));
const result={scope:'Candidate-conditional interval/source checks; no original-semantic certification',tree,head:'41a5a649ce7c471d58fbf5c4486ba5cda7a74e9d',named,
  finiteChecks:{alphabet:[10,20,30],lengths:[2,8],wordCount,pairCount,pUp,pDown,eExpansion,constantExtensions},
  RT30_parent:{Q:parentCore,W:parentCore.map(w)},sourceHashes,allAssertionsPassed:true};
writeFileSync(join(here,'primitive-core-extent-results.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({finiteChecks:result.finiteChecks,RT30_parent:result.RT30_parent,allAssertionsPassed:true}));
