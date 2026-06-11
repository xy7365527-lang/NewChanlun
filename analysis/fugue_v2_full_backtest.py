"""V-I 多重赋格 v2 全标的回测 — v2 vs E vs BH（Rust 引擎全 O(N) 驱动）。

═══════════════════════════════════════════════════════════════════════
v2 = 共享仓位多声部并发短差（用户 2026-06-09 裁定，否定旧 slice 模型）
═══════════════════════════════════════════════════════════════════════
- 入场一次满仓 100%（total_shares = INITIAL_CAPITAL/price，唯一一份共享仓位）。
- 多 FSM 各管各：entry 层以下每个 ladder 独立维护短差生命周期（多槽 `_SharedFugue.active`），
  全部作用于**同一满仓仓位**，每个短差闭合 profit 直接写入共享 cost_basis。
- 操作规模按子级别数均分总仓位（level_frac = 1/N_sub），非旧 10% 机动仓 base/step 魔数。
实现在 `fugue_version_i.run_version_i` / `_SharedFugue`（已是 v2 共享版）。

信号层全部 Rust 引擎驱动（O(N)，bi 原地维护 + segment resume + trend delta + segment 级增量化）：
  - E：`m1_e_rust_engine.compute_e_signals_rust`（逐位等价，已验证）。
  - I：`m1_i_rust_engine.compute_i_signals_rust`（逐位等价于 `compute_signals_i`，
        differential test PASS）。bi_zhongshu_new_signals/trend_new_signals/recursive_epoch
        全部下沉 Rust delta 接口 → 消除旧 O(strokes²) 墙 → 端到端 O(N)。

认识论等级：移植正确性 L0/L1（管线 bit-exact）；回测结论 L2（真实数据 1min 全历史）。
力度口径=价格振幅 fallback（非 MACD，O(N²) 不可行，诚实声明非阉割）。

三 floor 变体（含 bar / 线段起 / 缠师走势级口径）让数据经验裁决 bar 级是否噪音
（formalization-validity-domain：有效域可能 < 定义域）。⚠ 缠师原文（第53/35/31课）反对
bar 级短差；若 I_bar0 < I_move3 → 经验印证缠师原文。

输出：analysis/fugue_v2_full_backtest.md / data_cache/v2_full_backtest_results.json。
"""

from __future__ import annotations

import json
import math
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as F  # noqa: E402
import m1_e_rust_engine as RE  # noqa: E402
from fugue_version_i import (  # noqa: E402
    LADDER_MOVE,
    LADDER_SEG,
    extended_metrics,
    ladder_name,
    run_version_i,
)
from m1_i_rust_engine import compute_i_signals_rust  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "v2_full_backtest_results.json"
OUT_MD = ROOT / "analysis" / "fugue_v2_full_backtest.md"

# 标的 → 数据文件（OKLO=447K bars-schema 有时间戳；其余 parallel-array）。
SYMBOL_FILES = {
    "OKLO": DATA_DIR / "oklo_1m_databento.json",
    "ES": DATA_DIR / "es_1m_databento_10y.json",
    "CL": DATA_DIR / "cl_1m_databento_10y.json",
    "GC": DATA_DIR / "gc_1m_databento_10y.json",
    "ZN": DATA_DIR / "zn_1m_databento_10y.json",
    "DX": DATA_DIR / "dx_1m_databento_10y.json",
    "6E": DATA_DIR / "usd6e_1m_databento_10y.json",
    "BRN": DATA_DIR / "brn_1m_databento_10y.json",
    "BTC": DATA_DIR / "btc_1m_full.json",
    "QQQ": DATA_DIR / "qqq_1m_databento_full.json",
    "PINS": DATA_DIR / "pins_1m_databento_full.json",
}
ORDER = list(SYMBOL_FILES.keys())

# v2 三 floor 变体（同一信号 pass）：含 bar / 线段起 / 缠师走势级口径。
_VARIANTS = (("I_bar0", 0), ("I_seg2", LADDER_SEG), ("I_move3", LADDER_MOVE))


