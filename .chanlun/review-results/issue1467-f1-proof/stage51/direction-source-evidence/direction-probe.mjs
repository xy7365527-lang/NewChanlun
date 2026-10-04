import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../../../..');
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const sign = x => x > 0 ? 'U' : x < 0 ? 'D' : '0';
const alternating = ds => ds.length === 3 && ds.every(x => x === 'U' || x === 'D') && ds[0] !== ds[1] && ds[1] !== ds[2];
const gcd = (a,b) => b === 0n ? a : gcd(b, a % b);
function ratio(n,d) { const g=gcd(n,d); return `${n/g}/${d/g}`; }

function replay(source) {
  const book = new Map(Object.values(source.initial).map(x => [x.id, {...x, side:x.id==='ask' ? 'ask' : 'bid'}]));
  const actual=[];
  function state(event) {
    const bid=[...book.values()].filter(x=>x.side==='bid'), ask=[...book.values()].filter(x=>x.side==='ask');
    const bp=Math.max(...bid.map(x=>x.price)), ap=Math.min(...ask.map(x=>x.price));
    const bq=bid.filter(x=>x.price===bp).reduce((s,x)=>s+x.quantity,0), aq=ask.filter(x=>x.price===ap).reduce((s,x)=>s+x.quantity,0);
    return {event,q:bq,w:ratio(BigInt(ap*bq+bp*aq),BigInt(bq+aq))};
  }
  actual.push(state(0));
  for(const e of source.events) {
    if(e.kind==='add') { assert(!book.has(e.id)); book.set(e.id,{...e}); }
    else if(e.kind==='reduce') { const o=book.get(e.id); assert(o && o.quantity>=e.quantity); o.quantity-=e.quantity; if(o.quantity===0) book.delete(e.id); }
    else assert.fail(`unsupported event ${e.kind}`);
    actual.push(state(e.seq));
  }
  assert.deepEqual(actual,source.states);
  return actual;
}

function metrics(states,a,b) {
  const s=states.slice(a,b+1), q=s.map(x=>x.q), lo=Math.min(...q), hi=Math.max(...q);
  const steps=s.slice(1).map((x,i)=>({event:x.event,d:sign(x.q-s[i].q)})).filter(x=>x.d!=='0');
  const lows=s.filter(x=>x.q===lo).map(x=>x.event), highs=s.filter(x=>x.q===hi).map(x=>x.event);
  const order=(l,h)=>l<h?'U':l>h?'D':'0';
  const strong=lows.at(-1)<highs[0]?'U':highs.at(-1)<lows[0]?'D':null;
  const standard=q[0]===lo && q.at(-1)===hi && lo<hi ? 'U' : q[0]===hi && q.at(-1)===lo && lo<hi ? 'D':null;
  return {start:a,end:b,states:s,range:[lo,hi],actualStart:q[0],actualEnd:q.at(-1),net:sign(q.at(-1)-q[0]),strictSteps:steps,firstStrict:steps[0]?.d??null,lastStrict:steps.at(-1)?.d??null,lowEvents:lows,highEvents:highs,firstExtremaOrder:order(lows[0],highs[0]),lastExtremaOrder:order(lows.at(-1),highs.at(-1)),allExtremaOrder:strong,oppositeExtremaEndpoints:standard};
}

