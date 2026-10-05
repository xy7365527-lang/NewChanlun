import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../../../../..');
const manifest = JSON.parse(readFileSync(resolve(here, 'manifest-v1.json'), 'utf8'));
const checked = [];
for (const entry of manifest.files) {
  const bytes = readFileSync(resolve(root, entry.path));
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  if (sha256 !== entry.sha256 || bytes.length !== entry.bytes) throw new Error(`changed: ${entry.path}`);
  checked.push(entry.path);
}
console.log(JSON.stringify({status: 'FROZEN_BYTES_MATCH', count: checked.length,
  meaning: 'byte identity only; no independent review or mathematical qualification'}, null, 2));
