import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const dir=process.argv[2]??path.join(here,'results-v1');
const P=path.resolve(here,'../..');
const relation=(a,b)=>b[0]>a[1]?'Up':b[1]<a[0]?'Down':'Overlap';
const results=[];
for(const name of ['same_up_counterexample','same_down_mirror']) {
  const f=JSON.parse(fs.readFileSync(path.join(dir,name+'.json')));
  const old=JSON.parse(fs.readFileSync(path.join(P,f.sourceFile)));
  for(const snap of f.snapshots) {
    const cs=old.result.coreScan.completed.filter(c=>c.sealedAt<=snap.t);
    const end=snap.jointWithSuccession.consumeThrough;
    const m=cs.filter(c=>c.sourceEnd<=end).length;
    const unrestricted=[],restricted=[];
    // Enumerate subsets of core boundaries, independently of the event-edge DAG.
    for(let mask=0;mask<2**(m-1);mask++) {
      const bounds=[0];for(let i=1;i<m;i++)if(mask&(1<<(i-1)))bounds.push(i);bounds.push(m);
      const typed=[];let ok=true;
      for(let i=1;i<bounds.length;i++) {
        const a=bounds[i-1],b=bounds[i],own=cs.slice(a,b),next=cs[b];
        if(!next){ok=false;break;}
        let kind;
        if(own.length===1)kind='P';
        else {
          const d=relation(own[0].outer,own[1].outer);
          if(d==='Overlap'||!own.slice(1).every((c,k)=>relation(own[k].outer,c.outer)===d)){ok=false;break;}
          kind=d==='Up'?'U':'D';
        }
        const nextRel=relation(own.at(-1).outer,next.outer);
        if(kind==='P'?nextRel!=='Overlap':nextRel===(kind==='U'?'Up':'Down')){ok=false;break;}
        typed.push({from:a,to:b,kind,cores:own.map(c=>c.id)});
      }
      if(!ok)continue;
      unrestricted.push(typed);
      if(!typed.slice(1).some((x,i)=>x.kind===typed[i].kind&&x.kind!=='P'))restricted.push(typed);
    }
    assert.equal(unrestricted.length,1);
    assert.deepEqual(unrestricted[0].map(b=>b.cores),Array.from({length:m/2},(_,i)=>[`K${2*i}`,`K${2*i+1}`]));
    assert.equal(restricted.length,snap.t===15?1:0);
    const eventMultiplicity=unrestricted[0].slice(1).reduce((n,b)=>n*(cs[b.from].sourceStart-cs[b.from-1].sourceEnd+1),1);
    assert.equal(eventMultiplicity,snap.jointWithoutSuccession.pathCount);
    assert.equal(restricted.length,snap.jointWithSuccession.pathCount);
    const H=snap.localLift;
    const available=cs.filter(c=>H.sourceStart<=c.sourceStart&&c.sourceEnd<=H.sourceEnd);
    assert.equal(available.length,2);
    assert.deepEqual(available.map(c=>c.id),H.pairCoreIDs);
    assert.equal(H.obstruction.requiredDistinctOwnedLevel0Cores,3);
    assert.ok(H.obstruction.requiredDistinctOwnedLevel0Cores>available.length);
    assert.equal(snap.localAllCuts.necessary.length,0);
    for(const x of snap.leftWidenedNecessary.strictNecessary) {
      assert.equal(new Set(x.children.flatMap(c=>c.ownedCoreIDs)).size,3);
      assert.ok(x.children.every(c=>c.originalCompletionEvidence===null&&c.dirForZ3Evidence===null));
      assert.ok(x.qParentCore[0]<x.qParentCore[1]);
    }
    results.push({name,t:snap.t,corePartitionsExamined:2**(m-1),
      onlyLocalGatePartition:unrestricted[0],eventCutMultiplicity:eventMultiplicity,
      withSuccession:restricted.length,localLiftDistinctCoreBudget:[3,available.length]});
  }
}
const report={scope:'separate author core-composition check, not independent review',results,allAssertionsPassed:true};
fs.writeFileSync(path.join(dir,'core-proof-check.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report,null,2));
