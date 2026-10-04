import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)),P=path.resolve(here,'../..'),R=path.resolve(P,'../../..');
const m=JSON.parse(fs.readFileSync(path.join(here,'frozen-review-manifest-v2.json'),'utf8'));
const hash=f=>crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
for(const e of m.authorFiles)assert.equal(hash(path.join(P,e.path)),e.sha256,e.path);
for(const e of m.externalInputs)assert.equal(hash(path.join(P,e.path)),e.sha256,e.path);
for(const e of m.canonicalSources)assert.equal(hash(path.join(R,e.path)),e.sha256,e.path);
console.log(JSON.stringify({verified:true,authorFiles:m.authorFiles.length,externalInputs:m.externalInputs.length,
  canonicalSources:m.canonicalSources.length,entry:m.entry,scope:'byte integrity with explicit scope correction; not independent review'},null,2));
