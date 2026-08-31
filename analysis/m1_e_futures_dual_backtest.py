"""M1 多空双开对称扩展回测（#1309 · #1282 预注册判据，冻结，跑前不改一字）。

## 预注册判据（#1282 票面冻结，2026-08-29）

- H：操作层对称扩展至下跌侧（次级别顶背驰进空、L2 底背驰回补，联立门方向镜像），
  期货 7 标的 E 择时达到统计确立。
- 过判据（唯一）：**7/7 标的百分位全部越过各自随机中位**（符号检验单侧 p=0.0078）。
  不过 = M1 落档「期货对称扩展不可证」，同为有效结论。
- 对照设计（Q3）：①前后对照 = 原 E 单边（long-only）百分位 vs 多空双开百分位，
  同标的同窗配对，逐标的记变化量；②随机基线 = prereg-windows-v0 百分位法
  （含 #1281 补的 ZN/6E 窗口）= 真策略成绩在随机进出分布中的排位。
- 数据与窗口：prereg 表冻结值，7 标的（ES/GC/CL/ZN/6E/BRN/DX）10 年 1min，
  窗口照 prereg-windows-v0 + §9 补录。
- 纪律附款：跑前不搜索、不调参、不挑窗口；跑完结果原样入档；不做参数网格搜索、
  不加新对照品种、不中途改判据。

## 本票正确性口径

「正确」= 忠实执行预注册，不是「实验必须过」。过与不过都原样报。

## 实现

- 长腿 = 既有 `run_swing_trading(MODE_NONE)` 逐位等价（次级别底背驰进多 +
  L2 顶背驰离场）；空腿 = 联立门方向镜像（次级别顶背驰进空 + L2 底背驰回补），
  双腿各自独立满仓、可同时在场——`fugue_alpha_diagnosis.run_swing_trading_dual`。
- 百分位 = 随机双开基线（双腿各 N 笔 × 平均持有、entry 均匀随机，5000 次秩统计），
  与单边 P3 百分位法同构——`p3_random_gate_control.large_bootstrap_percentile_dual`。
- 每标的只跑一次 Rust 引擎（E 信号），长/空/双开三套 metrics + 两套百分位共享同一
  signals/trades；增量落盘防中断（resume 跳过已完成标的）。

认识论等级：移植正确性 L0/L1（长腿 bit-exact；空腿 = 同判据方向镜像）；回测结论
L2（真实数据 7 期货 1min，含否定性结果）。
"""
# 冻结声明：简化实验代理，已退役（#1312/#1315 实验结论以生产回放为准）；名分=研发档案，不参生产准入。

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as F  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_DIR = ROOT / ".chanlun" / "review-results"
OUT_JSON = OUT_DIR / "issue1309-dual-open-backtest.json"
OUT_MD = OUT_DIR / "issue1309-dual-open-backtest.md"

# 标的 → 10-16y 1min databento 数据文件（与 m1_e_futures_backtest.SYMBOL_FILES 逐项一致）。
# 本地镜像而非 import：m1_e_futures_backtest / m1_e_rust_engine / p3_random_gate_control
# 三个模块顶部都 import newchan_rust（Rust 扩展），沙盒未构建时会在 import 阶段先崩，
# 使 main() 的「缺数据 → 卡点报告 exit=2」路径不可达。数据文件被 gitignore、缺数据是
# 沙盒常态，缺数据卡点检查必须不依赖 Rust 扩展。
SYMBOL_FILES = {
    "ES": DATA_DIR / "es_1m_databento_10y.json",
    "GC": DATA_DIR / "gc_1m_databento_10y.json",
    "CL": DATA_DIR / "cl_1m_databento_10y.json",
    "ZN": DATA_DIR / "zn_1m_databento_10y.json",
    "BRN": DATA_DIR / "brn_1m_databento_10y.json",
    "6E": DATA_DIR / "usd6e_1m_databento_10y.json",
    "DX": DATA_DIR / "dx_1m_databento_10y.json",
}

# 7 标的（#1282 冻结表顺序）。
SYMBOL_ORDER = ("ES", "GC", "CL", "ZN", "6E", "BRN", "DX")

# 过判据（#1282 唯一）：7/7 标的百分位全部越过各自随机中位（符号检验单侧 p=0.0078）。
SIGN_TEST_P_7OF7 = 0.5 ** 7  # = 0.0078125

