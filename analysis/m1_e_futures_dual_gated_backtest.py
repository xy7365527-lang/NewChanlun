"""M1 状态门控多空双开回测——第二轮预注册（#1312 · #1282 冻结判据，跑前不改一字）。

## 预注册判据（#1282 票面冻结，与第一轮 #1309 逐字同文，本轮零改动）

- H：操作层对称扩展至下跌侧（次级别顶背驰进空、L2 底背驰回补，联立门方向镜像），
  期货 7 标的 E 择时达到统计确立。
- 过判据（唯一）：**7/7 标的百分位全部越过各自随机中位**（符号检验单侧 p=0.0078）。
  不过 = M1 落档「期货对称扩展不可证」，同为有效结论。
- 对照设计（Q3）：①前后对照 = 原 E 单边（long-only）百分位 vs 多空双开百分位，
  同标的同窗配对，逐标的记变化量；②随机基线 = prereg-windows-v0 百分位法
  （含 #1281 补的 ZN/6E 窗口）= 真策略成绩在随机进出分布中的排位。
- 数据与窗口：prereg 表冻结值，7 标的（ES/GC/CL/ZN/6E/BRN/DX）10 年 1min。
- 纪律附款：跑前不搜索、不调参、不挑窗口；跑完结果原样入档；不做参数网格搜索、
  不加新对照品种、不中途改判据。

## 第二轮与第一轮的唯一差异（#1312）

进场加一道 **41课门状态闸**（`analysis/gate_state_columns.StateGate`）：
空腿进场拒于 `l2_up_unexhausted=true`、多腿镜像拒于 `l2_down_unexhausted=true`。
门列由 Rust 生产函数本体现算落盘（`rust/src/trading/gate_state_dump.rs` 驱动
`trend_exhaustion.rs`），Python 侧零判据实现。**判据/窗口/基线/过判据一字不动**——
状态闸是 H 的操作面门控，不是新判据（#1267 结构判据版 41课门本就在生产册）。
fatigue 为可选配置臂（`--fatigue`），默认关；列不可用时开臂即 fail-fast。

## 本票正确性口径

「正确」= 忠实执行预注册，不是「实验必须过」。第二轮过与不过都是有效结论，原样报。

认识论等级：移植正确性 L0/L1（门列 = 生产函数读数，Rust 侧测试锁；无门路径与
第一轮逐位等价）；回测结论 L2（真实数据 7 期货 1min，含否定性结果）。

用法：
```bash
# 前置①：7 标的 1min 数据（Databento，gitignored）
# 前置②：v2r 磁带 + v3 门状态列
PYTHONPATH=src uv run python analysis/_dump_tape_rust.py ES GC CL ZN 6E BRN DX
cd rust && cargo test --release gate_state_dump_prereg7 -- --ignored --nocapture
# 跑批
PYTHONPATH=src:analysis uv run python analysis/m1_e_futures_dual_gated_backtest.py
```
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as F  # noqa: E402
import gate_state_columns as G  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_DIR = ROOT / ".chanlun" / "review-results"
OUT_JSON = OUT_DIR / "issue1312-state-gated-backtest.json"
OUT_MD = OUT_DIR / "issue1312-state-gated-backtest.md"
# 第一轮（#1309 无门基线）归档结果——两轮对照表的第一轮列取自此，不重跑不改写。
ROUND1_JSON = OUT_DIR / "issue1309-dual-open-backtest.json"

# 标的 → 10-16y 1min databento 数据文件（与 m1_e_futures_dual_backtest 逐项一致）。
# 本地镜像而非 import：三个引擎模块顶部 import newchan_rust，沙盒未构建时会在
# import 阶段先崩，使「缺数据/缺门列 → 卡点报告 exit=2」路径不可达（#1309 同款处理）。
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

# P3 随机门控触发阈值：N < MIN_N_FOR_P3 时统计功效过低，不跑随机门控（第一轮同值）。
MIN_N_FOR_P3 = 10


def _out_path(sym: str, fatigue: bool) -> Path:
    tag = "_fat" if fatigue else ""
    return DATA_DIR / f"_m1e_dual_gated{tag}_fut_{sym}.json"


def _avg_hold(trades: list[F.CompletedTrade]) -> float:
    return (
        sum(t.exit_bar - t.entry_bar for t in trades) / len(trades)
        if trades else 0.0
    )


def _trade_key(trades: list[F.CompletedTrade]) -> list[tuple]:
    """逐笔可比较指纹（双跑逐位自检用）。"""
    return [
        (t.entry_bar, t.entry_price, t.exit_bar, t.exit_price, t.pnl_pct, t.exit_reason)
        for t in trades
    ]


def gate_sanity(
    gate: G.StateGate,
    long_trades: list[F.CompletedTrade],
    short_trades: list[F.CompletedTrade],
    ungated_long: list[F.CompletedTrade],
    ungated_short: list[F.CompletedTrade],
) -> dict:
    """门 sanity 核验（#1312 要做 ④）：unexhausted=true 区间内无逆势单 + 拦截计数。

    ①硬断言：门控后每笔进场 bar 的对应 unexhausted 列必须为 false（门真的在拦）；
    ②抽验：最长连续 unexhausted 区间（= 已知趋势延续段）内逆势进场数必须为 0；
    ③拦截量：第一轮（无门）进场中落在 unexhausted 区间、被本轮拒掉的笔数。
    """
    for t in short_trades:
        if gate.up_unexhausted[t.entry_bar]:
            raise AssertionError(
                f"[{gate.symbol}] 门 sanity 失败：空腿进场 bar={t.entry_bar} 落在 "
                "l2_up_unexhausted=true 区间（门未拦住逆势单）"
            )
    for t in long_trades:
        if gate.down_unexhausted[t.entry_bar]:
            raise AssertionError(
                f"[{gate.symbol}] 门 sanity 失败：多腿进场 bar={t.entry_bar} 落在 "
                "l2_down_unexhausted=true 区间（门未拦住逆势单）"
            )
    up_win = G.longest_unexhausted_window(gate.up_unexhausted)
    dn_win = G.longest_unexhausted_window(gate.down_unexhausted)
    in_up_win = sum(1 for t in short_trades if up_win[0] <= t.entry_bar < up_win[1])
    in_dn_win = sum(1 for t in long_trades if dn_win[0] <= t.entry_bar < dn_win[1])
    blocked_short = sum(1 for t in ungated_short if gate.up_unexhausted[t.entry_bar])
    blocked_long = sum(1 for t in ungated_long if gate.down_unexhausted[t.entry_bar])
    return {
        "longest_up_unexhausted_window": list(up_win),
        "longest_down_unexhausted_window": list(dn_win),
        "short_entries_in_longest_up_window": in_up_win,
        "long_entries_in_longest_down_window": in_dn_win,
        "round1_short_entries_blocked": blocked_short,
        "round1_long_entries_blocked": blocked_long,
        "gate_open_rates": G.gate_open_rates(gate),
    }


def run_symbol(symbol: str, use_fatigue: bool = False) -> tuple[str, dict]:
    """单标的：load → 门列 → Rust E 信号（一次）→ 门控双开 metrics + 百分位 → 落盘。"""
    out_path = _out_path(symbol, use_fatigue)
    if out_path.exists():
        try:
            cached = json.loads(out_path.read_text())
            print(f"  [{symbol:4s}] 已完成（resume，跳过）", flush=True)
            return symbol, cached
        except (json.JSONDecodeError, OSError):
            pass  # 损坏 → 重跑

    # Rust 扩展依赖延迟 import：缺数据/缺门列时 main() 在此之前即报卡点退出。
    import m1_e_rust_engine as RE  # noqa: E402
    import p3_random_gate_control as P3  # noqa: E402
    from m1_e_futures_backtest import load_ohlc  # noqa: E402

    path = SYMBOL_FILES[symbol]
    opens, highs, lows, closes = load_ohlc(path)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    gate = G.load_state_gate(symbol, closes, use_fatigue=use_fatigue)

    # ── E 信号（唯一 O(N²) 计算，门控/无门共享）──
    t0 = time.time()
    print(f"  [{symbol:4s}] 启动：{n:,} bars，Rust E 信号计算中…", flush=True)
    signals = RE.compute_e_signals_rust(opens, highs, lows, closes)
    sig_s = time.time() - t0

    # ── 无门双开（= 第一轮口径，同 signals 重算；两轮对照与门拦截计数的配对基准）──
    ungated_long, ungated_short = F.run_swing_trading_dual(signals)
    ungated_m = F.compute_dual_metrics(ungated_long, ungated_short)

    # ── 门控双开（第二轮）──
    gated_long, gated_short = F.run_swing_trading_dual(signals, gate)
    gated_m = F.compute_dual_metrics(gated_long, gated_short)

    # ── 双跑逐位自检（同输入两次驱动，逐笔指纹必须全同）──
    recheck_long, recheck_short = F.run_swing_trading_dual(signals, gate)
    selfcheck_ok = (
        _trade_key(recheck_long) == _trade_key(gated_long)
        and _trade_key(recheck_short) == _trade_key(gated_short)
    )
    if not selfcheck_ok:
        raise AssertionError(f"[{symbol}] 双跑逐位自检失败：门控双开非确定性")

    sanity = gate_sanity(gate, gated_long, gated_short, ungated_long, ungated_short)

    gated_pct = None
    if gated_m["n"] >= MIN_N_FOR_P3:
        gated_pct = P3.large_bootstrap_percentile_dual(
            closes,
            gated_m["n_long"], int(round(_avg_hold(gated_long))),
            gated_m["n_short"], int(round(_avg_hold(gated_short))),
            gated_m["total_compound"],
        )
    ungated_pct = None
    if ungated_m["n"] >= MIN_N_FOR_P3:
        ungated_pct = P3.large_bootstrap_percentile_dual(
            closes,
            ungated_m["n_long"], int(round(_avg_hold(ungated_long))),
            ungated_m["n_short"], int(round(_avg_hold(ungated_short))),
            ungated_m["total_compound"],
        )

    def _p3(p) -> dict | None:
        return {
            "e_percentile": p.e_percentile,
            "p_random_ge_e": p.p_random_ge_e,
            "rnd_median": p.rnd_median,
            "rnd_mean": p.rnd_mean,
            "rnd_std": p.rnd_std,
        } if p else None

    def _metrics(m, longs, shorts) -> dict:
        return {
            "compound": m["total_compound"],
            "long_compound": m["long_compound"],
            "short_compound": m["short_compound"],
            "n_trades": m["n"],
            "n_long": m["n_long"],
            "n_short": m["n_short"],
            "win_rate": m["win_rate"],
            "max_dd": m["max_dd"],
            "avg_hold_long": _avg_hold(longs),
            "avg_hold_short": _avg_hold(shorts),
        }

    out: dict = {
        "symbol": symbol,
        "n_bars": n,
        "bh": bh,
        "sig_s": sig_s,
        "use_fatigue": use_fatigue,
        "gate_ladder": gate.ladder,
        "gated": _metrics(gated_m, gated_long, gated_short),
        "gated_p3": _p3(gated_pct),
        "ungated": _metrics(ungated_m, ungated_long, ungated_short),
        "ungated_p3": _p3(ungated_pct),
        "gated_pct_minus_ungated_pct": (
            gated_pct.e_percentile - ungated_pct.e_percentile
            if gated_pct and ungated_pct else None
        ),
        # 过判据主口径（#1282 冻结）：百分位 > 50 = 越过各自随机中位。
        "gated_beats_median": (
            gated_pct.e_percentile > 50.0 if gated_pct else None
        ),
        "selfcheck_ok": selfcheck_ok,
        "gate_sanity": sanity,
    }
    out_path.write_text(json.dumps(out, indent=2, ensure_ascii=False))

    gp = out["gated_p3"]
    up = out["ungated_p3"]
    gp_s = f"gated%ile={gp['e_percentile']:.0f}" if gp else "gated跳过"
    up_s = f"ungated%ile={up['e_percentile']:.0f}" if up else "ungated跳过"
    delta = out["gated_pct_minus_ungated_pct"]
    delta_s = f"{delta:+5.1f}" if delta is not None else "—"
    print(
        f"  [{symbol:4s}] {n:>9,} bars | 门控复合={gated_m['total_compound']:+10.2f}% "
        f"(长{gated_m['n_long']}/空{gated_m['n_short']}) | {up_s} {gp_s} Δ={delta_s} | "
        f"拦截 长{sanity['round1_long_entries_blocked']}/空"
        f"{sanity['round1_short_entries_blocked']} | 信号 {sig_s/60:.1f}min",
        flush=True,
    )
    return symbol, out


def _sign_test_p(n_pass: int, n_symbols: int) -> float:
    """单侧符号检验 p（至少 n_pass/n_symbols 跑赢中位，H0 = p=0.5）。"""
    from math import comb
    return sum(comb(n_symbols, k) for k in range(n_pass, n_symbols + 1)) / (2 ** n_symbols)


def _round1() -> dict:
    """第一轮（#1309 无门）归档结果；缺档 ⇒ 空表（对照列打 —）。"""
    if not ROUND1_JSON.exists():
        return {}
    try:
        return json.loads(ROUND1_JSON.read_text())
    except (json.JSONDecodeError, OSError):
        return {}


