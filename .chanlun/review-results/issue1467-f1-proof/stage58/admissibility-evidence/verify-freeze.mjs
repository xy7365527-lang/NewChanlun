import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const evidence = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(evidence, '../../../../..');
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const manifest = JSON.parse(fs.readFileSync(path.join(evidence, 'manifest-v1.json'), 'utf8'));
const failures = [];
let checked = 0;
for (const item of [...manifest.packet, ...manifest.dependencies]) {
  const filename = path.join(root, item.path);
  if (!fs.existsSync(filename)) {
    failures.push({ path: item.path, reason: 'missing' });
    continue;
  }
  const bytes = fs.readFileSync(filename);
  if (bytes.length !== item.bytes || hash(bytes) !== item.sha256) {
    failures.push({ path: item.path, reason: 'content differs' });
  }
  checked++;
}
const source = JSON.parse(fs.readFileSync(path.join(evidence, 'source-excerpts-v1.json'), 'utf8'));
let linesChecked = 0;
for (const item of source.sources) {
  const bytes = fs.readFileSync(path.join(root, item.path));
  if (hash(bytes) !== item.sha256) failures.push({ path: item.path, reason: 'source differs' });
  const lines = bytes.toString('utf8').split(/\r?\n/);
  for (const line of item.lines) {
    if (lines[line.line - 1] !== line.text) failures.push({ path: item.path, line: line.line, reason: 'excerpt differs' });
    linesChecked++;
  }
}
console.log(JSON.stringify({ schema: 'stage58-freeze-check-v1', checked, linesChecked, failures, checksIdentityOnly: true }, null, 2));
process.exitCode = failures.length ? 1 : 0;
