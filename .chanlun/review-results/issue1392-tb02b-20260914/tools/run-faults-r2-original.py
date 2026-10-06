"""#1392：复用正式故障驱动，仅选四个事务点的两 arm；输入原件不改。"""
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

WORK = Path('/Users/silencehan/Projects/NewChanlun-1392-tb02b')
sys.path[:0] = [str(WORK / 's_session/tests'), str(WORK / 's_session')]
from tb02b_harness import prepare, dump, first_difference, read_records
from tb02b_independent_harness import cancel_runtime

def main():
    output = Path(sys.argv[1])
    node_root = Path('/Users/silencehan/.cache/codex-runtimes/codex-primary-runtime/dependencies/node')
    binary = WORK / 'rust/target/release/s_structure_session'
    plan = prepare(output, binary, Path(sys.executable), node_root / 'bin/node',
                   node_root / 'node_modules/playwright', 25520)
    runs = [r for r in plan['runs'] if r['case_id'].startswith('revision_')]
    assert len(runs) == 8
    dump(output / 'FAULT-PLAN.json', {**plan, 'runs': runs, 'scope': '仅四个实际事务点的双跑；普通轨迹另见 independent-suite-r2'})
    env = dict(os.environ)
    env['TB02B_CHROMIUM_EXECUTABLE'] = '/Users/silencehan/Library/Caches/ms-playwright/chromium-1228/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing'
    def launch(run):
        directory = Path(run['directory'])
        with (directory / 'driver.stdout').open('xb') as out, (directory / 'driver.stderr').open('xb') as err:
            process = subprocess.Popen(run['command'], stdout=out, stderr=err, env=env, start_new_session=True)
            try:
                process.wait(timeout=300)
                result = {'case_id': run['case_id'], 'arm': run['arm'], 'exit': process.returncode}
            except subprocess.TimeoutExpired:
                result = {'case_id': run['case_id'], 'arm': run['arm'], 'exit': None,
                          'error': 'timeout', 'cleanup': cancel_runtime(process, run, directory)}
        dump(directory / 'EXIT.json', result)
        return result
    outcomes = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        for start in range(0, len(runs), 2):
            batch = list(pool.map(launch, runs[start:start + 2]))
            outcomes.extend(batch)
            if any(r['exit'] != 0 for r in batch):
                dump(output / 'RESULT.json', {'status': 'FAIL_FAULT_RUNTIME', 'runs': outcomes})
                return 1
    comparisons = []
    for left, right in zip(runs[::2], runs[1::2]):
        cores = []
        for run in (left, right):
            ev = Path(run['directory']) / 'evidence'
            result = json.loads((ev / 'RESULT.json').read_text())
            assert result['status'] == 'CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER'
            assert not result['cleanup']['errors']
            assert json.loads((ev / 'browser/RESULT.json').read_text())['status'] == 'passed'
            core = list(read_records(ev / 'semantic-core.jsonl'))
            assert len(core) == 5 * run['input_count'] + len(plan['expected_tables']) + 2
            assert sorted(r['table'] for r in core if r.get('kind') == 'authoritative_table') == plan['expected_tables']
            assert any(r.get('kind') == 'authoritative_schema' for r in core)
            cores.append(core)
        difference = first_difference(*cores)
        entry = {'case_id': left['case_id'], 'difference': difference, 'records': len(cores[0]),
                 'core_sha256': [hashlib.sha256((Path(r['directory']) / 'evidence/semantic-core.jsonl').read_bytes()).hexdigest() for r in (left, right)]}
        dump(output / (left['case_id'] + '-COMPARE.json'), entry)
        assert difference is None
        assert entry['core_sha256'][0] == entry['core_sha256'][1]
        comparisons.append(entry)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == plan['binary_sha256']
    result = {'status': 'PASS_BOUNDED_FOUR_FAULT_BOUNDARIES', 'runs': outcomes, 'comparisons': comparisons,
              'binary_sha256': plan['binary_sha256'], 'scope': '四个具名事务点实际SIGKILL/独立恢复/重投幂等/Q存活/公开旧cut与当前GUI；每点两新进程语义核心全字段相等。',
              'parent_goal_complete': False}
    dump(output / 'RESULT.json', result)
    print(json.dumps(result, ensure_ascii=False))
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
