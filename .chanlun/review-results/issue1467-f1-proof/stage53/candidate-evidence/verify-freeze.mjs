import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)),P=path.resolve(here,'../..');
const manifest=JSON.parse(fs.readFileSync(path.join(here,'frozen-author-manifest-v1.json'),'utf8'));
for(const item of [...manifest.authorFiles,...manifest.externalInputs]){
  const file=path.resolve(P,item.path),bytes=fs.readFileSync(file);
  assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'),item.sha256,item.path);
  assert.equal(bytes.length,item.bytes,item.path);
}
console.log(JSON.stringify({ok:true,authorFiles:manifest.authorFiles.length,externalInputs:manifest.externalInputs.length}));
