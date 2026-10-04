import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const D=path.dirname(new URL(import.meta.url).pathname);
const m=JSON.parse(fs.readFileSync(path.join(D,'manifest-v1.json'),'utf8'));
let n=0;const failed=[];
for(const e of [...m.author,...m.dependencies,...m.external]){
 const b=fs.readFileSync(e.path);n++;
 if(b.length!==e.bytes||crypto.createHash('sha256').update(b).digest('hex')!==e.sha256)failed.push(e.path);
}
const excerpts=JSON.parse(fs.readFileSync(path.join(D,'source-excerpts-v1.json'),'utf8'));
const R='/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
let lines=0;
for(const e of excerpts.sources){const a=fs.readFileSync(path.join(R,e.path),'utf8').split('\n');for(const l of e.lines){lines++;if(a[l.line-1]!==l.text)failed.push(e.path+':'+l.line);}}
console.log(JSON.stringify({status:failed.length?'failed':'passed',checkedFiles:n,excerptLines:lines,failed,scope:'字节与来源行；不是数学独评'}));
process.exitCode=failed.length?1:0;
