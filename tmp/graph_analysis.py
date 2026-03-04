import re, os, glob

settled_dir = '.chanlun/genealogy/settled'
files = sorted(glob.glob(os.path.join(settled_dir, '*.md')))

def get_id(fname):
    base = os.path.basename(fname).replace('.md','')
    m = re.match(r'^(\d{3}[a-z]?)', base)
    return m.group(1) if m else base

all_ids = set()
forward = {}

for f in files:
    nid = get_id(f)
    all_ids.add(nid)
    forward[nid] = set()

for f in files:
    nid = get_id(f)
    with open(f, 'r', encoding='utf-8') as fh:
        content = fh.read()

    refs = set()

    # Pattern 1: file references like '001-degenerate-segment.md'
    for m in re.finditer(r'(\d{3}[a-z]?)-[a-z]', content):
        ref_id = m.group(1)
        if ref_id != nid and ref_id in all_ids:
            refs.add(ref_id)

    # Pattern 2: structured refs like '前置: 013（' or '**前置**: 013'
    for m in re.finditer(r'(?:\*{0,2}(?:前置|关联|推导自|同构|触发|矛盾双源|父记录|兄弟记录|前提谱系|后续|输入|parent)\*{0,2})[：:]\s*[\"]*(\d{3}[a-z]?)', content):
        ref_id = m.group(1)
        if ref_id != nid and ref_id in all_ids:
            refs.add(ref_id)

    # Pattern 3: YAML parent field
    for m in re.finditer(r'parent:\s*[\"]*(\d{3}[a-z]?)', content):
        ref_id = m.group(1)
        if ref_id != nid and ref_id in all_ids:
            refs.add(ref_id)

    # Pattern 4: list-style refs like '- 018:' or '- **027**:'
    for m in re.finditer(r'[-*]\s*\*?\*?(\d{3}[a-z]?)\*?\*?\s*[：:号]', content):
        ref_id = m.group(1)
        if ref_id != nid and ref_id in all_ids:
            refs.add(ref_id)

    # Pattern 5: inline refs like '见012' or '详见020'
    for m in re.finditer(r'(?:见|参见|详见)\s*(\d{3}[a-z]?)', content):
        ref_id = m.group(1)
        if ref_id != nid and ref_id in all_ids:
            refs.add(ref_id)

    forward[nid] = refs

# Build backward map
backward = {nid: set() for nid in all_ids}
for src, targets in forward.items():
    for tgt in targets:
        if tgt in backward:
            backward[tgt].add(src)

prereq_count = {nid: len(forward.get(nid, set())) for nid in all_ids}
dependent_count = {nid: len(backward.get(nid, set())) for nid in all_ids}

print('=== 谱系引用图统计 ===')
print(f'总节点：{len(all_ids)}')
print()

roots = sorted([n for n in all_ids if prereq_count[n] == 0])
print(f'根节点（无前置，入度=0）：{len(roots)}')
for r in roots:
    print(f'  {r} (被 {dependent_count[r]} 条后续引用)')
print()

hubs = sorted([(n, dependent_count[n]) for n in all_ids if dependent_count[n] >= 4], key=lambda x: -x[1])
print(f'Hub 节点（被引用>=4次）：{len(hubs)}')
for h, d in hubs:
    refs_from = sorted(backward[h])
    print(f'  {h} (被引用 {d} 次) <- {refs_from}')
print()

sorted_ids = sorted(all_ids)
last_5 = set(sorted_ids[-5:])
print(f'最近5条谱系：{sorted(last_5)}')
print()

dead = sorted([n for n in all_ids if dependent_count[n] == 0 and n not in last_5])
print(f'死分支（无后续引用，非最近5条）：{len(dead)}')
for d2 in dead:
    print(f'  {d2} (前置={sorted(forward.get(d2, set()))})')
print()

isolated = sorted([n for n in all_ids if prereq_count[n] == 0 and dependent_count[n] == 0])
print(f'孤立节点（无前置且无后续）：{len(isolated)}')
for i in isolated:
    print(f'  {i}')
print()

print('=== 完整邻接表 ===')
for nid in sorted(all_ids):
    fwd = sorted(forward.get(nid, set()))
    bwd = sorted(backward.get(nid, set()))
    print(f'{nid}: 引用->{fwd}  被引用<-{bwd}')
