import assert from 'node:assert/strict';
import fs from 'node:fs';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { replay, directD54 } from './direct-semantics-v1.mjs';
const dir=fileURLToPath(new URL('.',import.meta.url));
const control=JSON.parse(fs.readFileSync(dir+'main16-control-v1.json','utf8'));
const original=JSON.parse(fs.readFileSync(new URL('../../stage58/admissibility-evidence/main8-model-v1.json',import.meta.url),'utf8'));
assert.deepEqual(control.initial,original.input.initial);
assert.deepEqual(control.events.slice(0,8),original.input.events);
const outputs=[];
let prior=[];
for(let n=0;n<=16;n++) {
 const run=replay(control,n), d=directD54(run);
 assert.equal(d.domain,'all-quotes-ready');
 assert.deepEqual(d.outputs.slice(0,prior.length),prior);
 assert.deepEqual([...d.outputs.flatMap(x=>x.ownedEvents),...d.tail.eventIds],control.events.slice(0,n).map(x=>x.id));
 prior=d.outputs;
 outputs.push({n,quotes:run.quotes,flows:run.outputs,closed:d.outputs,tail:d.tail,terminal:d.terminal});
}
const a=outputs[16].closed[0],b=outputs[16].closed[1];
assert.equal(outputs[16].closed.length,2);
assert.equal(a.start,0);assert.equal(a.end,6);assert.equal(a.relationshipPublishedAt,8);
assert.deepEqual(a.core,[100,102]);assert.deepEqual(a.wholeOuter,[1,102]);
assert.equal(b.start,6);assert.equal(b.end,14);assert.equal(b.relationshipPublishedAt,16);
assert.deepEqual(b.core,[1,99]);assert.deepEqual(b.wholeOuter,[1,110]);
assert.deepEqual(b.seed,[4,5,6]);assert.deepEqual(b.M,[4,5,6,7]);
assert.deepEqual(b.memberOuter,[1,99]);assert.equal(b.L,8);assert.equal(b.R,9);
assert.deepEqual(b.ownedEvents,control.events.slice(6,14).map(x=>x.id));
assert.deepEqual(outputs[16].tail,{start:14,end:16,eventIds:control.events.slice(14).map(x=>x.id)});
assert.ok(a.memberEvents.every(x=>!b.memberEvents.includes(x)));
assert.equal(outputs[7].closed.length,0);
for(let n=8;n<=15;n++)assert.equal(outputs[n].closed.length,1);
const q=outputs[16].quotes;
assert.deepEqual(q.slice(8),[
 {bid:1,ask:99},{bid:1,ask:99},{bid:3,ask:99},{bid:1,ask:99},
 {bid:1,ask:102},{bid:1,ask:110},{bid:103,ask:110},{bid:103,ask:109},{bid:104,ask:109}
]);
// Closed intervals below record the actual full L1 support, not a core proxy.
const supportHull=(l,r)=>[Math.min(...q.slice(l,r+1).map(x=>x.bid)),Math.max(...q.slice(l,r+1).map(x=>x.ask))];
assert.deepEqual(supportHull(0,6),a.wholeOuter);
assert.deepEqual(supportHull(6,14),b.wholeOuter);
const returnControl=structuredClone(control);
returnControl.id='D59-return16';
returnControl.events[14]={id:'D59-return-E15',action:'cancel',side:'bid',orderId:'s59-b103',price:103};
returnControl.events[15]={id:'D59-return-E16',action:'add',side:'bid',orderId:'s59-b104',price:104};
const ret=replay(returnControl,16), rd=directD54(ret);
assert.equal(rd.outputs.length,1);
assert.equal(rd.terminal.tag,'AwaitPair');
assert.deepEqual(ret.outputs[9].geometry.range,[1,110]);
const status={schema:'stage59-common-model-author-check-v1',status:'pass',independentReview:false,marketSamples:0,controls:2,mainPrefixes:17,returnControlPrefixes:1,
 main:{completedEnd58:2,semanticCompleted:null,first:{span:[a.start,a.end],core:a.core,whole:a.wholeOuter,publishedAt:a.relationshipPublishedAt},second:{span:[b.start,b.end],core:b.core,whole:b.wholeOuter,publishedAt:b.relationshipPublishedAt,seed:b.seed,M:b.M,memberOuter:b.memberOuter,L:b.L,R:b.R},activeTail:outputs[16].tail},
 returnControl:{end58Count:rd.outputs.length,R:ret.outputs[9].geometry.range,terminal:rd.terminal},
 originalF2:{eligibleTriple:null,parent:null,note:'仅两份End58；未把它们认证为原义完成，未调用F2'},
 logicalCheck:{assumptions:['identity-preserving End58 to completed constructor embedding','one initial kernel per cycle preserved','same level and immediate source successor preserved','non-same-level decomposition role with NoPP'],conclusion:'该组前提在main16无模型；不推出任意读法或全部F1无模型'}};
fs.writeFileSync(dir+'main16-results-v1.json',JSON.stringify({control,outputs,returnControl:{input:returnControl,quotes:ret.quotes,flows:ret.outputs,relation:rd}},null,2)+'\n');
fs.writeFileSync(dir+'self-check-v1.json',JSON.stringify(status,null,2)+'\n');
console.log(JSON.stringify(status,null,2));
