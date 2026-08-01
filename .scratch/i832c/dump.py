import json, sys, re
recs = json.load(open('.scratch/i832c/recs.json'))
zone = sys.argv[1]
pat = sys.argv[2] if len(sys.argv) > 2 else None
rx = re.compile(pat) if pat else None
for r in recs:
    if r['z'] != zone:
        continue
    t = r['t'].strip()
    if not t or t.startswith('!['):
        continue
    if rx and not rx.search(t):
        continue
    print(f"{r['f']}:{r['n']}\t{t}")
