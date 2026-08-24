#!/usr/bin/env python3
"""#1221 探针分类器（纯标准库，可复现）。

订正 #1206 的前瞻口径：坐实检查窗口以候选 bar 为终点（backward）。
三分类、双口径、参考极值口径与 #1206 逐字同一，只改窗口方向。
另加两个读数：①「自前高/前低形成以来」的终点窗（左端点=参考极值 index）；② 前瞻×终点交叉表。

输入: price_json + raw_dump + structure_dump + sample_keys + sample_structure
输出: 全量 308 与样本（42 键 → 44 例 cond1）的 三分类 × 双口径 × 多 horizon ×
      两方向（forward=#1206 前瞻 / backward=#1221 终点口径）读数表 + from-ref + 交叉表（stdout）。

窗口（逐字，见报告）：
  forward  = (start, start+H]   —— #1206 前瞻，代码实现 [start+1 : start+H]（候选 bar 不计）
  backward = (start-H, start]   —— #1221 终点口径，代码实现 [start-H+1 : start+1]（候选 bar 计入）
  from_ref = (argmax_ref, start] —— 自前高/前低形成以来（左端点 = 参考极值 index）
"""
import json, sys
from collections import Counter

def load_window(price_json):
    d = json.load(open(price_json))
    dates = d['dates']
    win = [i for i, dt in enumerate(dates) if '2024-01-01' <= dt[:10] <= '2025-01-01']
    return ([d['highs'][i] for i in win],
            [d['lows'][i] for i in win],
            [d['closes'][i] for i in win])

def is_cond1(r):
    return r['stepfail'] in ('type23_descend_nodiv_condSome(1)', 'type1_div_fail_Some(1)')

def ref_extreme(s, high, low):
    """前高(Short)/前低(Long)。返回 (val, kind, fallback, argmax_idx)。与 #1206 逐字同一
    （argmax_idx 为参考极值在窗口内的绝对 index，供 from_ref 窗使用）。"""
    d = s['dir']
    subs = s.get('sub_moves') or []
    ai = s.get('anchor_idx')
    anchor = subs[ai] if (ai is not None and ai < len(subs)) else None
    anchor_dir = anchor['dir'] if anchor is not None else ('Down' if d == 'Short' else 'Up')
    j_start = (ai - 1) if ai is not None else (len(subs) - 1)
    for j in range(j_start, -1, -1):
        m = subs[j]
        if m['dir'] != anchor_dir:
            if d == 'Short':
                vals = high[m['start']:m['end']+1]
                v = max(vals)
                return v, 'high', False, m['start'] + vals.index(v)
            else:
                vals = low[m['start']:m['end']+1]
                v = min(vals)
                return v, 'low', False, m['start'] + vals.index(v)
    em = s.get('exec_move') or {}
    if 'start' in em and 'end' in em:
        if d == 'Short':
            vals = high[em['start']:em['end']+1]
            v = max(vals)
            return v, 'high', True, em['start'] + vals.index(v)
        else:
            vals = low[em['start']:em['end']+1]
            v = min(vals)
            return v, 'low', True, em['start'] + vals.index(v)
    return None, None, True, None

