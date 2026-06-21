#!/usr/bin/env python3
"""翻空后自我修正 + 更高级别涌现 分析（耦合版 commit c06db11942）。

回答两个问题：
  Q1 翻空后次级别短差有没有自我修正？（按 origin 标签 entry/flip/sink 逐笔拆）
  Q2 翻空后有没有涌现出更高级别？（ladder 激活分布 + 翻空后是否出现更高 ladder）

trade schema (12): [ladder, entry_bar, entry_price, exit_bar, exit_price, shares,
                    weight_at_entry, deferred_bars, partial, exit_reason, polarity, origin]
"""
import json
import sys
from pathlib import Path

CACHE = Path(__file__).parent / "data_cache"

# schema 列索引
LADDER, EB, EP, XB, XP, SH, W, DEF, PART, XREASON, POL, ORIGIN = range(12)


def load(sym, mode):
    p = CACHE / f"t_engine_{sym}_{mode}_trades.json"
    if not p.exists():
        return None
    return json.load(open(p))


def pnl(t):
    """该笔已实现 PnL（多：shares*(exit-entry)；空：shares*(entry-exit)）。"""
    if t[POL] == "long":
        return t[SH] * (t[XP] - t[EP])
    else:
        return t[SH] * (t[EP] - t[XP])


def analyze(sym, mode):
    d = load(sym, mode)
    if d is None:
        print(f"  [skip] {sym}_{mode} 不存在")
        return
    trades = d["trades"]
    strat = (d['final_nav'] / 100_000 - 1) * 100  # INITIAL_CAPITAL=100,000；每槽位=12,500
    print(f"\n{'='*78}\n{sym} / {mode}  —  n_bars={d['n_bars']:,}  n_trades={len(trades)}  "
          f"final_nav={d['final_nav']:,.0f}  (INITIAL=100,000 → strat={strat:+.1f}%；每槽位 12,500)")
    print(f"{'='*78}")

    # ── A. origin × polarity 组合统计 ──
    from collections import defaultdict
    grp = defaultdict(lambda: {"n": 0, "pnl": 0.0, "bars": 0})
    for t in trades:
        key = (t[ORIGIN], t[POL])
        grp[key]["n"] += 1
        grp[key]["pnl"] += pnl(t)
        grp[key]["bars"] += (t[XB] - t[EB])
    print("\n[A] 按 origin × polarity 拆（origin: entry=独立首入 / flip=独立翻转 / sink=短差机动仓）")
    print(f"  {'origin':<8}{'pol':<7}{'n':>6}{'总PnL':>16}{'均持bar':>12}")
    for key in sorted(grp):
        g = grp[key]
        avgbar = g["bars"] / g["n"] if g["n"] else 0
        print(f"  {key[0]:<8}{key[1]:<7}{g['n']:>6}{g['pnl']:>16,.0f}{avgbar:>12,.0f}")

    # ── B. exit_reason 拆 ──
    xr = defaultdict(int)
    for t in trades:
        xr[t[XREASON]] += 1
    print(f"\n[B] exit_reason: " + ", ".join(f"{k}={v}" for k, v in sorted(xr.items())))

    # ── C. ladder 激活分布（Q2：哪些级别真激活了）──
    lad_n = defaultdict(int)
    lad_pnl = defaultdict(float)
    for t in trades:
        lad_n[t[LADDER]] += 1
        lad_pnl[t[LADDER]] += pnl(t)
    print(f"\n[C] ladder 激活分布（ladder = T_level + 3；8 槽位 3..10；>=11 被 stream 丢弃）")
    print(f"  {'ladder':<8}{'T_lvl':<7}{'n_trades':>10}{'总PnL':>16}")
    for lad in sorted(lad_n):
        print(f"  {lad:<8}{lad-3:<7}{lad_n[lad]:>10}{lad_pnl[lad]:>16,.0f}")
    maxlad = max(lad_n) if lad_n else 0
    print(f"  → 最高激活 ladder={maxlad} (T_level={maxlad-3})。"
          f"{'⚠ 触及顶 ladder=10，更高级别会被丢弃' if maxlad>=10 else '未触顶，更高级别槽位仍空'}")

    # ── D. Q1 自我修正：翻空(flip-short) 与 次级别 sink-long 的对冲关系 ──
    flip_short = [t for t in trades if t[ORIGIN] == "flip" and t[POL] == "short"]
    flip_long = [t for t in trades if t[ORIGIN] == "flip" and t[POL] == "long"]
    sink_long = [t for t in trades if t[ORIGIN] == "sink" and t[POL] == "long"]
    sink_short = [t for t in trades if t[ORIGIN] == "sink" and t[POL] == "short"]
    print(f"\n[D] Q1 自我修正解剖（翻空 = flip-short 独立翻转；次级别做多对冲 = sink-long）")
    print(f"  flip-short（顶层/父空 翻空核心腿）: n={len(flip_short)}, "
          f"PnL={sum(pnl(t) for t in flip_short):,.0f}, "
          f"均持={sum(t[XB]-t[EB] for t in flip_short)/max(len(flip_short),1):,.0f}bar")
    print(f"  flip-long （翻回多头核心腿）       : n={len(flip_long)}, "
          f"PnL={sum(pnl(t) for t in flip_long):,.0f}")
    print(f"  sink-long （父空时 次级别做多对冲）: n={len(sink_long)}, "
          f"PnL={sum(pnl(t) for t in sink_long):,.0f}  ← 自我修正对冲腿")
    print(f"  sink-short（父多时 次级别做空短差）: n={len(sink_short)}, "
          f"PnL={sum(pnl(t) for t in sink_short):,.0f}")

    # ── E. 自我修正周期：每个 flip-short 窗口内，有多少 sink-long 对冲，覆盖多少亏损 ──
    # flip-short 腿 [eb,xb] 内、ladder 更低的 sink-long 视为对冲（父空→子做多）
    print(f"\n[E] 自我修正周期重构（逐个 flip-short 窗口，找窗口内更低 ladder 的 sink-long 对冲）")
    if not flip_short:
        print("  无 flip-short：该标的/模式下顶层从未翻空（无自我修正可言）。")
    else:
        # 按亏损排序，看最伤的几个翻空窗口
        ranked = sorted(flip_short, key=lambda t: pnl(t))
        covered_cnt = 0
        for t in flip_short:
            eb, xb, lad = t[EB], t[XB], t[LADDER]
            hedges = [s for s in sink_long if s[LADDER] < lad and not (s[XB] < eb or s[EB] > xb)]
            if hedges:
                covered_cnt += 1
        print(f"  flip-short 窗口数={len(flip_short)}，其中 {covered_cnt} 个窗口内有 sink-long 对冲 "
              f"({100*covered_cnt/len(flip_short):.1f}%)")
        print(f"  最伤的 5 个翻空窗口（窗口内对冲覆盖情况）：")
        print(f"    {'lad':<5}{'entry_bar':>10}{'exit_bar':>10}{'持bar':>8}{'翻空PnL':>14}"
              f"{'对冲n':>7}{'对冲PnL':>14}{'净':>14}")
        for t in ranked[:5]:
            eb, xb, lad = t[EB], t[XB], t[LADDER]
            hedges = [s for s in sink_long if s[LADDER] < lad and not (s[XB] < eb or s[EB] > xb)]
            hpnl = sum(pnl(s) for s in hedges)
            tp = pnl(t)
            print(f"    {lad:<5}{eb:>10}{xb:>10}{xb-eb:>8}{tp:>14,.0f}"
                  f"{len(hedges):>7}{hpnl:>14,.0f}{tp+hpnl:>14,.0f}")

    # ── F. 翻回多头：flip-short 多久后同 ladder 翻回 long（完整周期时长）──
    print(f"\n[F] 完整周期（翻空→…→翻回多头）：flip-short 平仓原因分布")
    fs_xr = defaultdict(int)
    fs_dur = []
    for t in flip_short:
        fs_xr[t[XREASON]] += 1
        fs_dur.append(t[XB] - t[EB])
    if fs_xr:
        print(f"  flip-short exit_reason: " + ", ".join(f"{k}={v}" for k, v in sorted(fs_xr.items())))
        fs_dur.sort()
        n = len(fs_dur)
        print(f"  翻空持仓时长(bar)分布: min={fs_dur[0]:,} "
              f"p50={fs_dur[n//2]:,} max={fs_dur[-1]:,}")
        print(f"  注：exit_reason=flip ⟹ 同 ladder 翻回多头（独立周期闭合）；"
              f"recover/drain ⟹ 被更高涌现级别接管为短差后平仓（见 Q2）")


def main():
    syms = sys.argv[1:] if len(sys.argv) > 1 else ["BTC", "OKLO"]
    for sym in syms:
        for mode in ["structural"]:
            analyze(sym, mode)


if __name__ == "__main__":
    main()
