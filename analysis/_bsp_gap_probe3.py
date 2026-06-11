"""探针3：修复模拟（不改引擎）— 量化两个修复的预期信号增量。

Sim1（递归层 move seg_end 扩展，对齐 level-1 语义）：
  非末 move seg_end = 下一 move seg_start - 1；末 move = len(prev)-1。
  重调 R.divergences_from_moves_v1 + R.buysellpoints_from_level，
  统计 L4/L5 终态 type1/type2 confirmed 增量。

Sim2（笔中枢层 type1 锚冻结在趋势极值段）：
  c_end' = c 段内极值段索引（down→最低 low 段，up→最高 high 段）。
  对终态全部 settled trend move 复刻振幅背驰判定，统计：
  原定义 vs 冻结极值定义下 type1 存活数、以及由此解锁的 type2 数。

用法：PYTHONPATH=src .venv/bin/python analysis/_bsp_gap_probe3.py
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import MAX_LEVELS  # noqa: E402

CONFIRM_RATIO = 0.9


def amplitude_force(segs: list, s: int, e: int) -> float:
    """复刻 compute_force 价格振幅 fallback：(max high - min low) × duration。

    segs 行 = (direction, high, low, i0, i1)。
    """
    if s > e or s < 0 or e >= len(segs):
        return 0.0
    high = max(x[1] for x in segs[s:e + 1])
    low = min(x[2] for x in segs[s:e + 1])
    duration = max(segs[e][4] - segs[s][3], 1)
    return (high - low) * duration


def find_next_dir(segs: list, start: int, direction: str) -> int | None:
    for k in range(start, len(segs)):
        if segs[k][0] == direction:
            return k
    return None


def trend_div_sim(segs, zss, mv, c_freeze: bool):
    """复刻 detect_trend_divergence（无MACD），可选 c_end 冻结在极值段。

    zss 行 = (zd, zg, seg_start, seg_end, settled)；
    mv = (kind, dir, seg_start, seg_end, zs_start, zs_end, zs_count, settled)。
    返回 (c_end_used, force_a, force_c) 或 None。
    """
    kind, dirn, _ss, seg_end, zs_s, zs_e, zs_c, _st = mv
    if kind != "trend" or zs_c < 2:
        return None
    idx = [k for k in range(zs_s, min(zs_e + 1, len(zss))) if zss[k][4]]
    if len(idx) < 2:
        return None
    zp, zl = zss[idx[-2]], zss[idx[-1]]
    a_s, a_e = zp[3] + 1, zl[2] - 1
    if a_s > a_e:
        a_s = a_e = zp[3]
    c_s, c_e = zl[3] + 1, seg_end
    n = len(segs)
    if c_s > c_e or a_s >= n or c_e >= n:
        return None
    if c_freeze:
        # c_end' = c 段内趋势极值段（down→最低 low；up→最高 high）
        rng = range(c_s, c_e + 1)
        if dirn == "down":
            c_e = min(rng, key=lambda k: segs[k][2])
        else:
            c_e = max(rng, key=lambda k: segs[k][1])
    fa = amplitude_force(segs, a_s, a_e)
    fc = amplitude_force(segs, c_s, c_e)
    if fa <= 0 or fc >= fa:  # 三维度退化为 T2
        return None
    return (c_e, fa, fc)


def main() -> None:
    opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES["OKLO"])
    n = len(closes)
    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    for i in range(n):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
    print("engine final state ready")

    # ════════ Sim1：递归层 seg_end 扩展 ════════
    recursive = orch.current_recursive()
    level_moves_map = {lid: mvs for (lid, _z, mvs) in recursive}
    moves_l1 = orch.current_moves()
    for (lid, zhs, mvs) in recursive:
        ladder = lid + 2
        prev = moves_l1 if lid - 1 == 1 else level_moves_map.get(lid - 1, [])
        seg_in = [(m[0][1], m[1][0], m[1][1], m[1][2], m[1][3]) for m in prev]
        zs5 = [(z[0], z[1], z[2], z[3], z[5]) for z in zhs]
        zs7 = [(z[0], z[1], z[2], z[3], z[5], z[7], z[6]) for z in zhs]
        # 扩展 seg_end
        heads = [list(m[0]) for m in mvs]
        for k, h in enumerate(heads):
            if k + 1 < len(heads):
                h[3] = heads[k + 1][2] - 1
            else:
                h[3] = len(seg_in) - 1
        mv_ext = [tuple(h) for h in heads]
        divs = R.divergences_from_moves_v1(seg_in, zs5, mv_ext, lid)
        div_in = [(d[0][0], d[0][1], d[0][7], d[0][5], d[0][6], d[1][0], d[1][1])
                  for d in divs]
        bsps = R.buysellpoints_from_level(seg_in, zs7, mv_ext, div_in, lid)
        from collections import Counter
        cb = Counter((b[0][0], b[0][1], b[0][5]) for b in bsps)
        cd = Counter(d[0][0] for d in divs)
        print(f"\n== Sim1 ladder{ladder}（seg_end 扩展后终态）==")
        print(f"  背驰: {dict(cd)} (共{len(divs)})")
        print(f"  BSP (kind, side, confirmed): {dict(cb)}")
        for d in divs:
            if d[0][0] == "trend":
                fa, fc = d[1][0], d[1][1]
                print(f"    trend div dir={d[0][1]} seg_c=[{d[0][5]},{d[0][6]}] "
                      f"ratio={fc / fa:.4f} confirmed(≤0.9)={fc / fa <= 0.9}")

    # ════════ Sim2：笔中枢层 c_end 冻结极值段 ════════
    strokes = [s for s in orch.current_strokes() if s[7]]  # confirmed only
    segs = [(s[2], s[3], s[4], s[0], s[1]) for s in strokes]  # (dir,high,low,i0,i1)
    zs_in = [(s[0], s[1], s[3], s[4], True) for s in strokes]  # (i0,i1,high,low,True)
    zhs = R.zhongshu_from_strokes(zs_in)
    # ZhongshuTuple: 与 _level_bsps 一致索引（z[0]=zd z[1]=zg z[2]=seg_start
    # z[3]=seg_end z[5]=settled）
    zss = [(z[0], z[1], z[2], z[3], z[5]) for z in zhs]
    mvs = R.moves_from_zhongshus([tuple(z) for z in zhs], len(segs))
    heads = [m[0] for m in mvs]
    n_trend = sum(1 for h in heads if h[0] == "trend" and h[6] >= 2)
    print(f"\n== Sim2 ladder2 终态：strokes={len(segs)} zhongshus={len(zhs)} "
          f"moves={len(heads)} (trend zs_c>=2: {n_trend}) ==")

    # 失败原因分布（冻结定义下）
    reasons = {"not_trend": 0, "settled_zs<2": 0, "c_empty": 0, "fc>=fa": 0, "ok": 0}
    ratios: list[float] = []
    for h in heads:
        kind, dirn, _ss, seg_end, zs_s, zs_e, zs_c, _st = h
        if kind != "trend" or zs_c < 2:
            reasons["not_trend"] += 1
            continue
        idx = [k for k in range(zs_s, min(zs_e + 1, len(zss))) if zss[k][4]]
        if len(idx) < 2:
            reasons["settled_zs<2"] += 1
            continue
        zp, zl = zss[idx[-2]], zss[idx[-1]]
        a_s, a_e = zp[3] + 1, zl[2] - 1
        if a_s > a_e:
            a_s = a_e = zp[3]
        c_s, c_e = zl[3] + 1, seg_end
        if c_s > c_e or a_s >= len(segs) or c_e >= len(segs):
            reasons["c_empty"] += 1
            continue
        rng = range(c_s, c_e + 1)
        c_e2 = (min(rng, key=lambda k: segs[k][2]) if dirn == "down"
                else max(rng, key=lambda k: segs[k][1]))
        fa = amplitude_force(segs, a_s, a_e)
        fc = amplitude_force(segs, c_s, c_e2)
        if fa <= 0 or fc >= fa:
            reasons["fc>=fa"] += 1
            if fa > 0:
                ratios.append(fc / fa)
            continue
        reasons["ok"] += 1
        ratios.append(fc / fa)
    print(f"  冻结定义失败原因: {reasons}")
    if ratios:
        rs = sorted(ratios)
        qs = [rs[int(len(rs) * q)] for q in (0.1, 0.25, 0.5, 0.75, 0.9)]
        print(f"  force_c/force_a 分位 (10/25/50/75/90%): "
              f"{', '.join(f'{x:.2f}' for x in qs)}")

    # ── B2 模拟：c 段免 move 边界截断——延伸到下一 move 首中枢覆盖区内的极值段 ──
    b2 = {"t1": 0, "t1_confirmed": 0, "t1_buy_conf": 0, "t1_sell_conf": 0}
    t2_unlock = {"n": 0, "confirmed": 0, "buy_conf": 0}
    for k, h in enumerate(heads):
        kind, dirn, _ss, _se, zs_s, zs_e, zs_c, settled = h
        if kind != "trend" or zs_c < 2 or not settled:
            continue
        idx = [j for j in range(zs_s, min(zs_e + 1, len(zss))) if zss[j][4]]
        if len(idx) < 2:
            continue
        zp, zl = zss[idx[-2]], zss[idx[-1]]
        a_s, a_e = zp[3] + 1, zl[2] - 1
        if a_s > a_e:
            a_s = a_e = zp[3]
        c_s = zl[3] + 1
        # 越界搜索上限：下一 move 首中枢的 seg_end（无下一 move → n-1）
        if k + 1 < len(heads):
            nz = heads[k + 1][4]
            search_end = zss[nz][3] if nz < len(zss) else len(segs) - 1
        else:
            search_end = len(segs) - 1
        if c_s > search_end or c_s >= len(segs):
            continue
        search_end = min(search_end, len(segs) - 1)
        rng = range(c_s, search_end + 1)
        c_e = (min(rng, key=lambda j: segs[j][2]) if dirn == "down"
               else max(rng, key=lambda j: segs[j][1]))
        fa = amplitude_force(segs, a_s, a_e)
        fc = amplitude_force(segs, c_s, c_e)
        if fa <= 0 or fc >= fa:
            continue
        b2["t1"] += 1
        side = "buy" if dirn == "down" else "sell"
        confirmed = fc / fa <= CONFIRM_RATIO
        if confirmed:
            b2["t1_confirmed"] += 1
            b2["t1_buy_conf" if side == "buy" else "t1_sell_conf"] += 1
        t1_price = segs[c_e][2] if side == "buy" else segs[c_e][1]
        if side == "buy":
            rb = find_next_dir(segs, c_e + 1, "up")
            cb_ = find_next_dir(segs, rb + 1, "down") if rb is not None else None
            if cb_ is not None:
                t2_unlock["n"] += 1
                if segs[cb_][2] >= t1_price:
                    t2_unlock["confirmed"] += 1
                    if confirmed:
                        t2_unlock["buy_conf"] += 1
        else:
            pb = find_next_dir(segs, c_e + 1, "down")
            rb_ = find_next_dir(segs, pb + 1, "up") if pb is not None else None
            if rb_ is not None:
                t2_unlock["n"] += 1
                if segs[rb_][1] <= t1_price:
                    t2_unlock["confirmed"] += 1
    print(f"  B2（c 段越界极值）: type1存活={b2['t1']} confirmed={b2['t1_confirmed']} "
          f"(buy={b2['t1_buy_conf']} sell={b2['t1_sell_conf']})")
    print(f"  B2 解锁 type2: 总数={t2_unlock['n']} confirmed={t2_unlock['confirmed']} "
          f"(源type1 buy confirmed的: {t2_unlock['buy_conf']})")


if __name__ == "__main__":
    main()
