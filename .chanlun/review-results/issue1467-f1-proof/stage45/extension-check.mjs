import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';

// A finite relation checker, not an order-book engine or an original-meaning oracle.
const here = path.dirname(fileURLToPath(import.meta.url));
const root = '/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const dir = (a,b) => b > a ? 1 : b < a ? -1 : 0;
const ids = n => Array.from({length:n},(_,i)=>i);
const outerDir=(a,b)=>b.DD>a.GG?1:b.GG<a.DD?-1:0;
function extRel(values,retained) {
  const outer=values.map(c=>({DD:c,GG:c})); // Explicit point-outer contract P.
  return outer.slice(1).every((b,i)=>retained[i]===outerDir(outer[i],b));
}
function partitions(n) {
  return ids(2 ** Math.max(0,n-1)).map(mask => {
    const p = []; let block = [];
    for (let i=0;i<n;i++) { block.push(i); if(i===n-1 || (mask & 2**i)) {p.push(block);block=[];} }
    return p;
  });
}
function gamma(values,p) {
  // Ordered exclusive/exhaustive ownership; each nonsingleton block has one strict direction.
  const flat=p.flat();
  if(JSON.stringify(flat)!==JSON.stringify(ids(values.length))) return false;
  return p.every(b=>b.length===1 || b.slice(1).every((i,j)=>dir(values[b[j]],values[i])===dir(values[b[0]],values[b[1]]) && dir(values[b[0]],values[b[1]])!==0));
}
function bridgeFailures(values,p) {
  const owner=Array(values.length); p.forEach((b,x)=>b.forEach(i=>owner[i]=x));
  const failures=[];
  for(let i=0;i+2<values.length;i++) {
    const j=i+1,k=i+2,d=dir(values[i],values[j]);
    if(owner[i]===owner[j] && d!==0 && dir(values[j],values[k])===d && owner[k]!==owner[j])
      failures.push({i,j,k,d,ownerIJ:owner[j],ownerK:owner[k]});
  }
  return failures;
}
function rt(values) {
  const turns=[];
  for(let b=1;b+1<values.length;b++) if(dir(values[b-1],values[b])!==dir(values[b],values[b+1])) turns.push(b);
  const C=[];let a=0;
  for(const b of turns) {
    if(b-a<2) break;
    const own=ids(b-a).map(i=>a+i);
    const states = [a===0?values[0]:values[a-1],...values.slice(a,b)];
    C.push({own,d:dir(values[a],values[a+1]),start:3*a+1,end:3*b,confirm:3*(b+2),range:[Math.min(...states),Math.max(...states)],nextCore:b});
    a=b;
  }
  return {C,tail:ids(values.length-a).map(i=>a+i)};
}
function exactCompletedRefinement(p,C) {
  return C.every(m=>p.some(b=>JSON.stringify(b)===JSON.stringify(m.own)));
}
function caseCheck(values) {
  const out=rt(values), all=partitions(values.length), valid=all.filter(p=>gamma(values,p));
  const exact=valid.filter(p=>exactCompletedRefinement(p,out.C));
  const exactAndJ=exact.filter(p=>bridgeFailures(values,p).length===0);
  const witnesses=out.C.map(m=>{
    const [i,j,k]=[m.own.at(-2),m.own.at(-1),m.nextCore];
    assert.equal(dir(values[i],values[j]),m.d);
    assert.equal(dir(values[j],values[k]),m.d);
    assert(!m.own.includes(k));
    assert(m.end<3*k+1);
    const retained=values.slice(1).map((b,q)=>dir(values[q],b));
    assert(extRel(values,retained));
    return {ownedLastPair:[i,j],excludedNext:k,d:m.d,edgeValues:[values[i],values[j],values[k]],outerBounds:[i,j,k].map(q=>({DD:values[q],GG:values[q]})),end:m.end,nextSourceStart:3*k+1,confirm:m.confirm,EXTrel:extRel(values,retained),J:false};
  });
  assert.equal(exactAndJ.length,0);
  return {values,...out,witnesses,allPartitions:all.length,gammaPartitions:valid.length,exactCompletedRefinements:exact.length,exactCompletedRefinementsWithJ:exactAndJ.length};
}

