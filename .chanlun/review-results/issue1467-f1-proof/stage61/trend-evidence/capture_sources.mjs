import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)),R=path.resolve(here,'../../../../..');
const specs={'018':[24,26,28],'035':[14,16,18,22],'037':[16,18,20,22],
 '057':[26,28,32,40],'067':[18,20,22,28],'084':[52,54,56,60]};
const output=[];
for(const [lesson,nums] of Object.entries(specs)){
 const relative=`docs/chanlun/text/blog/${lesson}-第${Number(lesson)}课.md`,file=path.join(R,relative);
 const bytes=fs.readFileSync(file),lines=bytes.toString().split('\n'),bodyEnd=lines.findIndex(s=>s.includes('↑正文'))+1;
 assert(bodyEnd>0);
 const selected=nums.map(n=>{
  const raw=lines[n-1];assert(n<bodyEnd);assert(!/^\s*[（(](?:注|娇)/.test(raw));
  let used=raw,excluded=[];
  if(lesson==='018'&&n===24){const note='(娇：盘整和趋势）';assert(raw.includes(note));used=raw.replace(note,'');excluded=[note];}
  if(lesson==='067'&&(n===20||n===28)){const marker=n===20?'（娇：':'（娇注：';const i=raw.indexOf(marker);assert(i>0);used=raw.slice(0,i);excluded=[raw.slice(i)];}
  assert(!/[（(](?:注|娇)/.test(used));
  return {line:n,raw,author_text_used:used,excluded_editorial:excluded,attribution:'author body; header/boundary/prefix/inline note checked'};
 });
 output.push({path:relative,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),author_header:lines.find(s=>s.startsWith('作者：')),body_end_line:bodyEnd,selected});
}
for(const [relative,ranges] of [['.chanlun/definitions/beichi.md',[[307,323],[335,341],[508,533]]],['.chanlun/definitions/zhongshu.md',[[132,170]]]]){
 const bytes=fs.readFileSync(path.join(R,relative)),lines=bytes.toString().split('\n');
 output.push({path:relative,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),attribution:'repository doctrinal ruling, not a verbatim author formula',
  excerpts:ranges.map(([a,b])=>({from:a,to:b,text:lines.slice(a-1,b).join('\n')}))});
}
const dst=process.argv[2];assert(dst&&path.isAbsolute(dst));fs.writeFileSync(dst,JSON.stringify({scope:'Stage61 trend sources only; 027 excluded',sources:output},null,2)+'\n');
console.log(dst);