const reports=[];
for(const name of ['same_up_counterexample','same_down_mirror']) {
  const sourcePath=path.resolve(here,`../../stage50/candidate-evidence/results-v1/${name}.json`);
  const bytes=fs.readFileSync(sourcePath), d=JSON.parse(bytes), states=replay(d.source);
  const candidates=d.snapshots[0].leftWidenedNecessary.strictNecessary;
  assert.deepEqual(candidates.map(x=>x.cuts),[[0,5,8,12],[0,5,9,12]]);
  const partitions=candidates.map(p=>{
    const children=p.children.map(c=>{
      assert.equal(c.type,'P'); assert.equal(c.conceptDirection,null);
      assert.equal(c.originalCompletionEvidence,null); assert.equal(c.dirForZ3Evidence,null);
      const m=metrics(states,c.start,c.end);
      assert.deepEqual(m.range,c.qRange);
      const core=c.coreEvidence[0], cm=metrics(states,core.sourceStart,core.sourceEnd);
      assert.equal(cm.strictSteps.length,3); assert(alternating(cm.strictSteps.map(x=>x.d)));
      assert.equal(cm.firstStrict,core.startDirection==='Up'?'U':'D');
      return {...m,ownedCoreID:core.id,coreSource:[core.sourceStart,core.sourceEnd],coreFormation:cm.firstStrict,coreFormationSteps:cm.strictSteps,conceptDirection:null,originalCompletionEvidence:null,dirForZ3Evidence:null};
    });
    const fields=['net','firstStrict','lastStrict','coreFormation','firstExtremaOrder','lastExtremaOrder','allExtremaOrder','oppositeExtremaEndpoints'];
    const directions=Object.fromEntries(fields.map(f=>{const values=children.map(c=>c[f]);return [f,{values,alternates:alternating(values)}];}));
    const intersection=[Math.max(...children.map(c=>c.range[0])),Math.min(...children.map(c=>c.range[1]))];
    assert.deepEqual(intersection,p.qParentCore); assert(intersection[0]<intersection[1]);
    const crossings=children.map(c=>{
      const witnesses={U:[],D:[]};
      for(let i=0;i<c.states.length;i++)for(let j=i+1;j<c.states.length;j++) {
        const a=c.states[i],b=c.states[j];
        if(a.q<=intersection[0] && b.q>=intersection[1])witnesses.U.push([a.event,b.event]);
        if(a.q>=intersection[1] && b.q<=intersection[0])witnesses.D.push([a.event,b.event]);
      }
      return {source:[c.start,c.end],values:['U','D'].filter(x=>witnesses[x].length>0),witnesses};
    });
    const alternatingChoices=[];
    for(const a of crossings[0].values)for(const b of crossings[1].values)for(const c of crossings[2].values)if(alternating([a,b,c]))alternatingChoices.push([a,b,c]);
    return {cuts:p.cuts,children,intersection,directions,crossings,alternatingCrossingChoices:alternatingChoices,status:'necessary pieces only; no original completed P or F2 qualification issued'};
  });
  reports.push({name,sourceFile:path.relative(root,sourcePath),sourceSHA256:sha(bytes),replayedEvents:d.source.events.length,replayedStates:states.length,partitions});
}

let endpointTuples=0,strictStandardTriples=0;
for(let a=0;a<=4;a++)for(let b=0;b<=4;b++)for(let c=0;c<=4;c++)for(let d=0;d<=4;d++) {
  endpointTuples++;
  const xs=[a,b,c,d]; if(xs.slice(1).some((x,i)=>x===xs[i]))continue;
  const ranges=xs.slice(1).map((x,i)=>[Math.min(xs[i],x),Math.max(xs[i],x)]);
  const common=[Math.max(...ranges.map(x=>x[0])),Math.min(...ranges.map(x=>x[1]))];
  if(common[0]>=common[1])continue;
  strictStandardTriples++;
  assert(alternating(xs.slice(1).map((x,i)=>sign(x-xs[i]))));
  assert.deepEqual(common,[Math.max(ranges[0][0],ranges[2][0]),Math.min(ranges[0][1],ranges[2][1])]);
}
const weakCounterexample=[[0,4,1],[1,5,2],[2,6,3]].map(q=>metrics(q.map((x,event)=>({event,q:x})),0,2));
assert.deepEqual(weakCounterexample.map(x=>x.net),['U','U','U']);
assert.deepEqual(weakCounterexample.map(x=>x.allExtremaOrder),['U','U','U']);
const result={schema:'stage51-direction-diagnostics-v1',scope:'finite diagnostics and conditional algebra only; no source semantic completion assertion',reports,standardBridgeFiniteCheck:{endpointTuples,strictStandardTriples,passed:true,ordinaryProof:'p-direction-scope.md section 6'},weakCounterexample:{children:weakCounterexample,commonRange:[2,4],scope:'countermodel to numerical implication only, not asserted original completed moves'}};
fs.writeFileSync(path.join(here,'direction-results.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({replayedEvents:reports.reduce((s,x)=>s+x.replayedEvents,0),partitions:reports.map(x=>({name:x.name,directions:x.partitions.map(p=>({cuts:p.cuts,directions:p.directions}))})),standardBridgeFiniteCheck:result.standardBridgeFiniteCheck},null,2));
