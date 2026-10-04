import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const here=dirname(fileURLToPath(import.meta.url));
const root=resolve(here,'../../../../..');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const frozen=JSON.parse(readFileSync(resolve(here,'frozen-manifest.json'),'utf8'));
for(const f of frozen.artifacts) {
  assert.equal(sha(readFileSync(resolve(root,f.path))),f.sha256,`artifact changed: ${f.path}`);
}
const sources=JSON.parse(readFileSync(resolve(here,'source-manifest.json'),'utf8'));
for(const s of sources.sources) {
  assert.equal(sha(readFileSync(resolve(root,s.path))),s.sha256,`source changed: ${s.path}`);
}
console.log(JSON.stringify({status:'frozen',artifacts:frozen.artifacts.length,sources:sources.sources.length,
  semanticReview:'not_independently_reviewed'}));
