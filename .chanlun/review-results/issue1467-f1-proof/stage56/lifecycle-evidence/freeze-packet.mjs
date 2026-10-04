import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';

const E=path.dirname(fileURLToPath(import.meta.url));
const R=path.resolve(E,'../../../../..');
const P='.chanlun/review-results/issue1467-f1-proof';
const digest=p=>{const b=fs.readFileSync(path.join(R,p));return {path:p,bytes:b.length,sha256:crypto.createHash('sha256').update(b).digest('hex')};};
const sourceSelections=[
  {file:'017-第17课.md',lines:[12,36,40,44,46,52,60,98],qualification:'所用正文均在12署名与98正文界之间；32编注不采用。'},
  {file:'018-第18课.md',lines:[12,24,26,28,30,32,40,44,46,48,64,68],qualification:'24仅用作者句，排除行内(娇：盘整和趋势）；46/48仅冻结作排除证据，不作作者前提。其他采用行在正文界68之前。'},
  {file:'020-第20课.md',lines:[12,48,52,54,58,60,94],qualification:'采用行在正文界94前，未见行首/行内编注；没有以原始首腿方向认定原义方向或Zn。'},
  {file:'031-第31课.md',lines:[376,378,380,382,384,386],qualification:'382只取==后的作者答复；386有问答交错，仅用“盘整哪里有什么方向，只有趋势才有方向”及随后作者核数答复。读者前提不是作者主张。'},
  {file:'035-第35课.md',lines:[12,16,18,22,24,38],qualification:'采用正文，署名原年份20007不擅改；本稿新Ω不沿用三笔点核目录。'},
  {file:'037-第37课.md',lines:[164,166,168,170,172,174],qualification:'164/166作者块；170/174只取问答分隔后作者回答；同式归属不扩大为所有视图禁共享。'},
  {file:'038-第38课.md',lines:[12,14,18,22,24,26,28,42,44,290,292,294,296,298,300],qualification:'14/42是排除的编注；18至28为正文；300只取等号后的作者答复，290/292为其作者块。'},
  {file:'061-第61课.md',lines:[12,44,56],qualification:'44为作者正文，未见行首/行内注；只核固定前三seed之后第四/第五角色。'},
  {file:'084-第84课.md',lines:[12,52,54,56,60,68],qualification:'采用行在12署名与68正文界之间，无行首/行内编注。'}
];
const sources=sourceSelections.map(s=>{
  const relative='docs/chanlun/text/blog/'+s.file,lines=fs.readFileSync(path.join(R,relative),'utf8').split(/\r?\n/);
  return {...digest(relative),qualification:s.qualification,excerpts:s.lines.map(n=>({line:n,text:lines[n-1]}))};
});
fs.writeFileSync(path.join(E,'source-excerpts-v1.json'),JSON.stringify({scope:'仓内指定转载可见作者归属；非历史评论身份的外部认证。027:854/856不入依赖。',sources},null,2)+'\n');
const dependencies=[
  P+'/GoalReframe-v4.md',P+'/stage56/Intent-v1.md',
  P+'/stage54/initial-contract-audit.md',P+'/stage54/independent-initial-contract-review.md',
  P+'/stage54/candidate-evidence/contract-v1.md',P+'/stage54/candidate-evidence/ordinary-proofs-v1.md',
  P+'/stage54/independent-exit-release-review.md',
  P+'/stage54/source-evidence/DirectionMappingQualification-v1.md',P+'/stage54/independent-direction-mapping-repair-review.md',
  P+'/stage55/ScopeCorrections-v1.md',P+'/stage55/independent-scope-repair-review.md',
  P+'/stage54/candidate-evidence/relation-reference.mjs',P+'/stage53/candidate-evidence/relation-reference.mjs',
  P+'/stage54/candidate-evidence/results-v1/six_event_diagnostic.json',
  '.chanlun/definitions/zhongshu.md',...sources.map(s=>s.path)
];
const authors=[P+'/stage56/initial-p-lifecycle.md',...['check-two-controls.mjs','two-controls-v1.json','source-excerpts-v1.json','run-receipt.json','freeze-packet.mjs','verify-freeze.mjs'].map(x=>P+'/stage56/lifecycle-evidence/'+x)];
const manifest={schema:'stage56-initial-lifecycle-author-packet-v1',root:R,scope:'作者冻结；仅内容身份；新独评待主控。没有锁ResearchProgress。',authorFiles:authors.map(digest),dependencies:dependencies.map(digest)};
fs.writeFileSync(path.join(E,'manifest-v1.json'),JSON.stringify(manifest,null,2)+'\n');
console.log(JSON.stringify({ok:true,authorFiles:authors.length,dependencies:dependencies.length,sourceLines:sources.reduce((n,s)=>n+s.excerpts.length,0)}));
