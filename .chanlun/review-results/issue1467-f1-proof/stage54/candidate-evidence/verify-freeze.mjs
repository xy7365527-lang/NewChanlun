import fs from 'node:fs';import path from 'node:path';import crypto from 'node:crypto';import assert from 'node:assert/strict';import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)),P=path.resolve(here,'../..');
const manifest=JSON.parse(fs.readFileSync(path.join(here,'frozen-author-manifest-v1.json'),'utf8'));
for(const f of [...manifest.authorFiles,...manifest.externalInputs]){
  const bytes=fs.readFileSync(path.join(P,f.path));assert.equal(bytes.length,f.bytes,f.path+' bytes');
  assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'),f.sha256,f.path+' hash');
}
const inputLock=JSON.parse(fs.readFileSync(path.join(here,'input-lock.json'),'utf8'));
for(const f of inputLock.files){const bytes=fs.readFileSync(path.join(P,f.path));assert.equal(bytes.length,f.bytes);assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'),f.sha256);}
console.log(JSON.stringify({ok:true,baseline:manifest.baseline,authorFiles:manifest.authorFiles.length,externalInputs:manifest.externalInputs.length}));
