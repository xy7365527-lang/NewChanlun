import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)), root=path.resolve(here,'../../../../..'), p=path.resolve(here,'../..');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const specs=[
 ['docs/chanlun/text/blog/017-第17课.md',[[12,14],[34,60],[98,98]],'正文；16/28/32注不用于论证'],
 ['docs/chanlun/text/blog/018-第18课.md',[[12,12],[24,44],[50,68]],'正文；24与50行内娇注、46–48整段编注不作为作者依据'],
 ['docs/chanlun/text/blog/020-第20课.md',[[12,12],[48,62],[94,94]],'正文；56/62行内注不作为作者依据；50图片未作证明'],
 ['docs/chanlun/text/blog/029-第29课.md',[[12,16],[38,44],[90,90],[340,364]],'340作者块、342提问者、358问题、360分隔、362作者答复；356编注不用、364下一块'],
 ['docs/chanlun/text/blog/031-第31课.md',[[12,12],[32,32],[376,393]],'384作者块；386是混合问答，只取===后的盘整无方向答复；390重复不独立计证'],
 ['docs/chanlun/text/blog/033-第33课.md',[[12,30],[52,52],[184,198]],'正文与答疑分开；192是读者提出三P构成高一级中枢，196是作者强调级别的回答'],
 ['docs/chanlun/text/blog/035-第35课.md',[[12,38]],'14–36正文，无所用行内编注'],
 ['docs/chanlun/text/blog/038-第38课.md',[[12,44]],'18–36正文；14/42注不作作者依据'],
 ['docs/chanlun/text/blog/052-第52课.md',[[306,335]],'310/312作者块；316–324读者问；326分隔、328作者答；330混合续问答，不以332重复计证'],
 ['docs/chanlun/text/blog/084-第84课.md',[[12,12],[50,68]],'52–60正文；F1初始与F2完整递归角色有别'],
 ['.chanlun/definitions/zhongshu.md',[[81,109]],'当前教义正本；本研究不改裁定'],
 ['.chanlun/definitions/qushi.md',[[179,213]],'当前教义正本；兼容缝、未裁接口与概念无方向分开'],
 ['AGENTS.md',[[54,109]],'#804与#812的规则和E1撤销边界'],
 ['rust/src/theta_v0/classifier/center.rs',[[180,186],[240,298]],'当前代码，仅证明实现和载荷情况，不裁原义'],
 ['rust/src/theta_v0/classifier/recursive_tower.rs',[[391,405],[640,644]],'当前L0/window分派'],
 ['rust/src/theta_v0/classifier/cand_predicate.rs',[[240,251]],'单子hi比较兼容行为'],
 ['rust/src/theta_v0/parser/segment.rs',[[435,447],[731,743]],'发射后方向反转两条路径'],
];
const files=specs.map(([f,ranges,attribution])=>{
 const b=fs.readFileSync(path.join(root,f)),ls=b.toString().split('\n');
 return {path:f,sha256:sha(b),attribution,excerpts:ranges.map(([start,end])=>({start,end,lines:ls.slice(start-1,end).map((text,i)=>({line:start+i,text}))}))};
});
fs.writeFileSync(path.join(here,'source-excerpts.json'),JSON.stringify({sourceRoot:root,head:'cfe88d5782aabad24c2c14d07498cbe7c4fb1b0b',files},null,2)+'\n');
const records=[];
function walk(dir){for(const ent of fs.readdirSync(dir,{withFileTypes:true})){const f=path.join(dir,ent.name);if(ent.isDirectory())walk(f);else if(ent.name.endsWith('.md')){
 const rel=path.relative(p,f),m=rel.match(/^(?:stage|Stage)(\d+)/); if(!m || Number(m[1])<42 || Number(m[1])>50)continue;
 const b=fs.readFileSync(f),ls=b.toString().split('\n');
 const hits=ls.flatMap((text,i)=>/029(?::|-|.*第29|.*三买)|不回前核|前 P.*点核/.test(text)?[{line:i+1,text,context:ls.slice(Math.max(0,i-2),Math.min(ls.length,i+3))}]:[]);
 if(hits.length)records.push({path:rel,sha256:sha(b),hits});
}}}
walk(p);
fs.writeFileSync(path.join(here,'029-dependency-inventory.json'),JSON.stringify({scope:'All Markdown under Stage42–50 top-level and stage42–50 trees; textual hits include hashes/provenance and unrelated 029 passages; semantic classification in p-direction-scope.md',records},null,2)+'\n');
const issueRecords=[];
for(const n of [804,812,815,900]){
 const f=path.join(here,`issue-${n}.json`),b=fs.readFileSync(f),d=JSON.parse(b);
 const blocks=[{url:d.url,role:'body',body:d.body},...d.comments.map(c=>({url:c.url,role:'comment',createdAt:c.createdAt,body:c.body}))];
 issueRecords.push({number:n,title:d.title,state:d.state,url:d.url,sha256:sha(b),blocks:blocks.filter(x=> n===804 || /Z-3|E1|盘整.{0,18}方向|单中枢|无方向|三值|兼容缝/.test(x.body))});
}
fs.writeFileSync(path.join(here,'issue-selected-blocks.json'),JSON.stringify(issueRecords,null,2)+'\n');
console.log(JSON.stringify({sourceFiles:files.length,dependencyMarkdown:records.length,issues:issueRecords.length}));
