import re, os, glob

settled_dir = '.chanlun/genealogy/settled'
files = sorted(glob.glob(os.path.join(settled_dir, '*.md')))

def get_id(fname):
    base = os.path.basename(fname).replace('.md','')
    m = re.match(r'^(\d{3}[a-z]?)', base)
    return m.group(1) if m else base

all_ids = set()
strict_forward = {}

for f in files:
    nid = get_id(f)
    all_ids.add(nid)
    strict_forward[nid] = set()

for f in files:
    nid = get_id(f)
    with open(f, 'r', encoding='utf-8') as fh:
        content = fh.read()
    refs = set()
    for m in re.finditer(r'(?:\*{0,2}(?:前置|推导自|父记录|前提谱系|parent)\*{0,2})[：:]\s*[\"]*(\d{3}[a-z]?)', content):
        ref_id = m.group(1)
        if ref_id != nid and ref_id in all_ids:
            refs.add(ref_id)
    for m in re.finditer(r'parent:\s*[\"]*(\d{3}[a-z]?)', content):
        ref_id = m.group(1)
        if ref_id != nid and ref_id in all_ids:
            refs.add(ref_id)
    strict_forward[nid] = refs

strict_roots = sorted([n for n in all_ids if len(strict_forward[n]) == 0])
print('严格根节点（仅计前置/推导自，入度=0）:')
for r in strict_roots:
    print(f'  {r}')
