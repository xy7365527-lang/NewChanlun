import collections
import json
from pathlib import Path
import sqlite3

A = Path(__file__).parent / 'artifact'
def load_rows(name):
    return [json.loads(line) for line in (A / name).read_text().splitlines()]
originals = load_rows('originals.jsonl')
resends = load_rows('resends.jsonl')
storage = load_rows('storage.jsonl')
result = json.loads((A / 'RESULT.json').read_text())
assert len(originals) == len(resends) == 21
assert len({row['request']['message_id'] for row in originals}) == 21
assert collections.Counter(row['request']['payload']['op'] for row in originals) == {'CaptureExternal': 6, 'ResolveEvidence': 3, 'ApplyAllocation': 12}
expected_kind = {'CaptureExternal': 'Recorded', 'ResolveEvidence': 'Resolved', 'ApplyAllocation': 'Applied'}
for original, resend in zip(originals, resends):
    assert original['owner'] == resend['owner']
    assert original['request'] == resend['request']
    assert original['response']['payload']['kind'] == expected_kind[original['request']['payload']['op']]
    x, y = dict(original['response']), dict(resend['response'])
    for field in ['producer_epoch', 'source_epoch']:
        assert x.pop(field) == '1' and y.pop(field) == '2'
    assert x == y
assert result['next'] == 22
assert result['sequences'] == [10, 4, 4, 4]
assert result['epochs'] == [2, 2, 2, 2]
assert result['recovery_and_resend_invariants'] == 'passed'
assert len(storage) == 75
phases = collections.Counter(row['phase'] for row in storage)
assert phases == {'initialized': 4, 'original': 21, 'before_recover': 4, 'after_recover': 4, 'before_resend': 21, 'after_resend': 21}
by_key = {(row['owner'], row['phase'], row['message_id']): row for row in storage}
for original in originals:
    key = original['request']['message_id']
    before = next(row for row in storage if row['phase'] == 'before_resend' and row['message_id'] == key)
    after = by_key[(before['owner'], 'after_resend', key)]
    assert before['seq'] == after['seq'] and before['writer_epoch'] == after['writer_epoch']
    assert {k: v for k, v in before['storage'].items() if k != 'connection_max_page_count'} == {k: v for k, v in after['storage'].items() if k != 'connection_max_page_count'}
    assert after['storage']['connection_max_page_count'] == 1016
summary = {'responses': dict(collections.Counter(row['response']['payload']['kind'] for row in originals)), 'resend_pairs_exact_except_epochs': 21, 'storage_phases': dict(phases), 'owners': {}}
for owner in ['E', 'B:chong-N1', 'B:chong-N2', 'B:chong-N3']:
    rows = [row for row in storage if row['owner'] == owner]
    before, after = by_key[(owner, 'before_recover', '')], by_key[(owner, 'after_recover', '')]
    assert before['seq'] == after['seq']
    assert before['writer_epoch'] == 1 and after['writer_epoch'] == 2
    assert before['storage']['current_image_bytes'] == after['storage']['current_image_bytes']
    for row in rows:
        s = row['storage']
        assert s['capacity_bytes'] == 33513472 and s['carrier_page_limit'] == 1016
        assert s['page_count'] <= 1016
        assert s['cleanup_margin_bytes'] == s['free_bytes'] - s['cleanup_needed_bytes'] > 0
        assert s['next_write_margin_bytes'] == s['free_bytes'] - s['next_write_needed_bytes'] > 0
        assert s['allocated'] == sum(f['allocated_bytes'] for f in s['files'])
    summary['owners'][owner] = {'max_observed_page_count': max(r['storage']['page_count'] for r in rows), 'min_cleanup_margin': min(r['storage']['cleanup_margin_bytes'] for r in rows), 'max_allocated': max(r['storage']['allocated'] for r in rows), 'final_image_bytes': after['storage']['current_image_bytes'], 'connection_max_page_counts': sorted({r['storage']['connection_max_page_count'] for r in rows})}
hand = json.loads((A / 'HAND-ORACLE.json').read_text())
books = []
for index, dirname in enumerate(['e', 'b0', 'b1', 'b2']):
    path = A / 'carriers' / dirname / 'owner.db'
    assert path.is_file() and not path.with_name('owner.db-wal').exists()
    with sqlite3.connect(f'file:{path}?mode=ro&immutable=1', uri=True) as db:
        meta = dict(db.execute('select k,v from meta'))
        domain = meta['domain_id']
        assert meta['writer_epoch'] == '2'
        counts = {t: db.execute(f'select count(*) from {t}').fetchone()[0] for t in ['commits', 'messages', 'receipts', 'applications', 'content_staging']}
        assert counts == dict(zip(['commits', 'messages', 'receipts', 'applications', 'content_staging'], [11, 10, 10, 0, 0] if index == 0 else [5, 4, 4, 4, 0]))
        seq, image = db.execute('select seq,image from commits order by seq desc limit 1').fetchone()
        assert seq == result['sequences'][index]
        assert len(image.encode()) == summary['owners'][domain]['final_image_bytes']
        messages = {json.loads(row[3])['message_id']: row for row in db.execute('select k,payload_hash,receipt_key,request_json from messages')}
        for original in [row for row in originals if row['owner'] == index]:
            request = original['request']
            message = messages[request['message_id']]
            assert json.loads(message[3]) == request and message[1] == request['payload_hash']
            receipt = json.loads(db.execute('select json from receipts where k=?', (message[2],)).fetchone()[0])
            assert receipt == original['response']['payload']
        summary['owners'][domain]['db_counts'] = counts
        if index:
            book = json.loads(db.execute('select json from book').fetchone()[0])
            assert book == result['final_books'][index - 1]
            assert int(book['actual_quantity']) == hand['reports'][-1]['book_quantity'][index - 1]
            assert int(book['cost_basis']['K']) == hand['reports'][-1]['K'][index - 1]
            books.append([book['actual_quantity'], book['cost_basis']['K']])
    if index:
        with sqlite3.connect(f'file:{path}?mode=ro&immutable=1', uri=True) as db:
            histories = [json.loads(x[0])['book'] for x in db.execute('select image from commits where seq>=2 order by seq')]
        assert len(histories) == 3
        for report, book in zip(hand['reports'], histories):
            assert int(book['actual_quantity']) == report['book_quantity'][index - 1]
            assert int(book['cost_basis']['K']) == report['K'][index - 1]
cleanup = dict(line.split('=', 1) for line in (A / 'cleanup.txt').read_text().splitlines())
assert cleanup['command_exit'] == cleanup['cleanup_failed'] == '0'
assert cleanup['root_exists'] == 'no'
log = (A / 'cargo-test.log').read_text()
assert 'fin2_capacity_shared_carrier_and_non_hfs_are_rejected ... ok' in log
assert 'test result: ok. 2 passed; 0 failed; 0 ignored;' in log
assert 'finished in 20.75s' in log
recoveries = load_rows('recoveries.jsonl')
assert len(recoveries) == 4
assert all(row['kind'] == 'Recovered' and row['writer_epoch'] == '2' for row in recoveries)
assert len({row['runtime']['filesystem'].split('source=')[1] for row in recoveries}) == 4
summary['final_books_q_K'] = books
summary['cleanup'] = cleanup
summary['two_ignored_tests'] = 'actual 2 passed, 0 failed, 0 ignored, 20.75s'
summary['limits'] = ['Recovery byte equality and full frozen table comparisons are execution assertions in the passed test; only the final DB snapshot is archived.']
print(json.dumps(summary, ensure_ascii=False, indent=2))
