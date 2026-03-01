"""L2 回测实跑脚本 — 真实数据全流程。

用 yfinance 拉取日线数据，灌入 BacktestOrchestrator 全流程。
所有判断基于当前已确认结构（不用未来数据）。

认识论标注：L2（真实数据验证）。
谱系引用：267号操作方法论 v1、281号回测框架。
"""

from __future__ import annotations

import json
import sys
from datetime import datetime
from pathlib import Path

# 项目根
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import yfinance as yf
import pandas as pd

from newchan.backtest.orchestrator import (
    BacktestOrchestrator,
    BacktestOrchestratorConfig,
    BacktestOrchestratorResult,
    run_backtest,
)
from newchan.backtest.types import (
    CostCurvePoint,
    K4State,
    StateMachineState,
    TradeAction,
)
from newchan.types import Bar

# ═══════════════════════════════════════════════════════════════
# 配置
# ═══════════════════════════════════════════════════════════════

K4_SYMBOLS = ("SPY", "GLD", "TLT")
CANDIDATE_SYMBOLS = ("XLK", "XLF", "XLE", "XLV", "XLY")
ALL_SYMBOLS = K4_SYMBOLS + CANDIDATE_SYMBOLS

RESULTS_DIR = ROOT / "tmp" / "backtest-l2-results"


# ═══════════════════════════════════════════════════════════════
# 数据拉取
# ═══════════════════════════════════════════════════════════════


def fetch_data(symbols: tuple[str, ...]) -> dict[str, pd.DataFrame]:
    """拉取所有标的的日线 OHLCV 数据（最大历史）。"""
    data: dict[str, pd.DataFrame] = {}
    for sym in symbols:
        print(f"  拉取 {sym} ...", end=" ", flush=True)
        df = yf.download(sym, period="max", interval="1d", progress=False)
        # yfinance 可能返回 MultiIndex columns，扁平化
        if isinstance(df.columns, pd.MultiIndex):
            df.columns = df.columns.get_level_values(0)
        print(f"{len(df)} bars")
        data[sym] = df
    return data


def align_data(
    data: dict[str, pd.DataFrame],
) -> tuple[pd.DatetimeIndex, dict[str, pd.DataFrame]]:
    """对齐所有标的到公共交易日。"""
    # 取所有标的的交集日期
    common_index: pd.DatetimeIndex | None = None
    for sym, df in data.items():
        idx = df.index
        if common_index is None:
            common_index = idx
        else:
            common_index = common_index.intersection(idx)

    if common_index is None or len(common_index) == 0:
        raise ValueError("无公共交易日")

    common_index = common_index.sort_values()
    aligned = {sym: df.loc[common_index] for sym, df in data.items()}
    return common_index, aligned


def df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """DataFrame → Bar 列表。"""
    bars: list[Bar] = []
    for idx, row in df.iterrows():
        ts = idx.to_pydatetime() if hasattr(idx, "to_pydatetime") else idx
        bars.append(Bar(
            ts=ts,
            open=float(row["Open"]),
            high=float(row["High"]),
            low=float(row["Low"]),
            close=float(row["Close"]),
            volume=float(row["Volume"]) if "Volume" in row.index else None,
        ))
    return bars


# ═══════════════════════════════════════════════════════════════
# 结果序列化
# ═══════════════════════════════════════════════════════════════


def serialize_result(result: BacktestOrchestratorResult) -> dict:
    """将回测结果序列化为可 JSON 化的字典。"""
    return {
        "config": {
            "initial_capital": result.config.initial_capital,
            "margin_ratio": result.config.margin_ratio,
            "short_diff_ratio": result.config.short_diff_ratio,
            "k4_symbols": list(result.config.k4_symbols),
            "candidate_symbols": list(result.config.candidate_symbols),
            "max_capital": result.config.max_capital,
        },
        "summary": {
            "total_bars": len(result.steps),
            "total_actions": len(result.actions),
            "k4_changes": len(result.k4_changes),
            "stock_changes": len(result.stock_changes),
            "final_cost_basis": result.cost_curve[-1].cost_basis if result.cost_curve else 0.0,
            "total_recovered": result.cost_curve[-1].cumulative_recovered if result.cost_curve else 0.0,
        },
        "k4_changes": [
            {"bar_idx": idx, "config_label": label}
            for idx, label in result.k4_changes
        ],
        "stock_changes": [
            {"bar_idx": idx, "symbol": sym}
            for idx, sym in result.stock_changes
        ],
        "actions": [
            {
                "bar_idx": a.bar_idx,
                "action": a.action,
                "price": a.price,
                "quantity": a.quantity,
                "cost_basis": a.cost_basis,
                "trigger": a.trigger,
            }
            for a in result.actions
        ],
        "cost_curve": [
            {
                "bar_idx": p.bar_idx,
                "cost_basis": p.cost_basis,
                "total_shares": p.total_shares,
                "cumulative_recovered": p.cumulative_recovered,
            }
            for p in result.cost_curve
        ],
    }


