#!/usr/bin/env python3
"""#1223 探针分类器（纯标准库，可复现）：结构口径 × 价格口径 交叉读数。

结构口径（本票）：每个被拒候选的「后续走势类型发展」三臂，判据 = Rust 探针用生产结构函数
（xzd_c3_new_center_breakout 新中枢+反向突破、xzd_force_exception 强力不背驰创新高）落盘的三布尔：
  放行（真误拒）     = breakout_ok ∧ ¬force_exception
  例外（拒得对）     = force_exception
  拒（中枢继续，拒得对） = ¬breakout_ok
三窗口径：
  - 终点窗（生产可达）：confirm_index = 候选评估 bar（无前瞻）；
  - 前瞻窗 bar+H（上限）：confirm_index = 候选 bar + H；
  - 前瞻窗 si+H（上限，与 #1206 前瞻价格窗同锚）：confirm_index = source_index + H。

价格口径（对照臂，逐字复用 #1221/#1206）：ref_extreme 前高/前低 + 三分类（未破/已破/例外），
窗口方向照 #1221（终点窗向后 / from-ref / #1206 前瞻窗向前）。

输入: price_json + raw_dump + struct_dump（本票 Rust 探针落盘）
输出: 结构口径三臂表（终点+前瞻各 H）+ 与价格口径并排表 + 交叉表（差异归因）+ 逐格样例键。
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
    """与 #1221 issue1221-classify.py 逐字同一。返回 (val, kind, fallback, argmax_idx, ref_end_idx)。"""
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
                return v, 'high', False, m['start'] + vals.index(v), m['end']
            else:
                vals = low[m['start']:m['end']+1]
                v = min(vals)
                return v, 'low', False, m['start'] + vals.index(v), m['end']
    em = s.get('exec_move') or {}
    if 'start' in em and 'end' in em:
        if d == 'Short':
            vals = high[em['start']:em['end']+1]
            v = max(vals)
            return v, 'high', True, em['start'] + vals.index(v), em['end']
        else:
            vals = low[em['start']:em['end']+1]
            v = min(vals)
            return v, 'low', True, em['start'] + vals.index(v), em['end']
    return None, None, True, None, None

def classify_price(s, high, low, close, direction='backward', H=720):
    """价格三分类（与 #1221 逐字同一）。返回 '未破'/'已破'/'例外'。"""
    d = s['dir']
    rv, kind, fb, _arg, _ref_end = ref_extreme(s, high, low)
    if rv is None:
        return None
    start = s['source_index']
    if direction == 'forward':
        end = min(start + H, len(high))
        if start + 1 >= end:
            return '未破'
        ah = high[start+1:end]; al = low[start+1:end]; ac = close[start+1:end]
    else:
        b0 = max(0, start - H + 1)
        end = start + 1
        ah = high[b0:end]; al = low[b0:end]; ac = close[b0:end]
        if len(ah) < 2:
            return '未破'
    if kind == 'high':
        loose_brk = [x < rv for x in al]
        new_ext = [x > rv for x in ac]
    else:
        loose_brk = [x > rv for x in ah]
        new_ext = [x < rv for x in ac]
    idxs = [i for i, b in enumerate(loose_brk) if b]
    if not idxs:
        return '未破'
    first = idxs[0]
    return '例外' if any(new_ext[first+1:]) else '已破'

def classify_price_from_ref(s, high, low, close):
    """自前高/前低形成以来的终点窗价格分类（与 #1221 §4.2 逐字同一）。"""
    d = s['dir']
    rv, kind, fb, argmax, ref_end = ref_extreme(s, high, low)
    if rv is None:
        return None
    start = s['source_index']
    l0 = argmax
    if l0 is None or start <= l0:
        return None
    ah = high[l0+1:start+1]; al = low[l0+1:start+1]; ac = close[l0+1:start+1]
    if len(ah) < 1:
        return None
    if kind == 'high':
        loose_brk = [x < rv for x in al]
        new_ext = [x > rv for x in ac]
    else:
        loose_brk = [x > rv for x in ah]
        new_ext = [x < rv for x in ac]
    idxs = [i for i, b in enumerate(loose_brk) if b]
    if not idxs:
        return '未破'
    first = idxs[0]
    return '例外' if any(new_ext[first+1:]) else '已破'

