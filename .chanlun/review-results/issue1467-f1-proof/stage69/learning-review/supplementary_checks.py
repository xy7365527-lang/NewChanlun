#!/usr/bin/env python3
"""No-fit supplementary checks and final input immutability verification."""
import importlib.util
import json
import pathlib
import platform
import subprocess
import sys

sys.dont_write_bytecode = True
OUT = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('independent', OUT/'independent_audit.py')
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)
ev = audit.read(audit.E/'stage56/consumer-author/run-v4/evaluation-only.json')
groups = {part: {e['event_group'] for e in ev if e['partition'] == part and e['loss_eligible'] and e['event_group'] is not None}
          for part in ['early', 'middle', 'late']}
result_path = OUT/'independent-results.json'
original_bytes = result_path.read_bytes()
independent = json.loads(original_bytes)
old_dependence = independent['label_dependence']
new_dependence = {part: {
    'decisions': sum(e['partition'] == part and e['loss_eligible'] for e in ev),
    'unique_nonnull_event_groups': len(groups[part]),
    'null_event_group_rows': sum(e['partition'] == part and e['loss_eligible'] and e['event_group'] is None for e in ev)}
    for part in ['early', 'middle', 'late']}
if old_dependence != new_dependence:
    (OUT/'independent-results-before-null-group-correction.json').write_bytes(original_bytes)
    independent['label_dependence'] = new_dependence
    audit.write('independent-results.json', independent)
    audit.write('null-group-count-correction.json', {
        'reason': 'Null event_group is not a witness; exclude it from group identities.',
        'before': old_dependence, 'after': new_dependence,
        'initial_result_sha256': audit.sha(original_bytes),
        'corrected_result_sha256': audit.sha(result_path.read_bytes()),
        'new_fits': 0,
        'numeric_fit_predictions_and_metrics_unchanged': True})
prep = audit.preparation([[None, 2.0], [None, 2.0]], ['empty', 'constant'])
assert [p['mean'] for p in prep] == [0, 2]
assert [p['scale'] for p in prep] == [1, 1]
assert audit.design([None, 2], prep) == [1, 0, 0, 1, 0]
counts = {}
for arm in ['A', 'B']:
    counts[arm] = len(audit.lines(audit.RUN/f'{arm}-predictions.jsonl'))
node = audit.read(audit.RUN/'process-receipts.json')[0]['command'][0]
before = audit.read(OUT/'input-hashes-before.json')
after = audit.snapshot([pathlib.Path(f['path']) for f in before])
assert after == before
audit.write('input-hashes-after.json', after)
result = {'status': 'passed', 'new_fits': 0, 'inputs_unchanged': True,
    'python_executable': sys.executable, 'python_version': platform.python_version(),
    'node_executable': node,
    'node_version': subprocess.check_output([node, '--version'], text=True).strip(),
    'platform': platform.platform(),
    'all_missing_constant_preprocessing': prep,
    'cross_partition_shared_groups': {a+'-'+b: sorted(groups[a] & groups[b])
        for a, b in [('early', 'middle'), ('middle', 'late'), ('early', 'late')]},
    'unique_event_groups_all_eligible': len(set.union(*groups.values())),
    'saved_market_decisions_per_arm': counts,
    'note': 'Unique witness groups are not an effective independent sample size.'}
audit.write('supplementary-results.json', result)
print(json.dumps(result, indent=2))
