import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

// 作者小证据：身份候选的有限算例；不是原义判定器或完整 F1/F2。
const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../../../../..');
const sha = s => createHash('sha256').update(s).digest('hex');
const sources = [
  ['docs/chanlun/text/blog/017-第17课.md', [[38,46]], '正文界98；所取行无编注'],
  ['docs/chanlun/text/blog/018-第18课.md', [[24,28],[40,44],[50,58],[62,68]], '正文界68；24与50有行内娇注，60及46–48是整段编注；不得引用注家文字'],
  ['docs/chanlun/text/blog/020-第20课.md', [[16,24],[28,40],[48,60],[94,94]], '正文界94；22、24、28、36、56含注，按报告逐句排除'],
  ['docs/chanlun/text/blog/030-第30课.md', [[82,94]], '82缠师署名；84–88提问；90分隔；92作者答复'],
  ['docs/chanlun/text/blog/032-第32课.md', [[163,168]], '163缠师署名；165的==后才是答复；167重复加粗不另计'],
  ['docs/chanlun/text/blog/033-第33课.md', [[12,24]], '12作者署名；16正文无注；9数次级走势，非原始事件'],
  ['docs/chanlun/text/blog/035-第35课.md', [[12,24],[38,38]], '正文界38；16–24无行首/行内注'],
  ['docs/chanlun/text/blog/037-第37课.md', [[164,177],[247,265],[313,327]], '署名/时间164–166、247–249、313–315；170/172/174须仅取分隔符后回答；176重复不另计'],
  ['docs/chanlun/text/blog/084-第84课.md', [[12,12],[52,60],[68,68]], '正文界68；52–60所用行无注'],
  ['.chanlun/definitions/zhongshu.md', [[71,77],[130,146],[178,190],[194,244],[246,290],[300,338]], '现行教义，只读；不将备选点基底反套递归父核严格正宽'],
  ['.chanlun/definitions/level_recursion.md', [[24,32],[75,85],[99,110],[142,148]], '现行教义，只读；完成条目不能代替具体完成证书'],
  ['.chanlun/definitions/qushi.md', [[40,70],[78,91],[124,150]], '现行教义，只读；数相应级别的实例，非seed数量'],
];
const manifest = sources.map(([path,ranges,attribution]) => {
  const bytes=readFileSync(resolve(root,path));
  const lines=bytes.toString('utf8').split('\n');
  return {path,sha256:sha(bytes),attribution,excerpts:ranges.map(([from,to]) =>
    ({from,to,lines:lines.slice(from-1,to).map((text,i)=>({line:from+i,text}))}))};
});
writeFileSync(resolve(here,'source-manifest.json'),JSON.stringify({sourceRoot:root,expectedHead:'2e3551802306b704e759631487a75498b3b08b12',sources:manifest},null,2)+'\n');