# P3 随机门控触发阈值：N < MIN_N_FOR_P3 时统计功效过低，不跑随机门控（同 m1_e_futures_backtest）。
MIN_N_FOR_P3 = 10

# 原 E 单边百分位（归档值，analysis/p3_futures_random_gate.md，2026-08-29 前已入档）——
# 仅用于报告对照列；前后对照主口径以本次同窗重算为准。
ARCHIVED_LONG_PCT = {
    "ES": 54, "GC": 60, "CL": 51, "ZN": 52, "6E": 46, "BRN": 56, "DX": 62,
}


def _out_path(sym: str) -> Path:
    return DATA_DIR / f"_m1e_dual_fut_{sym}.json"


def _avg_hold(trades: list[F.CompletedTrade]) -> float:
    return (
        sum(t.exit_bar - t.entry_bar for t in trades) / len(trades)
        if trades else 0.0
    )


def run_symbol(symbol: str) -> tuple[str, dict]:
    """单标的：load → Rust E 信号（一次）→ 长/空/双开 metrics + 两套百分位 → 落盘。"""
    out_path = _out_path(symbol)
    if out_path.exists():
        try:
            cached = json.loads(out_path.read_text())
            print(f"  [{symbol:4s}] 已完成（resume，跳过）", flush=True)
            return symbol, cached
        except (json.JSONDecodeError, OSError):
            pass  # 损坏 → 重跑

    # Rust 扩展依赖延迟到真正需要计算时才 import——缺数据时 main() 在 run_symbol 之前
    # 即报卡点退出，缺数据路径不依赖 newchan_rust 是否已构建。
    import m1_e_rust_engine as RE  # noqa: E402
    import p3_random_gate_control as P3  # noqa: E402
    from m1_e_futures_backtest import load_ohlc  # noqa: E402

    path = SYMBOL_FILES[symbol]
    opens, highs, lows, closes = load_ohlc(path)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    # ── E 信号（唯一 O(N²) 计算，长/空/双开共享）──
    t0 = time.time()
    print(f"  [{symbol:4s}] 启动：{n:,} bars，Rust E 信号计算中…", flush=True)
    signals = RE.compute_e_signals_rust(opens, highs, lows, closes)
    sig_s = time.time() - t0

    # ── 前后对照：原 E 单边（long-only）──
    long_only_trades, _ = F.run_swing_trading(signals, F.MODE_NONE)
    long_only_m = F.compute_metrics(long_only_trades)
    long_pct = None
    if long_only_m["n"] >= MIN_N_FOR_P3:
        long_pct = P3.large_bootstrap_percentile(
            closes, long_only_m["n"],
            int(round(_avg_hold(long_only_trades))),
            long_only_m["total_compound"],
        )

    # ── 多空双开（后）──
    dual_long, dual_short = F.run_swing_trading_dual(signals)
    dual_m = F.compute_dual_metrics(dual_long, dual_short)
    dual_pct = None
    if dual_m["n"] >= MIN_N_FOR_P3:
        dual_pct = P3.large_bootstrap_percentile_dual(
            closes,
            dual_m["n_long"], int(round(_avg_hold(dual_long))),
            dual_m["n_short"], int(round(_avg_hold(dual_short))),
            dual_m["total_compound"],
        )

    out: dict = {
        "symbol": symbol,
        "n_bars": n,
        "bh": bh,
        "sig_s": sig_s,
        "long_only": {
            "compound": long_only_m["total_compound"],
            "n_trades": long_only_m["n"],
            "win_rate": long_only_m["win_rate"],
            "max_dd": long_only_m["max_dd"],
            "avg_hold": _avg_hold(long_only_trades),
        },
        "long_only_p3": {
            "e_percentile": long_pct.e_percentile,
            "p_random_ge_e": long_pct.p_random_ge_e,
            "rnd_median": long_pct.rnd_median,
            "rnd_mean": long_pct.rnd_mean,
            "rnd_std": long_pct.rnd_std,
        } if long_pct else None,
        "dual": {
            "compound": dual_m["total_compound"],
            "long_compound": dual_m["long_compound"],
            "short_compound": dual_m["short_compound"],
            "n_trades": dual_m["n"],
            "n_long": dual_m["n_long"],
            "n_short": dual_m["n_short"],
            "win_rate": dual_m["win_rate"],
            "max_dd": dual_m["max_dd"],
            "avg_hold_long": _avg_hold(dual_long),
            "avg_hold_short": _avg_hold(dual_short),
        },
        "dual_p3": {
            "e_percentile": dual_pct.e_percentile,
            "p_random_ge_e": dual_pct.p_random_ge_e,
            "rnd_median": dual_pct.rnd_median,
            "rnd_mean": dual_pct.rnd_mean,
            "rnd_std": dual_pct.rnd_std,
        } if dual_pct else None,
        "dual_pct_minus_long_pct": (
            dual_pct.e_percentile - long_pct.e_percentile
            if dual_pct and long_pct else None
        ),
        # 过判据主口径：百分位 > 50 = 越过各自随机中位。
        "dual_beats_median": (
            dual_pct.e_percentile > 50.0 if dual_pct else None
        ),
    }
    out_path.write_text(json.dumps(out, indent=2, ensure_ascii=False))

    lp = out["long_only_p3"]
    dp = out["dual_p3"]
    lp_s = f"long%ile={lp['e_percentile']:.0f}" if lp else "long跳过"
    dp_s = f"dual%ile={dp['e_percentile']:.0f}" if dp else "dual跳过"
    delta = out["dual_pct_minus_long_pct"]
    delta_s = f"{delta:+5.1f}" if delta is not None else "—"
    print(
        f"  [{symbol:4s}] {n:>9,} bars | long复合={long_only_m['total_compound']:+10.2f}% "
        f"dual复合={dual_m['total_compound']:+10.2f}% | {lp_s} {dp_s} "
        f"Δ={delta_s} | 信号 {sig_s/60:.1f}min",
        flush=True,
    )
    return symbol, out


