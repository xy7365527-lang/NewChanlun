#!/usr/bin/env python3
"""核对本次比较的冻结输入与既有回执；不声称完整 Lean 环境重查。"""
import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.dont_write_bytecode = True
BASE = Path('/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage67')
OUT = BASE / 'closure-comparison'
AUTHOR = BASE / 'closure-author'
READBACK = BASE / 'closure-readback'
SCRIPTS = Path('/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts')
sys.path.insert(0, str(SCRIPTS))
from lean_evidence import payload_digest

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def load(path):
    return json.loads(Path(path).read_text())

def write(name, data):
    (OUT / name).write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')

expected_source = 'e1c96a05d4f2f5450f9056cafe54254e8a97413083927ba2949ae4d27794c6c4'
expected_semantic = '3e08e1a29c0cad6ba177dfafa96330465a026b161a5040fceb0e5bd007889a21'
expected_environment = '91900355fecfb9b95efe60d4e842536ee784f9ed503b932764c095020573c4a8'
manifest = load(AUTHOR / 'exact/run-manifest.json')
target = manifest['target']
readback_final = load(READBACK / 'FINAL.json')
files = [AUTHOR / p for p in ['ClosureRoots.lean', 'Proof.md', 'contract.json', 'direct.stdout', 'direct.stderr', 'direct-receipt.json', 'author-receipt.json', 'lean-toolchain', 'exact/run-manifest.json']]
files += [READBACK / p for p in ['Readback.md', 'FINAL.json', 'input-hashes.json', 'direct.stdout', 'direct.stderr', 'direct-receipt.json', 'InspectDeclarations.lean', 'inspect.stdout', 'inspect.stderr', 'inspect-receipt.json']]
checks = []

def check(name, result):
    checks.append({'name': name, 'passed': bool(result)})

check('frozen_formal_source', digest(AUTHOR / 'ClosureRoots.lean') == expected_source)
check('recorded_semantic_identity', target['semantic_sha256'] == expected_semantic)
check('recorded_environment_identity', target['environment_sha256'] == expected_environment)
check('manifest_payload_digest', manifest['report_sha256'] == payload_digest(manifest))
run_dir = Path(manifest['evidence']['run_directory'])
check('immutable_per_run_manifest_equal', load(run_dir / 'run-manifest.json') == manifest)
check('contract_equal_to_recorded_contract', load(AUTHOR / 'contract.json') == manifest['evidence']['contract'])
check('core_recorded_exact_root_and_closed', manifest['exact_root_passed'] is True and manifest['root_closure']['status'] == 'closed')
check('core_expected_type_match', target['comparison']['status'] == 'matched' and target['comparison']['definitionally_equal'] is True)
check('core_no_open_dependencies', not target['axioms'] and not target['unexpected_axioms'] and not target['unknown_dependencies'] and not target['unsafe_dependencies'])
for item in load(READBACK / 'input-hashes.json'):
    check('readback_frozen_input:' + item['path'], digest(item['path']) == item['expected_sha256'] == item['actual_sha256'])
for name, expected in readback_final['artifact_sha256'].items():
    check('readback_artifact:' + name, digest(READBACK / name) == expected)
for name in ['direct', 'inspect']:
    receipt = load(READBACK / (name + '-receipt.json'))
    check('readback_exit:' + name, receipt['exit'] == 0)
    check('readback_runtime:' + name, digest(receipt['command'][0]) == receipt['lean_sha256'])
    for channel in ['stdout', 'stderr']:
        check('readback_log:' + name + '.' + channel, digest(READBACK / (name + '.' + channel)) == receipt[channel + '_sha256'])
check('independent_direct_output_equal', (READBACK / 'direct.stdout').read_bytes() == (AUTHOR / 'direct.stdout').read_bytes())
check('inspection_contains_exact_frozen_source', (READBACK / 'InspectDeclarations.lean').read_bytes().startswith((AUTHOR / 'ClosureRoots.lean').read_bytes()))
for name, expected in manifest['evidence']['compiled_artifacts'].items():
    check('recorded_compiled_artifact:' + name, digest(name) == expected)
check('recorded_extraction_file', digest(target['extraction_file']) == manifest['evidence']['extraction_sha256'])
for name, expected in manifest['evidence']['generated_sources'].items():
    check('recorded_generated_source:' + name, digest(run_dir / name) == expected)
for name, expected in manifest['evidence']['tool_hashes'].items():
    check('recorded_verifier_tool:' + name, digest(SCRIPTS / name) == expected)
for name, expected in manifest['environment']['binary_hashes'].items():
    check('recorded_runtime_binary:' + name, digest(Path(manifest['environment']['prefix']) / name) == expected)
for job in manifest['build']['commands']:
    check('recorded_job_exit:' + job['job_id'], job['status'] == 'passed' and job['exit_code'] == 0)
    for channel in ['stdout', 'stderr']:
        check('recorded_job_log:' + job['job_id'] + '.' + channel, digest(job[channel + '_log']) == job[channel + '_sha256'])
write('input-hashes.json', [{'path': str(p), 'sha256': digest(p), 'bytes': p.stat().st_size} for p in files])
receipt = {
    'checked_at_utc': datetime.now(timezone.utc).isoformat(),
    'status': 'passed' if all(c['passed'] for c in checks) else 'failed',
    'script': str(OUT / 'check_inputs.py'),
    'script_sha256': digest(OUT / 'check_inputs.py'),
    'verification_mode': 'existing_evidence_reuse_with_current_packet_and_receipt_hash_checks',
    'compiler_executed_by_comparer': False,
    'full_inventory_recheck_executed': False,
    'module_resolution_rechecked': False,
    'limitation': '未新跑 lean_evidence.py 的完整重查，未重新解析全部导入；该命令会在作者目录写 rechecks，超出本比较者写入职责。复用现有 exact 根及独立盲读回复跑，未修改 machine manifest。',
    'recorded_run_id': manifest['run_id'],
    'recorded_exact_root_passed': manifest['exact_root_passed'],
    'source_sha256': expected_source,
    'semantic_sha256': expected_semantic,
    'environment_sha256': expected_environment,
    'checks': checks,
}
write('receipt.json', receipt)
print(json.dumps({'status': receipt['status'], 'checks': len(checks), 'failed': [c['name'] for c in checks if not c['passed']]}, ensure_ascii=False))
raise SystemExit(0 if receipt['status'] == 'passed' else 1)