def load_ohlc(path: Path):
    """统一加载：bars-schema（{"bars":[{ts,open,...}]}）或 parallel-array（opens/highs/...）。

    删除任一 OHLC 为 nan 或 ≤0 的 bar（污染 PH/MACD 下游 + 令价格阈值止损误触发），
    dates/years 同步对齐（by_year 需要；无时间戳标的 years=None）。
    """
    raw = json.loads(path.read_text())
    if "bars" in raw:  # bars-schema（有真实 ts）
        bars = raw["bars"]
        o_in = [b["open"] for b in bars]
        h_in = [b["high"] for b in bars]
        l_in = [b["low"] for b in bars]
        c_in = [b["close"] for b in bars]
        d_in = [b["ts"] for b in bars]
    else:  # parallel-array
        o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
        d_in = raw.get("dates")

    _max = int(os.environ.get("BT_MAX_BARS", "0"))
    if _max > 0:  # 冒烟测试：限制 bar 数
        o_in, h_in, l_in, c_in = o_in[:_max], h_in[:_max], l_in[:_max], c_in[:_max]
        if d_in is not None:
            d_in = d_in[:_max]

    opens: list[float] = []
    highs: list[float] = []
    lows: list[float] = []
    closes: list[float] = []
    years: list[int] = []
    for idx in range(len(c_in)):
        o, h, l, c = float(o_in[idx]), float(h_in[idx]), float(l_in[idx]), float(c_in[idx])
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        if o <= 0 or h <= 0 or l <= 0 or c <= 0:
            continue
        opens.append(o)
        highs.append(h)
        lows.append(l)
        closes.append(c)
        if d_in is not None:
            years.append(int(str(d_in[idx])[:4]))

    # 孤立坏 bar 清洗（spike-and-revert）：close 相对前一 bar 跳变 >50% 且后一
    # bar 回到前一 bar ±5% 内 → 数据源坏 tick（BRN 2024-02-19 17:41 OHLC=0.08）。
    # 真实极端行情不满足该合取：CL 2020-04 负油价周为连续下跌（后 bar 不回归），
    # 周日开盘 gap 实测 ≤36%（不过 50% 阈值）。
    drop = {i for i in range(1, len(closes) - 1)
            if abs(closes[i] / closes[i - 1] - 1) > 0.5
            and abs(closes[i + 1] / closes[i - 1] - 1) < 0.05}
    if drop:
        keep = [i for i in range(len(closes)) if i not in drop]
        opens = [opens[i] for i in keep]
        highs = [highs[i] for i in keep]
        lows = [lows[i] for i in keep]
        closes = [closes[i] for i in keep]
        years = [years[i] for i in keep] if d_in is not None else years
    return opens, highs, lows, closes, (years if d_in is not None else None)


def _metric_row(m: dict) -> dict:
    return {
        "n": m["n"],
        "win_rate": m["win_rate"],
        "compound": m["total_compound"],
        "sharpe": m["sharpe"],
        "max_dd": m["max_dd"],
        "profit_factor": (None if m["profit_factor"] == float("inf")
                          else round(m["profit_factor"], 3)),
        "n_with_cr": m.get("n_with_cr", 0),
    }


def run_symbol(symbol: str) -> tuple[str, dict]:
    path = SYMBOL_FILES[symbol]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    # ── E 基线（Rust 驱动，O(N)）──
    t0 = time.time()
    e_signals = RE.compute_e_signals_rust(opens, highs, lows, closes)
    e_trades, _ = F.run_swing_trading(e_signals, F.MODE_NONE)
    em = extended_metrics(e_trades, years)
    e_s = time.time() - t0
    del e_signals
    print(
        f"  [{symbol:4s}/E] {n:>9,} bars BH={bh:+9.2f}% | E复利={em['total_compound']:+12.2f}% "
        f"超额={em['total_compound']-bh:+12.2f}% 交易={em['n']:4d} 夏普={em['sharpe']:+.3f} "
        f"MDD={em['max_dd']:+7.2f}% | {e_s:.0f}s",
        flush=True,
    )

    # ── v2 完整版（Rust 驱动，O(N)）：单次信号 pass → 三 floor 变体 ──
    t1 = time.time()
    i_signals = compute_i_signals_rust(opens, highs, lows, closes)
    sig_s = time.time() - t1

    variants: dict[str, dict] = {}
    for tag, floor in _VARIANTS:
        i_trades, i_extra = run_version_i(i_signals, floor_ladder=floor, stop_mode="none")
        im = extended_metrics(i_trades, years)
        variants[tag] = {
            "floor": ladder_name(floor),
            "metrics": _metric_row(im),
            "excess": round(im["total_compound"] - bh, 4),
            "fsm_contribution": i_extra["fsm_contribution"],
            "ladder_attribution": i_extra["ladder_attribution"],
            "addon_2buy_marks": i_extra["addon_2buy_marks"],
        }
        print(
            f"  [{symbol:4s}/{tag} floor={ladder_name(floor):8s}] "
            f"复利={im['total_compound']:+12.2f}% 超额={im['total_compound']-bh:+12.2f}% "
            f"交易={im['n']:4d} 夏普={im['sharpe']:+.3f} MDD={im['max_dd']:+7.2f}% "
            f"降成本笔={im.get('n_with_cr',0)}",
            flush=True,
        )
    i_s = time.time() - t1
    del i_signals

    return symbol, {
        "n_bars": n,
        "bh": round(bh, 4),
        "first": closes[0],
        "last": closes[-1],
        "E": {"metrics": _metric_row(em), "excess": round(em["total_compound"] - bh, 4),
              "by_year": em.get("by_year", {})},
        "variants": variants,
        "e_seconds": round(e_s, 1),
        "i_seconds": round(i_s, 1),
        "sig_seconds": round(sig_s, 1),
    }


