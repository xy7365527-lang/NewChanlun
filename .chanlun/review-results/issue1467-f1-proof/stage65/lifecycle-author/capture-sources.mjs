import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const here=path.dirname(new URL(import.meta.url).pathname);
const R='/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const wanted={
  '035-第35课.md':[14,16,18,20,22,24],
  '038-第38课.md':[14,18,22,24,26,36,42],
  '039-第39课.md':[14,16,26,28,30,32,34,36,38],
  '043-第43课.md':[14,18,20,30,34,38,42,44,48],
  '084-第84课.md':[52,54,56],
  '037-第37课.md':[16,18,20,22],
  '018-第18课.md':[24,26,28,30,32],
  '061-第61课.md':[26,28]
};
const entries=[];
for(const [name,numbers] of Object.entries(wanted)) {
  const rel='docs/chanlun/text/blog/'+name,p=path.join(R,rel);
  const lines=fs.readFileSync(p,'utf8').split('\n');
  const boundary=lines.findIndex(x=>x.includes('↑正文'))+1;
  for(const line of numbers) {
    const raw=lines[line-1];
    const lead=/^[（(]?\s*(?:娇注|娇|注)[：:]/.test(raw);
    const inline=[...raw.matchAll(/[（(](?:娇注|娇|注)[：:][^）)]*[）)]/g)].map(x=>x[0]);
    const body=lead?null:raw.replace(/[（(](?:娇注|娇|注)[：:][^）)]*[）)]/g,'〔编者插注剔除〕');
    entries.push({path:rel,fileSha256:sha(p),line,bodyBoundary:boundary,
      raw,authorStatus:lead?'editorial-not-author':line<boundary?'author-body-with-inline-exclusions':'outside-body-needs-speaker-audit',
      excludedInline:inline,acceptedAuthorText:body,
      use:lead?'excluded': 'local source reading; no independent original-manuscript authenticity claim'});
  }
}
const definitions={
  '.chanlun/definitions/beichi.md':[[309,323],[336,357],[521,537],[561,581]],
  '.chanlun/definitions/qushi.md':[[124,151]],
  '.chanlun/definitions/zhongshu.md':[[130,146],[180,186]],
  '.chanlun/definitions/level_recursion.md':[[283,295]]
};
const doctrine=[];
for(const [rel,ranges] of Object.entries(definitions)) {
  const p=path.join(R,rel),lines=fs.readFileSync(p,'utf8').split('\n');
  for(const [start,end] of ranges)doctrine.push({path:rel,fileSha256:sha(p),start,end,text:lines.slice(start-1,end).join('\n')});
}
fs.writeFileSync(path.join(here,'source-evidence.json'),JSON.stringify({scope:'local canonical text, current line verification and attribution; 027:854/856 not read or used',entries,doctrine},null,2)+'\n');
const dependencies=[
  '.chanlun/review-results/issue1467-f1-proof/Stage64DynamicCandidatesAndCausalFrontier.md',
  '.chanlun/review-results/issue1467-f1-proof/stage64/dynamic-review/review.md',
  '.chanlun/review-results/issue1467-f1-proof/stage64/adoption/ScopeCorrections-v1.md',
  '.chanlun/review-results/issue1467-f1-proof/stage64/completion-author/v2/Report.md',
  '.chanlun/review-results/issue1467-f1-proof/stage64/completion-author/v3/Report.md',
  '.chanlun/review-results/issue1467-f1-proof/stage64/final-integration-review/review.md'
];
fs.writeFileSync(path.join(here,'source-bindings.json'),JSON.stringify(dependencies.map(rel=>({path:path.join(R,rel),sha256:sha(path.join(R,rel))})),null,2)+'\n');
console.log(JSON.stringify({sourceLines:entries.length,doctrineRanges:doctrine.length,dependencies:dependencies.length}));
