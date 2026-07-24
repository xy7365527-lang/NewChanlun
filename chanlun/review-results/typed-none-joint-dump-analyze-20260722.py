#!/usr/bin/env python3
"""#164 全量联合覆盖分析（终版）。

方法 = #107 §3.2 同一代理（已用 162 样本逐值复算验证：7/76/70/9、83/94/104/81、153 全对）：
候选 source_index 是否落在级别 ℓ 任一中枢的**初始** [si, ei]（new_center 事件口径，不应用
extend；重复中枢 id 的每次 new_center 各算一个实例）。

总体 = candidates.jsonl 全量 1724 门候选；typed_none = single_found=false（1651 个；
v4_C 原档 1653 = 1651 + 2 个因 nest_index 装配演进翻转的候选，±0.12pp 括弧另注）。
"""
import json, collections

TOWER = '/tmp/v4_C/wf7/tower_events.jsonl'
CANDS = '/tmp/wf7_repro_dump/wf7/candidates.jsonl'

# ── 中枢初始区间（new_center 口径，#107 复算验证过的方法）──
init = collections.defaultdict(list)
final = collections.defaultdict(list)  # extend 全量应用版（敏感性附注）
last = {}
with open(TOWER) as f:
    for line in f:
        e = json.loads(line)
        if e['kind'] == 'level_upgrade':
            continue
        parts = e['detail'].split()
        lvl = e['level']; cid = int(parts[1][1:])
        if e['kind'] == 'new_center':
            si = int(parts[4][3:]); ei = int(parts[5][3:])
            init[lvl].append((si, ei))
            final[lvl].append([si, ei])
            last[(lvl, cid)] = len(final[lvl]) - 1
        elif e['kind'] == 'extend':
            final[lvl][last[(lvl, cid)]][1] = int(parts[5].split('->')[1])
LEVELS = sorted(init)  # [1,2,3,4]
print('中枢实例数（初始区间）:', {l: len(v) for l, v in sorted(init.items())})

def cov(table, lvl, q):
    return any(si <= q <= ei for si, ei in table[lvl])

cands = [json.loads(l) for l in open(CANDS)]
none = [c for c in cands if not c['single_found']]
n = len(none)
print(f'全量门候选 {len(cands)}；typed_none（single_found=false）n={n}\n')

# ══ Q1 主表 A：L0 typed_none 子集，#107 四类全量版 ══
l0 = [c for c in none if c['level'] == 0]
cat = collections.Counter(); pl = collections.Counter()
for c in l0:
    q = c['source_index']
    cv = {l: cov(init, l, q) for l in LEVELS}
    for l, v in cv.items():
        if v: pl[l] += 1
    l1 = cv[1]; hi = any(cv[l] for l in (2, 3, 4))
    if l1 and hi: cat['L1+higher'] += 1
    elif l1: cat['L1_only'] += 1
    elif hi: cat['NOT_L1_but_L2+'] += 1
    else: cat['nowhere'] += 1
m = len(l0)
print(f'══ Q1-A L0 typed_none（n={m}）#107 四类全量版 ══')
for k in ['L1_only', 'L1+higher', 'NOT_L1_but_L2+', 'nowhere']:
    print(f'  {k:16s} {cat[k]:5d}  {cat[k]/m*100:6.2f}%')
print(f'  per-level: ' + '  '.join(f'L{l}={pl[l]} ({pl[l]/m*100:.1f}%)' for l in LEVELS))
print(f'  联合覆盖: {m-cat["nowhere"]} ({(m-cat["nowhere"])/m*100:.2f}%)')

# ══ Q1 主表 B：全量 typed_none（所有候选级），桥可搜级（ℓ≥lvl+1）联合覆盖 ══
print(f'\n══ Q1-B 全量 typed_none（n={n}），按桥可搜级 ℓ≥候选级+1 的联合覆盖 ══')
union_searchable = 0
nowhere_searchable = 0
flat_union = 0  # #107 口径平铺并集 L1-L4（不论候选级）
for c in none:
    q = c['source_index']
    if any(cov(init, l, q) for l in LEVELS if l >= c['level'] + 1):
        union_searchable += 1
    else:
        nowhere_searchable += 1
    if any(cov(init, l, q) for l in LEVELS):
        flat_union += 1
print(f'  桥可搜级联合覆盖: {union_searchable} ({union_searchable/n*100:.2f}%)   nowhere(可搜级皆无): {nowhere_searchable} ({nowhere_searchable/n*100:.2f}%)')
print(f'  (#107 平铺并集 L1∪L2∪L3∪L4 不论候选级: {flat_union} ({flat_union/n*100:.2f}%), 平铺 nowhere: {n-flat_union} ({(n-flat_union)/n*100:.2f}%))')

