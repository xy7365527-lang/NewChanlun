#!/usr/bin/env python3
"""Q2 深度：翻空后是否涌现更高级别 + 更高级别的方向 + 是否接管翻空为次级别短差。

机制（读自 t_engine.rs:521-537 + step_coupled）：
  - levels: Vec<LevelEngine> 固定 MAX_LADDER=11 槽位，**不动态扩展**（new() 一次性建好）。
  - "涌现更高级别" = 预分配的更高 ladder 槽位（funded if 3..10）开始收到 BSP。
  - ladder>=11 的 BSP 被 stream.rs:156 丢弃。
  - 父级 = levels[k+1]，每 bar 现算。ladder K 翻空时若 K+1 空 → 独立翻空(parent=None)。
    之后 K+1 涌现活跃 → ladder K 切到 step_coupled → 其满仓空头被当作 mobile 腿，
    在 restore-BSP 时 recover 平掉（t_engine.rs:387-394）⟹ 翻空被重读为次级别短差。
"""
import json
import sys
from pathlib import Path
from collections import defaultdict

CACHE = Path(__file__).parent / "data_cache"
LADDER, EB, EP, XB, XP, SH, W, DEF, PART, XREASON, POL, ORIGIN = range(12)


def pnl(t):
    return t[SH] * (t[XP] - t[EP]) if t[POL] == "long" else t[SH] * (t[EP] - t[XP])


def overlap(a0, a1, b0, b1):
    return not (a1 < b0 or a0 > b1)


def analyze(sym, mode="structural"):
    p = CACHE / f"t_engine_{sym}_{mode}_trades.json"
    if not p.exists():
        print(f"[skip] {sym}_{mode}")
        return
    d = json.load(open(p))
    trades = d["trades"]
    cols = len(d["trade_schema"])
    if cols < 12:
        print(f"[skip] {sym}_{mode} 仅 {cols} 列（旧引擎，无 origin），跳过")
        return
    print(f"\n{'='*78}\n{sym}/{mode}: n_trades={len(trades)} final_nav={d['final_nav']:,.0f}")

    # ── 各 ladder 的方向暴露（多/空 bar 计）：更高级别整体是涨还是跌 ──
    print("\n[Q2a] 各 ladder 方向画像（持多 bar / 持空 bar / 净 PnL）")
    lad_long_bars = defaultdict(int)
    lad_short_bars = defaultdict(int)
    lad_pnl = defaultdict(float)
    for t in trades:
        if t[POL] == "long":
            lad_long_bars[t[LADDER]] += (t[XB] - t[EB])
        else:
            lad_short_bars[t[LADDER]] += (t[XB] - t[EB])
        lad_pnl[t[LADDER]] += pnl(t)
    for lad in sorted(set(lad_long_bars) | set(lad_short_bars)):
        lb, sb = lad_long_bars[lad], lad_short_bars[lad]
        tot = lb + sb
        bias = "偏多↑" if lb > sb * 1.2 else ("偏空↓" if sb > lb * 1.2 else "均衡")
        print(f"  ladder{lad}(T{lad-3}): 多{lb:>9,}bar 空{sb:>9,}bar  多占{100*lb/max(tot,1):4.0f}%  "
              f"{bias}  PnL={lad_pnl[lad]:>+12,.0f}")

    # ── 翻空(flip-short)窗口内，是否涌现更高 ladder，方向是什么 ──
    flip_short = [t for t in trades if t[ORIGIN] == "flip" and t[POL] == "short"]
    print(f"\n[Q2b] flip-short 窗口内更高 ladder 涌现 + 方向（核心问题：翻空是否其实是次级别短差）")
    print(f"  flip-short 总数={len(flip_short)}")
    takeover = 0   # 窗口内涌现了更高 ladder
    higher_up = 0  # 涌现的更高 ladder 是上涨(持多)的
    reinterp = 0   # 该 flip-short 被 recover/drain 平掉（耦合接管）
    rows = []
    for t in flip_short:
        eb, xb, K = t[EB], t[XB], t[LADDER]
        # 窗口内、ladder 更高、entry 在本窗口开始之后涌现的腿
        higher = [s for s in trades if s[LADDER] > K and overlap(s[EB], s[XB], eb, xb)
                  and s[EB] >= eb]
        if higher:
            takeover += 1
            up_legs = [s for s in higher if s[POL] == "long"]
            if up_legs:
                higher_up += 1
            if t[XREASON] in ("recover", "drain"):
                reinterp += 1
            rows.append((K, eb, xb, xb - eb, pnl(t), t[XREASON],
                         len(higher), len(up_legs),
                         max((s[LADDER] for s in higher), default=K)))
    print(f"  → 翻空窗口内涌现了更高 ladder 的: {takeover}/{len(flip_short)} "
          f"({100*takeover/max(len(flip_short),1):.0f}%)")
    print(f"  → 其中涌现的更高 ladder 含上涨(持多)腿的: {higher_up}/{takeover if takeover else 1} "
          f"= 你的Q2c场景（更高级别向上，翻空沦为回调里的次级别操作）")
    print(f"  → 翻空腿被 recover/drain 接管平仓（重读为次级别短差）: {reinterp} (Q2d 直接证据)")
    if rows:
        print(f"\n  涌现接管样本（按翻空亏损排序）：")
        print(f"    {'空lad':<6}{'eb':>9}{'xb':>9}{'持bar':>8}{'翻空PnL':>12}{'平因':>9}"
              f"{'更高腿n':>8}{'其中多':>7}{'最高lad':>8}")
        for r in sorted(rows, key=lambda x: x[4])[:8]:
            print(f"    {r[0]:<6}{r[1]:>9}{r[2]:>9}{r[3]:>8}{r[4]:>+12,.0f}{r[5]:>9}"
                  f"{r[6]:>8}{r[7]:>7}{r[8]:>8}")

    # ── exit_reason 全景（recover/drain = 耦合接管的指纹）──
    xr = defaultdict(int)
    for t in trades:
        xr[(t[ORIGIN], t[XREASON])] += 1
    print(f"\n[Q2c] origin×exit_reason 全景（recover/drain ⟹ 该腿曾被更高涌现级别接管）")
    for k in sorted(xr):
        print(f"    {k[0]:<7}× {k[1]:<9} : {xr[k]}")


def main():
    for sym in (sys.argv[1:] or ["OKLO"]):
        analyze(sym)


if __name__ == "__main__":
    main()