const v=[10,20,30,25];
const models=[{name:'join',partition:[[0,1,2],[3]],complete:[{own:[0,1,2],end:9,confirm:12}]},{name:'split',partition:[[0,1],[2,3]],complete:[{own:[0,1],end:6,confirm:12}]}].map(m=>({...m,Gamma:gamma(v,m.partition),EXTrel:extRel(v,v.slice(1).map((x,i)=>dir(v[i],x))),EXTown:exactCompletedRefinement(m.partition,m.complete),J:bridgeFailures(v,m.partition).length===0,bridgeFailures:bridgeFailures(v,m.partition)}));
assert(models.every(m=>m.Gamma && m.EXTrel && m.EXTown));
assert(models[0].J && !models[1].J);
const four=caseCheck(v);
const thirty=caseCheck([10,20,30,25,15,5,10,20,30,25]);
assert.deepEqual(thirty.C.map(m=>[m.end,m.confirm]),[[6,12],[15,21],[24,30]]);
assert.deepEqual(thirty.C.map(m=>m.range),[[10,20],[15,30],[5,20]]);
const intersection=[Math.max(...thirty.C.map(m=>m.range[0])),Math.min(...thirty.C.map(m=>m.range[1]))];
assert.deepEqual(intersection,[15,20]);
assert.deepEqual([four.witnesses.length,thirty.witnesses.length],[1,3]);
// A geometry-only negative control: point labels do not determine true external ranges.
assert(30>20);assert(!(26>35));
// J is weaker than a fully maximal-run decomposition. All singleton cells satisfy it vacuously.
assert(bridgeFailures([10,20,30],[[0],[1],[2]]).length===0);
// Exhaustive auxiliary falsification of the general RT-to-not-J lemma in a finite grid.
let words=0,emitted=0;
function visit(a,n) {
  if(a.length===n) {
    words++; const out=rt(a);
    for(const m of out.C) {
      emitted++;
      const i=m.own.at(-2),j=m.own.at(-1),k=m.nextCore;
      assert.equal(dir(a[i],a[j]),dir(a[j],a[k]));assert(!m.own.includes(k));
    }
    return;
  }
  for(const x of [10,20,30]) if(x!==a.at(-1)) visit([...a,x],n);
}
for(let n=1;n<=9;n++)visit([],n);
const sourcePaths=['docs/chanlun/text/blog/018-第18课.md','docs/chanlun/text/blog/020-第20课.md','docs/chanlun/text/blog/035-第35课.md','docs/chanlun/text/blog/037-第37课.md','docs/chanlun/text/blog/084-第84课.md','.chanlun/definitions/qushi.md','.chanlun/definitions/level_recursion.md','docs/adr/0011-operation-decomposition-layer.md','.chanlun/review-results/issue1467-f1-proof/Stage44BaseCompletionAndB.md','.chanlun/review-results/issue1467-f1-proof/stage44/ScopeCorrections-v2.md','.chanlun/review-results/issue1467-f1-proof/stage44/turn-boundary-candidate.md'];
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const result={scope:'Finite relation models only. No claim of original-meaning qualification or complete Omega.',outerContract:'P: every eligible same-level core has DD=GG=c; other outer rules are different contracts.',node:process.version,models,four,thirty,intersection,enumeration:{alphabet:[10,20,30],lengths:[1,9],adjacentDistinct:true,words,emitted,allRTEmissionsRefuteJ:true},sources:sourcePaths.map(p=>({path:p,sha256:sha(path.join(root,p))})),checkerSha256:sha(fileURLToPath(import.meta.url)),allAssertionsPassed:true};
fs.writeFileSync(path.join(here,'extension-check-results.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({allAssertionsPassed:true,models:models.map(m=>({name:m.name,Gamma:m.Gamma,EXTrel:m.EXTrel,EXTown:m.EXTown,J:m.J})),fourWitnesses:four.witnesses,thirtyWitnesses:thirty.witnesses,partitionCounts:[four,thirty].map(c=>({all:c.allPartitions,gamma:c.gammaPartitions,exact:c.exactCompletedRefinements,exactAndJ:c.exactCompletedRefinementsWithJ})),words,emitted},null,2));