def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# V-I 多重赋格 v2 全标的回测 — v2 vs E vs BH（Rust 引擎全 O(N) 驱动）\n")
    L.append(
        "> **v2** = 共享仓位多声部并发短差（满仓入场 + entry 层以下各级别独立 FSM 并发短差"
        "作用于同一 cost_basis + 操作规模按子级别数均分 level_frac=1/N_sub）。**E** = 背驰"
        "定位器进出场（无降成本）。**BH** = buy & hold。信号层全部 Rust 引擎驱动，逐位等价于 "
        "Python（differential test PASS）。认识论 **L2**（真实数据 1min 全历史，含否定性结果）。\n")
    L.append(
        "> 力度口径=价格振幅 fallback（非 MACD，O(N²) 不可行）。三 floor 变体（含 bar / 线段起 / "
        "缠师走势级口径）让数据裁决 bar 级是否噪音——缠师原文（第53/35/31课）反对 bar 级短差，"
        "若 I_bar0 < I_move3 则经验印证缠师。\n")
    L.append("## v2 vs E vs BH 对照（三 floor 变体）\n")
    L.append("| 标的 | bars | BH% | 策略 | 复利% | 超额% | 夏普 | MDD% | 盈亏比 | 交易 | 降成本笔 |")
    L.append("|------|------|-----|------|-------|-------|------|------|--------|------|----------|")
    for s in ORDER:
        r = results.get(s)
        if not r:
            continue
        e = r["E"]["metrics"]
        pf_e = "—" if e["profit_factor"] is None else f"{e['profit_factor']:.2f}"
        L.append(
            f"| **{s}** | {r['n_bars']:,} | {r['bh']:+.1f} | E基线 | {e['compound']:+.1f} | "
            f"{r['E']['excess']:+.1f} | {e['sharpe']:+.2f} | {e['max_dd']:+.1f} | {pf_e} | {e['n']} | — |")
        for tag, _ in _VARIANTS:
            v = r["variants"].get(tag)
            if not v:
                continue
            m = v["metrics"]
            pf = "—" if m["profit_factor"] is None else f"{m['profit_factor']:.2f}"
            L.append(
                f"| {s} | | | v2/{tag}({v['floor']}) | {m['compound']:+.1f} | "
                f"{v['excess']:+.1f} | {m['sharpe']:+.2f} | {m['max_dd']:+.1f} | {pf} | {m['n']} | "
                f"{m['n_with_cr']} |")
    L.append("")
    L.append("## 级别归属分布（揭示高层稀疏有效域）— v2/I_bar0\n")
    L.append("| 标的 | 归属(级别→笔数) | 2买标记 |")
    L.append("|------|----------------|--------|")
    for s in ORDER:
        r = results.get(s)
        if not r:
            continue
        v = r["variants"].get("I_bar0")
        if not v:
            continue
        L.append(f"| {s} | {v['ladder_attribution']} | {v['addon_2buy_marks']} |")
    L.append("")
    L.append("## 每级 FSM 独立贡献度（多重赋格各声部）— v2/I_bar0\n")
    L.append("> `recovered_cash`=该级短差累计回收现金；`n_short_diffs`=完成短差次数；"
             "`slice_net_gain`=该级短差净增益。bar 级若巨负 → 经验印证缠师'太小级别短差无意义'。\n")
    L.append("| 标的 | 级别 | 回收现金 | 短差次数 | 净增益 |")
    L.append("|------|------|---------|---------|--------|")
    for s in ORDER:
        r = results.get(s)
        if not r:
            continue
        v = r["variants"].get("I_bar0")
        if not v:
            continue
        for lvl, d in v["fsm_contribution"].items():
            L.append(f"| {s} | {lvl} | {d['recovered_cash']:+.0f} | {d['n_short_diffs']} | "
                     f"{d['slice_net_gain']:+.0f} |")
    L.append("")
    L.append("## 耗时\n")
    L.append("| 标的 | bars | E耗时s | I信号s | I总s |")
    L.append("|------|------|--------|--------|------|")
    for s in ORDER:
        r = results.get(s)
        if not r:
            continue
        L.append(f"| {s} | {r['n_bars']:,} | {r.get('e_seconds','-')} | "
                 f"{r.get('sig_seconds','-')} | {r.get('i_seconds','-')} |")
    L.append("")
    missing = [s for s in ORDER if s not in results]
    if missing:
        L.append(f"## 缺失标的（数据未找到或崩溃）\n\n{', '.join(missing)}\n")
    OUT_MD.write_text("\n".join(L))