def struct_arm(e):
    if e['breakout_ok'] and not e['force_exception']:
        return '放行'
    if e['force_exception']:
        return '例外'
    return '拒'

def main():
    price_json, raw_dump, struct_dump = sys.argv[1:4]
    high, low, close = load_window(price_json)
    raw = [json.loads(l) for l in open(raw_dump)]
    cands = [r for r in raw if r.get('would_close') is True and r.get('admit') is False and is_cond1(r)]
    structs = [json.loads(l) for l in open(struct_dump)]
    idx = {(s['bar'], s['level'], s['source_index'], s['dir']): s for s in structs}
    joined = []
    for r in cands:
        key = (r['bar'], r['level'], r['source_index'], r['dir'])
        s = idx.get(key)
        if s is None:
            continue
        s = dict(s)
        s['_raw'] = r
        joined.append(s)
    print(f"全量 cond1 出场拒 = {len(cands)}；结构 dump 命中 = {len(joined)}")
    assert len(joined) == len(cands), (len(joined), len(cands))
    n = len(joined)

    # ── 结构口径三臂 ──
    print("\n===== 结构口径三臂 =====")
    print(f"{'窗口':<20}{'拒(中枢继续)':>12}{'放行(真误拒)':>12}{'例外(拒得对)':>12}")
    c_end = Counter(struct_arm(s['end']) for s in joined)
    print(f"{'终点窗(confirm=bar)':<20}{c_end['拒']:>12}{c_end['放行']:>12}{c_end['例外']:>12}")
    for label, getter in [('前瞻 bar+H', lambda s, H: s['fwd']),
                          ('前瞻 si+H', lambda s, H: s['fwd_si'])]:
        for H in ['360', '720', '1440', '2880']:
            c = Counter(struct_arm(getter(s, H)[H]) for s in joined if getter(s, H).get(H))
            pct = c['放行'] / n * 100
            print(f"{label+' H='+H:<20}{c['拒']:>12}{c['放行']:>12}{c['例外']:>12}   ({pct:.1f}%)")

    print("\n按方向拆（终点窗；前瞻 bar+H H=720）：")
    for d in ['Short', 'Long']:
        sub = [s for s in joined if s['dir'] == d]
        ce = Counter(struct_arm(s['end']) for s in sub)
        cf = Counter(struct_arm(s['fwd']['720']) for s in sub)
        print(f"  {d} n={len(sub)}：终点 拒={ce['拒']} 放行={ce['放行']} 例外={ce['例外']}  |  前瞻720 拒={cf['拒']} 放行={cf['放行']} 例外={cf['例外']}")

    t2 = sum(1 for s in joined if s['end']['type2_confirmed'])
    print(f"\n终点窗 type2_confirmed（诊断，不参三臂）：{t2}/{n} = {t2/n*100:.1f}%")

    # ── 价格口径（对照臂，复现 #1221/#1206）──
    print("\n===== 价格口径（对照臂，复现 #1221/#1206）=====")
    p_end = Counter(classify_price(s, high, low, close, 'backward') for s in joined)
    p_ref = Counter(); pm_ref = 0
    for s in joined:
        r = classify_price_from_ref(s, high, low, close)
        if r is None: pm_ref += 1
        else: p_ref[r] += 1
    p_fwd = Counter(classify_price(s, high, low, close, 'forward') for s in joined)
    print(f"价格 终点窗(向后720)：未破={p_end['未破']} 已破={p_end['已破']} 例外={p_end['例外']}（真误拒 {p_end['已破']/n*100:.1f}%）")
    print(f"价格 from-ref：未破={p_ref['未破']} 已破={p_ref['已破']} 例外={p_ref['例外']} missing={pm_ref}（真误拒 {p_ref['已破']/n*100:.1f}%）")
    print(f"价格 前瞻窗(向前720)：未破={p_fwd['未破']} 已破={p_fwd['已破']} 例外={p_fwd['例外']}（真误拒 {p_fwd['已破']/n*100:.1f}%）")

    # ── 并排对比表 ──
    print("\n===== 并排对比：真误拒份额（结构口径 vs 价格口径）=====")
    fwd720 = Counter(struct_arm(s['fwd']['720']) for s in joined)
    fwd_si720 = Counter(struct_arm(s['fwd_si']['720']) for s in joined)
    print(f"  终点窗：结构 0/308 = {c_end['放行']/n*100:.1f}%  vs  价格(向后) {p_end['已破']/n*100:.1f}%  vs  价格(from-ref) {p_ref['已破']/n*100:.1f}%")
    print(f"  前瞻窗：结构 bar+H {fwd720['放行']/n*100:.1f}% / si+H {fwd_si720['放行']/n*100:.1f}%  vs  价格(向前) {p_fwd['已破']/n*100:.1f}%")

    # ── 交叉表（差异归因）──
    def xtab(keyfn1, keyfn2, rows1, rows2, title):
        cells = Counter()
        for s in joined:
            a, b = keyfn1(s), keyfn2(s)
            if a is None: a = 'missing'
            cells[(a, b)] += 1
        print(f"\n{title}")
        header = ''.join(f"{b:>12}" for b in rows2)
        print(f"{'':<14}{header}")
        for a in rows1:
            row = ''.join(f"{cells[(a,b)]:>12}" for b in rows2)
            print(f"{a:<14}{row}")
        return cells

    pe = lambda s: classify_price(s, high, low, close, 'backward')
    se = lambda s: struct_arm(s['end'])
    xtab(pe, se, ['未破', '已破', '例外'], ['拒', '放行', '例外'],
         "交叉表 A：终点窗 价格(向后720) × 结构(confirm=bar)")
    pr = lambda s: classify_price_from_ref(s, high, low, close)
    xtab(pr, se, ['未破', '已破', '例外', 'missing'], ['拒', '放行', '例外'],
         "交叉表 B：from-ref 价格 × 结构(confirm=bar)")
    pf = lambda s: classify_price(s, high, low, close, 'forward')
    sf = lambda s: struct_arm(s['fwd']['720'])
    xtab(pf, sf, ['未破', '已破', '例外'], ['拒', '放行', '例外'],
         "交叉表 C1：前瞻窗 价格(向前720) × 结构(confirm=bar+720)")
    sf2 = lambda s: struct_arm(s['fwd_si']['720'])
    xtab(pf, sf2, ['未破', '已破', '例外'], ['拒', '放行', '例外'],
         "交叉表 C2：前瞻窗 价格(向前720) × 结构(confirm=si+720)")

    # ── 逐格样例键（bar, level, source_index, dir）──
    print("\n===== 交叉表逐格样例键（bar, level, source_index, dir）=====")
    def samples(keyfn1, keyfn2, rows1, rows2, title, limit=5):
        print(f"-- {title} --")
        for a in rows1:
            for b in rows2:
                ex = [s for s in joined if (keyfn1(s) or 'missing') == a and keyfn2(s) == b][:limit]
                keys = [(s['bar'], s['level'], s['source_index'], s['dir']) for s in ex]
                if keys:
                    print(f"  {a}×{b}: {keys}")
    samples(pe, se, ['未破', '已破', '例外'], ['拒', '放行', '例外'], "交叉表 A 样例")
    samples(pf, sf, ['未破', '已破', '例外'], ['拒', '放行', '例外'], "交叉表 C1 样例")

if __name__ == '__main__':
    main()