def _sign_test_p(n_pass: int, n_symbols: int) -> float:
    """单侧符号检验 p（至少 n_pass/n_symbols 跑赢中位，H0 = p=0.5）。"""
    from math import comb
    return sum(comb(n_symbols, k) for k in range(n_pass, n_symbols + 1)) / (2 ** n_symbols)


def write_report(results: dict[str, dict]) -> None:
    symbols = [s for s in SYMBOL_ORDER if s in results]
    n_pass = sum(1 for s in symbols if results[s]["dual_beats_median"] is True)
    n_total = len(symbols)
    verdict = "过（7/7 全部越过随机中位）" if n_pass == n_total else "不过（如实入档）"

    L: list[str] = []
    L.append("# #1309 多空双开对称扩展回测结果（#1282 预注册判据，冻结）\n")
    L.append(
        "> 7 标的（ES/GC/CL/ZN/6E/BRN/DX）1min 全量（prereg 表冻结值）| 零交易成本 | "
        "过判据 = 7/7 百分位全部越过各自随机中位（符号检验单侧 p=0.0078）。\n"
    )
    L.append(
        "> 本票「正确」= 忠实执行预注册，不是「实验必须过」——过与不过都原样报。\n"
    )
    L.append("\n## 0. 判决\n")
    L.append(
        f"**{n_pass}/{n_total} 标的百分位越过随机中位** → 判定：**{verdict}**。"
        f"（7/7 符号检验单侧 p = {SIGN_TEST_P_7OF7:.4f}；本次实际命中 {n_pass}/{n_total}，"
        f"单侧 p = {_sign_test_p(n_pass, n_total):.4f}）\n"
    )

    L.append("\n## 1. 多空双开百分位（主判据）\n")
    L.append(
        "| 标的 | Bars | dual复利% | 长腿复利% | 空腿复利% | 交易(长/空) | "
        "P(随机≥真实) | 双开百分位 | 随机中位% | 越过中位 |\n"
        "|------|------|----------|-----------|-----------|-------------|"
        "-------------|-----------|----------|----------|"
    )
    for sym in symbols:
        r = results[sym]
        d = r["dual"]
        dp = r["dual_p3"]
        if dp is None:
            row = (
                f"| {sym} | {r['n_bars']:,} | {d['compound']:+.2f} | "
                f"{d['long_compound']:+.2f} | {d['short_compound']:+.2f} | "
                f"{d['n_long']}/{d['n_short']} | — | — | — | N<{MIN_N_FOR_P3} 跳过 |"
            )
        else:
            beat = "✅" if r["dual_beats_median"] else "❌"
            row = (
                f"| {sym} | {r['n_bars']:,} | {d['compound']:+.2f} | "
                f"{d['long_compound']:+.2f} | {d['short_compound']:+.2f} | "
                f"{d['n_long']}/{d['n_short']} | {dp['p_random_ge_e']:.1%} | "
                f"{dp['e_percentile']:.0f} | {dp['rnd_median']:+.2f} | {beat} |"
            )
        L.append(row)

    L.append("\n## 2. 前后对照（Q3.1：原 E 单边百分位 vs 双开百分位，同标的同窗配对）\n")
    L.append(
        "| 标的 | 原E单边百分位（本次重算） | 归档原值 | 双开百分位 | 变化量 |\n"
        "|------|--------------------------|----------|-----------|--------|"
    )
    for sym in symbols:
        r = results[sym]
        lp = r["long_only_p3"]
        dp = r["dual_p3"]
        lp_s = f"{lp['e_percentile']:.0f}" if lp else "—"
        dp_s = f"{dp['e_percentile']:.0f}" if dp else "—"
        delta = r["dual_pct_minus_long_pct"]
        delta_s = f"{delta:+.1f}" if delta is not None else "—"
        arch = ARCHIVED_LONG_PCT.get(sym, "—")
        L.append(f"| {sym} | {lp_s} | {arch} | {dp_s} | {delta_s} |")

    L.append("\n## 3. 预注册冻结文本逐条对照（#1282 票面）\n")
    L.append(
        "| 冻结项 | 票面 | 本跑执行 | 核对 |\n"
        "|--------|------|----------|------|\n"
        "| 假设 H | 操作层对称扩展至下跌侧（次级别顶背驰进空、L2 底背驰回补、联立门方向镜像） | "
        "`run_swing_trading_dual`：空腿 = up_move_settled∧exit_div_ok 进空、l2_flip_long∧entry_div_ok 回补 | ✅ |\n"
        "| 过判据 | 7/7 标的百分位全部越过各自随机中位（符号检验单侧 p=0.0078） | "
        "每标的分位 >50 计数，7/7 判定 + p=0.0078125 | ✅ |\n"
        "| 前后对照 | 原E单边百分位 vs 双开百分位，同标的同窗配对逐标记变化量 | "
        "同 signals 同窗重算两套百分位，逐标 Δ | ✅ |\n"
        "| 随机基线 | prereg-windows-v0 百分位法（含 #1281 补 ZN/6E 窗），真策略在随机进出分布中的排位 | "
        "`large_bootstrap_percentile_dual`：双腿各 N×H 随机进出 5000 次 | ✅ |\n"
        "| 数据与窗口 | 7 标的 10 年 1min，窗口照 prereg-windows-v0 + §9 补录 | "
        "SYMBOL_FILES 全量 1min（prereg 表冻结值，与归档单边同窗） | ✅ |\n"
        "| 纪律附款 | 不搜索/不调参/不挑窗/不加对照/不改判据 | "
        "判据与窗口零改动，结果原样入档 | ✅ |\n"
    )

    L.append("\n## 4. 纪律附款执行记录\n")
    L.append(
        "- 跑前不搜索、不调参、不挑窗口：判据/窗口逐字取自 #1282 冻结文本与 "
        "prereg-windows-v0 冻结表，未改一字。\n"
        "- 不做参数网格搜索、不加新对照品种、不中途改判据。\n"
        "- 结果原样入档（本文件 + issue1309-dual-open-backtest.json）。\n"
    )

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    OUT_MD.write_text("\n".join(L) + "\n")
    OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))


def main() -> int:
    missing = [s for s in SYMBOL_ORDER if not SYMBOL_FILES.get(s, Path("")).exists()]
    if missing:
        print(
            "卡点：缺少期货 1min 数据文件（Databento 外部依赖，沙盒无 DATABENTO_API_KEY）：",
            flush=True,
        )
        for s in missing:
            print(f"  - {SYMBOL_FILES[s]}", flush=True)
        print(
            "解除条件：设置 DATABENTO_API_KEY / DATABENTO_KEY 后运行 "
            "`PYTHONPATH=src uv run python scripts/fetch_1m_databento_10y.py all`，"
            "并构建 Rust 扩展 `cd rust && uv run maturin develop --release`。"
            "续跑命令见 #1309 票面卡点报告。",
            flush=True,
        )
        return 2

    results: dict[str, dict] = {}
    t_all = time.time()
    for sym in SYMBOL_ORDER:
        _, out = run_symbol(sym)
        results[sym] = out
    elapsed = time.time() - t_all
    print(f"\n总耗时 {elapsed/60:.1f}min\n")

    write_report(results)
    print(f"汇总 JSON：{OUT_JSON}")
    print(f"报告 MD：{OUT_MD}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