def _merge_and_write(symbols: list[str]) -> None:
    results: dict[str, dict] = {}
    for s in symbols:
        f = DATA_DIR / f"v2_result_{s}.json"
        if f.exists():
            try:
                results.update(json.loads(f.read_text()))
            except Exception:
                pass
    OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False, default=str))
    _write_report(results)


def main() -> None:
    symbols = [s for s in ORDER if SYMBOL_FILES[s].exists()]
    skipped = [s for s in ORDER if not SYMBOL_FILES[s].exists()]
    only = os.environ.get("BT_SYMBOLS")
    if only:
        symbols = [s.strip().upper() for s in only.split(",") if s.strip()]
    workers = int(os.environ.get("BT_WORKERS", "0")) or min(len(symbols), os.cpu_count() or 2)

    # ── 单标的模式（子进程隔离）：跑一个标的，写独立结果文件，退出（释放内存）──
    single = os.environ.get("BT_SINGLE")
    if single:
        sym = single.strip().upper()
        _, out = run_symbol(sym)
        (DATA_DIR / f"v2_result_{sym}.json").write_text(
            json.dumps({sym: out}, ensure_ascii=False, default=str))
        print(f"  ✓ {sym} 写入 v2_result_{sym}.json", flush=True)
        return

    print(f"V-I v2 全标的回测：{len(symbols)} 标的，{workers} 子进程并发（隔离）")
    print(f"标的：{', '.join(symbols)}")
    if skipped:
        print(f"⚠ 数据未找到，跳过：{', '.join(skipped)}")
    print(flush=True)

    def _done(s: str) -> bool:
        return (DATA_DIR / f"v2_result_{s}.json").exists()

    # 大标的优先启动（墙钟由最大标的决定，先发车）。
    def _bars_hint(s: str) -> int:
        try:
            return SYMBOL_FILES[s].stat().st_size
        except OSError:
            return 0
    queue = sorted([s for s in symbols if not _done(s)], key=_bars_hint, reverse=True)
    print(f"待跑：{queue}（已完成：{[s for s in symbols if _done(s)]}）\n", flush=True)

    logs_dir = ROOT / "analysis" / "logs"
    logs_dir.mkdir(exist_ok=True)
    t_all = time.time()
    running: dict = {}
    failed: list[str] = []

    while queue or running:
        while queue and len(running) < workers:
            s = queue.pop(0)
            lf = open(logs_dir / f"v2_{s}.log", "w")
            env = {**os.environ, "BT_SINGLE": s}
            env.pop("BT_SYMBOLS", None)
            p = subprocess.Popen(
                [sys.executable, str(Path(__file__).resolve())],
                env=env, stdout=lf, stderr=subprocess.STDOUT)
            running[p] = (s, lf)
            print(f"  ▶ {s} 启动（PID {p.pid}，日志 v2_{s}.log）", flush=True)
        for p in list(running):
            rc = p.poll()
            if rc is None:
                continue
            s, lf = running.pop(p)
            lf.close()
            if rc == 0 and _done(s):
                _merge_and_write(symbols)
                print(f"  ✓ {s} 完成（rc=0，{time.time()-t_all:.0f}s）", flush=True)
            else:
                failed.append(s)
                print(f"  ✗ {s} 崩溃（rc={rc}，见 v2_{s}.log）——隔离，不影响其他标的", flush=True)
        time.sleep(5)

    _merge_and_write(symbols)
    print(f"\n总耗时 {time.time()-t_all:.0f}s")
    if failed:
        print(f"⚠ 崩溃标的（需单独诊断）：{failed}")
    print(f"报告：{OUT_MD}\n结果：{OUT_JSON}")


if __name__ == "__main__":
    main()