# ══ Q2：按候选级分层 —— 桥级（lvl+1）/ 更深级覆盖构成 ══
print(f'\n══ Q2 按候选级分层：桥级=lvl+1 覆盖 vs 仅更深级覆盖 vs nowhere ══')
print(f'{"候选级":>4} {"n":>5} {"桥级覆盖":>10} {"仅桥级":>8} {"桥+深":>8} {"非桥但深":>9} {"nowhere":>9}   非桥但深的级别构成')
agg_deeper = collections.Counter()
for cl in sorted({c['level'] for c in none}):
    rows = [c for c in none if c['level'] == cl]
    bridge = cl + 1
    cnt = collections.Counter(); comp = collections.Counter()
    for c in rows:
        q = c['source_index']
        b = cov(init, bridge, q) if bridge in LEVELS else False
        deeper = tuple(l for l in LEVELS if l > bridge and cov(init, l, q))
        if b and deeper: cnt['b+d'] += 1
        elif b: cnt['b_only'] += 1
        elif deeper:
            cnt['not_b'] += 1
            comp['+'.join(f'L{l}' for l in deeper)] += 1
            agg_deeper['+'.join(f'L{l}' for l in deeper)] += 1
        else: cnt['nw'] += 1
    nn = len(rows)
    print(f'L{cl:>3} {nn:>5} {cnt["b_only"]+cnt["b+d"]:>10} {cnt["b_only"]:>8} {cnt["b+d"]:>8} '
          f'{cnt["not_b"]:>9} {cnt["nw"]:>9}   {dict(comp)}')
print(f'  非桥但深的总构成（全体候选级合计）: {dict(agg_deeper)}')

# ══ Q2 附：L0 子集中 NOT_L1_but_L2+ 的级别构成（#107 可比口径）══
comp_l0 = collections.Counter()
for c in l0:
    q = c['source_index']
    if not cov(init, 1, q):
        deeper = tuple(l for l in (2, 3, 4) if cov(init, l, q))
        if deeper:
            comp_l0['+'.join(f'L{l}' for l in deeper)] += 1
print(f'  L0 typed_none 的 NOT_L1_but_L2+ 级别构成: {dict(comp_l0)}（合计 {sum(comp_l0.values())}）')

# ══ Q3：nowhere（桥可搜级皆无覆盖）按候选级 ══
print(f'\n══ Q3 真残值（nowhere）══')
nw_total = sum(1 for c in none if not any(cov(init, l, c['source_index']) for l in LEVELS if l >= c['level'] + 1))
nw_struct_terminal = sum(1 for c in none if c['level'] >= max(LEVELS))
print(f'  桥可搜级皆无覆盖: {nw_total}/{n} = {nw_total/n*100:.2f}%')
print(f'  其中候选级=L4（塔顶，结构上无可搜级）: {nw_struct_terminal}')
nw_l0 = sum(1 for c in l0 if not any(cov(init, l, c['source_index']) for l in LEVELS))
print(f'  L0 typed_none 的 nowhere: {nw_l0}/{len(l0)} = {nw_l0/len(l0)*100:.2f}%（#107 样本估计 5.6%）')

# ══ 实际真链恢复（multi_hits，因果守卫后）vs 代理上界 ══
print(f'\n══ 实际恢复（typed_lookup_multi 真链命中，非代理）══')
mh = [c for c in none if c['multi_hits']]
hit_lvl = collections.Counter(h[0] for c in mh for h in c['multi_hits'])
print(f'  single_none 中 multi 命中: {len(mh)}/{n} = {len(mh)/n*100:.2f}%，n_delta 全过')
print(f'  命中级别构成: {dict(sorted(hit_lvl.items()))}')
print(f'  按候选级: ' + str({cl: f"{sum(1 for c in mh if c['level']==cl)}/{sum(1 for c in none if c['level']==cl)}" for cl in sorted({{c["level"] for c in none}})}))

# ══ 敏感性附注：extend 全量应用版（区间随时间扩展，覆盖近乎平凡）══
fu = sum(1 for c in none if any(cov(final, l, c['source_index']) for l in LEVELS))
print(f'\n══ 敏感性附注：extend 应用后的终态区间，平铺并集覆盖 {fu}/{n} = {fu/n*100:.2f}%（区间延展后代理趋于平凡，区分度丧失；#107 口径=初始区间）')
