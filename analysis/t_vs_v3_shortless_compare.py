"""T 赋格（root 双向）vs v3 赋格（root-only-long + H¹ 短差）三方对比。

═══════════════ 引擎区别（关键） ═══════════════

  T  : 双向 root（涌现向下→建空头核心仓）+ H¹ 双向短差。data_cache/t_fugue_<SYM>_<MODE>.json
  v3 : Long-only root（涌现向下→空仓观望）+ H¹ 双向短差。data_cache/fugue_v3_<SYM>.json
       ⇒ 两者都有 short 交易；唯一结构差 = **root 层做不做空**。
  ⚠ confound：v3 复用 spiral 信号层，T 是 standalone 递归 ⇒ v3 vs T 非受控实验。
     真正受控的「做空 vs 不做空」= **T 引擎内部：整体 strat vs 多头净（剔除 short 腿）**。

pnl 口径（与 analyze_t/analyze_v3 一致）: long=sh*(xp-ep); short=sh*(ep-xp)。
  各引擎四象限/多空合计 ≈ strat_pct（全平仓，已验证 OKLO-406% 吻合）。

short 腿 reason 细分：
  H¹短差（机动仓高抛低吸）: reduce, recover
  核心仓/root/收尾        : core_clear, core_clear_short, liq_long, liq_short, eod

认识论等级：L3（真实数据逐笔归因）。
用法: PYTHONPATH=trading_system .venv/bin/python analysis/t_vs_v3_shortless_compare.py
"""

from __future__ import annotations

import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
CACHE = REPO_ROOT / "trading_system" / "data_cache"
REPORT = REPO_ROOT / "analysis" / "reports" / "t_vs_v3_shortless_report.md"
INIT = 100_000.0
SYMS = ("CL", "BRN", "DX", "GC", "ES", "QQQ", "BTC", "OKLO")
MODES = ("structural", "and", "or")
SHORTSWING = {"reduce", "recover"}  # H¹ 机动短差腿


def trade_pnl(ep: float, xp: float, sh: float, pol: str) -> float:
    return sh * (xp - ep) if pol == "long" else sh * (ep - xp)


def decompose(trades: list) -> dict:
    """逐笔 → 多头净 / 空头净 / 空头细分（短差 vs 核心）。单位 = NAV 贡献%。"""
    long_pnl = short_pnl = 0.0
    short_swing = short_core = 0.0
    for (_lad, _eb, ep, _xb, xp, sh, _w, _df, _p, reason, pol) in trades:
        pnl = trade_pnl(ep, xp, sh, pol)
        if pol == "long":
            long_pnl += pnl
        else:
            short_pnl += pnl
            if reason in SHORTSWING:
                short_swing += pnl
            else:
                short_core += pnl
    f = 100.0 / INIT
    return {
        "long_net": long_pnl * f,
        "short_net": short_pnl * f,
        "short_swing": short_swing * f,
        "short_core": short_core * f,
    }


def load_T(sym: str, mode: str) -> dict | None:
    f = CACHE / f"t_fugue_{sym}_{mode}.json"
    if not f.exists():
        return None
    d = json.loads(f.read_text())
    dec = decompose(d["trades"])
    dec.update(strat=d["strat_pct"], bh=d["bh_pct"], ns=d["n_short"], nl=d["n_long"])
    return dec


def load_v3(sym: str) -> dict | None:
    f = CACHE / f"fugue_v3_{sym}.json"
    if not f.exists():
        return None
    d = json.loads(f.read_text())
    dec = decompose(d["trades"])
    dec.update(strat=d["strat_pct"], bh=d["bh_pct"], ns=d.get("n_short", 0), nl=d["n_long"])
    return dec


