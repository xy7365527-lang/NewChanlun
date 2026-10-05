import assert from 'node:assert/strict';
import fs from 'node:fs';

// 记录级压力控制。table项模拟有限检查器的已接受证据，不是源义证书。
// End或Iref不属于输入、依赖或执行路径。
function scan(k, table, s0=0) {
  assert.ok(Number.isInteger(k)&&Number.isInteger(s0)&&0<=s0&&s0<=k);
  let s=s0; const records=[];
  for(let steps=0; steps<=k-s0; steps++) {
    const bucket=table.filter(w=>w.ready<=k&&w.s===s);
    const unique=new Map();
    for(const w of bucket) {
      const record={identity:{reading:'test',level:0,start:s},end:w.u,fields:{label:w.semanticLabel}};
      unique.set(JSON.stringify(record),record); // proofId is deliberately absent.
    }
    if(unique.size>1)return {status:'ContractConflict',frontier:s,published:[],distinct:[...unique.values()]};
    if(unique.size===0)return {status:'ok',records,frontier:s,buffer:[s,k]};
    const record=[...unique.values()][0];
    if(!Number.isInteger(record.end)||!(s<record.end&&record.end<=k))
      return {status:'ContractConflict',frontier:s,published:[],invalid:record};
    records.push({...record,own:Array.from({length:record.end-s},(_,i)=>s+1+i)});
    s=record.end;
  }
  throw Error('strict-progress invariant violated');
}
const prefix=(a,b)=>a.length<=b.length&&a.every((r,i)=>JSON.stringify(r)===JSON.stringify(b[i]));
function coverage(k,result,s0=0) {
  assert.equal(result.status,'ok');
  const published=result.records.flatMap(r=>r.own);
  const buffer=Array.from({length:k-result.frontier},(_,i)=>result.frontier+i+1);
  assert.deepEqual([...published,...buffer],Array.from({length:k-s0},(_,i)=>s0+i+1));
  assert.equal(new Set([...published,...buffer]).size,k-s0);
  let s=s0;for(const r of result.records){assert.equal(r.identity.start,s);assert.ok(r.end>s);s=r.end;}
  assert.equal(s,result.frontier);
}

const conflict=scan(2,[
 {proofId:'X',s:0,u:1,ready:2,semanticLabel:'X'},
 {proofId:'Y',s:0,u:2,ready:2,semanticLabel:'Y'},
]);
assert.equal(conflict.status,'ContractConflict');assert.deepEqual(conflict.published,[]);
const sameRecord=scan(2,[
 {proofId:'proof-A',s:0,u:1,ready:2,semanticLabel:'X'},
 {proofId:'proof-B',s:0,u:1,ready:2,semanticLabel:'X'},
]);
assert.equal(sameRecord.records.length,1);coverage(2,sameRecord);

const table=[
 {proofId:'X-proof',s:0,u:1,ready:5,semanticLabel:'X'},
 {proofId:'Y-proof',s:1,u:2,ready:2,semanticLabel:'Y'},
];
const timeline=Array.from({length:6},(_,k)=>({k,...scan(k,table)}));
for(const row of timeline)coverage(row.k,row);
for(let k=0;k<5;k++)assert.ok(prefix(timeline[k].records,timeline[k+1].records));
assert.deepEqual(timeline[2].records,[]);
assert.deepEqual(timeline[5].records.map(r=>r.fields.label),['X','Y']);
assert.deepEqual(timeline[5].buffer,[2,5]);
const ready=[5,2],publish=ready.map((_,n)=>Math.max(...ready.slice(0,n+1)));
assert.deepEqual(publish,[5,5]);
const oldAt2=table.filter(w=>w.ready<=2).sort((a,b)=>a.s-b.s).map(w=>w.semanticLabel);
const oldAt5=table.filter(w=>w.ready<=5).sort((a,b)=>a.s-b.s).map(w=>w.semanticLabel);
assert.deepEqual(oldAt2,['Y']);assert.deepEqual(oldAt5,['X','Y']);
assert.equal(prefix(oldAt2,oldAt5),false);
const result={scope:'两个局部身份/调度压力例；不是缠论实例或H1-H6验收',
 conflict,duplicateProofs:sameRecord,timeline,ready,publish,old:{at2:oldAt2,at5:oldAt5,prefixStable:false},allAssertionsPassed:true};
if(process.argv[2])fs.writeFileSync(process.argv[2],JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({conflictRejected:true,duplicateProofsCollapsed:true,publication:publish,checkedPrefixes:timeline.length,allAssertionsPassed:true}));