def classify(s, high, low, close, ref='si', H=720, direction='backward'):
    d = s['dir']
    rv, kind, fb, _arg = ref_extreme(s, high, low)
    if rv is None:
        return None
    start = s['source_index'] if ref == 'si' else s['bar']
    if direction == 'forward':
        # 与 #1206 逐字同一：之后窗口 (start, start+H]，代码实现 [start+1 : start+H]
        end = min(start + H, len(high))
        if start + 1 >= end:
            return dict(ref_val=rv, kind=kind, fallback=fb, res={'loose': '未破', 'strict': '未破'})
        ah = high[start+1:end]; al = low[start+1:end]; ac = close[start+1:end]
    else:
        # #1221 终点口径：之前窗口 (start-H, start]，代码实现 [start-H+1 : start+1]（候选 bar 计入）
        b0 = max(0, start - H + 1)
        ah = high[b0:start+1]; al = low[b0:start+1]; ac = close[b0:start+1]
        if len(ah) < 2:
            return dict(ref_val=rv, kind=kind, fallback=fb, res={'loose': '未破', 'strict': '未破'})
    if kind == 'high':
        loose_brk = [x < rv for x in al]
        strict_brk = [x < rv for x in ac]
        new_ext = [x > rv for x in ac]
    else:
        loose_brk = [x > rv for x in ah]
        strict_brk = [x > rv for x in ac]
        new_ext = [x < rv for x in ac]
    res = {}
    for name, brk in [('loose', loose_brk), ('strict', strict_brk)]:
        idxs = [i for i, b in enumerate(brk) if b]
        if not idxs:
            res[name] = '未破'
        else:
            first = idxs[0]
            if any(new_ext[first+1:]):
                res[name] = '例外'
            else:
                res[name] = '已破'
    return dict(ref_val=rv, kind=kind, fallback=fb, res=res)

def classify_from_ref(s, high, low, close):
    """自前高/前低形成以来（左端点 = 参考极值 index）的终点窗分类。"""
    d = s['dir']
    rv, kind, fb, argmax = ref_extreme(s, high, low)
    if rv is None or argmax is None:
        return None
    start = s['source_index']
    if start <= argmax:
        return None
    ah = high[argmax+1:start+1]; al = low[argmax+1:start+1]; ac = close[argmax+1:start+1]
    if len(ah) < 1:
        return None
    if kind == 'high':
        loose_brk = [x < rv for x in al]
        strict_brk = [x < rv for x in ac]
        new_ext = [x > rv for x in ac]
    else:
        loose_brk = [x > rv for x in ah]
        strict_brk = [x > rv for x in ac]
        new_ext = [x < rv for x in ac]
    res = {}
    for name, brk in [('loose', loose_brk), ('strict', strict_brk)]:
        idxs = [i for i, b in enumerate(brk) if b]
        if not idxs:
            res[name] = '未破'
        else:
            first = idxs[0]
            if any(new_ext[first+1:]):
                res[name] = '例外'
            else:
                res[name] = '已破'
    return dict(ref_val=rv, kind=kind, fallback=fb, res=res)

def summarize(cands, structs, high, low, close, ref, H, direction):
    idx = {(s['bar'], s['level'], s['source_index'], s['dir'], s['stepfail']): s for s in structs}
    out = Counter(); missing = 0
    for r in cands:
        key = (r['bar'], r['level'], r['source_index'], r['dir'], r['stepfail'])
        s = idx.get(key)
        if s is None:
            missing += 1; continue
        cls = classify(s, high, low, close, ref=ref, H=H, direction=direction)
        if cls is None:
            missing += 1; continue
        for k, v in cls['res'].items():
            out[(r['dir'], k, v)] += 1
    return out, missing

def summarize_from_ref(cands, structs, high, low, close):
    idx = {(s['bar'], s['level'], s['source_index'], s['dir'], s['stepfail']): s for s in structs}
    out = Counter(); missing = 0
    for r in cands:
        key = (r['bar'], r['level'], r['source_index'], r['dir'], r['stepfail'])
        s = idx.get(key)
        if s is None:
            missing += 1; continue
        cls = classify_from_ref(s, high, low, close)
        if cls is None:
            missing += 1; continue
        for k, v in cls['res'].items():
            out[(r['dir'], k, v)] += 1
    return out, missing

def total(out, k):
    return {v: sum(out.get((d, k, v), 0) for d in ['Short', 'Long']) for v in ['未破', '已破', '例外']}

def row(out, d, k):
    return {v: out.get((d, k, v), 0) for v in ['未破', '已破', '例外']}

def fmt(t):
    return f"未破={t['未破']:3d} 已破={t['已破']:3d} 例外={t['例外']:3d}  n={sum(t.values())}"

