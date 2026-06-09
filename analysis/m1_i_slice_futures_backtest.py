"""期货全标的 Version I（slice 模型）回测 + E vs I vs BH 对比。

## 用户追加要求（2026-06-09）

在 E 回测 + P3 之外，每个期货品种也跑 Version I 回测，对比 **E vs I vs BH** 三者。
**Version I 用原版 slice 模型**（核心仓全仓不做短差 + 机动仓 MANEUVER_RATIO 切 disjoint
slice 分给各级别独立 FSM），**非** 2026-06-09 重构的共享仓位（shared）版本。

## slice 隔离（不碰工作区 shared）

工作区 `fugue_version_i.py` 当前是 shared 模型（未提交改动）。为多 session 安全，
本脚本用 `fugue_version_i_slice`（从 `git show HEAD:` 提取的 slice 版独立副本），
**不修改工作区**——shared 改动保留给其他工位。O(N) 引擎 `compute_i_signals_rust`
产出的 `BarSignalI`（字段在 slice/shared 版完全一致）由 slice 版 `run_version_i` 消费
（鸭子类型，smoke 验证通过）。

## O(N²) 诚实声明（关键）

信号层 `compute_i_signals_rust` 的笔中枢路径已用 `bi_zhongshu_new_signals` 增量化
（O(N)），**但** 两堵 O(N²) 墙仍在、且不可由接口切换消除：
  1. `orch.process_bar` 内部 **segment 层全量重算**（E/I 共有；segment 增量 resume 有
     bug `project_segment_resume_regression`，不能用）；
  2. `orch.current_buysellpoints()` **走势级每 bar 调用**（`project_bi_zhongshu_on2_two_walls`
     记载的第二堵墙，尚未增量化）。
故 I 信号 ≈ 2.85× E 信号（ES 5.59M：E 59min / I 169min）。这是引擎根本，非"用错接口"。

## E 复用

E 基线复用 `m1_e_futures_backtest` 已落盘的 `_m1e_fut_{sym}.json`（同一
`compute_e_signals_rust`，逐字一致），**不重跑 E**（省 ~40min/标的）。

认识论：移植正确性 L0/L1（slice run_version_i = git HEAD bit-exact，引擎 O(N) 增量
differential-test 守卫）；回测结论 L2（真实数据，7 期货标的 1min 全量；含否定性结果）。
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

import fugue_version_i_slice as VI  # noqa: E402  slice 版（git HEAD 副本）
import m1_e_futures_backtest as EM  # noqa: E402  复用 load_ohlc / SYMBOL_FILES
from m1_i_rust_engine import compute_i_signals_rust  # noqa: E402  O(N) 笔中枢引擎

DATA_DIR = ROOT / "analysis" / "data_cache"

# slice-I 三变体（与 m1_vi shared 回测同口径，便于 slice vs shared 对照）：
#   I_none = 无止损（原版 Version I slice）；I_stopA = 硬止损 A；I_stopB = A + 切片独立止损。
I_VARIANTS = (("I_none", "none"), ("I_stopA", "A"), ("I_stopB", "B"))


def _out_path(sym: str) -> Path:
    return DATA_DIR / f"_i_slice_fut_{sym}.json"


def _e_result(sym: str) -> dict:
    """读复用的 E 结果（m1_e_futures_backtest 落盘）。缺失则返回空（回测仍出 I/BH）。"""
    p = DATA_DIR / f"_m1e_fut_{sym}.json"
    if p.exists():
        return json.loads(p.read_text())
    return {}


def run_symbol(symbol: str) -> tuple[str, dict]:
    """单标的：复用 E → O(N) I 信号 → slice run_version_i ×3 变体 → 增量落盘。"""
    out_path = _out_path(symbol)
    if out_path.exists():
        try:
            cached = json.loads(out_path.read_text())
            print(f"  [{symbol:4s}] slice-I 已完成（resume，跳过）", flush=True)
            return symbol, cached
        except (json.JSONDecodeError, OSError):
            pass

    path = EM.SYMBOL_FILES[symbol]
    opens, highs, lows, closes = EM.load_ohlc(path)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    e = _e_result(symbol)
    e_compound = e.get("compound")
    e_excess = e.get("excess")
    e_n = e.get("n_trades")
    e_mdd = e.get("max_dd")

    # ── Version I 信号层（单次 O(N) 笔中枢 + O(N²) segment/走势 pass）──
    t1 = time.time()
    print(f"  [{symbol:4s}] slice-I 信号计算中：{n:,} bars（I≈2.85×E，segment 墙）…",
          flush=True)
    i_signals = compute_i_signals_rust(opens, highs, lows, closes)
    sig_s = time.time() - t1

    variants: dict[str, dict] = {}
    for tag, sm in I_VARIANTS:
        trades, extra = VI.run_version_i(
            i_signals, floor_ladder=VI.MIN_FLOOR_LADDER, stop_mode=sm)
        m = VI.extended_metrics(trades)
        variants[tag] = {
            "stop_mode": sm,
            "compound": m["total_compound"],
            "excess": m["total_compound"] - bh,
            "n_trades": m["n"],
            "win_rate": m["win_rate"],
            "sharpe": m["sharpe"],
            "max_dd": m["max_dd"],
            "n_core_stops": extra.get("n_core_stops", 0),
            "n_voice_stops": extra.get("n_voice_stops", 0),
            "ladder_attribution": extra.get("ladder_attribution", {}),
            "addon_2buy_marks": extra.get("addon_2buy_marks", 0),
        }
        print(
            f"  [{symbol:4s}/{tag:8s}] 复利={m['total_compound']:+11.2f}% "
            f"超额={m['total_compound']-bh:+11.2f}% 交易={m['n']:5d} "
            f"夏普={m['sharpe']:+.3f} MDD={m['max_dd']:+7.2f}% "
            f"核心止损={extra.get('n_core_stops',0)}",
            flush=True,
        )
    del i_signals

    out = {
        "symbol": symbol,
        "n_bars": n,
        "bh": bh,
        "model": "slice",
        "floor": VI.ladder_name(VI.MIN_FLOOR_LADDER),
        "maneuver_ratio": VI.MANEUVER_RATIO,
        "E": {"compound": e_compound, "excess": e_excess,
              "n_trades": e_n, "max_dd": e_mdd},
        "variants": variants,
        "i_signal_seconds": round(sig_s, 1),
    }
    out_path.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol:4s}] ✓ 写入 {out_path.name}（I 信号 {sig_s/60:.1f}min）", flush=True)
    return symbol, out


def main() -> None:
    symbols = [s for s in EM.SYMBOL_FILES if EM.SYMBOL_FILES[s].exists()]
    only = os.environ.get("BT_SYMBOLS")
    if only:
        symbols = [s.strip().upper() for s in only.split(",") if s.strip()]

    # I 信号内存大（5.5M BarSignalI 含 6 tuple/bar）——并行度默认 6 防 OOM（96GB）。
    workers = int(os.environ.get("BT_WORKERS", "6"))
    workers = min(workers, len(symbols))
    print(f"slice-I 期货回测：{len(symbols)} 标的，{workers} 进程并行（E 复用，不重跑）")
    print(f"标的：{', '.join(symbols)}\n")

    results: dict[str, dict] = {}
    t_all = time.time()
    if workers > 1 and len(symbols) > 1:
        from concurrent.futures import ProcessPoolExecutor

        with ProcessPoolExecutor(max_workers=workers) as ex:
            for sym, out in ex.map(run_symbol, symbols):
                results[sym] = out
    else:
        for sym in symbols:
            _, out = run_symbol(sym)
            results[sym] = out

    elapsed = time.time() - t_all
    print(f"\n总耗时 {elapsed/60:.1f}min\n")

    agg = DATA_DIR / "m1_i_slice_futures_results.json"
    agg.write_text(json.dumps(results, indent=2, ensure_ascii=False))
    print(f"汇总 JSON：{agg}")


if __name__ == "__main__":
    main()
