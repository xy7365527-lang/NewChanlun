#!/usr/bin/env python3
"""#1394：现成正式二进制、同源 S16、无新输入的 advance 对照。"""
from pathlib import Path
import hashlib
import json
import sqlite3
import subprocess
import time
import shutil

HERE = Path(__file__).resolve().parent
ROOT = Path('/Users/silencehan/Projects/NewChanlun-1394-content-v3')
CANDIDATE = Path('/Users/silencehan/Projects/NewChanlun-1394-same-material-20261001')
SOURCE = Path('/Users/silencehan/Projects/NewChanlun-1394-audit-shadow-20261001/.scratch/issue1394-audit-readonly-stage-probe-20261001/source-A')
BASE_BINARY = ROOT / '.scratch/issue1394-formal-fullclock-prefix16-20261001/release1/s_structure_session-release-ed1f184'
CANDIDATE_BINARY = CANDIDATE / '.scratch/issue1394-same-material-20261001/target/release/s_structure_session'
PROFILE = ROOT / 's_session/profiles/ohlc_integer_tb02a_v1.json'
CLOCK = ROOT / '.scratch/issue1394-full-history-formal-prep-20261001/inputs/clock-plan.full9011.draft.json'
EXPECTED = ROOT / '.scratch/issue1394-formal-fullclock-prefix16-20261001/release1/A/final.fullDB.json'
BINARIES = {
    'baseline': (BASE_BINARY, 'b4c81c9468be1d1911eecbd48deb1be9e9a2f3a0d082fd8f7474cf9e30efc7f5'),
    'candidate': (CANDIDATE_BINARY, 'c098bf76bb7870443b7564714ba2f9110f2c5593a3180ca13f9888c01ee16d17'),
}

def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def save(path, value):
    with path.open('x') as stream:
        stream.write(canonical(value) + '\n')

def identity(path):
    info = path.stat()
    return {'path': str(path), 'bytes': info.st_size, 'inode': info.st_ino,
            'mtime_ns': info.st_mtime_ns, 'sha256': sha(path.read_bytes())}

def source_identity():
    return [identity(SOURCE / name) for name in ('s.sqlite', 's.sqlite-wal', 's.sqlite-shm')]

def logical_dump(db, properties):
    connection = sqlite3.connect(db.as_uri() + '?mode=ro', uri=True)
    try:
        connection.execute('PRAGMA query_only=ON')
        schema = [list(row) for row in connection.execute('SELECT type,name,tbl_name,rootpage,sql FROM sqlite_schema ORDER BY type,name')]
        tables = {}
        for (name,) in connection.execute("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"):
            quoted = '"' + name.replace('"', '""') + '"'
            columns = [list(row) for row in connection.execute('PRAGMA table_xinfo(' + quoted + ')')]
            rows = [[{'blob_hex': value.hex()} if isinstance(value, bytes) else value for value in row]
                    for row in connection.execute('SELECT * FROM ' + quoted)]
            rows.sort(key=canonical)
            tables[name] = {'columns': columns, 'row_count': len(rows), 'rows': rows}
        values = {key: connection.execute('PRAGMA ' + key).fetchone()[0] for key in properties}
        return {'schema': schema, 'db_properties': values, 'tables': tables}
    finally:
        connection.close()

def compare(expected, actual):
    result = {'schema_equal': canonical(expected['schema']) == canonical(actual['schema']),
              'db_properties_equal': canonical(expected['db_properties']) == canonical(actual['db_properties']),
              'table_names_equal': set(expected['tables']) == set(actual['tables']), 'tables': {}}
    for name, table in expected['tables'].items():
        other = actual['tables'].get(name)
        if other is None:
            result['tables'][name] = {'pass': False, 'missing': True}
            continue
        rows = sorted(table['rows'], key=canonical)
        all_equal = canonical(rows) == canonical(other['rows'])
        columns_equal = canonical(table['columns']) == canonical(other['columns'])
        row_count_equal = canonical(table['row_count']) == canonical(other['row_count'])
        result['tables'][name] = {
            'columns_equal': columns_equal,
            'row_count_equal': row_count_equal,
            'all_fields_and_blob_equal': all_equal, 'row_count': len(rows),
            'blob_cells': sum(isinstance(value, dict) and 'blob_hex' in value for row in rows for value in row),
            'logical_sha256': sha(canonical(other['rows']).encode()),
            'pass': columns_equal and row_count_equal and all_equal,
        }
    result['pass'] = (result['schema_equal'] and result['db_properties_equal'] and result['table_names_equal']
                      and all(table['pass'] for table in result['tables'].values()))
    return result

def pending_count(db):
    connection = sqlite3.connect(db.as_uri() + '?mode=ro', uri=True)
    try:
        connection.execute('PRAGMA query_only=ON')
        return connection.execute("SELECT COUNT(*) FROM s_input_messages WHERE status='AcceptedPending'").fetchone()[0]
    finally:
        connection.close()

