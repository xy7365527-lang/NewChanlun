import glob, os, re, json, sys
from collections import Counter

FILES = sorted(glob.glob('docs/chanlun/text/blog/*.md'))
BARE_TS = re.compile(r'^\s*\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\s*$')
DP_HDR = re.compile(r'^\s*每日解盘')
QA_START = re.compile(r'^\s*(\[匿名\]|缠中说禅：\s*$|\[.{0,12}\]\s*$)')

recs = []
for f in FILES:
    if f.endswith('INDEX.md'):
        continue
    base = os.path.basename(f)
    L = open(f).read().split('\n')
    idx = [i for i, l in enumerate(L) if '↑正文' in l]
    start = idx[-1] + 1 if idx else None
    zone = '正文'
    for i, l in enumerate(L):
        if start is not None and i == start:
            zone = '答疑'
        if start is None or i >= start:
            if DP_HDR.match(l):
                zone = '每日解盘'
            elif zone == '每日解盘' and (QA_START.match(l) or BARE_TS.match(l)):
                zone = '答疑'
        recs.append({'f': base, 'n': i + 1, 'z': zone, 't': l})

json.dump(recs, open('.scratch/i832c/recs.json', 'w'), ensure_ascii=False)
print(Counter(r['z'] for r in recs))
print('非空行:', Counter(r['z'] for r in recs if r['t'].strip()))