def main() -> int:
    T = {(s, m): load_T(s, m) for s in SYMS for m in MODES}
    V = {s: load_v3(s) for s in SYMS}
    out: list[str] = []
    w = out.append

    w("# T（root 双向）vs v3（root-only-long）三方对比\n")
    w("- pnl = 已实现现金/初始资本 = NAV 贡献%；多空合计 ≈ strat_pct（全平仓）")
    w("- 受控对比 = T 内部（整体 vs 多头净）；v3 = 旁证（信号层 confound）")
    w("- 认识论等级 **L3**\n")

    # ── 表A：三方整体 strat ──
    w("## A. 整体盈亏三方对比（strat_pct）\n")
    w("| 标的 | BH% | **v3**(rootLong) | T_struct | T_and | T_or | T最优 | v3 vs T最优 |")
    w("|------|----:|----:|----:|----:|----:|----:|:--|")
    for s in SYMS:
        v = V[s]
        ts = {m: T[(s, m)] for m in MODES}
        t_best_m = max(MODES, key=lambda m: ts[m]["strat"] if ts[m] else -9e9)
        t_best = ts[t_best_m]["strat"]
        v3s = v["strat"] if v else float("nan")
        win = "v3 胜" if v3s > t_best else "T 胜"
        bh = v["bh"] if v else ts["structural"]["bh"]
        w(f"| {s} | {bh:+.0f} | **{v3s:+.1f}** | {ts['structural']['strat']:+.1f} "
          f"| {ts['and']['strat']:+.1f} | {ts['or']['strat']:+.1f} | {t_best:+.1f}({t_best_m[:1]}) | {win} |")
    w("")

    # ── 表B：T 内部受控「做空 vs 不做空」──
    w("## B. T 内部受控对比：剔除空头是否改善？（多头净 = T 不做任何 short 腿）\n")
    w("| 标的/模式 | T整体strat | T多头净 | T空头净 | 剔除空头改善 | 空头净符号 |")
    w("|------|----:|----:|----:|----:|:--|")
    improve = 0
    total = 0
    for s in SYMS:
        for m in MODES:
            t = T[(s, m)]
            if not t:
                continue
            total += 1
            delta = t["long_net"] - t["strat"]  # 剔除空头后相对整体的改善（≈ -short_net + MtM残差）
            imp = "✓" if t["long_net"] > t["strat"] else "✗"
            if t["long_net"] > t["strat"]:
                improve += 1
            sign = "+赚" if t["short_net"] > 0 else "−亏"
            w(f"| {s}/{m} | {t['strat']:+.1f} | {t['long_net']:+.1f} | {t['short_net']:+.1f} "
              f"| {imp} ({delta:+.0f}) | {sign} |")
    w(f"\n→ **剔除空头改善的组数：{improve}/{total}**\n")

    # ── 表C：空头腿来源细分（root 核心空头 vs H¹ 短差空头）──
    w("## C. 空头净来源细分：root 核心空头（T 独有）vs H¹ 短差空头（两引擎共有）\n")
    w("| 引擎/标的 | 空头净 | H¹短差空头 | 核心空头(root) | root 占空头亏损 |")
    w("|------|----:|----:|----:|:--|")
    # T：取各标的 structural（root 双向最纯，无 MACD 门控干扰）
    for s in SYMS:
        t = T[(s, "structural")]
        if not t:
            continue
        rootshare = (f"{t['short_core']/t['short_net']*100:.0f}%"
                     if t["short_net"] != 0 else "—")
        w(f"| T:{s}/struct | {t['short_net']:+.1f} | {t['short_swing']:+.1f} "
          f"| {t['short_core']:+.1f} | {rootshare} |")
    w("| | | | | |")
    for s in SYMS:
        v = V[s]
        if not v:
            continue
        w(f"| v3:{s} | {v['short_net']:+.1f} | {v['short_swing']:+.1f} "
          f"| {v['short_core']:+.1f} | n/a |")
    w("\n> ⚠ v3 root 恒 long-only，**无 root 核心空头**；v3 的「核心空头」列 = H¹ 机动空头腿被")
    w("> core_clear/liq/eod 事件清算的部分（仍是机动仓，非 root）。v3 全部 short 净 = H¹ 机动短差。\n")

    # ── 维度3 作答 ──
    w("## D. 维度3：「不做空」是否比「做空但亏」更好？\n")
    # 受控
    w(f"**受控（T 引擎内部）**：剔除所有 short 腿后改善的组 = **{improve}/{total}**。")
    t_short_neg = sum(1 for k in T if T[k] and T[k]["short_net"] < 0)
    t_short_tot = sum(1 for k in T if T[k])
    w(f"- T 空头净为负的组 = {t_short_neg}/{t_short_tot}（空头腿整体是拖累）。\n")
    # root 核心空头危害
    t_core_total = sum(T[(s, "structural")]["short_core"] for s in SYMS if T[(s, "structural")])
    t_swing_total = sum(T[(s, "structural")]["short_swing"] for s in SYMS if T[(s, "structural")])
    w(f"**root 空头 vs 短差空头（T structural 合计）**：")
    w(f"- 核心空头(root) 合计 = **{t_core_total:+.0f}%**；H¹短差空头合计 = **{t_swing_total:+.0f}%**。\n")
    # v3 旁证
    v3_win = sum(1 for s in SYMS if V[s] and V[s]["strat"] > max(T[(s, m)]["strat"] for m in MODES))
    w(f"**旁证（v3 root-only-long vs T 最优，有 confound）**：v3 胜 = {v3_win}/{len(SYMS)}。")
    v3_short_neg = sum(1 for s in SYMS if V[s] and V[s]["short_net"] < 0)
    w(f"- v3 空头净（纯 H¹ 短差）为负的组 = {v3_short_neg}/{len(SYMS)}。\n")

    report = "\n".join(out)
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(report, encoding="utf-8")
    print(report)
    print(f"\n[报告已写入 {REPORT}]")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
