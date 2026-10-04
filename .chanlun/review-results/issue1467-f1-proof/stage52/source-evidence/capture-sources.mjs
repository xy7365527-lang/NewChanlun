import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../../../..');
const P = '.chanlun/review-results/issue1467-f1-proof/';
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const specs = [
  ['docs/chanlun/text/blog/017-第17课.md', [[36,60],[96,98]]],
  ['docs/chanlun/text/blog/018-第18课.md', [[24,44],[50,68]]],
  ['docs/chanlun/text/blog/020-第20课.md', [[48,62],[94,94]]],
  ['docs/chanlun/text/blog/024-第24课.md', [[30,42],[52,52],[148,160]]],
  ['docs/chanlun/text/blog/029-第29课.md', [[340,364]]],
  ['docs/chanlun/text/blog/035-第35课.md', [[12,38]]],
  ['docs/chanlun/text/blog/037-第37课.md', [[164,177]]],
  ['docs/chanlun/text/blog/038-第38课.md', [[12,44],[290,304]]],
  ['docs/chanlun/text/blog/039-第39课.md', [[12,40],[48,48]]],
  ['docs/chanlun/text/blog/043-第43课.md', [[12,20],[42,52]]],
  ['docs/chanlun/text/blog/046-第46课.md', [[148,154]]],
  ['docs/chanlun/text/blog/052-第52课.md', [[12,12],[26,26],[306,335]]],
  ['docs/chanlun/text/blog/084-第84课.md', [[50,60],[68,68]]],
  ['.chanlun/definitions/zhongshu.md', [[16,37],[300,342]]],
  ['.chanlun/definitions/level_recursion.md', [[75,85],[283,295],[329,340]]],
  ['.chanlun/definitions/qushi.md', [[124,151],[192,201],[232,237]]],
  ['docs/adr/0011-operation-decomposition-layer.md', [[15,35],[53,98],[122,126]]],
  [P+'Stage51PDirectionAndSourceCorrection.md', []],
  [P+'stage51/ScopeClarifications-v1.md', []],
  [P+'stage43/panzheng-completion.md', [[183,241]]],
  [P+'stage43/owned-core-candidate.md', [[235,246]]],
  [P+'stage44/f1-completion-scope.md', [[13,23],[104,139]]],
  [P+'stage44/ScopeCorrections-v2.md', [[1,23]]],
  [P+'Stage44BaseCompletionAndB.md', []],
  [P+'stage45/extension-ownership.md', [[15,23]]],
  [P+'stage46/completed-endpoint-scope.md', [[5,35],[84,124]]],
  [P+'stage47/base-complete-construction.md', [[27,34],[174,180]]],
  [P+'stage48/mature-core-completion.md', [[112,124]]],
  [P+'stage49/f2-input-contract.md', [[7,31]]],
  [P+'stage50/expansion-handoff-contract.md', [[23,36],[86,114],[162,166]]],
  [P+'stage50/handoff-candidate.md', [[65,74]]],
  [P+'stage51/p-direction-scope.md', [[88,94],[139,145]]],
  [P+'stage49/candidate-evidence/results-v1/same_up_counterexample.json', []],
  [P+'stage52/candidate-evidence/definition-v1.md', []],
];
const sources = specs.map(([relativePath,ranges]) => {
  const bytes=fs.readFileSync(path.join(root,relativePath));
  const lines=bytes.toString('utf8').split('\n');
  return {path:relativePath,sha256:sha(bytes),bytes:bytes.length,ranges,
    excerpts:ranges.map(([start,end])=>({start,end,lines:lines.slice(start-1,end).map((text,i)=>({line:start+i,text}))}))};
});
const issues=[];
for(const [issue,commentID] of [[812,5147131478],[827,5153724096]]) {
  const raw=execFileSync('gh',['api',`repos/xy7365527-lang/NewChanlun/issues/comments/${commentID}`],{encoding:'utf8'});
  const d=JSON.parse(raw);
  const captured={issue,commentID,url:d.html_url,author:d.user.login,createdAt:d.created_at,updatedAt:d.updated_at,body:d.body};
  const filename=`issue-${issue}-comment-${commentID}.json`;
  fs.writeFileSync(path.join(here,filename),JSON.stringify(captured,null,2)+'\n');
  issues.push({issue,commentID,url:d.html_url,filename,sha256:sha(fs.readFileSync(path.join(here,filename))),apiResponseSHA256:sha(raw)});
}
const data={root,head:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),capturedAt:new Date().toISOString(),scope:'Local primary-text excerpts and two live ruling comments; no doctrine edits.',sources,issues};
fs.writeFileSync(path.join(here,'source-excerpts.json'),JSON.stringify(data,null,2)+'\n');
fs.writeFileSync(path.join(here,'source-lock.json'),JSON.stringify({...data,sources:sources.map(({excerpts,...rest})=>rest)},null,2)+'\n');
console.log(JSON.stringify({head:data.head,sourceFiles:sources.length,liveComments:issues.length,sourceLockSHA256:sha(fs.readFileSync(path.join(here,'source-lock.json')))}));