def serialize_state_log(result: BacktestOrchestratorResult) -> list[dict]:
    """生成状态转移日志。"""
    log: list[dict] = []
    prev_state = None
    for step in result.steps:
        if step.sm_state != prev_state:
            log.append({
                "bar_idx": step.bar_idx,
                "bar_ts": step.bar_ts.isoformat() if isinstance(step.bar_ts, datetime) else str(step.bar_ts),
                "new_state": step.sm_state.value,
                "held_symbol": step.held_symbol,
                "cost_basis": step.cost_basis,
                "cumulative_recovered": step.cumulative_recovered,
                "trigger": step.action.trigger if step.action else "initial",
            })
            prev_state = step.sm_state
    return log


def serialize_k4_log(result: BacktestOrchestratorResult) -> list[dict]:
    """生成 K4 配置变化日志（含时间戳）。"""
    log: list[dict] = []
    for bar_idx, label in result.k4_changes:
        step = result.steps[bar_idx] if bar_idx < len(result.steps) else None
        ts = step.bar_ts.isoformat() if step and isinstance(step.bar_ts, datetime) else ""
        k4 = step.k4_state if step else None
        log.append({
            "bar_idx": bar_idx,
            "bar_ts": ts,
            "config_label": label,
            "polarity": k4.polarity if k4 else None,
            "e_direction": k4.e.d_reading.direction.value if k4 else None,
            "au_direction": k4.au.d_reading.direction.value if k4 else None,
            "r_direction": k4.r.d_reading.direction.value if k4 else None,
        })
    return log


