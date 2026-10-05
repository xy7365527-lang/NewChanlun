import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../../../..');
const m = JSON.parse(fs.readFileSync(path.join(here, 'manifest-v1.json'), 'utf8'));
const sha = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
for (const e of [...m.artifacts, ...m.inputs]) {
  const file = path.resolve(root, e.path);
  assert.equal(fs.statSync(file).size, e.bytes, `size: ${e.path}`);
  assert.equal(sha(file), e.sha256, `hash: ${e.path}`);
}
const sources = JSON.parse(fs.readFileSync(path.join(here, 'source-evidence.json'), 'utf8'));
for (const e of sources) {
  const lines = fs.readFileSync(path.resolve(root, e.path), 'utf8').split('\n');
  assert.equal(sha(path.resolve(root, e.path)), e.sha256);
  for (const line of e.lines) assert.equal(lines[line.n - 1], line.text, `${e.path}:${line.n}`);
}
console.log(JSON.stringify({ status: 'PASS', artifacts: m.artifacts.length, inputs: m.inputs.length,
  source_files: sources.length, source_lines: sources.reduce((n, s) => n + s.lines.length, 0),
  scope: 'content identity and source capture only; not independent semantic review' }, null, 2));
