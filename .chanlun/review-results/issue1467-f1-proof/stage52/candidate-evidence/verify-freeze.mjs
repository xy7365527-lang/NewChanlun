import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)),P=path.resolve(here,'../..'),R=path.resolve(P,'../../..');
const manifest=JSON.parse(fs.readFileSync(path.join(here,'frozen-manifest.json'),'utf8'));
const hash=f=>crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
for(const e of manifest.authorFiles)assert.equal(hash(path.join(P,e.path)),e.sha256,e.path);
for(const e of manifest.externalInputs)assert.equal(hash(path.join(P,e.path)),e.sha256,e.path);
for(const e of manifest.canonicalSources)assert.equal(hash(path.join(R,e.path)),e.sha256,e.path);
console.log(JSON.stringify({verified:true,authorFiles:manifest.authorFiles.length,externalInputs:manifest.externalInputs.length,
  canonicalSources:manifest.canonicalSources.length,scope:'byte integrity only; not an independent semantic review'},null,2));
