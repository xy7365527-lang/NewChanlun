#!/usr/bin/env python3
"""#1206 探针分类器（纯标准库，可复现）。

输入: price_json + raw_dump + structure_dump + sample_keys + sample_structure
输出: 全量 308 与样本 44 的三分类 × 双口径 × 多 horizon 读数表（stdout）。

口径（逐字）见报告；本脚本只产读数。
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
    """前高(Short)/前低(Long)。返回 (val, kind, fallback)。"""
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
                return max(high[m['start']:m['end']+1]), 'high', False
            else:
                return min(low[m['start']:m['end']+1]), 'low', False
    em = s.get('exec_move') or {}
    if 'start' in em and 'end' in em:
        if d == 'Short':
            return max(high[em['start']:em['end']+1]), 'high', True
        else:
            return min(low[em['start']:em['end']+1]), 'low', True
    return None, None, True

def classify(s, high, low, close, ref='si', H=720):
    d = s['dir']
    rv, kind, fb = ref_extreme(s, high, low)
    if rv is None:
        return None
    start = s['source_index'] if ref == 'si' else s['bar']
    end = min(start + H, len(high))
    if start + 1 >= end:
        return dict(ref_val=rv, kind=kind, fallback=fb, res={'loose': '未破', 'strict': '未破'})
    ah = high[start+1:end]; al = low[start+1:end]; ac = close[start+1:end]
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

def summarize(cands, structs, high, low, close, ref, H):
    idx = {(s['bar'], s['level'], s['source_index'], s['dir'], s['stepfail']): s for s in structs}
    out = Counter(); missing = 0
    for r in cands:
        key = (r['bar'], r['level'], r['source_index'], r['dir'], r['stepfail'])
        s = idx.get(key)
        if s is None:
            missing += 1; continue
        cls = classify(s, high, low, close, ref=ref, H=H)
        if cls is None:
            missing += 1; continue
        for k, v in cls['res'].items():
            out[(r['dir'], k, v)] += 1
    return out, missing

def render(out, label):
    print(f"  {label}")
    for d in ['Short', 'Long']:
        for k in ['loose', 'strict']:
            print(f"    {d:6s} {k:6s}: 未破={out.get((d,k,'未破'),0):3d} 已破={out.get((d,k,'已破'),0):3d} 例外={out.get((d,k,'例外'),0):3d}  n={sum(out.get((d,k,v),0) for v in ['未破','已破','例外'])}")
    for k in ['loose', 'strict']:
        n = {v: sum(out.get((d,k,v),0) for d in ['Short','Long']) for v in ['未破','已破','例外']}
        print(f"    合计 {k:6s}: 未破={n['未破']:3d} 已破={n['已破']:3d} 例外={n['例外']:3d}  n={sum(n.values())}")

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
        for ref in ['si', 'bar']:
            o, miss = summarize(cands, full_struct, high, low, close, ref, H)
            print(f"  -- ref={ref}（全量，missing={miss}）--")
            render(o, "全量308")
        so, smiss = summarize(samp_cands, samp_struct, high, low, close, 'si', H)
        print(f"  -- ref=si（样本，missing={smiss}）--")
        render(so, "样本44")

if __name__ == '__main__':
    main()
