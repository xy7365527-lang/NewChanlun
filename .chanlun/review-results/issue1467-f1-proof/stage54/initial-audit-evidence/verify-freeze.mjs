import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { resolve, dirname } from 'node:path';
import assert from 'node:assert/strict';

const evidence = dirname(fileURLToPath(import.meta.url));
const root = resolve(evidence, '../../../../..');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const lock = JSON.parse(readFileSync(resolve(evidence, 'source-and-contract-lock.json')));
let checkedLines = 0;
for (const entry of lock.files) {
  const bytes = readFileSync(resolve(root, entry.path));
  assert.equal(sha(bytes), entry.sha256, entry.path);
  assert.equal(bytes.length, entry.bytes, entry.path);
  const lines = bytes.toString('utf8').split(/\r?\n/);
  for (const range of entry.ranges ?? []) {
    assert.equal(range.lines.length, range.to - range.from + 1);
    for (const line of range.lines) {
      assert.equal(lines[line.line - 1], line.text, `${entry.path}:${line.line}`);
      checkedLines++;
    }
  }
}
const first = JSON.parse(readFileSync(resolve(evidence, 'source-readback-freeze-v1.json')));
for (const [path, expected] of Object.entries(first.files)) {
  assert.equal(sha(readFileSync(resolve(root, path))), expected, path);
}
const final = JSON.parse(readFileSync(resolve(evidence, 'frozen-author-manifest-v1.json')));
for (const entry of final.files) {
  assert.equal(sha(readFileSync(resolve(root, entry.path))), entry.sha256, entry.path);
}
assert.equal(lock.authorship_suspension.used_as_proof, false);
console.log(JSON.stringify({
  status: 'byte_and_line_checks_passed',
  sourceAndContractFiles: lock.files.length,
  excerptLines: checkedLines,
  firstReadbackFiles: Object.keys(first.files).length,
  authorPacketFiles: final.files.length,
  semanticVerification: 'not_performed_by_this_script',
  independentReview: 'pending'
}, null, 2));
