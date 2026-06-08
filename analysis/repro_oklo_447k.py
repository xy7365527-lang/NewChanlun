"""任务A：447K OKLO（oklo_1m_databento.json, alphavantage 全时段）复现 E +932% + I 对比。

- E：fugue_alpha_diagnosis.compute_signals → run_swing_trading(MODE_NONE)（任务A.1，逐字调用）
- I：fugue_version_i.compute_signals_i → run_version_i（任务A.2）
- 逐笔对比 E vs I 的买卖点（任务A.3/A.4）

数据 schema 区别：447K 文件用 bars 列表（{ts,open,high,low,close,volume}），
与脚本默认的扁平数组 _full.json 不同 → 本 driver 单独适配，不污染原数据映射。
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as diag  # noqa: E402
from fugue_alpha_diagnosis import (  # noqa: E402
    MODE_NONE,
    compute_signals,
    compute_metrics,
    run_swing_trading,
)
import fugue_version_i as vi  # noqa: E402
from fugue_version_i import (  # noqa: E402
    compute_signals_i,
    run_version_i,
    extended_metrics,
)

DATA = ROOT / "analysis" / "data_cache" / "oklo_1m_databento.json"


def load_447k():
    raw = json.loads(DATA.read_text())
    bars = raw["bars"]
    opens = [float(b["open"]) for b in bars]
    highs = [float(b["high"]) for b in bars]
    lows = [float(b["low"]) for b in bars]
    closes = [float(b["close"]) for b in bars]
    years = [int(str(b["ts"])[:4]) for b in bars]
    return opens, highs, lows, closes, years


def main() -> None:
    print(f"加载 {DATA.name} ...")
    opens, highs, lows, closes, years = load_447k()
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  {n:,} bars  BH={bh:+.2f}%  价 {closes[0]} → {closes[-1]}")
    print(f"  年份范围 {years[0]}–{years[-1]}")

    # ── E 版本（任务A.1，逐字 diagnosis 路径）──
    t0 = time.time()
    e_sig = compute_signals(opens, highs, lows, closes)
    print(f"  [E] compute_signals {time.time()-t0:.1f}s")
    e_trades, _ = run_swing_trading(e_sig, MODE_NONE)
    e_m = compute_metrics(e_trades)
    e_ext = extended_metrics(e_trades, years)

    # ── I 版本（任务A.2）──
    t0 = time.time()
    e_sig_i, i_sig = compute_signals_i(opens, highs, lows, closes)
    print(f"  [I] compute_signals_i {time.time()-t0:.1f}s")
    i_trades, i_extra = run_version_i(i_sig)
    i_m = compute_metrics(i_trades)
    i_ext = extended_metrics(i_trades, years)

    # ── tape-equality 交叉检查：version_i 的 E 信号是否等价 diagnosis 的 E ──
    e2_trades, _ = run_swing_trading(e_sig_i, MODE_NONE)
    e2_m = compute_metrics(e2_trades)
    tape_eq = (
        e_m["total_compound"] == e2_m["total_compound"]
        and len(e_trades) == len(e2_trades)
    )

    print("\n" + "=" * 64)
    print("  结果汇总（447K OKLO alphavantage 全时段）")
    print("=" * 64)
    print(f"  BH 复利           {bh:+12.2f}%")
    print(f"  E 复利            {e_m['total_compound']:+12.2f}%  超额 {e_m['total_compound']-bh:+10.2f}%")
    print(f"  E (via version_i) {e2_m['total_compound']:+12.2f}%  tape_eq={tape_eq}")
    print(f"  I 复利            {i_m['total_compound']:+12.2f}%  超额 {i_m['total_compound']-bh:+10.2f}%")
    print(f"  +932.77% 复现？   {'✅' if abs(e_m['total_compound']-932.77) < 0.5 else '❌ 实测=' + format(e_m['total_compound'],'+.2f')}")
    print()
    hdr = f"  {'指标':<14}{'E':>16}{'I':>16}"
    print(hdr)
    print("  " + "-" * 46)
    for label, key in [
        ("交易笔数", "n"), ("胜率%", "win_rate"), ("复利%", "total_compound"),
        ("超额%", None), ("夏普", "sharpe"), ("最大回撤%", "max_dd"),
        ("降成本笔", "n_with_cr"),
    ]:
        if key is None:
            ev, iv = e_m["total_compound"]-bh, i_m["total_compound"]-bh
        else:
            ev, iv = e_m[key], i_m[key]
        print(f"  {label:<14}{ev:>16.2f}{iv:>16.2f}")
    print(f"  I 归属(ladder→笔)  {i_extra['ladder_attribution']}")
    print(f"  I 平均持仓bar      {i_extra['ladder_avg_held_bars']}")
    print(f"  I 2买标记          {i_extra['addon_2buy_marks']}")

    # ── 逐笔对比（任务A.3）──
    diff = compare_trades(e_trades, i_trades)

    # 写 JSON
    out = {
        "data_file": DATA.name, "n_bars": n, "bh": bh,
        "first": closes[0], "last": closes[-1],
        "year_range": [years[0], years[-1]],
        "E": {"metrics": e_m, "by_year": e_ext.get("by_year")},
        "I": {"metrics": i_m, "by_year": i_ext.get("by_year"),
              "ladder_attribution": i_extra["ladder_attribution"],
              "ladder_avg_held_bars": i_extra["ladder_avg_held_bars"],
              "addon_2buy_marks": i_extra["addon_2buy_marks"]},
        "tape_equality_E": tape_eq,
        "reproduces_932": abs(e_m["total_compound"] - 932.77) < 0.5,
        "trade_diff_summary": diff["summary"],
        "E_trades": [_t2d(t) for t in e_trades],
        "I_trades": [_t2d(t) for t in i_trades],
    }
    outp = ROOT / "analysis" / "data_cache" / "repro_oklo_447k_results.json"
    outp.write_text(json.dumps(out, indent=2, ensure_ascii=False, default=str))
    print(f"\n详细结果写入：{outp.name}")


def _t2d(t):
    return {
        "entry_bar": t.entry_bar, "entry_price": t.entry_price,
        "exit_bar": t.exit_bar, "exit_price": t.exit_price,
        "pnl_pct": round(t.pnl_pct, 4), "exit_reason": t.exit_reason,
        "n_short_diffs": t.n_short_diffs,
    }


def compare_trades(e_trades, i_trades) -> dict:
    """对比 E/I 买卖点。以 entry_bar 为键找共同/独有的进场。"""
    e_entries = {t.entry_bar: t for t in e_trades}
    i_entries = {t.entry_bar: t for t in i_trades}
    common = sorted(set(e_entries) & set(i_entries))
    e_only = sorted(set(e_entries) - set(i_entries))
    i_only = sorted(set(i_entries) - set(e_entries))

    # 共同进场中，出场点不同的
    exit_diff = [
        b for b in common if e_entries[b].exit_bar != i_entries[b].exit_bar
    ]
    print("\n" + "=" * 64)
    print("  E vs I 买卖点对比（任务A.3）")
    print("=" * 64)
    print(f"  E 进场点数={len(e_entries)}  I 进场点数={len(i_entries)}")
    print(f"  共同进场bar={len(common)}  E独有={len(e_only)}  I独有={len(i_only)}")
    print(f"  共同进场但出场bar不同={len(exit_diff)}")

    if e_only:
        print(f"\n  ── E 独有进场（I 未进，前20）──")
        for b in e_only[:20]:
            t = e_entries[b]
            print(f"    entry_bar={b:>7} @ {t.entry_price:>8.2f}  exit_bar={t.exit_bar:>7} "
                  f"pnl={t.pnl_pct:+7.2f}% reason={t.exit_reason}")
    if i_only:
        print(f"\n  ── I 独有进场（E 未进，前20）──")
        for b in i_only[:20]:
            t = i_entries[b]
            print(f"    entry_bar={b:>7} @ {t.entry_price:>8.2f}  exit_bar={t.exit_bar:>7} "
                  f"pnl={t.pnl_pct:+7.2f}% reason={t.exit_reason}")
    if exit_diff:
        print(f"\n  ── 共同进场，出场不同（前20）──")
        for b in exit_diff[:20]:
            et, it = e_entries[b], i_entries[b]
            print(f"    entry_bar={b:>7}  E:exit={et.exit_bar:>7}(pnl{et.pnl_pct:+6.1f}% "
                  f"{et.exit_reason})  I:exit={it.exit_bar:>7}(pnl{it.pnl_pct:+6.1f}% "
                  f"{it.exit_reason} cr={it.n_short_diffs})")

    return {"summary": {
        "e_entries": len(e_entries), "i_entries": len(i_entries),
        "common": len(common), "e_only": len(e_only), "i_only": len(i_only),
        "exit_diff": len(exit_diff),
    }, "e_only": e_only, "i_only": i_only, "exit_diff": exit_diff}


if __name__ == "__main__":
    main()