function platforms(xs) {
  const ps=[];
  for (let i=0;i<xs.length;i++) {
    const p=ps.at(-1);
    if(p && p.value===xs[i]) p.end=i+1;
    else ps.push({start:i+1,end:i+1,value:xs[i]});
  }
  return ps.map(p=>({...p,seedCount:Math.max(0,p.end-p.start-1),mature:p.end-p.start>=2,birth:p.start+2}));
}
function hull(xs) {return [Math.min(...xs),Math.max(...xs)];}
function m2(a,b) {
  if(b.outer[0]>a.outer[1]) return 'up_numeric';
  if(b.outer[1]<a.outer[0]) return 'down_numeric';
  if((b.core[0]>a.core[1] && b.outer[0]<=a.outer[1]) ||
     (b.core[1]<a.core[0] && b.outer[1]>=a.outer[0])) return 'expansion_numeric';
  return 'neither_new_trend_nor_separated_core_expansion';
}
function snapshot(xs) {
  const all=platforms(xs), seeds=all.filter(p=>p.mature);
  const tokens=seeds.map(p=>({...p,core:[p.value,p.value],outer:[p.value,p.value]}));
  const groups=[];
  for(const s of seeds) {
    const g=groups.at(-1);
    if(g && g.value===s.value) g.seedStarts.push(s.start);
    else groups.push({start:s.start,value:s.value,birth:s.birth,seedStarts:[s.start]});
  }
  const instances=groups.map((g,i)=>{
    const next=groups[i+1];
    const end=next?next.start-1:xs.length;
    return {...g,end,core:[g.value,g.value],outer:hull(xs.slice(g.start-1,end)),
      sourceEvents:Array.from({length:end-g.start+1},(_,j)=>g.start+j),
      closed:!!next,confirmation:next?next.birth:null,
      membershipStatus:next?'frozen':'provisional_at_this_prefix'};
  });
  const leadingEnd=groups.length?groups[0].start-1:xs.length;
  const leadingNoCore=Array.from({length:leadingEnd},(_,i)=>i+1);
  const owned=[...leadingNoCore,...instances.flatMap(g=>g.sourceEvents)];
  assert.deepEqual(owned,Array.from({length:xs.length},(_,i)=>i+1));
  for (const g of instances) {
    assert(g.outer[0]<=g.value && g.outer[1]>=g.value);
    for (const start of g.seedStarts) {
      assert.equal(xs[start-1],g.value);
      assert.equal(xs[start],g.value);
      assert.equal(xs[start+1],g.value);
    }
  }
  return {xs,platforms:all,tokens,instances,leadingNoCore,
    tokenRelations:tokens.slice(1).map((b,i)=>m2(tokens[i],b)),
    instanceRelations:instances.slice(1).map((b,i)=>m2(instances[i],b))};
}
const named = {
  constant3:[10,10,10], constant6:Array(6).fill(10),constant9:Array(9).fill(10),
  shortReturn:[10,10,10,20,10,10,10],
  shortReturn2:[10,10,10,20,20,10,10,10],
  alternatingMature:[10,10,10,20,20,20,10,10,10,20,20,20],
  noCorePreserved:[10,10,10,30,25,20,20,20],
  earlierSpike:[10,10,10,30,20,20,20,15,15,15],
  noMature:[10,20,10,20,10,20],
  ascending:[10,10,10,20,20,20,30,30,30],
};
const examples=Object.fromEntries(Object.entries(named).map(([name,xs])=>[name,snapshot(xs)]));
assert.equal(examples.constant6.platforms[0].seedCount,4);
assert.equal(examples.constant9.platforms[0].seedCount,7);
assert.equal(examples.constant9.instances.length,1);
assert.equal(examples.shortReturn.tokens.length,2);
assert.equal(examples.shortReturn.instances.length,1);
assert.deepEqual(examples.shortReturn.instances[0].outer,[10,20]);
assert.equal(examples.alternatingMature.instances.length,4);
assert.deepEqual(examples.alternatingMature.instanceRelations,['up_numeric','down_numeric','up_numeric']);
assert.deepEqual(examples.earlierSpike.tokens.map(t=>t.value),[10,20,15]);
assert.equal(examples.earlierSpike.tokenRelations[0],'up_numeric');
assert.equal(examples.earlierSpike.instanceRelations[0],'expansion_numeric');
assert.deepEqual(examples.noCorePreserved.instances[0].sourceEvents,[1,2,3,4,5]);
assert.equal(examples.noMature.instances.length,0);
assert.equal(examples.noMature.leadingNoCore.length,6);

// 小域逐前缀：核查来源全保留，以及已经封定的核实例字段稳定。
// active 的临时成员允许变化，故绝不把它计作已确认的完整成员。
let words=0,prefixes=0,frozenComparisons=0;
function visit(xs) {
  if(xs.length) {
    words++;
    const full=snapshot(xs);
    for(let n=0;n<=xs.length;n++) {
      prefixes++;
      for(const prior of snapshot(xs.slice(0,n)).instances.filter(g=>g.closed)) {
        const same=full.instances.find(g=>g.start===prior.start);
        assert.deepEqual(same,prior);
        frozenComparisons++;
      }
    }
  }
  if(xs.length<8) for(const v of [10,20,30]) visit([...xs,v]);
}
visit([]);
const result={candidate:'C_RET-v0',status:'author_only_not_semantic_admission',
  arithmeticScope:'synthetic observation words; no claim of literal order-message reachability',
  examples,checks:{words,prefixes,frozenComparisons,coverage:true,closedIdentityStable:true},
  notProved:['source necessity of seed identity policy','complete move existence/uniqueness','P completion','a1','F2 closure','market value']};
writeFileSync(resolve(here,'results.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify(result.checks));