def run_case(label, kind, expected, originals):
    case = HERE / label
    case.mkdir()
    copies = []
    for original in originals:
        origin = Path(original['path'])
        target = case / origin.name
        subprocess.run(['cp', '-c', '-p', str(origin), str(target)], check=True)
        bound = identity(target)
        assert bound['sha256'] == original['sha256'] and bound['bytes'] == original['bytes']
        assert bound['inode'] != original['inode']
        copies.append(bound)
    db = case / 's.sqlite'
    before = logical_dump(db, expected['db_properties'])
    before_check = compare(expected, before)
    assert before_check['pass'], before_check
    assert pending_count(db) == 0, '禁止带 pending 的恢复对照'
    binary, digest = BINARIES[kind]
    assert identity(binary)['sha256'] == digest
    command = [str(binary), 'advance', '--protocol-revision', 's-session/3', '--db', str(db),
               '--profile', str(PROFILE), '--writer-epoch', '1', '--clock-plan', str(CLOCK)]
    save(case / 'PLAN.json', {'kind': kind, 'command': command, 'copies_before_sqlite': copies,
                            'binary_sha256': digest, 'before_full_compare': before_check,
                            'scope': 'no input, no pending; existing non-test CLI open+summary audits; no service resource acceptance'})
    with (case / 'stdout.json').open('xb') as stdout, (case / 'stderr.txt').open('xb') as stderr:
        started = time.monotonic_ns()
        child = subprocess.Popen(command, stdout=stdout, stderr=stderr)
        print(json.dumps({'case': label, 'pid': child.pid, 'stage': 'RUNNING', 'free_bytes': shutil.disk_usage(HERE).free}), flush=True)
        samples = []
        try:
            while child.poll() is None:
                sample = subprocess.run(['ps', '-o', 'rss=', '-p', str(child.pid)], capture_output=True, text=True, check=False)
                if sample.stdout.strip():
                    samples.append({'elapsed_ns': time.monotonic_ns() - started,
                                    'rss_bytes': int(sample.stdout.strip()) * 1024,
                                    'free_bytes': shutil.disk_usage(HERE).free})
                time.sleep(.05)
            exit_code = child.wait()
            elapsed_ns = time.monotonic_ns() - started
        finally:
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
    after = logical_dump(db, expected['db_properties'])
    after_check = compare(expected, after)
    reply = json.loads((case / 'stdout.json').read_bytes()) if exit_code == 0 else None
    assert source_identity() == originals, '旧 owned 原件变动'
    result = {'label': label, 'kind': kind, 'pid': child.pid, 'exit_code': exit_code,
              'process_wall_ns_including_poll_resolution': elapsed_ns, 'stdout': identity(case / 'stdout.json'),
              'stderr': identity(case / 'stderr.txt'), 'reply': reply,
              'after_full_compare': after_check, 'pending_after': pending_count(db),
              'rss_sampling_target_ns': 50_000_000, 'samples': len(samples),
              'rss_sampled_peak_bytes': max((sample['rss_bytes'] for sample in samples), default=None),
              'sampled_peak_is_lower_bound': True, 'heap_peak': 'UNMEASURED',
              'counter_hash_sql_stages': 'UNMEASURED_NON_TEST_BINARY',
              'child_terminal_reaped': child.poll() is not None,
              'source_originals_unchanged': True}
    save(case / 'SAMPLES.json', samples)
    save(case / 'RESULT.json', result)
    assert exit_code == 0 and after_check['pass'] and result['pending_after'] == 0, result
    assert reply['ok'] is True and reply['generation'] == '16' and reply['accepted_pending'] == '0', reply
    print(json.dumps({'case': label, 'exit': exit_code, 'wall_seconds': elapsed_ns / 1e9,
                      'rss_sampled_peak_bytes': result['rss_sampled_peak_bytes']}), flush=True)
    return result

def main():
    expected = json.loads(EXPECTED.read_bytes())
    originals = source_identity()
    for binary, digest in BINARIES.values():
        assert identity(binary)['sha256'] == digest
    assert identity(CLOCK)['sha256'] == 'b02ac2444161e1e7e327fcf2a2bb79b294356a3dd95a59a6404c5b92deb04f01'
    bindings = [identity(path) for path in (BASE_BINARY, CANDIDATE_BINARY, PROFILE, CLOCK, EXPECTED)]
    save(HERE / 'BINDING-BEFORE.json', {'source_owned_originals': originals, 'inputs': bindings,
                                     'observer': identity(Path(__file__)), 'no_build': True})
    results = []
    for label, kind in [('A-baseline', 'baseline'), ('B-candidate', 'candidate'),
                        ('C-candidate', 'candidate'), ('D-baseline', 'baseline')]:
        results.append(run_case(label, kind, expected, originals))
    assert all(canonical(result['reply']) == canonical(results[0]['reply']) for result in results), 'CLI 全回复字段或JSON类型不同'
    assert all((HERE / result['label'] / 'stdout.json').read_bytes() ==
               (HERE / results[0]['label'] / 'stdout.json').read_bytes() for result in results), 'CLI 原生完整回复字节不同'
    assert [identity(path) for path in (BASE_BINARY, CANDIDATE_BINARY, PROFILE, CLOCK, EXPECTED)] == bindings
    save(HERE / 'RESULT.json', {'status': 'PASS_BOUNDED_NON_TEST_NO_INPUT_ABBA', 'results': results,
                              'all_reply_fields_equal': True, 'all_full_db_rows_and_blobs_equal': True,
                              'inputs_and_originals_unchanged': True, 'all_children_terminal_reaped': True,
                              'build': 'NOT_NEEDED_EXISTING_BINARIES', 'full9011': False,
                              'resource_gates': 'NOT_SERVICE_DOMAIN', 'instrumented_counter_pilot': 'NOT_RUN',
                              'scope': 'S16 complete clone, no-input existing advance CLI; all9011 clock loaded, only16 committed'})

if __name__ == '__main__':
    main()
