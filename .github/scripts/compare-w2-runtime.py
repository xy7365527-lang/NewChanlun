"""Compare all W2 SQLite state and public roots without comparing page layout."""

import itertools
import json
from pathlib import Path
import sqlite3
import sys


def load(path):
    return json.loads(path.read_text())


def quoted(name):
    return '"' + name.replace('"', '""') + '"'


def compare_databases(left, right):
    with sqlite3.connect(left.as_uri() + '?mode=ro', uri=True) as a, sqlite3.connect(right.as_uri() + '?mode=ro', uri=True) as b:
        schema = "SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name"
        assert a.execute(schema).fetchall() == b.execute(schema).fetchall(), 'SQLite schema differs'
        tables = [row[0] for row in a.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")]
        counts = {}
        for table in tables:
            columns = [row[1] for row in a.execute(f'PRAGMA table_xinfo({quoted(table)})')]
            order = ','.join(quoted(column) for column in columns)
            query = f'SELECT * FROM {quoted(table)} ORDER BY {order}'
            sentinel = object()
            count = 0
            for index, (row_a, row_b) in enumerate(itertools.zip_longest(a.execute(query), b.execute(query), fillvalue=sentinel)):
                assert row_a == row_b, f'SQLite table {table} differs at canonical row {index}'
                count += 1
            counts[table] = count
        return counts


def compare(root):
    manifests = [load(root / side / 'w2.manifest.json') for side in ('a', 'b')]
    for side, manifest in zip(('a', 'b'), manifests):
        assert manifest['status'] == 'complete', f'{side} audit did not complete'
        assert manifest['publication_generation'] == '106', f'{side} is not the full input'
        assert manifest['head'] == (root / 'source-head.txt').read_text().strip(), f'{side} source mismatch'
        compare_databases(root / side / 'w2.run' / 's.sqlite', root / side / 'w2.sqlite')
    for key in ('head', 'rule_revision', 'publication_generation', 'session_generation', 'sqlite_tables', 'public_generations', 'steps'):
        assert manifests[0][key] == manifests[1][key], f'manifest {key} differs'
    assert load(root / 'a' / 'w2.json') == load(root / 'b' / 'w2.json'), 'public Q or roots differ'
    assert load(root / 'a' / 'w2.clock.json') == load(root / 'b' / 'w2.clock.json'), 'clock plans differ'
    tables = compare_databases(root / 'a' / 'w2.sqlite', root / 'b' / 'w2.sqlite')
    result = {'status': 'complete', 'head': manifests[0]['head'], 'publication_generations': 106, 'canonical_sqlite_tables': tables, 'public_q_equal': True, 'clock_equal': True}
    (root / 'comparison.json').write_text(json.dumps(result, sort_keys=True) + '\n')
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    compare(Path(sys.argv[1]).resolve())
