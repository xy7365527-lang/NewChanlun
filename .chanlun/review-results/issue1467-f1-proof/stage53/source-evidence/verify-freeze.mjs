import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../../../..');
const manifest = JSON.parse(fs.readFileSync(path.join(here, 'frozen-manifest.json'), 'utf8'));
const sha = p => createHash('sha256').update(fs.readFileSync(p)).digest('hex');
for (const item of [...manifest.authorFiles, ...manifest.externalInputs]) {
  const file = path.join(root, item.path);
  assert.equal(sha(file), item.sha256, item.path);
  assert.equal(fs.statSync(file).size, item.bytes, item.path);
}
const excerpts = JSON.parse(fs.readFileSync(path.join(here, 'source-excerpts.json'), 'utf8'));
let lineCount = 0;
for (const source of excerpts.sources) {
  const file = path.join(root, source.path);
  assert.equal(sha(file), source.sha256, source.path);
  const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
  for (const x of source.lines) {
    assert.equal(lines[x.line - 1], x.text, `${source.path}:${x.line}`);
    lineCount++;
  }
}
console.log(JSON.stringify({ status: 'freeze-and-excerpt-check-pass',
  authorFiles: manifest.authorFiles.length, externalInputs: manifest.externalInputs.length,
  sourceFiles: excerpts.sources.length, excerptLines: lineCount,
  independentReview: false }));