def write_report(results: dict[str, dict], use_fatigue: bool) -> None:
    symbols = [s for s in SYMBOL_ORDER if s in results]
    r1 = _round1()
    n_pass = sum(1 for s in symbols if results[s]["gated_beats_median"] is True)
    n_total = len(symbols)
    verdict = "过（7/7 全部越过随机中位）" if n_pass == n_total else "不过（如实入档）"
    arm = "开（fatigue 可选臂）" if use_fatigue else "关（默认）"

    L: list[str] = []
    L.append("# #1312 状态门控多空双开回测结果（第二轮，#1282 预注册判据，冻结）\n")
    L.append(
        "> 7 标的（ES/GC/CL/ZN/6E/BRN/DX）1min 全量（prereg 表冻结值）| 零交易成本 | "
        "过判据 = 7/7 百分位全部越过各自随机中位（符号检验单侧 p=0.0078）。\n"
    )
    L.append(
        f"> 第二轮唯一差异 = 进场加 41课门状态闸（空腿拒于 l2_up_unexhausted、多腿拒于 "
        f"l2_down_unexhausted；门列由 Rust 生产函数现算）。fatigue 臂：{arm}。\n"
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

    L.append("\n## 1. 第二轮（状态门控）双开百分位（主判据）\n")
    L.append(
        "| 标的 | Bars | dual复利% | 长腿复利% | 空腿复利% | 交易(长/空) | "
        "P(随机≥真实) | 双开百分位 | 随机中位% | 越过中位 |\n"
        "|------|------|----------|-----------|-----------|-------------|"
        "-------------|-----------|----------|----------|"
    )
    for sym in symbols:
        r = results[sym]
        d = r["gated"]
        dp = r["gated_p3"]
        if dp is None:
            L.append(
                f"| {sym} | {r['n_bars']:,} | {d['compound']:+.2f} | "
                f"{d['long_compound']:+.2f} | {d['short_compound']:+.2f} | "
                f"{d['n_long']}/{d['n_short']} | — | — | — | N<{MIN_N_FOR_P3} 跳过 |"
            )
        else:
            beat = "✅" if r["gated_beats_median"] else "❌"
            L.append(
                f"| {sym} | {r['n_bars']:,} | {d['compound']:+.2f} | "
                f"{d['long_compound']:+.2f} | {d['short_compound']:+.2f} | "
                f"{d['n_long']}/{d['n_short']} | {dp['p_random_ge_e']:.1%} | "
                f"{dp['e_percentile']:.0f} | {dp['rnd_median']:+.2f} | {beat} |"
            )

    L.append("\n## 2. 两轮对照表（#1312 要做 ③：逐品种 Δ 百分位 = 状态门价值读数）\n")
    L.append(
        "> 第一轮 = #1309 无门双开归档值（`issue1309-dual-open-backtest.json`）；"
        "第二轮 = 本跑门控值；「本跑无门重算」= 同 signals 同窗重算的无门对照"
        "（第一轮口径复现校验，两值应一致）。\n"
    )
    L.append(
        "| 标的 | 第一轮百分位（归档） | 本跑无门重算 | 第二轮百分位（门控） | "
        "Δ 百分位（二−一） | 门拦截进场(长/空) |\n"
        "|------|--------------------|-------------|--------------------|"
        "------------------|------------------|"
    )
    for sym in symbols:
        r = results[sym]
        gp = r["gated_p3"]
        up = r["ungated_p3"]
        a = r1.get(sym, {}).get("dual_p3") or {}
        a_s = f"{a['e_percentile']:.1f}" if a else "—"
        up_s = f"{up['e_percentile']:.1f}" if up else "—"
        gp_s = f"{gp['e_percentile']:.1f}" if gp else "—"
        if gp and a:
            d_s = f"{gp['e_percentile'] - a['e_percentile']:+.1f}"
        else:
            d_s = "—"
        sn = r["gate_sanity"]
        L.append(
            f"| {sym} | {a_s} | {up_s} | {gp_s} | {d_s} | "
            f"{sn['round1_long_entries_blocked']}/{sn['round1_short_entries_blocked']} |"
        )

    L.append("\n## 3. 门 sanity 核验（#1312 要做 ④）\n")
    L.append(
        "> 硬断言（跑内，失败即 AssertionError 中止）：门控后**无任何**进场落在对应 "
        "unexhausted=true 区间。下表为抽验读数——最长连续 unexhausted 区间（= 已知趋势"
        "延续段）内的逆势进场数必须为 0。\n"
    )
    L.append(
        "| 标的 | up 未衰竭 bar 占比 | down 未衰竭 bar 占比 | 最长 up 未衰竭段 | "
        "该段内空腿进场 | 最长 down 未衰竭段 | 该段内多腿进场 | fatigue 列 |\n"
        "|------|------------------|--------------------|----------------|"
        "--------------|------------------|--------------|-----------|"
    )
    for sym in symbols:
        sn = results[sym]["gate_sanity"]
        gr = sn["gate_open_rates"]
        uw = sn["longest_up_unexhausted_window"]
        dw = sn["longest_down_unexhausted_window"]
        fat = (
            f"可用（{gr['fatigued_bars']} bar 衰竭）" if gr["fatigue_available"]
            else "能力缺失（磁带无 run_high 行）"
        )
        L.append(
            f"| {sym} | {gr['up_unexhausted_pct']:.1f}% | {gr['down_unexhausted_pct']:.1f}% | "
            f"[{uw[0]:,},{uw[1]:,}) | {sn['short_entries_in_longest_up_window']} | "
            f"[{dw[0]:,},{dw[1]:,}) | {sn['long_entries_in_longest_down_window']} | {fat} |"
        )

    L.append("\n## 4. 双跑逐位自检\n")
    ok_all = all(results[s]["selfcheck_ok"] for s in symbols)
    L.append(
        f"逐标的同输入两次驱动门控双开，逐笔（进出场 bar/价/pnl/reason）指纹全同："
        f"**{'PASS' if ok_all else 'FAIL'}**（{sum(1 for s in symbols if results[s]['selfcheck_ok'])}"
        f"/{n_total} 标的）。\n"
    )

    L.append("\n## 5. 预注册冻结文本逐条对照（#1282 票面）\n")
    L.append(
        "| 冻结项 | 票面 | 本跑执行 | 核对 |\n"
        "|--------|------|----------|------|\n"
        "| 假设 H | 操作层对称扩展至下跌侧（次级别顶背驰进空、L2 底背驰回补、联立门方向镜像） | "
        "`run_swing_trading_dual`：空腿 = up_move_settled∧exit_div_ok 进空、"
        "l2_flip_long∧entry_div_ok 回补（判据零改动，仅进场加 41课门状态闸） | ✅ |\n"
        "| 过判据 | 7/7 标的百分位全部越过各自随机中位（符号检验单侧 p=0.0078） | "
        "每标的分位 >50 计数，7/7 判定 + p=0.0078125（与第一轮同口径） | ✅ |\n"
        "| 前后对照 | 原E单边百分位 vs 双开百分位，同标的同窗配对逐标记变化量 | "
        "两轮对照表：第一轮归档 vs 本跑无门重算 vs 第二轮门控，逐标 Δ | ✅ |\n"
        "| 随机基线 | prereg-windows-v0 百分位法（含 #1281 补 ZN/6E 窗） | "
        "`large_bootstrap_percentile_dual`（双腿各 N×H 随机进出 5000 次，函数零改动） | ✅ |\n"
        "| 数据与窗口 | 7 标的 10 年 1min，窗口照 prereg-windows-v0 + §9 补录 | "
        "SYMBOL_FILES 全量 1min（与第一轮同窗同文件） | ✅ |\n"
        "| 纪律附款 | 不搜索/不调参/不挑窗/不加对照/不改判据 | "
        "判据与窗口零改动；门为在册生产判据（#1267/#1264）现算，非新判据；结果原样入档 | ✅ |\n"
    )

    L.append("\n## 6. 纪律附款执行记录\n")
    L.append(
        "- 跑前不搜索、不调参、不挑窗口：判据/窗口逐字取自 #1282 冻结文本，未改一字。\n"
        "- 门列为生产函数本体现算（`rust/src/trading/gate_state_dump.rs` 驱动 "
        "`trend_exhaustion.rs`/`fatigue_gate.rs`），Python 侧零判据实现——防判据分叉。\n"
        "- 不做参数网格搜索、不加新对照品种、不中途改判据。\n"
        "- 结果原样入档（本文件 + issue1312-state-gated-backtest.json）。\n"
    )

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    OUT_MD.write_text("\n".join(L) + "\n")
    OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    use_fatigue = "--fatigue" in argv

    missing_data = [s for s in SYMBOL_ORDER if not SYMBOL_FILES.get(s, Path("")).exists()]
    missing_gate = [s for s in SYMBOL_ORDER if not G.gate_path(s).exists()]
    if missing_data or missing_gate:
        print("卡点：第二轮跑批的输入不齐（外部依赖）。", flush=True)
        if missing_data:
            print("  缺期货 1min 数据文件（Databento 外部依赖，沙盒无 DATABENTO_API_KEY）：",
                  flush=True)
            for s in missing_data:
                print(f"    - {SYMBOL_FILES[s]}", flush=True)
        if missing_gate:
            print("  缺 v3 门状态列（需先有数据 → v2r 磁带 → Rust 现算落列）：", flush=True)
            for s in missing_gate:
                print(f"    - {G.gate_path(s)}", flush=True)
        print(
            "解除条件：①设置 DATABENTO_API_KEY / DATABENTO_KEY 后运行 "
            "`PYTHONPATH=src uv run python scripts/fetch_1m_databento_10y.py all`；"
            "②构建 Rust 扩展 `cd rust && uv run maturin develop --release`；"
            "③落磁带与门列："
            f"{G.REGEN_CMD.replace('<SYM>', 'ES GC CL ZN 6E BRN DX')}。",
            flush=True,
        )
        return 2

    results: dict[str, dict] = {}
    t_all = time.time()
    for sym in SYMBOL_ORDER:
        _, out = run_symbol(sym, use_fatigue)
        results[sym] = out
    print(f"\n总耗时 {(time.time() - t_all)/60:.1f}min\n")

    write_report(results, use_fatigue)
    print(f"汇总 JSON：{OUT_JSON}")
    print(f"报告 MD：{OUT_MD}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
