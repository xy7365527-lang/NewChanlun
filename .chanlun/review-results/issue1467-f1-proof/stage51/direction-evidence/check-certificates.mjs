// Separate author-side certificate check: use first/last threshold hits, without importing the producer.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const dir=process.argv[2]??path.join(here,'results-v1');
const read=n=>JSON.parse(fs.readFileSync(path.join(dir,n),'utf8'));
let windows=0,witnessPairs=0,assignments=0,tuples=0;
function check(r) {
  if(!r.strict)return;
  windows++;
  const [L,U]=r.numericalH;assert.ok(L<U);
  const options=[];
  for(const c of r.children) {
    const states=c.sourceStates,values=states.map(s=>s.q);
    assert.deepEqual(c.qRange,[Math.min(...values),Math.max(...values)]);
    const low=states.filter(s=>s.q<=L).map(s=>s.event),high=states.filter(s=>s.q>=U).map(s=>s.event);
    assert.ok(low.length&&high.length);
    const oracle={U:Math.min(...low)<Math.max(...high),D:Math.min(...high)<Math.max(...low)};
    assert.deepEqual(c.directions,['U','D'].filter(d=>oracle[d]));options.push(oracle);
    for(const d of ['U','D']) {
      const first=d==='U'?low:high,second=d==='U'?high:low;
      const expected=first.flatMap(i=>second.filter(j=>i<j).map(j=>[i,j]));
      assert.deepEqual(c.witnesses[d].map(w=>w.indices),expected);witnessPairs+=expected.length;
      for(const w of c.witnesses[d]) {
        const [i,j]=w.indices;
        const a=states.find(s=>s.event===i),b=states.find(s=>s.event===j);
        assert.deepEqual(w.q,[a.q,b.q]);assert.deepEqual(w.w,[a.w,b.w]);assert.equal(w.pointsAvailableAt,j);
      }
    }
    assert.equal(c.conceptDirection,null);assert.equal(c.originalCompletionEvidence,null);assert.equal(c.originalZ3Evidence,null);
  }
  const accepted=[];
  for(const row of r.allEightAssignments) {
    assignments++;
    const word=[...row.word],truth=word.map((d,i)=>options[i][d]);
    assert.deepEqual(row.perChildHasWitness,truth);
    assert.equal(row.feasible,truth.every(Boolean));
    assert.equal(row.alternating,word[0]!==word[1]&&word[1]!==word[2]);
    assert.equal(row.accepted,row.feasible&&row.alternating);
    if(row.accepted)accepted.push(row.word);
  }
  assert.equal(r.allEightAssignments.length,8);assert.deepEqual(r.selections.map(s=>s.word),accepted);
  assert.equal(r.selectionCount,accepted.length);assert.ok(r.selectionCount<=2);
  let total=0;
  for(const s of r.selections) {
    const expected=[...s.word].reduce((n,d,i)=>n*r.children[i].witnesses[d].length,1);
    assert.equal(s.witnessTupleCount,expected);assert.equal(s.allWitnessIndexTuples.length,expected);
    assert.equal(new Set(s.allWitnessIndexTuples.map(t=>JSON.stringify(t))).size,expected);
    for(const t of s.allWitnessIndexTuples)for(let i=0;i<3;i++)
      assert.ok(r.children[i].witnesses[s.word[i]].some(w=>JSON.stringify(w.indices)===JSON.stringify(t[i])));
    total+=expected;
  }
  tuples+=total;assert.equal(r.witnessTupleCount,total);
  assert.equal(r.semanticEligibleAt,null);assert.equal(r.originalParentCore,null);
  assert.equal(r.sourceRangesAvailableAt,r.cuts[3]);
}
for(const n of ['same_up_counterexample','same_down_mirror'])for(const r of read(`${n}.json`).records)check(r);
for(const n of ['negative-order-flow','net-direction-mismatch','two-selections'])check(read(`${n}.json`).result);
const summary={windows,witnessPairs,assignments,witnessTuples:tuples,assertionsPassed:true,
  method:'first/last threshold indices plus full point-pair/certificate consistency',independentReview:false};
fs.writeFileSync(path.join(dir,'certificate-check.json'),JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary,null,2));
