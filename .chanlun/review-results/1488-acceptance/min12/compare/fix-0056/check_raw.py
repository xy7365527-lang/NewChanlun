"""s2-fix-0056/F1 补证：旧 raw_position 与新 raw_first/raw_last 按 (family:kind, 路径) 多重集比对，并核 raw_first==raw_last。
用法：python3 check_raw.py [<含 k15.json k31.json g-0016.json g-0032.json 的目录，默认 /tmp/rv0055>]"""
import collections, json, os, sys

D = sys.argv[1] if len(sys.argv) > 1 else '/tmp/rv0055'


def walk(o, p=''):
    if isinstance(o, dict):
        for k, v in o.items():
            if k in ('raw_position', 'raw_first', 'raw_last'):
                yield p, k, json.dumps(v, sort_keys=True)
            else:
                yield from walk(v, p + '.' + k)
    elif isinstance(o, list):
        for v in o:
            yield from walk(v, p + '[]')


def load(path):
    d = json.load(open(path))
    return (d['projection'] if 'projection' in d else d)['rows']


res = {}
for n, (o, g) in {'15': ('k15.json', 'g-0016.json'), '31': ('k31.json', 'g-0032.json')}.items():
    old, new = collections.Counter(), collections.Counter()
    eq = bad = 0
    for r in load(os.path.join(D, o)):
        for p, k, v in walk(r['record']):
            old[(r['family'] + ':' + str(r['record'].get('kind')), p, k, v)] += 1
    for r in load(os.path.join(D, g)):
        rec = r['record']
        seen = {}
        for p, k, v in walk(rec):
            seen.setdefault(p, {})[k] = v
        for p, ks in seen.items():
            if 'raw_first' in ks:
                eq += ks['raw_first'] == ks.get('raw_last'); bad += ks['raw_first'] != ks.get('raw_last')
                new[(r['family'] + ':' + str(rec.get('kind')), p, 'raw_position', ks['raw_first'])] += 1
            if 'raw_position' in ks:
                new[(r['family'] + ':' + str(rec.get('kind')), p, 'raw_position', ks['raw_position'])] += 1
    res['known' + n] = {'old_raw_position_coords': sum(old.values()), 'new_coord_points': sum(new.values()),
                        'multiset_equal': old == new, 'raw_first_eq_raw_last': eq, 'raw_first_ne_raw_last': bad,
                        'old_uses_raw_first': any(k[2] != 'raw_position' for k in old)}
print(json.dumps(res, indent=1))
