"""OKLO L1 缺失 type1 走势反转的深度实证刻画。

公理（编排者）: 走势终完美 → 每个走势方向反转（笔端点）必有次级别 type1 背驰。
实测: L1=ladder3, 105 个走势反转中仅 47 confirmed type1。
本脚本对缺失反转做失败模式分解。

L1 路径（与 organic_signals/current_buysellpoints 同源）:
  - moves = orch.current_moves()
  - segments = orch.current_segments()  → seg_in (dir, high, low, i0, i1)
  - zhongshus = orch.current_zhongshus() → zs5 (zd, zg, seg_start, seg_end, settled)
  - divs = R.divergences_from_moves_v1(seg_in, zs5, mv_in, level_id=1)
  - bsps = orch.current_buysellpoints()  (level-1 走势级 type1/2/3)

走势反转 = 相邻 settled move 方向变化处。完成 move = 反转前的 move。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from organic_signals import MAX_LEVELS  # noqa: E402

LEVEL_ID = 1  # L1 走势级 = ladder3


def build_l1_inputs(orch):
    """从 orchestrator 状态构造 L1 divergences_from_moves_v1 的输入。"""
    segs = orch.current_segments()
    # SegmentTuple head = (s0, s1, i0, i1, direction, high, low, confirmed, kind)
    seg_in = [(s[0][4], s[0][5], s[0][6], s[0][2], s[0][3]) for s in segs]
    zss = orch.current_zhongshus()
    # ZhongshuTuple = (zd, zg, seg_start, seg_end, seg_count, settled, ...)
    zs5 = [(z[0], z[1], z[2], z[3], z[5]) for z in zss]
    mvs = orch.current_moves()
    mv_in = [m[0] for m in mvs]  # MoveTuple head (8 fields)
    return seg_in, zs5, mv_in, segs, zss, mvs


def compute_force_fallback(seg_in, s0, s1):
    """逐位复刻 divergence.rs::compute_force fallback (无 MACD)。
    范围非法返回 0.0。"""
    n = len(seg_in)
    if s0 > s1 or s0 < 0 or s1 >= n:
        return 0.0
    i0 = seg_in[s0][3]  # seg i0
    i1 = seg_in[s1][4]  # seg i1
    high = seg_in[s0][1]
    low = seg_in[s0][2]
    for k in range(s0 + 1, s1 + 1):
        high = max(high, seg_in[k][1])
        low = min(low, seg_in[k][2])
    duration = max(i1 - i0, 1)
    return (high - low) * duration


def collect_settled_zs_indices(zs5, zs_start, zs_end):
    """复刻 collect_settled_zs_indices: [zs_start, zs_end] 范围内 settled 索引。"""
    upper = min(zs_end + 1, len(zs5))
    return [i for i in range(zs_start, upper) if zs5[i][4]]  # zs5[i][4] = settled


def next_settled_zs_seg_end(zs5, after):
    """复刻 next_settled_zs_seg_end: after 之后第一个 settled 中枢的 seg_end。"""
    for i in range(after + 1, len(zs5)):
        if zs5[i][4]:
            return zs5[i][3]  # seg_end
    return None


def trend_extreme_seg(seg_in, lo, hi, direction):
    """复刻 divergence.rs::trend_extreme_seg (B2 C段越界极值, 平值取首个)。"""
    best = lo
    if direction == "up":
        bv = seg_in[lo][1]
        for k in range(lo + 1, hi + 1):
            if seg_in[k][1] > bv:
                bv = seg_in[k][1]
                best = k
    else:
        bv = seg_in[lo][2]
        for k in range(lo + 1, hi + 1):
            if seg_in[k][2] < bv:
                bv = seg_in[k][2]
                best = k
    return best


def replicate_detect_trend(seg_in, zs5, head):
    """逐位复刻 divergence.rs::detect_trend_divergence (fallback, 无 MACD)。
    返回 (reason, info_dict)；reason='PASS' 时 info 含 force_a/force_c/c_start/c_end。
    bit-exact 守卫: 全 move 上 PASS 数 == 引擎 divergences_from_moves_v1 数。"""
    n = len(seg_in)
    kind, direction = head[0], head[1]
    zs_start, zs_end, zc = head[4], head[5], head[6]
    if kind != "trend" or zc < 2:
        return ("precond_kind_zc", {})
    szs = collect_settled_zs_indices(zs5, zs_start, zs_end)
    if len(szs) < 2:
        return ("lt2_settled_zs", {"n_settled": len(szs)})
    zs_prev = zs5[szs[-2]]
    zs_last = zs5[szs[-1]]
    a_start = zs_prev[3] + 1
    a_end = zs_last[2] - 1
    a_collapsed = a_start > a_end
    if a_collapsed:
        a_start = zs_prev[3]
        a_end = zs_prev[3]
    c_start = zs_last[3] + 1
    nxt = next_settled_zs_seg_end(zs5, szs[-1])
    search_end = min(nxt if nxt is not None else n - 1, n - 1)
    if c_start > search_end or a_start >= n:
        return ("c_segment_empty", {"c_start": c_start, "search_end": search_end})
    c_end = trend_extreme_seg(seg_in, c_start, search_end, direction)
    force_a = compute_force_fallback(seg_in, a_start, a_end)
    force_c = compute_force_fallback(seg_in, c_start, c_end)
    if force_a <= 0:
        return ("force_a_le0", {"force_a": force_a})
    info = {
        "force_a": force_a, "force_c": force_c, "ratio": force_c / force_a,
        "a_start": a_start, "a_end": a_end, "c_start": c_start, "c_end": c_end,
        "a_collapsed": a_collapsed, "center_idx": szs[-1],
    }
    if not (force_c < force_a):
        return ("t2_fail", info)
    return ("PASS", info)


def main():
    sym = "OKLO"
    o, h, l, c, _ = load_ohlc(SYMBOL_FILES[sym])
    print(f"# {sym}: {len(c)} bars", file=sys.stderr)
    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS, require_settled_subseg=True)
    for i in range(len(c)):
        orch.process_bar(o[i], h[i], l[i], c[i])

    seg_in, zs5, mv_in, segs, zss, mvs = build_l1_inputs(orch)
    print(f"# L1: {len(mv_in)} moves, {len(seg_in)} segments, {len(zs5)} zhongshus",
          file=sys.stderr)

    # 全量 L1 背驰（仅用于 bit-exact 守卫的总数对照）
    divs = R.divergences_from_moves_v1(seg_in, zs5, mv_in, LEVEL_ID)

    # L1 type1 BSP (来自 current_buysellpoints)
    bsps = orch.current_buysellpoints()
    # BspTuple head = (kind, side, level_id, seg_idx, move_seg_start, confirmed, settled)
    type1_bsps = [b for b in bsps if b[0][0] == "type1"]
    type3_bsps = [b for b in bsps if b[0][0] == "type3"]
    # type1 BSP 按 move_seg_start 索引（= 关联 move 的 seg_start）
    type1_confirmed_by_mss = {}
    type1_candidate_by_mss = {}
    for b in type1_bsps:
        mss = b[0][4]  # move_seg_start
        if b[0][5]:  # confirmed
            type1_confirmed_by_mss.setdefault(mss, []).append(b)
        else:
            type1_candidate_by_mss.setdefault(mss, []).append(b)

    # 识别走势反转: 相邻 settled move 方向变化
    settled_moves = [(idx, m) for idx, m in enumerate(mvs) if m[0][7]]  # m[0][7]=settled
    reversals = []  # (completed_move_idx, completed_move, next_move)
    for k in range(1, len(settled_moves)):
        prev_idx, prev_m = settled_moves[k - 1]
        cur_idx, cur_m = settled_moves[k]
        if prev_m[0][1] != cur_m[0][1]:  # direction differs
            reversals.append((prev_idx, prev_m, cur_m))

    print(f"# {len(reversals)} 走势方向反转 (相邻 settled move)", file=sys.stderr)

    # ── bit-exact 守卫: 全 move 上 replicate PASS 数 == 引擎 div 数 ──
    rep_pass = sum(1 for m in mvs if replicate_detect_trend(seg_in, zs5, m[0])[0] == "PASS")
    assert rep_pass == len(divs), \
        f"replicate 不 bit-exact: PASS={rep_pass} engine_divs={len(divs)}"
    print(f"# bit-exact 守卫通过: replicate PASS={rep_pass} == engine divs={len(divs)}",
          file=sys.stderr)

    # 对每个反转分类（用 bit-exact replicate）
    records = []
    for comp_idx, comp_m, next_m in reversals:
        head = comp_m[0]  # (kind,dir,seg_start,seg_end,zs_start,zs_end,zs_count,settled)
        kind, direction = head[0], head[1]
        seg_start, seg_end = head[2], head[3]
        zs_count = head[6]
        settled_zs = collect_settled_zs_indices(zs5, head[4], head[5])
        n_settled_zs = len(settled_zs)

        reason, info = replicate_detect_trend(seg_in, zs5, head)
        # div 是否针对该 move 发射 (PASS) + 是否 confirmed (ratio<=0.9)
        has_div = reason == "PASS"
        div_confirmed = has_div and (info["force_c"] / info["force_a"] <= 0.9)

        # 是否有匹配该 move 的 type1 BSP（按 move_seg_start）
        has_conf = seg_start in type1_confirmed_by_mss
        has_cand = (not has_conf) and (seg_start in type1_candidate_by_mss)

        records.append({
            "comp_idx": comp_idx, "kind": kind, "dir": direction,
            "seg_start": seg_start, "seg_end": seg_end, "zs_count": zs_count,
            "n_settled_zs": n_settled_zs,
            "reason": reason, "info": info,
            "has_div": has_div, "div_confirmed": div_confirmed,
            "has_conf_type1": has_conf, "has_cand_type1": has_cand,
        })

    # ── 分类汇总 ──
    n_total = len(records)
    confirmed = [r for r in records if r["has_conf_type1"]]
    candidate_only = [r for r in records if r["has_cand_type1"]]
    missing = [r for r in records if not r["has_conf_type1"] and not r["has_cand_type1"]]

    print("\n========== 总分布 ==========")
    print(f"总反转: {n_total}")
    print(f"  confirmed type1: {len(confirmed)}")
    print(f"  candidate-only type1: {len(candidate_only)}")
    print(f"  missing (无 type1, 仅 type3): {len(missing)}")

    # ── 54(=missing) 失败模式分解（按 replicate reason）──
    from collections import Counter
    miss_reason = Counter(r["reason"] for r in missing)
    # 进一步: PASS 但被 BSP 构造丢弃（div 发射但无 confirmed type1 BSP）
    m_div_no_bsp = [r for r in missing if r["has_div"]]
    # 无 div 发射的失败子模式
    m_t2_fail = [r for r in missing if r["reason"] == "t2_fail"]
    m_c_empty = [r for r in missing if r["reason"] == "c_segment_empty"]
    m_force_a_le0 = [r for r in missing if r["reason"] == "force_a_le0"]
    m_lt2_settled = [r for r in missing if r["reason"] == "lt2_settled_zs"]
    m_precond = [r for r in missing if r["reason"] == "precond_kind_zc"]
    m_no_div = m_t2_fail + m_c_empty + m_force_a_le0 + m_lt2_settled + m_precond

    print(f"\n========== {len(missing)} MISSING 失败模式分解 ==========")
    for reason, ct in miss_reason.most_common():
        print(f"  {reason}: {ct}")
    print(f"\n  无 div 发射 (背驰根本没算出): {len(m_no_div)}")
    print(f"    其中 t2_fail (force_c>=force_a 未衰竭): {len(m_t2_fail)}")
    print(f"    其中 c_segment_empty (C段空):          {len(m_c_empty)}")
    print(f"    其中 force_a<=0:                        {len(m_force_a_le0)}")
    print(f"    其中 <2 settled zs:                     {len(m_lt2_settled)}")
    print(f"    其中 precond(kind/zc):                  {len(m_precond)}")
    print(f"  有 div 发射但无 type1 BSP (BSP 构造丢弃): {len(m_div_no_bsp)}")

    # t2_fail 力度比统计
    if m_t2_fail:
        import statistics
        ratios = [r["info"]["ratio"] for r in m_t2_fail]
        a_dur = [seg_in[r["info"]["a_end"]][4] - seg_in[r["info"]["a_start"]][3]
                 for r in m_t2_fail]
        c_dur = [seg_in[r["info"]["c_end"]][4] - seg_in[r["info"]["c_start"]][3]
                 for r in m_t2_fail]
        print(f"\n  t2_fail force_c/force_a 比: min={min(ratios):.2f} "
              f"median={statistics.median(ratios):.2f} max={max(ratios):.2f}")
        print(f"  t2_fail A段时长(bar) median={statistics.median(a_dur):.0f} "
              f"C段时长 median={statistics.median(c_dur):.0f} "
              f"(C段 B2 越界极值延展至下一 settled 中枢, 单段A vs 多段C)")

    # candidate-only 的 div 状态
    print(f"\n========== candidate-only ({len(candidate_only)}) 的 div 状态 ==========")
    for r in candidate_only:
        info = r["info"]
        print(f"  move#{r['comp_idx']} seg_start={r['seg_start']} dir={r['dir']} "
              f"reason={r['reason']} force_a={info.get('force_a')} "
              f"force_c={info.get('force_c')} ratio={info.get('ratio')}")

    # ── 5 个具体 trace（t2_fail 为主）──
    print("\n========== 5 个具体缺失反转 trace ==========")
    traces = []
    for r in m_t2_fail[:5]:
        info = r["info"]
        t = (f"move#{r['comp_idx']} dir={r['dir']} zs[?] zc={r['zs_count']} "
             f"settled_zs={r['n_settled_zs']} "
             f"A段=seg[{info['a_start']}..{info['a_end']}]"
             f"{'(单段)' if info['a_collapsed'] else ''} "
             f"C段=seg[{info['c_start']}..{info['c_end']}] "
             f"force_a={info['force_a']:.2f} force_c={info['force_c']:.2f} "
             f"ratio_c/a={info['ratio']:.2f} -> T2失败(force_c>=force_a)无div")
        traces.append(t)
        print("  " + t)

    return {
        "n_total": n_total,
        "n_confirmed": len(confirmed),
        "n_candidate_only": len(candidate_only),
        "n_missing": len(missing),
        "no_div_emitted": len(m_no_div),
        "div_but_no_bsp": len(m_div_no_bsp),
        "c_segment_empty": len(m_c_empty),
        "fewer_than_2_settled_zs": len(m_lt2_settled),
        "t2_fail": len(m_t2_fail),
        "force_a_le0": len(m_force_a_le0),
        "traces": traces,
    }


if __name__ == "__main__":
    res = main()
    print("\n## RESULT_DICT ##")
    import json
    print(json.dumps(res, indent=2, ensure_ascii=False))
