#!/usr/bin/env python3
"""Final no-fit integrity check and review manifest creation."""
import ast
import datetime
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parent
for p in root.glob('*.py'):
    ast.parse(p.read_text(), filename=str(p))
before = json.loads((root/'input-hashes-before.json').read_text())
after = []
for entry in before:
    p = Path(entry['path'])
    blob = p.read_bytes()
    after.append({'path': str(p), 'bytes': len(blob),
                  'sha256': hashlib.sha256(blob).hexdigest()})
assert before == after
assert json.loads((root/'input-hashes-after.json').read_text()) == before
for p in (root/'__pycache__').glob('*.pyc'):
    p.unlink()
if (root/'__pycache__').exists():
    (root/'__pycache__').rmdir()
files = []
for p in sorted(root.rglob('*')):
    if p.is_file() and p.name not in ['manifest.json', 'manifest.sha256']:
        blob = p.read_bytes()
        files.append({'name': str(p.relative_to(root)), 'bytes': len(blob),
                      'sha256': hashlib.sha256(blob).hexdigest()})
total = sum(f['bytes'] for f in files)
assert total < 128*1024*1024
manifest = {'profile': 'stage69-independent-learning-review/1',
    'created_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'verdict': 'accepted-with-explicit-boundaries',
    'author_manifest_sha256': '903a7e367e4e55c16ecb6709b17e41316b5edf237308eb2eb2ea3fc237b4ed7c',
    'author_final_sha256': '2b304dcf42f295507b5c26918a520af592d522341ce23b01146ef3172fefe15a',
    'independent_full_refits': 2, 'additional_successful_author_fits': 0,
    'market_decisions': 572, 'negative_process_controls': 16,
    'input_files_verified_unchanged': len(before),
    'bytes_excluding_manifest_and_checksum': total, 'files': files}
(root/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
pin = hashlib.sha256((root/'manifest.json').read_bytes()).hexdigest()
(root/'manifest.sha256').write_text(pin+'\n')
print(json.dumps({'status': 'passed', 'manifest_sha256': pin,
    'FINAL_sha256': hashlib.sha256((root/'FINAL.md').read_bytes()).hexdigest(),
    'files': len(files), 'bytes': total, 'unchanged_inputs': len(before)}))
