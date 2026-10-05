import fs from 'node:fs';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const dir=fileURLToPath(new URL('.',import.meta.url));
const root=path.resolve(dir,'../../../../..');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const manifest=JSON.parse(fs.readFileSync(dir+'manifest-v1.json','utf8'));
for(const record of [...manifest.packet,...manifest.dependencies]){
 const raw=fs.readFileSync(path.resolve(root,record.path));
 assert.equal(raw.length,record.bytes,record.path+' bytes');
 assert.equal(hash(raw),record.sha256,record.path+' sha256');
}
const sources=JSON.parse(fs.readFileSync(dir+'source-excerpts-v1.json','utf8')).sources;
let rows=0;
for(const src of sources){
 const raw=fs.readFileSync(path.resolve(root,src.path));assert.equal(hash(raw),src.sha256);
 const lines=raw.toString('utf8').split('\n');
 for(const e of src.excerpts){assert.equal(lines[e.line-1],e.text,src.path+':'+e.line);rows++;}
}
const extraction=JSON.parse(fs.readFileSync(dir+'semantics-extraction-v1.json','utf8'));
const original=fs.readFileSync(path.resolve(root,extraction.source),'utf8');
assert.equal(hash(original),extraction.sha256);
const body=original.slice(original.indexOf(extraction.sliceStart),original.indexOf(extraction.sliceEnd));
assert.equal(hash(body),extraction.extractedBodySha256);
assert.ok(fs.readFileSync(dir+'direct-semantics-v1.mjs','utf8').includes(body));
console.log(JSON.stringify({status:'freeze-pass',packetFiles:manifest.packet.length,dependencyFiles:manifest.dependencies.length,sourceFiles:sources.length,sourceIndexedRows:rows,manifestSha256:hash(fs.readFileSync(dir+'manifest-v1.json')),scope:'字节、行号与提取对应；非来源作者身份、语义或独立审查结论'},null,2));
