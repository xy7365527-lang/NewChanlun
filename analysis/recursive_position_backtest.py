"""递归建仓回测 — RecursivePosition vs V2oa25 基线 × OKLO+BRN。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 编排者任务：递归建仓形式化+实验）
═══════════════════════════════════════════════════════════════════════
核心思想：级别不是事前知道的，是随结构涌现被追溯确认的。仓位管理反映
这个过程——量 = 确认深度的递增函数：
  confirmed 买点@k → slice k 填至 quota_k = base_frac × weight(k)
  confirmed 卖点@k → slice k 清至 0
无 FSM、无相位、无配对——唯一状态是 per-slice 持仓（资源状态）。
master/voice 是同一条规则在不同级别上的实例。

变体（base_frac 归一基 = 涌现塔到 ladder 6（recL4）的 5 个承载层假设；
超出由现金饱和截断、不足闲置，两侧可观测）：
  R_exp = exp2 权重，base=1/31（2^0+…+2^4 = 31）
  R_lin = linear 权重，base=1/15（1+…+5 = 15）
基线：V2oa25（在册最优实盘配置 = P5 域腿 + REV 配对 + θ 自适应默认门）。

守卫：
  1. O0≡P5 自动满足——run_recursive_rust 是独立入口，run_organic 路径
     零接触（cargo test 122 项全过）。
  2. 磁带指纹守卫（tape_fp）：与在册 cycle38 任务同口径。

认识论等级：设计 L0；本回测 OKLO+BRN 双标的 = L2（首验）。

输出：analysis/data_cache/recursive_position_backtest.json
用法：PYTHONPATH=src .venv/bin/python analysis/recursive_position_backtest.py
      （env：BT_SYMBOLS=OKLO,BRN  BT_FORCE=1 强制重跑）
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG, extended_metrics  # noqa: E402
from organic_fugue_rust_backtest import _trades_from_rust  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "recursive_position_backtest.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,BRN").split(",")]
FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))

# (名, base_frac, weight, sell_t1_only)
# 预注册臂：R_exp/R_lin（归一基 = 5 承载层 ladder 2..6 满仓假设）。
# 探索臂（首验否证后 post-hoc，数据挖掘风险显式标注）：
#   _s1  = 卖侧只消费 confirmed Sell1（type2/3 卖点是结构内短差点，
#          全清语义把每个卖点都当趋势终结——死因候选1：暴露塌缩）
#   R_exp4 = 归一基修正为实际涌现塔 4 层 ladder 2..5（2⁰+…+2³=15）
#          ——死因候选2：L6 quota 51.6% 永久闲置
REC_VARIANTS = [
    ("R_exp", 1.0 / 31.0, "exp2", False),
    ("R_lin", 1.0 / 15.0, "linear", False),
    ("R_exp_s1", 1.0 / 31.0, "exp2", True),
    ("R_exp4", 1.0 / 15.0, "exp2", False),
    ("R_exp4_s1", 1.0 / 15.0, "exp2", True),
]
BASELINE = "V2oa25"


def run_baseline(rtape, years: float, bh: float) -> dict:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, BASELINE, floor_ladder=FLOOR,
                              stop_mode="none", diag=False)
    el = time.time() - t0
    trades = _trades_from_rust(res["trades"])
    m = extended_metrics(trades, years)
    return {
        "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                      "sharpe", "max_dd")},
        "delta_vs_bh": round(m["total_compound"] - bh, 1),
        "elapsed_s": round(el, 3),
    }


def run_recursive_cell(rtape, name: str, base_frac: float, weight: str,
                       sell_t1_only: bool, n_bars: int, bh: float) -> dict:
    t0 = time.time()
    res = nr.run_recursive_rust(rtape, floor_ladder=FLOOR,
                                base_frac=base_frac, weight=weight,
                                sell_t1_only=sell_t1_only,
                                with_fills=True)
    el = time.time() - t0
    fills = res["fills"]
    n_fills = len(fills)
    # 建仓/减仓过程读数：per-ladder 买卖计数 + 同时持仓 slice 数分布
    per_ladder = {}
    for lad in range(FLOOR, 11):
        bf, sc = res["buy_fills"][lad], res["sell_clears"][lad]
        if bf or sc or res["buy_noops"][lad] or res["sell_noops"][lad]:
            per_ladder[str(lad)] = {
                "buy_fills": bf,
                "buy_noops": res["buy_noops"][lad],
                "buy_starved": res["buy_starved"][lad],
                "sell_clears": sc,
                "sell_noops": res["sell_noops"][lad],
            }
    # 换手率年化口径：总换手 / 年数
    return {
        "final_return_pct": round(res["final_return_pct"], 2),
        "delta_vs_bh": round(res["final_return_pct"] - bh, 1),
        "max_dd_pct": round(res["max_dd_pct"], 2),
        "avg_invested_frac": round(res["avg_invested_frac"], 4),
        "final_invested_frac": round(res["final_invested_frac"], 4),
        "turnover": round(res["turnover"], 1),
        "n_fills": n_fills,
        "per_ladder": per_ladder,
        "base_frac": round(base_frac, 5),
        "weight": weight,
        "sell_t1_only": sell_t1_only,
        "elapsed_s": round(el, 3),
    }


def process_symbol(symbol: str, prev: dict | None = None) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} — 递归建仓 vs {BASELINE}（floor={FLOOR}）"
          f"\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    span = (f"{years[0]}-{years[-1]}" if years else "?")
    print(f"  bars={n:,}  BH={bh:+.2f}%  span={span}", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    sig_s = time.time() - t0
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    if prev is not None and "tape_fp" in prev and prev["tape_fp"] != fp:
        raise SystemExit(f"磁带指纹不匹配：在册 {prev['tape_fp']} vs 本次 {fp}")
    print(f"  信号层 {sig_s:.1f}s  fp={fp}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)

    out = prev if prev is not None else {}
    out.update({"n_bars": n, "bh": round(bh, 2), "span": span,
                "sig_elapsed": round(sig_s, 1), "tape_fp": fp,
                "floor": FLOOR})
    out.setdefault("cells", {})
    force = os.environ.get("BT_FORCE", "0") == "1"

    if BASELINE not in out["cells"] or force:
        out["cells"][BASELINE] = run_baseline(rtape, years, bh)
    b = out["cells"][BASELINE]
    print(f"  [{BASELINE:8s}] 复利={b['metrics']['total_compound']:+10.2f}%"
          f" Δ(BH)={b['delta_vs_bh']:+8.1f}pp 笔={b['metrics']['n']}",
          flush=True)

    for name, base_frac, weight, s1 in REC_VARIANTS:
        if name not in out["cells"] or force:
            out["cells"][name] = run_recursive_cell(
                rtape, name, base_frac, weight, s1, n, bh)
        p = out["cells"][name]
        print(f"  [{name:8s}] 收益={p['final_return_pct']:+10.2f}%"
              f" Δ(BH)={p['delta_vs_bh']:+8.1f}pp"
              f" mdd={p['max_dd_pct']:.1f}%"
              f" 暴露={p['avg_invested_frac']:.2f}"
              f" 换手={p['turnover']:.0f}x"
              f" fills={p['n_fills']:,}"
              f" [{p['elapsed_s']:.2f}s]", flush=True)
        for lad, row in p["per_ladder"].items():
            print(f"      L{lad}: buy={row['buy_fills']}"
                  f" noop={row['buy_noops']} starved={row['buy_starved']}"
                  f" | sell={row['sell_clears']} noop={row['sell_noops']}",
                  flush=True)
    return out


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    wanted = [BASELINE] + [v[0] for v in REC_VARIANTS]
    for sym in SYMBOLS:
        done = (sym in results
                and all(v in results[sym].get("cells", {}) for v in wanted))
        if done and os.environ.get("BT_FORCE", "0") != "1":
            print(f"[skip] {sym} 已有全部 cell", flush=True)
            continue
        results[sym] = process_symbol(sym, prev=results.get(sym))
        OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))
        print(f"  [{sym}] 已落盘", flush=True)


if __name__ == "__main__":
    main()