# ═══════════════════════════════════════════════════════════════
# 主流程
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    print("=" * 60)
    print("L2 回测实跑 — 真实数据全流程")
    print("=" * 60)

    # 1. 拉取数据
    print("\n[1/4] 拉取数据")
    raw_data = fetch_data(ALL_SYMBOLS)

    # 2. 对齐
    print("\n[2/4] 对齐交易日")
    common_dates, aligned = align_data(raw_data)
    print(f"  公共交易日：{len(common_dates)} bars")
    print(f"  起止：{common_dates[0].strftime('%Y-%m-%d')} → {common_dates[-1].strftime('%Y-%m-%d')}")

    # 3. 转换为 Bar 列表
    print("\n[3/4] 转换 Bar 数据")
    bar_streams: dict[str, list[Bar]] = {}
    for sym in ALL_SYMBOLS:
        bar_streams[sym] = df_to_bars(aligned[sym])
        print(f"  {sym}: {len(bar_streams[sym])} bars")

    # 4. 运行回测
    print("\n[4/4] 运行回测")
    config = BacktestOrchestratorConfig(
        initial_capital=1_000_000.0,
        margin_ratio=1.0,
        short_diff_ratio=0.1,
        k4_symbols=K4_SYMBOLS,
        candidate_symbols=CANDIDATE_SYMBOLS,
    )

    k4_bar_streams = (
        bar_streams[K4_SYMBOLS[0]],
        bar_streams[K4_SYMBOLS[1]],
        bar_streams[K4_SYMBOLS[2]],
    )
    candidate_bar_streams = {
        sym: bar_streams[sym] for sym in CANDIDATE_SYMBOLS
    }

    import time
    t0 = time.time()
    orch = BacktestOrchestrator(config)
    total_bars = min(len(s) for s in k4_bar_streams)
    progress_interval = max(1, total_bars // 20)
    for i in range(total_bars):
        k4_bars = (
            k4_bar_streams[0][i],
            k4_bar_streams[1][i],
            k4_bar_streams[2][i],
        )
        cand_bars: dict[str, Bar] = {}
        for sym, bars in candidate_bar_streams.items():
            if i < len(bars):
                cand_bars[sym] = bars[i]
        orch.process_bar(k4_bars, cand_bars)
        if (i + 1) % progress_interval == 0 or i == total_bars - 1:
            elapsed = time.time() - t0
            pct = (i + 1) / total_bars * 100
            print(f"  进度: {i+1}/{total_bars} ({pct:.0f}%) {elapsed:.1f}s", flush=True)
    result = orch.result()
    elapsed = time.time() - t0
    print(f"  完成: {elapsed:.1f}s")

    # 5. 输出结果
    print("\n" + "=" * 60)
    print("结果摘要")
    print("=" * 60)
    print(f"  总 bar 数：{len(result.steps)}")
    print(f"  K4 配置变化次数：{len(result.k4_changes)}")
    print(f"  选股切换次数：{len(result.stock_changes)}")
    print(f"  总操作次数：{len(result.actions)}")

    if result.cost_curve:
        last_curve = result.cost_curve[-1]
        print(f"  最终持仓成本：{last_curve.cost_basis:.4f}")
        print(f"  累计回收金额：{last_curve.cumulative_recovered:.2f}")
        print(f"  总持仓份额：{last_curve.total_shares:.2f}")

    if result.actions:
        print("\n  操作明细（前20条）：")
        for a in result.actions[:20]:
            step = result.steps[a.bar_idx] if a.bar_idx < len(result.steps) else None
            ts = step.bar_ts.strftime("%Y-%m-%d") if step and isinstance(step.bar_ts, datetime) else f"bar#{a.bar_idx}"
            print(f"    [{ts}] {a.action:12s} price={a.price:.2f} qty={a.quantity:.2f} cost={a.cost_basis:.4f} | {a.trigger}")

    # 状态转移统计
    state_counts: dict[str, int] = {}
    for step in result.steps:
        sv = step.sm_state.value
        state_counts[sv] = state_counts.get(sv, 0) + 1
    print("\n  状态分布：")
    for state, count in sorted(state_counts.items(), key=lambda x: -x[1]):
        pct = count / len(result.steps) * 100
        print(f"    {state:15s}: {count:6d} bars ({pct:.1f}%)")

    # 6. 保存结果
    print("\n[保存结果]")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    # 主结果
    result_path = RESULTS_DIR / "backtest_result.json"
    with open(result_path, "w", encoding="utf-8") as f:
        json.dump(serialize_result(result), f, indent=2, ensure_ascii=False)
    print(f"  主结果 → {result_path}")

    # K4 配置变化日志
    k4_log_path = RESULTS_DIR / "k4_config_log.json"
    with open(k4_log_path, "w", encoding="utf-8") as f:
        json.dump(serialize_k4_log(result), f, indent=2, ensure_ascii=False)
    print(f"  K4 配置日志 → {k4_log_path}")

    # 状态转移日志
    state_log_path = RESULTS_DIR / "state_transition_log.json"
    with open(state_log_path, "w", encoding="utf-8") as f:
        json.dump(serialize_state_log(result), f, indent=2, ensure_ascii=False)
    print(f"  状态转移日志 → {state_log_path}")

    # 降成本曲线
    if result.cost_curve:
        curve_path = RESULTS_DIR / "cost_curve.json"
        with open(curve_path, "w", encoding="utf-8") as f:
            json.dump(
                [
                    {
                        "bar_idx": p.bar_idx,
                        "cost_basis": p.cost_basis,
                        "total_shares": p.total_shares,
                        "cumulative_recovered": p.cumulative_recovered,
                    }
                    for p in result.cost_curve
                ],
                f, indent=2,
            )
        print(f"  降成本曲线 → {curve_path}")

    # 短差回收明细
    short_diff_details: list[dict] = []
    for a in result.actions:
        if a.action in ("short_sell", "short_cover"):
            step = result.steps[a.bar_idx] if a.bar_idx < len(result.steps) else None
            ts = step.bar_ts.isoformat() if step and isinstance(step.bar_ts, datetime) else ""
            short_diff_details.append({
                "bar_idx": a.bar_idx,
                "bar_ts": ts,
                "action": a.action,
                "price": a.price,
                "quantity": a.quantity,
                "cost_basis_after": a.cost_basis,
                "trigger": a.trigger,
            })
    if short_diff_details:
        sd_path = RESULTS_DIR / "short_diff_details.json"
        with open(sd_path, "w", encoding="utf-8") as f:
            json.dump(short_diff_details, f, indent=2, ensure_ascii=False)
        print(f"  短差明细 → {sd_path}")

    print("\n完成。")


if __name__ == "__main__":
    main()
