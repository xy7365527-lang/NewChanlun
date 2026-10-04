import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

const E = path.dirname(fileURLToPath(import.meta.url));
const R = path.resolve(E, '../../../../..');
const P = '.chanlun/review-results/issue1467-f1-proof';
const hash = p => crypto.createHash('sha256').update(fs.readFileSync(path.join(R, p))).digest('hex');
const read = p => fs.readFileSync(path.join(R, p), 'utf8');
const write = (name, obj) => fs.writeFileSync(path.join(E, name), `${JSON.stringify(obj, null, 2)}\n`);
const specs = [
  ['docs/chanlun/text/blog/017-第17课.md', [[34,60],[1138,1152]], '正文及署名答疑；1146为读者问题，1150为作者答复；上下边并未标明core/outer'],
  ['docs/chanlun/text/blog/018-第18课.md', [[24,44]], '24含娇注；本稿仅用38–44作者正文，不用46之后编注'],
  ['docs/chanlun/text/blog/020-第20课.md', [[18,60]], '22/26/28/36/56有注；引用24用末注之前作者句；核心推理32/38/40/44/48/52/54/58无注'],
  ['docs/chanlun/text/blog/031-第31课.md', [[380,390]], '386混合读者问题与作者回答，390重复作者回答，不作独立第二来源'],
  ['docs/chanlun/text/blog/035-第35课.md', [[14,28]], '作者正文；所用片段无行首/行内注'],
  ['docs/chanlun/text/blog/037-第37课.md', [[14,22],[164,177]], '170/172/174须取分隔符后作者答复，176为重复摘录'],
  ['docs/chanlun/text/blog/038-第38课.md', [[14,28],[42,44]], '14与42为编注不用；18–28作者正文'],
  ['docs/chanlun/text/blog/049-第49课.md', [[128,138]], '134读者问题，136分隔符，138署名作者答复'],
  ['docs/chanlun/text/blog/057-第57课.md', [[26,42]], '作者正文；精确区间取文字，不从图像推价'],
  ['docs/chanlun/text/blog/083-第83课.md', [[14,24]], '14–22作者正文；24末注不用'],
  ['docs/chanlun/text/blog/084-第84课.md', [[50,60]], '作者正文；所用52/54/56/60无注'],
  ['.chanlun/definitions/zhongshu.md', [[45,146],[218,244],[246,296],[370,408]], '现行正本Z1/2/3/4/6/8、S4/5；工程现状陈述不在本轮复核范围'],
  ['.chanlun/definitions/qushi.md', [[40,61],[74,102],[124,151],[179,200]], '现行正本M1/M2及P方向接口'],
  ['.chanlun/definitions/level_recursion.md', [[99,110],[117,148],[228,264]], '现行递归与初始构造口径；不继承历史已实现标签'],
  ['.chanlun/definitions/beichi.md', [[313,323]], '现行完成判定归动力学口径'],
];
write('source-readback.json', specs.map(([p, ranges, audit]) => {
  const txt = read(p), lines = txt.split('\n');
  return {path:p,sha256:hash(p),bytes:Buffer.byteLength(txt),
    bodyMarkers:lines.map((s,i)=>s.includes('↑正文')?i+1:null).filter(Boolean),audit,
    excerpts:ranges.map(([start,end])=>({start,end,lines:lines.slice(start-1,end).map((text,i)=>({line:start+i,text}))}))};
}));

const rcPath = `${P}/stage49/candidate-evidence/results-v1/same_up_counterexample.json`;
const d = JSON.parse(read(rcPath)), q = d.source.states.map(s=>s.q);
const cores = d.result.coreScan.completed.slice(0,5);
assert.deepEqual(cores.map(c=>c.id), ['K0','K1','K2','K3','K4']);
assert.deepEqual(cores.map(c=>c.core), [[1010,1030],[1110,1130],[1136,1138],[1210,1230],[1236,1238]]);
assert.deepEqual(cores.map(c=>c.outer), [[1000,1040],[1100,1140],[1135,1140],[1200,1240],[1235,1240]]);
assert.deepEqual(cores.map(c=>[c.sourceStart,c.sourceEnd,c.bornAt,c.sealedAt]),
  [[0,4,5,7],[5,8,9,11],[9,12,13,15],[13,16,17,19],[17,20,21,23]]);
