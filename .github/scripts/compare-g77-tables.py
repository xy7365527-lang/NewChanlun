import json
from itertools import zip_longest
from pathlib import Path
import sqlite3
import sys


def quoted(name):
    return '"' + name.replace('"', '""') + '"'


def open_readonly(path):
    return sqlite3.connect(Path(path).resolve().as_uri() + '?mode=ro', uri=True)


a, b = [open_readonly(path) for path in sys.argv[1:3]]
schema_sql = 'SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name'
assert a.execute(schema_sql).fetchall() == b.execute(schema_sql).fetchall(), 'schema mismatch'
tables = a.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").fetchall()
results = []
sentinel = object()
for (name,) in tables:
    columns = [row[1] for row in a.execute('PRAGMA table_info(' + quoted(name) + ')')]
    fields = ','.join(quoted(c) for c in columns)
    types = ','.join('typeof(' + quoted(c) + ')' for c in columns)
    order = ','.join(str(i + 1) for i in range(len(columns) * 2))
    query = 'SELECT ' + fields + ',' + types + ' FROM ' + quoted(name) + ' ORDER BY ' + order
    count = 0
    for left, right in zip_longest(a.execute(query), b.execute(query), fillvalue=sentinel):
        assert left == right, (name, count, 'typed row mismatch')
        count += 1
    results.append({'table': name, 'rows': count, 'equal': True})
print(json.dumps({'schema_equal': True, 'tables': results, 'all_equal': True}, indent=2))
a.close()
b.close()