def main():
    price_json, raw_dump, struct_dump, sample_keys, sample_struct = sys.argv[1:6]
    high, low, close = load_window(price_json)
    raw = [json.loads(l) for l in open(raw_dump)]
    cands = [r for r in raw if r.get('would_close') is True and r.get('admit') is False and is_cond1(r)]
    full_struct = [json.loads(l) for l in open(struct_dump)]
    samp_struct = [json.loads(l) for l in open(sample_struct)]
    sk = [json.loads(l) for l in open(sample_keys)]
    sk_set = set((k['bar'], k['level'], k['source_index']) for k in sk)
    samp_cands = [r for r in cands if (r['bar'], r['level'], r['source_index']) in sk_set]
    print(f"全量 cond1 出场拒 = {len(cands)}（Long {sum(1 for r in cands if r['dir']=='Long')} / Short {sum(1 for r in cands if r['dir']=='Short')}）")
    print(f"样本（{len(sk)} 键）= {len(samp_cands)} cond1 候选")
    for H in [360, 720, 1440, 2880]:
        print(f"\n===== H = {H} bars =====")
        fw, _ = summarize(cands, full_struct, high, low, close, 'si', H, 'forward')
        bw, _ = summarize(cands, full_struct, high, low, close, 'si', H, 'backward')
        bs, _ = summarize(samp_cands, samp_struct, high, low, close, 'si', H, 'backward')
        for k in ['loose', 'strict']:
            print(f"  [{k}] #1206 前瞻(全量): {fmt(total(fw, k))}  |  #1221 终点(全量): {fmt(total(bw, k))}  |  #1221 终点(样本): {fmt(total(bs, k))}")
        print(f"        全量 Short  #1206: {fmt(row(fw,'Short','loose'))}  →  #1221: {fmt(row(bw,'Short','loose'))}")
        print(f"        全量 Long   #1206: {fmt(row(fw,'Long','loose'))}  →  #1221: {fmt(row(bw,'Long','loose'))}")
    # from-ref（自前高/前低形成以来）
    fr, frmiss = summarize_from_ref(cands, full_struct, high, low, close)
    print(f"\n===== from-ref（自前高/前低形成以来，全量，missing={frmiss}）=====")
    for k in ['loose', 'strict']:
        print(f"  [{k}] 全量: {fmt(total(fr, k))}")
    # 交叉表（forward × backward，H=720，宽松档）
    fw720, _ = summarize(cands, full_struct, high, low, close, 'si', 720, 'forward')
    bw720, _ = summarize(cands, full_struct, high, low, close, 'si', 720, 'backward')
    print("\n===== 交叉表 前瞻×终点（H=720，宽松档，全量）=====")
    # 逐候选重建交叉（summarize 聚合成 Counter，逐候选重算一遍更直白）
    idx = {(s['bar'], s['level'], s['source_index'], s['dir'], s['stepfail']): s for s in full_struct}
    cross = Counter()
    for r in cands:
        key = (r['bar'], r['level'], r['source_index'], r['dir'], r['stepfail'])
        s = idx.get(key)
        if s is None:
            continue
        f = classify(s, high, low, close, ref='si', H=720, direction='forward')
        b = classify(s, high, low, close, ref='si', H=720, direction='backward')
        if f is None or b is None:
            continue
        cross[(f['res']['loose'], b['res']['loose'])] += 1
    print("  前瞻\\终点  未坐实  已坐实  例外")
    for fw in ['未破', '已破', '例外']:
        print(f"  {fw:6s}  {cross.get((fw,'未破'),0):5d}  {cross.get((fw,'已破'),0):5d}  {cross.get((fw,'例外'),0):5d}")
    both = cross.get(('已破', '已破'), 0)
    fw_brk = sum(cross.get(('已破', b), 0) for b in ['未破', '已破', '例外'])
    bw_brk = sum(cross.get((f, '已破'), 0) for f in ['未破', '已破', '例外'])
    print(f"  交集(都已破)={both}  前瞻已破={fw_brk}  终点已破={bw_brk}")

if __name__ == '__main__':
    main()