const range = (start,end) => [Math.min(...q.slice(start,end+1)),Math.max(...q.slice(start,end+1))];
const intersect = xs => [Math.max(...xs.map(x=>x[0])),Math.min(...xs.map(x=>x[1]))];
const pieces = [[5,8],[8,9],[9,12]].map(([start,end],i)=>({
  label:['A','B','C'][i],start,end,events:Array.from({length:end-start},(_,j)=>start+j+1),
  range:range(start,end),endpoints:[q[start],q[end]],
  numericalDirection:q[end]>q[start]?'Up':'Down',
  whollySupportedCoreIDs:cores.filter(c=>c.sourceStart>=start&&c.sourceEnd<=end).map(c=>c.id)
}));
assert.deepEqual(pieces.map(x=>x.range), [[1100,1140],[1110,1140],[1135,1140]]);
assert.deepEqual(pieces.map(x=>x.numericalDirection), ['Down','Up','Down']);
assert.deepEqual(pieces[1].whollySupportedCoreIDs, []);
assert.deepEqual(intersect(pieces.map(x=>x.range)), [1135,1140]);
const checkpoints = [15,23].map(t=>({t,
  formedCoreIDs:cores.filter(c=>c.bornAt<=t).map(c=>c.id),
  sealedCoreIDs:cores.filter(c=>c.sealedAt<=t).map(c=>c.id),
  publishedOutputs:d.result.outputs.filter(m=>m.publishedAt<=t),
}));
assert.deepEqual(checkpoints.map(c=>c.publishedOutputs.map(m=>[m.id,m.type,m.start,m.end,m.confirmedAt,m.ownedCoreIDs])),
  [[['M0','U',0,8,15,['K0','K1']]],
   [['M0','U',0,8,15,['K0','K1']],['M1','U',8,16,23,['K2','K3']]]]);
assert.equal(checkpoints[0].formedCoreIDs.length,3);
const old = checkpoints[0].publishedOutputs[0];
const unused = checkpoints[0].formedCoreIDs.filter(id=>!old.ownedCoreIDs.includes(id));
assert.deepEqual(unused,['K2']);
const projection = checkpoints[1].publishedOutputs;
assert.equal(projection[0].end,projection[1].start);
assert.equal(projection[0].type,projection[1].type);
write('rc49-handoff-facts.json', {
  source:{path:rcPath,sha256:hash(rcPath)},cores,checkpoints,pieces,
  localTripleIntersection:intersect(pieces.map(x=>x.range)),
  expansionPairs:[[1,2],[3,4]].map(([a,b])=>({left:cores[a].id,right:cores[b].id,
    coreSeparated:cores[b].core[0]>cores[a].core[1],
    outerTouch:cores[b].outer[0]<=cores[a].outer[1],
    outerIntersection:intersect([cores[a].outer,cores[b].outer])})),
  H2:{availableCores:3,oldM0CoreCount:2,remainingCoreIDs:unused,maxCompleteMovesWithPreservedM0:1+unused.length,
      conditions:['fixed complete catalog','every completed level0 move owns >=1 catalog core','no reuse within same decomposition','M0 owns K0/K1']},
  scope:'Exact arithmetic and saved fields only; does not certify semantic core identity, Move completion, DirForZ3, or independent review.'
});
write('check-results.json', {passed:true,sourceFiles:specs.length,coresChecked:5,checkpoints:[15,23],
  localPieces:3,parentInterval:[1135,1140],middlePieceEventCount:1,middlePieceCoreCount:0,
  H2MaxCompleteMoves:2,sourceCapture:'Line snapshots and SHA256 of actual bytes',
  independence:'author self-check; no RC49 constructor imported, no formal verification or independent semantic review'});
console.log(JSON.stringify({passed:true,sourceFiles:specs.length,coresChecked:5,checkpoints:[15,23],H2MaxCompleteMoves:2}));
