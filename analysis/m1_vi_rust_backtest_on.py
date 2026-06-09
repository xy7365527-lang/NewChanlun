"""M1 Version I 串行回测 — 7 标的 1min 10 年期货（Rust O(N) 增量引擎）。

用户指令：V-I 串行回测 7 期货，**一次一个标的防 OOM**（单标的子进程，跑完退出释放
全部内存，再起下一个——与原 `m1_i_rust_backtest.py` 的 28 核并行相对）。

## 4 变体（用户指令）

| 变体    | 引擎调用                                          | 含义 |
|---------|--------------------------------------------------|------|
| E       | `run_swing_trading(MODE_NONE)`                   | 背驰定位器进出场基线，无降成本 |
| I       | `run_version_i(stop_mode="none")`                | Version I 完整版（多 FSM 多重赋格降成本，无止损） |
| I+stopA | `run_version_i(stop_mode="A")`                   | 硬止损 A：核心仓 `c < entry_price×0.98` 全仓清出 |
| I+stopB | `run_version_i(stop_mode="B")`                   | 止损 B：A ＋ 每 FSM 切片独立 cost_basis 止损（开放短差逆向亏 2% 回补冻结） |

floor = `MIN_FLOOR_LADDER`（canonical 默认 = bar，env `BT_FLOOR_LADDER` 可覆盖）；
maneuver_ratio = `MANEUVER_RATIO`（0.1）。三变体共用单次 `compute_i_signals_rust` pass。

## O(N) 增量声明

信号层 `compute_i_signals_rust` 内部已是 O(N) 全链增量：`orch.bi_zhongshu_new_signals`
（P1 delta 接口，Rust 内部维护四层增量器 + seen-set，替代原 `current_bi_zhongshu_
buysellpoints` 的 O(strokes²) 全量重算）；`orch.stroke_count()` O(1) 门控；
`orch.strokes_p1_since()` 仅 marshal 新增笔。逐位等价由 differential test 守卫。

认识论：移植正确性 L0/L1（管线 bit-exact）；回测结论 L2（真实数据，含否定性结果）。
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

import fugue_alpha_diagnosis as F  # noqa: E402
import m1_e_rust_engine as RE  # noqa: E402
from fugue_version_i import (  # noqa: E402
    MANEUVER_RATIO,
    MIN_FLOOR_LADDER,
    extended_metrics,
    ladder_name,
    run_version_i,
)
from m1_i_rust_engine import compute_i_signals_rust  # noqa: E402
from m1_i_rust_backtest import SYMBOL_FILES, _metric_row, load_ohlc  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = ROOT / "analysis" / "m1_vi_rust_results_on.json"
OUT_MD = ROOT / "analysis" / "m1_vi_rust_results_on.md"

# 用户指定顺序：ES → GC → CL → ZN → DX → BRN → 6E
SYMBOL_ORDER = ("ES", "GC", "CL", "ZN", "DX", "BRN", "6E")

# 4 变体：E 基线 + I（none）+ I+stopA（A）+ I+stopB（B）。floor 固定 canonical 默认。
_I_VARIANTS = (("I", "none"), ("I_stopA", "A"), ("I_stopB", "B"))


def _result_path(sym: str) -> Path:
    return DATA_DIR / f"vi_result_{sym}.json"


def run_symbol(symbol: str) -> tuple[str, dict]:
    path = SYMBOL_FILES[symbol]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    # ── E 基线（Rust 驱动）──
    t0 = time.time()
    e_signals = RE.compute_e_signals_rust(opens, highs, lows, closes)
    e_trades, _ = F.run_swing_trading(e_signals, F.MODE_NONE)
    em = extended_metrics(e_trades, years)
    e_s = time.time() - t0
    del e_signals, e_trades
    print(
        f"  [{symbol:4s}/E] {n:>9,} bars BH={bh:+9.2f}% | E复利={em['total_compound']:+11.2f}% "
        f"超额={em['total_compound']-bh:+11.2f}% 交易={em['n']:4d} 夏普={em['sharpe']:+.3f} "
        f"MDD={em['max_dd']:+7.2f}% | {e_s:.0f}s",
        flush=True,
    )

    # ── Version I 信号层（单次 O(N) Rust pass）→ 三止损变体复用 ──
    t1 = time.time()
    i_signals = compute_i_signals_rust(opens, highs, lows, closes)
    sig_s = time.time() - t1

    variants: dict[str, dict] = {}
    for tag, sm in _I_VARIANTS:
        i_trades, i_extra = run_version_i(
            i_signals, floor_ladder=MIN_FLOOR_LADDER, stop_mode=sm)
        im = extended_metrics(i_trades, years)
        variants[tag] = {
            "stop_mode": sm,
            "floor": ladder_name(MIN_FLOOR_LADDER),
            "metrics": _metric_row(im),
            "excess": round(im["total_compound"] - bh, 4),
            "fsm_contribution": i_extra["fsm_contribution"],
            "ladder_attribution": i_extra["ladder_attribution"],
            "addon_2buy_marks": i_extra["addon_2buy_marks"],
            "n_core_stops": i_extra["n_core_stops"],
            "n_voice_stops": i_extra["n_voice_stops"],
        }
        print(
            f"  [{symbol:4s}/{tag:8s} stop={sm:4s}] 复利={im['total_compound']:+11.2f}% "
            f"超额={im['total_compound']-bh:+11.2f}% 交易={im['n']:4d} 夏普={im['sharpe']:+.3f} "
            f"MDD={im['max_dd']:+7.2f}% 核心止损={i_extra['n_core_stops']} "
            f"voice止损={i_extra['n_voice_stops']}",
            flush=True,
        )
    i_s = time.time() - t1
    del i_signals

    return symbol, {
        "n_bars": n,
        "bh": round(bh, 4),
        "first": closes[0],
        "last": closes[-1],
        "floor": ladder_name(MIN_FLOOR_LADDER),
        "maneuver_ratio": MANEUVER_RATIO,
        "E": {"metrics": _metric_row(em), "excess": round(em["total_compound"] - bh, 4),
              "by_year": em.get("by_year", {})},
        "variants": variants,
        "e_seconds": round(e_s, 1),
        "i_signal_seconds": round(sig_s, 1),
        "i_seconds": round(i_s, 1),
    }


def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# M1 Version I 串行回测 — 7 标的 1min 10 年期货（Rust O(N) 增量引擎）\n")
    L.append(
        "> 一次一个标的子进程隔离（防 OOM）。4 变体：**E**（背驰定位器进出场基线，无降成本）"
        "/ **I**（Version I 完整版多 FSM 多重赋格降成本，无止损）/ **I+stopA**（硬止损 A：核心仓 "
        "`c < entry_price×0.98` 全仓清出）/ **I+stopB**（止损 B：A ＋ 每 FSM 切片独立 cost_basis "
        "止损）。信号层 O(N) 全链增量 Rust 引擎（`bi_zhongshu_new_signals` delta，逐位等价 "
        "`compute_signals_i`）。认识论 **L2**（真实数据，含否定性结果）。\n")
    L.append(
        f"> floor=`{ladder_name(MIN_FLOOR_LADDER)}`（MIN_FLOOR_LADDER={MIN_FLOOR_LADDER}，canonical "
        f"默认）；maneuver_ratio={MANEUVER_RATIO}；止损阈值 2%。三 I 变体共用单次信号 pass。\n")
    L.append("## E vs I × 止损（none/A/B）对照\n")
    L.append("| 标的 | bars | BH% | 变体 | 止损 | 复利% | 超额% | 夏普 | MDD% | 交易 | 核心止损 | voice止损 |")
    L.append("|------|------|-----|------|------|-------|-------|------|------|------|---------|----------|")
    for s in SYMBOL_ORDER:
        r = results.get(s)
        if not r:
            continue
        e = r["E"]["metrics"]
        L.append(
            f"| {s} | {r['n_bars']:,} | {r['bh']:+.1f} | E | — | {e['compound']:+.1f} | "
            f"{r['E']['excess']:+.1f} | {e['sharpe']:+.2f} | {e['max_dd']:+.1f} | {e['n']} | — | — |")
        for tag, _sm in _I_VARIANTS:
            v = r["variants"].get(tag)
            if not v:
                continue
            m = v["metrics"]
            L.append(
                f"| {s} | | | {tag} | {v['stop_mode']} | {m['compound']:+.1f} | "
                f"{v['excess']:+.1f} | {m['sharpe']:+.2f} | {m['max_dd']:+.1f} | {m['n']} | "
                f"{v['n_core_stops']} | {v['n_voice_stops']} |")
    L.append("")
    L.append("## 每级 FSM 独立贡献度（多重赋格各声部）— I（无止损）\n")
    L.append("| 标的 | 级别 | 回收现金 | 短差次数 | slice净增益 |")
    L.append("|------|------|---------|---------|-----------|")
    for s in SYMBOL_ORDER:
        r = results.get(s)
        if not r:
            continue
        v = r["variants"].get("I")
        if not v:
            continue
        for lvl, d in v["fsm_contribution"].items():
            L.append(f"| {s} | {lvl} | {d['recovered_cash']:+.0f} | {d['n_short_diffs']} | "
                     f"{d['slice_net_gain']:+.0f} |")
    L.append("")
    L.append("## 级别归属分布（揭示高层稀疏有效域）— I（无止损）\n")
    L.append("| 标的 | 归属(级别→笔数) | 2买标记 |")
    L.append("|------|----------------|--------|")
    for s in SYMBOL_ORDER:
        r = results.get(s)
        if not r:
            continue
        v = r["variants"].get("I")
        if not v:
            continue
        L.append(f"| {s} | {v['ladder_attribution']} | {v['addon_2buy_marks']} |")
    L.append("")
    L.append("## 耗时\n")
    L.append("| 标的 | bars | E耗时s | I信号s | I总耗时s |")
    L.append("|------|------|--------|--------|---------|")
    for s in SYMBOL_ORDER:
        r = results.get(s)
        if not r:
            continue
        L.append(f"| {s} | {r['n_bars']:,} | {r.get('e_seconds','-')} | "
                 f"{r.get('i_signal_seconds','-')} | {r.get('i_seconds','-')} |")
    OUT_MD.write_text("\n".join(L))


def _merge_and_write() -> None:
    """合并所有 vi_result_<sym>.json → 总 JSON + MD（按 SYMBOL_ORDER）。"""
    results: dict[str, dict] = {}
    for s in SYMBOL_ORDER:
        f = _result_path(s)
        if f.exists():
            try:
                results.update(json.loads(f.read_text()))
            except Exception:
                pass
    OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False, default=str))
    _write_report(results)


def main() -> None:
    # ── 单标的模式（子进程隔离）：跑一个标的 4 变体，写独立结果，退出释放全部内存 ──
    single = os.environ.get("BT_SINGLE")
    if single:
        sym = single.strip().upper()
        _, out = run_symbol(sym)
        _result_path(sym).write_text(
            json.dumps({sym: out}, ensure_ascii=False, default=str))
        print(f"  ✓ {sym} 写入 {_result_path(sym).name}", flush=True)
        return

    # ── 串行编排器：一次一个子进程（防 OOM），完成即合并刷新总表 ──
    import subprocess

    symbols = [s for s in SYMBOL_ORDER if SYMBOL_FILES.get(s) and SYMBOL_FILES[s].exists()]
    missing = [s for s in SYMBOL_ORDER if s not in symbols]
    if missing:
        print(f"⚠ 数据缺失，跳过：{missing}", flush=True)

    def _done(s: str) -> bool:
        return _result_path(s).exists()

    queue = [s for s in symbols if not _done(s)]
    print(f"M1 Version I 串行回测：{len(symbols)} 标的（一次一个，防 OOM）")
    print(f"顺序：{' → '.join(symbols)}")
    print(f"待跑：{queue}（已完成：{[s for s in symbols if _done(s)]}）\n", flush=True)

    logs_dir = ROOT / "analysis" / "logs"
    logs_dir.mkdir(exist_ok=True)
    t_all = time.time()
    failed: list[str] = []

    for s in queue:
        t0 = time.time()
        lf_path = logs_dir / f"vi_sym_{s}.log"
        env = {**os.environ, "BT_SINGLE": s}
        print(f"  ▶ {s} 启动（日志 {lf_path.name}）", flush=True)
        with open(lf_path, "w") as lf:
            rc = subprocess.call(
                [sys.executable, str(Path(__file__).resolve())],
                env=env, stdout=lf, stderr=subprocess.STDOUT)
        if rc == 0 and _done(s):
            _merge_and_write()
            print(f"  ✓ {s} 完成（rc=0，{time.time()-t0:.0f}s，累计 {time.time()-t_all:.0f}s）",
                  flush=True)
        else:
            failed.append(s)
            print(f"  ✗ {s} 失败（rc={rc}，见 {lf_path.name}）——隔离，继续下一标的", flush=True)

    _merge_and_write()
    print(f"\n总耗时 {time.time()-t_all:.0f}s")
    if failed:
        print(f"⚠ 失败标的（需单独诊断）：{failed}")
    print(f"报告：{OUT_MD}\n结果：{OUT_JSON}")


if __name__ == "__main__":
    main()
