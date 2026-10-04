import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)),repo=path.resolve(here,'../../../../..');
const manifest=JSON.parse(fs.readFileSync(path.join(here,'frozen-author-manifest-v1.json'),'utf8'));
for(const row of [...manifest.authorFiles,...manifest.externalInputs]){
  const b=fs.readFileSync(path.join(repo,row.path));
  assert.equal(b.length,row.bytes,row.path);
  assert.equal(crypto.createHash('sha256').update(b).digest('hex'),row.sha256,row.path);
}
const audit=JSON.parse(fs.readFileSync(path.join(here,'source-audit.json'),'utf8'));
let lines=0;
for(const source of audit.sources){const ls=fs.readFileSync(path.join(repo,source.path),'utf8').split('\n');
  for(const excerpt of source.excerpts){assert.equal(ls[excerpt.line-1],excerpt.text,`${source.path}:${excerpt.line}`);lines++;}}
assert.equal(lines,manifest.sourceExcerptLineCount);
console.log(JSON.stringify({ok:true,authorFiles:manifest.authorFiles.length,externalInputs:manifest.externalInputs.length,sourceExcerptLines:lines,
  claim:'byte-and-line-identity-only; not independent semantic acceptance'}));
