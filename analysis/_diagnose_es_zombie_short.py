"""诊断：fugue_v3 引擎中"回补信号存在但空头没被回补"（ES 僵尸空头根因）。

调研一次性脚本（不改引擎，纯读缓存 trades + 计数器）。

trade11 契约: [ladder, eb, ep, xb, xp, sh, w, def, part, reason, pol]
ladder 映射: 2=segment, 3=move/L1, 4=recL2, 5=recL3, 6=recL4, 7=recL5

会计语义（operate.rs / cycle.rs / accounting.rs）:
  - reduce_at 才记 trade；add_at（开仓）不记 trade。
  - sink @k  : reduce_at(k,"reduce")[平核心 chunk] + add_at(k-1, flip)[开子仓,不记 trade]
  - recover@k: reduce_at(k-1,"recover")[平子仓]    + add_at(k, parent)[升回,不记 trade]
  - 所以 reduce trade.ladder = 核心层 k；recover trade.ladder = 子层 k-1。
  - Long root: 核心=Long ⟹ sink 在 k-1 开 Short；recover 平 Short 子仓（trade.pol=short）。
  - Short root: 核心=Short ⟹ sink 在 k-1 开 Long；recover 平 Long 子仓（trade.pol=long）。

故一笔僵尸 Short 子仓若从未被 reduce 平掉，**不会出现在 trades**（除非 eod 强清）。
eod 的 short trade = 数据结尾仍存活的机动空头 = 僵尸空头。
"""
from __future__ import annotations
import json
from collections import Counter, defaultdict

LAD = {2: "seg", 3: "move/L1", 4: "recL2", 5: "recL3", 6: "recL4", 7: "recL5", 8: "recL6"}
S = {"lad": 0, "eb": 1, "ep": 2, "xb": 3, "xp": 4, "sh": 5, "w": 6, "df": 7, "pt": 8, "rs": 9, "pol": 10}


def load(sym: str) -> dict:
    return json.load(open(f"trading_system/data_cache/fugue_v3_{sym}.json"))


def short_pnl(t: list) -> float:
    return t[S["sh"]] * (t[S["ep"]] - t[S["xp"]])


def long_pnl(t: list) -> float:
    return t[S["sh"]] * (t[S["xp"]] - t[S["ep"]])


def pnl(t: list) -> float:
    return short_pnl(t) if t[S["pol"]] == "short" else long_pnl(t)


def diagnose(sym: str) -> None:
    d = load(sym)
    T = d["trades"]
    last = d["n_bars"] - 1
    print("=" * 100)
    print(f"标的 {sym}  bars={d['n_bars']:,}  strat={d['strat_pct']:+.1f}%  BH={d['bh_pct']:+.1f}%  "
          f"P1={'PASS' if d['P1_ge_bh'] else 'FAIL'}")
    print(f"  n_trades={d['n_trades']} (多 {d['n_long']} / 空 {d['n_short']})  "
          f"entries={d['entries']} core_clears={d['core_clears']} liq={d['liquidations']}  "
          f"sink={d['cycle_opens']} recover={d['cycle_closes']}")
    print(f"  exit_reasons={d['exit_reasons']}")

    # ── 1. eod 僵尸仓位（数据结尾仍存活）──
    eods = [t for t in T if t[S["rs"]] == "eod"]
    print(f"\n[1] eod 僵尸仓位（{len(eods)} 笔，exit_bar={last}）：")
    for t in sorted(eods, key=lambda x: x[S["eb"]]):
        held = t[S["xb"]] - t[S["eb"]]
        print(f"    L{t[S['lad']]}({LAD.get(t[S['lad']],'?'):7}) {t[S['pol']]:5} "
              f"eb={t[S['eb']]:>8} held={held:>9}b  basis={t[S['ep']]:.2f}→{t[S['xp']]:.2f} "
              f"({(t[S['xp']]/t[S['ep']]-1)*100:+.0f}%)  shares={t[S['sh']]:.2f}  pnl={pnl(t):>+12.0f}")
    zombie_short = [t for t in eods if t[S["pol"]] == "short"]
    if zombie_short:
        tot = sum(pnl(t) for t in zombie_short)
        print(f"    → 僵尸空头 {len(zombie_short)} 笔，合计亏损 {tot:+.0f}  "
              f"(占 strat 亏损 {abs(tot)/1000:.0f}K / 总损益 {(d['strat_pct']/100*1e6):.0f})")

    # ── 2. 空头退出路径分布 ──
    shorts = [t for t in T if t[S["pol"]] == "short"]
    print(f"\n[2] 空头 trade 退出路径（{len(shorts)} 笔）：")
    by_reason = Counter(t[S["rs"]] for t in shorts)
    for r, n in by_reason.most_common():
        rpnl = sum(pnl(t) for t in shorts if t[S["rs"]] == r)
        print(f"    {r:18} n={n:>4}  pnl={rpnl:>+12.0f}")

    # ── 3. sink/recover 按层平衡表（核心：Short 注入 vs 抽出）──
    # Long root: ladder=j 的 Short 注入 = sink@(j+1) = reduce(ladder=j+1, pol=long)
    #            ladder=j 的 Short 抽出 = recover@... 记 ladder=j, pol=short
    print(f"\n[3] 机动空头按层「注入(sink) vs 抽出(recover)」平衡表（Long root 视角）：")
    print(f"    {'子层 j':>10} {'Short注入':>10} {'Short抽出':>10} {'净累积':>10} {'强清':>6}  说明")
    # 注入：reduce trade（sink）pol=long → 在 ladder-1 开 Short。按 (ladder-1) 聚合。
    inject = Counter()
    for t in T:
        if t[S["rs"]] == "reduce" and t[S["pol"]] == "long":
            inject[t[S["lad"]] - 1] += 1
    # 抽出：recover trade pol=short，ladder=子层 j。
    extract = Counter()
    for t in T:
        if t[S["rs"]] == "recover" and t[S["pol"]] == "short":
            extract[t[S["lad"]]] += 1
    # 强清/强平/eod 平掉的 short（也是抽出，但非 recover）
    forced = Counter()
    for t in T:
        if t[S["pol"]] == "short" and t[S["rs"]] in ("core_clear_short", "liq_short", "eod", "core_clear"):
            forced[t[S["lad"]]] += 1
    all_j = sorted(set(inject) | set(extract) | set(forced))
    for j in all_j:
        inj, ext, frc = inject[j], extract[j], forced[j]
        net = inj - ext - frc
        flag = "  ⚠ 净累积>0 = 空头堆积" if net > 0 else ""
        print(f"    {j}({LAD.get(j,'?'):7}) {inj:>10} {ext:>10} {net:>10} {frc:>6}{flag}")

    # ── 4. nf fire regime 不对称（sell=sink信号 / buy=recover信号）──
    fb = d["fire_buy_by_ladder"]
    fs = d["fire_sell_by_ladder"]
    print(f"\n[4] nf fire 按层（Long root：nf_sell→sink 开空 / nf_buy→recover 平空）：")
    print(f"    {'层 k':>10} {'nf_sell(sink源)':>16} {'nf_buy(recover源)':>18} {'sell/buy 比':>12}")
    for k in range(2, 9):
        s, b = fs[k] if k < len(fs) else 0, fb[k] if k < len(fb) else 0
        ratio = f"{s/b:.1f}x" if b else "∞ (buy=0)"
        mark = "  ⚠" if b == 0 and s > 0 else ""
        print(f"    {k}({LAD.get(k,'?'):7}) {s:>16} {b:>18} {ratio:>12}{mark}")

    # ── 5. 核心周期重建（建仓→清仓时间线）──
    print(f"\n[5] 核心仓周期（F 建仓 → C 清仓 / eod）：")
    print(f"    根方向由首个非 reduce/recover 的同周期 trade 推断；entries={d['entries']}")
    # 用 core_clear / core_clear_short / eod 的 trade 标记周期边界
    boundaries = sorted([t for t in T if t[S["rs"]] in ("core_clear", "core_clear_short", "eod")],
                        key=lambda x: x[S["xb"]])
    print(f"    清仓/eod 事件 {len(boundaries)} 个：")
    seen_xb = []
    for t in boundaries:
        xb = t[S["xb"]]
        if xb not in [s[0] for s in seen_xb]:
            seen_xb.append((xb, t[S["rs"]]))
    for xb, rs in seen_xb[:30]:
        print(f"      exit_bar={xb:>8}  {rs}")


