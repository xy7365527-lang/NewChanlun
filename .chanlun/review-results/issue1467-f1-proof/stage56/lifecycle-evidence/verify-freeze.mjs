import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const E=path.dirname(fileURLToPath(import.meta.url)),R=path.resolve(E,'../../../../..');
const m=JSON.parse(fs.readFileSync(path.join(E,'manifest-v1.json')));
for(const f of [...m.authorFiles,...m.dependencies]){
  assert.ok(!f.path.includes('ResearchProgress'));
  const b=fs.readFileSync(path.join(R,f.path));assert.equal(b.length,f.bytes,f.path);
  assert.equal(crypto.createHash('sha256').update(b).digest('hex'),f.sha256,f.path);
}
const source=JSON.parse(fs.readFileSync(path.join(E,'source-excerpts-v1.json'))); let n=0;
for(const s of source.sources){const lines=fs.readFileSync(path.join(R,s.path),'utf8').split(/\r?\n/);for(const e of s.excerpts){assert.equal(lines[e.line-1],e.text,`${s.path}:${e.line}`);n++;}}
const c=JSON.parse(fs.readFileSync(path.join(E,'two-controls-v1.json')));
assert.deepEqual(c.totals,{controls:2,events:22,prefixes:24});
console.log(JSON.stringify({ok:true,authorFiles:m.authorFiles.length,dependencies:m.dependencies.length,sourceLines:n,independentReview:'pending',semanticEligibility:'unproved'}));
