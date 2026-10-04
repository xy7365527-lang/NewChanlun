// Read-only verification of the frozen Stage48 author packet.
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const here=dirname(fileURLToPath(import.meta.url));
const root=resolve(here,'../../../../..');
const m=JSON.parse(readFileSync(resolve(here,'frozen-manifest.json'),'utf8'));
for(const entry of [...m.packetFiles,...m.sourceFiles]) {
  const bytes=readFileSync(resolve(root,entry.path));
  const hash=createHash('sha256').update(bytes).digest('hex');
  assert.equal(hash,entry.sha256,entry.path);
}
const identity=createHash('sha256').update(JSON.stringify(m.packetFiles)).digest('hex');
assert.equal(identity,m.packetIdentity);
console.log(JSON.stringify({status:'hashes-match',packetIdentity:identity,packetFiles:m.packetFiles.length,sourceFiles:m.sourceFiles.length}));