def deep(sym: str) -> None:
    """深度：recover/reduce 的 (ladder, polarity) 联合分布 + seg Short 命运 + core 周期方向。"""
    d = load(sym)
    T = d["trades"]
    print("=" * 100)
    print(f"[深度] {sym}")

    # recover/reduce 的 (ladder, pol) 联合分布——区分 Long root（子仓 Short）vs Short root（子仓 Long）
    for reason in ("recover", "reduce"):
        c = Counter((t[S["lad"]], t[S["pol"]]) for t in T if t[S["rs"]] == reason)
        print(f"\n  {reason} trades 的 (ladder, polarity) 分布：")
        for (l, p), n in sorted(c.items()):
            # recover trade.ladder=子层；reduce trade.ladder=核心层
            note = ""
            if reason == "recover":
                note = "← Long root 平 Short 子仓" if p == "short" else "← Short root 平 Long 子仓"
            else:
                note = "← Long root sink(核心Long减仓)" if p == "long" else "← Short root sink(核心Short减仓)"
            print(f"      L{l}({LAD.get(l,'?'):7}) {p:5} n={n:>4}  {note}")

    # core_clear / core_clear_short 清掉的层 × polarity（清仓时清掉哪些残留 Short）
    print(f"\n  C 清仓平掉的层 (reason, ladder, pol)：")
    cc = Counter((t[S["rs"]], t[S["lad"]], t[S["pol"]]) for t in T
                 if t[S["rs"]] in ("core_clear", "core_clear_short"))
    for (r, l, p), n in sorted(cc.items()):
        print(f"      {r:16} L{l}({LAD.get(l,'?'):7}) {p:5} n={n}")

    # 僵尸 short（eod, shares 显著）的精确信息
    print(f"\n  eod 僵尸仓位（含空壳 shares≈0）：")
    for t in sorted([t for t in T if t[S["rs"]] == "eod"], key=lambda x: x[S["eb"]]):
        print(f"      L{t[S['lad']]}({LAD.get(t[S['lad']],'?'):7}) {t[S['pol']]:5} "
              f"eb={t[S['eb']]:>8} shares={t[S['sh']]:>10.4f} basis={t[S['ep']]:.2f} pnl={pnl(t):>+10.0f}")


def main() -> None:
    print("\n" + "█" * 100)
    print("诊断：fugue_v3「回补信号存在但空头未被回补」—— ES 僵尸空头 vs CL 健康对照")
    print("█" * 100)
    for sym in ["ES", "CL"]:
        diagnose(sym)
        print()
    print("\n" + "█" * 100)
    print("深度分析（recover/reduce 极性归属 + 清仓层 + 僵尸精确）")
    print("█" * 100)
    for sym in ["ES", "CL"]:
        deep(sym)
        print()


if __name__ == "__main__":
    main()
