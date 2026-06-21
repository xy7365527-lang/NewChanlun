#!/usr/bin/env python3
"""emergence ON vs OFF 的 reduce/recover 配对率分析 —— 回答「涌现升级后敞口为什么降」。

数据源（由 analysis/run_emergence_ab.sh 重跑生成，明确隔离 ON/OFF；现有 data_cache
缓存因文件名共享已被 ON/OFF 互相覆盖为同一版本，不可用于对比）：
  ON  = /tmp/t_on_v2/   (emergence default：核心仓随 level 涌现 relabel 上移)
  OFF = /tmp/t_off_v2/  (T_NO_EMERGENCE=1：消融门关，退化为纯 BSP 驱动)

trade schema (12 字段)：
  [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
   weight_at_entry, deferred_bars, partial, exit_reason, polarity, origin]

会计语义（rust/src/fugue_v3/accounting.rs + recursive_t/t_engine.rs）：
  - sink(parent,sub): reduce_at(parent, m=u_P/3, "reduce") + add_at(sub, m, flip(dP))
        ⟹ exit_reason="reduce" = 父级被下放减仓的 chunk；n_cycle_opens[parent]++
  - recover(parent,sub): reduce_at(sub, m=u_sub/3, "recover") + add_at(parent, m, dP)
        ⟹ exit_reason="recover" = 子级短差被升回平的 chunk；n_cycle_closes[parent]++
  - add_at 加权 basis 且**不更新 entry_bar**（仅空仓时设）⟹ recover trade 的
        entry_bar = 该 sub 本轮短差首次下放 bar；exit_bar−entry_bar = 归还时延。
  - drain: reduce_at(j, u_j/3, "drain") = 子级遗留同向仓减暴露（**不升回父级**）。

关键非对称（敞口降低的机制根因）：
  sink 减父级 u_P/3，recover 只平子级 u_sub/3 ≈ u_P/9 ⟹ 单次 recover 升回量 ≈ 单次
  sink 下放量的 1/3。即使次数 1:1 配对，金额上也几何衰减、永远追不平。
"""
from __future__ import annotations

import glob
import json
import os
import sys
from typing import Any

ON_DIR = "/tmp/t_on_v2"
OFF_DIR = "/tmp/t_off_v2"
INITIAL = 100_000.0

# trade 字段索引
LADDER, EB, EP, XB, XP, SH, W, DEF, PART, REASON, POL, ORIG = range(12)


def load(d: str, sym: str, mode: str) -> dict[str, Any] | None:
    p = os.path.join(d, f"t_engine_{sym}_{mode}_trades.json")
    if not os.path.exists(p):
        return None
    with open(p) as f:
        return json.load(f)


def syms_in(d: str, mode: str) -> list[str]:
    out = []
    for p in sorted(glob.glob(os.path.join(d, f"t_engine_*_{mode}_trades.json"))):
        out.append(os.path.basename(p).split("t_engine_")[1].split(f"_{mode}_")[0])
    return out


def total(arr: list[int] | None) -> int:
    return sum(arr) if arr else 0


def time_avg_exposure(d: dict[str, Any]) -> tuple[float, float]:
    """exposure_series=[[bar,lu,su],...] 的 **bar 加权** 时间平均（每状态持续到下一记录点）。"""
    es = d.get("exposure_series", [])
    n_bars = d.get("n_bars", 0)
    if not es:
        return 0.0, 0.0
    lu_acc = su_acc = 0.0
    span = 0
    for i, (bar, lu, su) in enumerate(es):
        end = es[i + 1][0] if i + 1 < len(es) else n_bars
        w = max(0, end - bar)
        lu_acc += lu * w
        su_acc += su * w
        span += w
    if span == 0:
        return 0.0, 0.0
    return lu_acc / span, su_acc / span


def reason_units(d: dict[str, Any]) -> dict[str, tuple[int, float]]:
    """按 exit_reason 聚合 (chunk数, units和)。"""
    agg: dict[str, list[float]] = {}
    for t in d["trades"]:
        r = t[REASON]
        a = agg.setdefault(r, [0, 0.0])
        a[0] += 1
        a[1] += t[SH]
    return {k: (int(v[0]), v[1]) for k, v in agg.items()}


def recover_gaps(d: dict[str, Any]) -> list[int]:
    """recover trade 的 (exit_bar − entry_bar) = 短差从首次下放到本次升回的时延。"""
    return [t[XB] - t[EB] for t in d["trades"] if t[REASON] == "recover"]


def pct(xs: list[int], q: float) -> int:
    if not xs:
        return 0
    s = sorted(xs)
    i = min(len(s) - 1, int(q * len(s)))
    return s[i]


