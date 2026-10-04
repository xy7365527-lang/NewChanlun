import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const repo=path.resolve(here,'../../../../..');
const P='.chanlun/review-results/issue1467-f1-proof';
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const entries=[
  {path:'docs/chanlun/text/blog/018-第18课.md',lines:[12,24,26,28,40,50,52,54,56,58,64,66,68],
   authorScope:'12署名正文；68为正文界。24有行内娇注，50有行内级别注，不把注当原文。26/28用于Move类型，50/52只用于连接级别和类型。'},
  {path:'docs/chanlun/text/blog/020-第20课.md',lines:[12,16,22,24,30,48,52,54,56,58,60,62,94],
   authorScope:'12署名正文，94为界。22前后有编注，24/56/62含行内注；论证不采用注中文字。52/54/58/60为本轮主要原文式。'},
  {path:'docs/chanlun/text/blog/061-第61课.md',lines:[12,26,44,56],
   authorScope:'12署名正文，56为界；44无行首或行内编注。26仅表明发展判断会随证据更新，不用它认证任意成员重分。'},
  {path:'docs/chanlun/text/blog/037-第37课.md',lines:[164,166,168,170,172,174],
   authorScope:'164明确缠中说禅，166时间；174在==后的作者回答限制同一式子；问题段不作作者断言，176加粗复写不作第二来源。'},
  {path:'docs/chanlun/text/blog/101-第101课.md',lines:[12,20],authorScope:'12署名正文，20无编注，支持三类点与破坏同一。'},
  {path:'docs/chanlun/text/blog/027-第27课.md',lines:[850,852,854,856,858],authorScope:'850为缠中说缠，854/856作者资格继续暂停。858另一署名不能回填。仅用于排除依赖。'},
  {path:'docs/chanlun/text/blog/102-第102课.md',lines:[12,38,48],authorScope:'38含娇注 a-连接段 A-中枢，不作为作者逐字释义。仅登记来源边界，不重裁Z-5。'},
  {path:'.chanlun/definitions/zhongshu.md',lines:[194,198,204,205,206,210,212,222,225,228,304,310,314,328,336,338,376,378,384,388,390,394,396,398,400,404,406,408],
   authorScope:'现行教义政策。其历史援引027不恢复目标答语作者资格，102内注同样不升级为正文。'},
];
const audit={date:'2026-10-05',baseline:'23f33fb8ae0df856bf2281d196e3d3190579f295',
  status:'author-source-audit-not-independent-review',
  statement:'逐字与哈希只绑定本地文本，不认证说话者。采用的作者判断及注释排除由每条authorScope记录。',
  sources:entries.map(e=>{const b=fs.readFileSync(path.join(repo,e.path));const lines=b.toString('utf8').split('\n');return {...e,sha256:hash(b),excerpts:e.lines.map(n=>({line:n,text:lines[n-1]}))};})};
fs.writeFileSync(path.join(here,'source-audit.json'),JSON.stringify(audit,null,2)+'\n');
const web={checkedDate:'2026-10-05',mechanism:'web.run search followed by open of primary author pages',
  entries:[
    {url:'https://blog.sina.com.cn/s/blog_486e105c010007zw.html',title:'教你炒股票20：缠中说禅走势中枢级别扩张及第三类买卖点',
     displayedDate:'2007-01-05 15:23:22',author:'缠中说禅',profile:'http://blog.sina.com.cn/u/1215172700',
     checkedLocalLines:[16,22,24,30,48,52,54,56,58,60,62],
     result:'原作者页正文包含指定Zn选择与四指标、初始核心、相邻中心关系、三类点和首次限制。本地编注不在对应原文中。',
     limitations:'未取得历史评论；网页图片未读取。本记录是回查摘要，不是逐字网页存档或评论身份认证。'},
    {url:'https://blog.sina.com.cn/s/blog_486e105c01000b9n.html',title:'教你炒股票61：区间套定位标准图解（分析示范六）',
     displayedDate:'2007-06-21 08:13:21',author:'缠中说禅',profile:'http://blog.sina.com.cn/u/1215172700',
     checkedLocalLines:[26,44],result:'原作者页正文包含发展中的比较证据及前三、第四、第五的角色次序和69-72/70-73归属判例。',
     limitations:'只核正文，不读取图像像素、不从图片重建价格、没有恢复027历史评论作者资格。'}]};
fs.writeFileSync(path.join(here,'web-crosscheck.json'),JSON.stringify(web,null,2)+'\n');
const authorPaths=[`${P}/stage54/exit-member-source-scope.md`,...[
  'check-member-source.mjs','member-source-checks.json','freeze-source-evidence.mjs','source-audit.json','web-crosscheck.json','verify-freeze.mjs','README.md'
].map(f=>`${P}/stage54/source-evidence/${f}`)];
const inputs=[...entries.map(e=>e.path),...[
 'Stage53SeedDepartureAndOuterObstruction.md','stage53/ScopeClarifications-v1.md',
 'stage53/source-evidence/AuthorshipQualification-v1.md','stage53/independent-authorship-repair-review.md',
 'stage53/candidate-evidence/contract-v1.md','stage53/candidate-evidence/reanchor-control-addendum-v1.md',
 'stage53/candidate-evidence/reanchor-control-input.json','stage53/candidate-evidence/results-v1/after-touch-comparison.json',
 'stage53/candidate-evidence/results-v1/reanchor_115_control.json',
 'stage53/source-evidence/issue-812-snapshot.json','stage53/source-evidence/issue-820-snapshot.json'
].map(f=>`${P}/${f}`)];
const bind=p=>{const b=fs.readFileSync(path.join(repo,p));return {path:p,bytes:b.length,sha256:hash(b)};};
const manifest={version:1,date:'2026-10-05',baseline:audit.baseline,status:'frozen-author-package-pending-independent-review',
  limitations:'本清单绑定字节身份与作者自检范围，不是独评通过、作者认证或语义成立证书。',
  authorFiles:authorPaths.map(bind),externalInputs:inputs.map(bind),sourceExcerptLineCount:audit.sources.reduce((n,s)=>n+s.excerpts.length,0)};
fs.writeFileSync(path.join(here,'frozen-author-manifest-v1.json'),JSON.stringify(manifest,null,2)+'\n');
console.log(JSON.stringify({authorFiles:manifest.authorFiles.length,externalInputs:manifest.externalInputs.length,sourceExcerptLineCount:manifest.sourceExcerptLineCount}));
