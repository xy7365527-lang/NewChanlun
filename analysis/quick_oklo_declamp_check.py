"""快速验证：A1 去截断后 OKLO 单标的（审计铁证）收益是否崩塌。

OKLO 无加仓版修复前 +652.27%（含截断提款机）。本脚本用去截断后的同一引擎重跑
OKLO，输出 BH/FSM/夏普，直接检验 A1 修复效果。认识论等级 L2。
"""
from __future__ import annotations
import importlib.util, sys, time, json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

spec = importlib.util.spec_from_file_location(
    "fugue_no_addon", ROOT / "analysis" / "fugue_no_addon_backtest_1min.py")
na = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = na
spec.loader.exec_module(na)


def sharpe(pnls):
    n = len(pnls)
    if n < 2:
        return 0.0
    m = sum(pnls) / n
    sd = (sum((x - m) ** 2 for x in pnls) / (n - 1)) ** 0.5
    return m / sd if sd > 0 else 0.0


def main():
    opens, highs, lows, closes, dates = na.load_1min("OKLO")
    n = len(closes)
    print(f"OKLO {n:,} bars")
    t0 = time.time()
    trades, *_ = na.run_backtest(opens, highs, lows, closes)
    el = time.time() - t0
    m = na.compute_metrics(trades)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    pnls = [t.pnl_pct for t in trades]
    out = {
        "symbol": "OKLO", "n_bars": n, "bh": bh,
        "fsm_compound": m["total_compound"], "win_rate": m["win_rate"],
        "n_trades": m["n"], "sharpe": sharpe(pnls), "pnls": pnls,
        "n_earn_shares": m.get("n_earn_shares", 0),
        "max_shares_growth": m.get("max_shares_growth", 1.0),
        "elapsed": el,
    }
    (ROOT / "analysis" / "quick_oklo_declamp_result.json").write_text(json.dumps(out, indent=2))
    print(f"\n=== OKLO 去截断 + 挣股数 ({el:.0f}s) ===")
    print(f"BH:   {bh:+.2f}%")
    print(f"FSM:  {m['total_compound']:+.2f}%  (修复前含截断: +652.27%; A1-only: +114.08%)")
    print(f"超额: {m['total_compound']-bh:+.2f}%")
    print(f"夏普: {sharpe(pnls):+.3f}, 胜率 {m['win_rate']:.0f}%, 交易 {m['n']}笔")
    print(f"挣股数: {m.get('n_earn_shares', 0)}笔进入挣股数阶段, "
          f"最大持仓增长 {m.get('max_shares_growth', 1.0):.2f}×")


if __name__ == "__main__":
    main()