def scenario_units(d: dict[str, Any]) -> dict[tuple[str, str], float]:
    """(exit_reason, polarity) → units 和。揭示 sink/recover 的方向场景：
      父持多：reduce,long(减多父) ↔ recover,short(平空子升回)
      父持空：reduce,short(减空父) ↔ recover,long(平多子升回)
    reduce 的 polarity = 父级方向；recover 的 polarity = 反父向(子级短差方向)。"""
    out: dict[tuple[str, str], float] = {}
    for t in d["trades"]:
        if t[REASON] in ("reduce", "recover"):
            key = (t[REASON], t[POL])
            out[key] = out.get(key, 0.0) + t[SH]
    return out


def fnav_pct(d: dict[str, Any]) -> float:
    return (d["final_nav"] / INITIAL - 1.0) * 100.0


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else "or"
    if not os.path.isdir(ON_DIR) or not os.path.isdir(OFF_DIR):
        print(f"[错误] 数据目录缺失：ON={ON_DIR} OFF={OFF_DIR}\n"
              f"先运行 analysis/run_emergence_ab.sh 重跑（~21min）。")
        sys.exit(1)

    syms = syms_in(OFF_DIR, mode) or syms_in(ON_DIR, mode)
    print("#" * 120)
    print(f"# emergence ON vs OFF — reduce/recover 配对率分析  (mode={mode})")
    print(f"# ON={ON_DIR}  OFF={OFF_DIR}")
    print("#" * 120)

    # ── 表1：配对率三口径 ──────────────────────────────────────────────
    print("\n【表1】配对率三口径：操作次数 / chunk次数 / 金额(units)  —— 次数像配平 ≠ 金额配平")
    print(f"{'标的':<6}{'版本':<5}{'strat%':>8} │{'sink次':>8}{'rec次':>7}{'rec/sink':>9} │"
          f"{'reduceΣu':>11}{'recoverΣu':>11}{'升回/下放':>9}")
    print("-" * 120)
    for sym in syms:
        for lab, dd in [("OFF", OFF_DIR), ("ON", ON_DIR)]:
            d = load(dd, sym, mode)
            if not d:
                print(f"{sym:<6}{lab:<5}  (缺)")
                continue
            opn = total(d.get("n_cycle_opens_by_ladder"))   # sink 操作次数
            cls = total(d.get("n_cycle_closes_by_ladder"))  # recover 操作次数
            ru = reason_units(d)
            red_u = ru.get("reduce", (0, 0.0))[1]
            rec_u = ru.get("recover", (0, 0.0))[1]
            op_ratio = cls / opn if opn else 0.0
            u_ratio = rec_u / red_u if red_u else 0.0
            print(f"{sym:<6}{lab:<5}{fnav_pct(d):>+7.1f} │{opn:>8}{cls:>7}{op_ratio:>8.1%} │"
                  f"{red_u:>11,.0f}{rec_u:>11,.0f}{u_ratio:>8.1%}")
        print("." * 120)

    # ── 表2：归还速度（recover gap 分位）──────────────────────────────
    print("\n【表2】归还速度：recover trade 的 (exit_bar−entry_bar) 分位  —— 短差从下放到升回要多少 bar")
    print(f"{'标的':<6}{'版本':<5}{'n_rec':>8}{'min':>8}{'p50':>10}{'p90':>11}{'max':>11}{'占总bar%(p50)':>14}")
    print("-" * 120)
    for sym in syms:
        for lab, dd in [("OFF", OFF_DIR), ("ON", ON_DIR)]:
            d = load(dd, sym, mode)
            if not d:
                continue
            g = recover_gaps(d)
            nb = d.get("n_bars", 1)
            p50 = pct(g, 0.5)
            print(f"{sym:<6}{lab:<5}{len(g):>8}{(min(g) if g else 0):>8}{p50:>10,}"
                  f"{pct(g, 0.9):>11,}{(max(g) if g else 0):>11,}{p50 / nb:>13.1%}")
        print("." * 120)

    # ── 表3：时间平均敞口 ON vs OFF（敞口为什么降的直接证据：exposure_series 真值）──
    print("\n【表3】时间平均敞口(units, bar加权)  —— 『敞口为什么降』的金标准（exposure_series 真值，非 trade 反推）")
    print(f"{'标的':<6}{'版本':<5}{'avg多u':>11}{'avg空u':>11}{'净敞口':>12}{'毛敞口':>11}{'净 ΔON−OFF':>13}")
    print("-" * 120)
    for sym in syms:
        net_off = None
        for lab, dd in [("OFF", OFF_DIR), ("ON", ON_DIR)]:
            d = load(dd, sym, mode)
            if not d:
                continue
            lu, su = time_avg_exposure(d)
            net = lu - su
            delta = "" if net_off is None else f"{net - net_off:>+13,.1f}"
            if lab == "OFF":
                net_off = net
            print(f"{sym:<6}{lab:<5}{lu:>11,.1f}{su:>11,.1f}{net:>+12,.1f}{lu + su:>11,.1f}{delta}")
        print("." * 120)

    # ── 表3b：方向场景配平（揭示核心方向 + 哪个场景 recover 跟不上 sink）──
    print("\n【表3b】方向场景配平(units)：父持多[reduce,long↔recover,short] / 父持空[reduce,short↔recover,long]")
    print(f"{'标的':<6}{'版本':<5} │{'父多:减多':>10}{'父多:升回空':>12}{'配平':>7} │"
          f"{'父空:减空':>10}{'父空:升回多':>12}{'配平':>7}")
    print("-" * 120)
    for sym in syms:
        for lab, dd in [("OFF", OFF_DIR), ("ON", ON_DIR)]:
            d = load(dd, sym, mode)
            if not d:
                continue
            sc = scenario_units(d)
            r_l = sc.get(("reduce", "long"), 0.0)    # 父多减仓
            rec_s = sc.get(("recover", "short"), 0.0)  # 平空子升回多父
            r_s = sc.get(("reduce", "short"), 0.0)   # 父空减仓
            rec_l = sc.get(("recover", "long"), 0.0)   # 平多子升回空父
            p1 = rec_s / r_l if r_l else 0.0
            p2 = rec_l / r_s if r_s else 0.0
            print(f"{sym:<6}{lab:<5} │{r_l:>10,.0f}{rec_s:>12,.0f}{p1:>6.0%} │"
                  f"{r_s:>10,.0f}{rec_l:>12,.0f}{p2:>6.0%}")
        print("." * 120)

    # ── 表4：未归还量化 ──────────────────────────────────────────────
    print("\n【表4】未归还量化：sink下放 vs recover升回 缺口 + 末端挂账短差")
    print(f"{'标的':<6}{'版本':<5}{'下放Σu':>11}{'升回Σu':>11}{'缺口Σu':>11}{'缺口%':>8}"
          f"{'末端空u':>10}{'sink次−rec次':>13}")
    print("-" * 120)
    for sym in syms:
        for lab, dd in [("OFF", OFF_DIR), ("ON", ON_DIR)]:
            d = load(dd, sym, mode)
            if not d:
                continue
            ru = reason_units(d)
            red_u = ru.get("reduce", (0, 0.0))[1]
            rec_u = ru.get("recover", (0, 0.0))[1]
            gap_u = red_u - rec_u
            gap_pct = gap_u / red_u if red_u else 0.0
            es = d.get("exposure_series", [])
            tail_su = es[-1][2] if es else 0.0
            opn = total(d.get("n_cycle_opens_by_ladder"))
            cls = total(d.get("n_cycle_closes_by_ladder"))
            print(f"{sym:<6}{lab:<5}{red_u:>11,.0f}{rec_u:>11,.0f}{gap_u:>+11,.0f}{gap_pct:>7.1%}"
                  f"{tail_su:>10,.1f}{opn - cls:>+13}")
        print("." * 120)

    # ── 表5：CL per-ladder 明细 ──────────────────────────────────────
    print("\n【表5】CL per-ladder（sink/recover/drain/liq 次数 + reduce/recover units）")
    print("  —— 定位哪个 level 减仓下放后归还最差")
    for lab, dd in [("OFF", OFF_DIR), ("ON", ON_DIR)]:
        d = load(dd, "CL", mode)
        if not d:
            continue
        opn = d.get("n_cycle_opens_by_ladder", [])
        cls = d.get("n_cycle_closes_by_ladder", [])
        liq = d.get("n_liquidations_by_ladder", [])
        ent = d.get("n_entries_by_ladder", [])
        # per-ladder reduce/recover units（按 trade ladder 聚合）
        red_by_l: dict[int, float] = {}
        rec_by_l: dict[int, float] = {}
        for t in d["trades"]:
            if t[REASON] == "reduce":
                red_by_l[t[LADDER]] = red_by_l.get(t[LADDER], 0.0) + t[SH]
            elif t[REASON] == "recover":
                rec_by_l[t[LADDER]] = rec_by_l.get(t[LADDER], 0.0) + t[SH]
        print(f"\n[CL {lab}]  strat={fnav_pct(d):+.1f}%   (L: entries sink recover drain? liq | reduceΣu recoverΣu)")
        for k in range(11):
            e = ent[k] if k < len(ent) else 0
            o = opn[k] if k < len(opn) else 0
            c = cls[k] if k < len(cls) else 0
            lq = liq[k] if k < len(liq) else 0
            ruL = red_by_l.get(k, 0.0)
            recL = rec_by_l.get(k, 0.0)
            if e or o or c or lq or ruL or recL:
                print(f"  L{k}: ent={e:<5} sink={o:<5} rec={c:<5} liq={lq:<4} │ "
                      f"reduceΣu={ruL:>10,.1f}  recoverΣu={recL:>10,.1f}")

    print("\n" + "#" * 120)
    print("# 注：reduce 的 trade ladder=父级(被下放减仓)，recover 的 trade ladder=子级(短差升回)。")
    print("#     故 per-ladder reduce/recover 不在同一 ladder 配平——sink 减父 L, recover 平子 L-Δ。")
    print("#" * 120)


if __name__ == "__main__":
    main()
