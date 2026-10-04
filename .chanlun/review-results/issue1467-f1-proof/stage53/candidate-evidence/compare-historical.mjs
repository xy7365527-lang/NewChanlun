// Compare new A53 outputs with frozen historical evidence, after the fresh
// prefix computation is complete. Historical results are never parser inputs.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)),P=path.resolve(here,'../..');
const read=p=>JSON.parse(fs.readFileSync(path.join(P,p),'utf8'));
const records=[];
for(const name of ['same_up_counterexample','same_down_mirror','p_completion_overlap','p_completion_disjoint']){
  const old=read(`stage49/candidate-evidence/results-v1/${name}.json`),fresh=read(`stage53/candidate-evidence/results-v1/${name}.json`);
  const last=fresh.prefixes.at(-1).results.A53;
  assert.equal(last.outputs.length,old.result.coreScan.completed.length);
  for(const [i,x] of last.outputs.entries()){
    const k=old.result.coreScan.completed[i];
    for(const key of ['seed','members','core'])assert.deepEqual(x[key],k[key]);
    assert.equal(x.memberStart,k.sourceStart);assert.equal(x.memberEnd,k.sourceEnd);
    assert.deepEqual(x.memberOuter,k.outer);assert.equal(x.relationshipPublishedAt,k.sealedAt);
  }
  const record={name,coresCompared:last.outputs.length,stage52PrefixesCompared:0};
  if(name.startsWith('same_')){
    const frozen=read(`stage52/candidate-evidence/results-v1/${name}.json`).onlinePrefixes;
    for(const p of frozen){
      const r=fresh.prefixes[p.t].results.A53;
      const mapped=r.outputs.map(x=>({core:`K${x.id}`,start:x.start,end:x.end,sourceEvents:x.ownedEvents,qRange:x.fullRange,evidenceKnownAt:x.relationshipPublishedAt,relationshipPublishedAt:x.relationshipPublishedAt,semanticEligibleAt:null}));
      assert.deepEqual(mapped,p.proposals);
      assert.deepEqual({start:r.tail.start,end:r.tail.end,sourceEvents:r.tail.ownedEvents},p.tail);
      record.stage52PrefixesCompared++;
    }
  }
  records.push(record);
}
const report={ok:true,comparisonIsAfterIndependentReplay:true,records};
fs.writeFileSync(path.join(here,'results-v1/historical-comparison.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report));
