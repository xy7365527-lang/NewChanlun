"""OQ5 补充实验——当前窗口切片 vs 2007-2009 对等比较。

编排者指令：获得一个和 2007-2009 窗口对等可比的当前时间切片。
全历史窗口跨越多个周期，背驰判断跨周期累积，不能直接和单一窗口比较。

窗口：
  - current_2020_2026: 2020-01-01 ~ 2026-03-01（COVID→加息→当前）
  - liquidity_crisis_2007_2009: 2007-01-01 ~ 2009-12-31（重跑，确保方法一致）

边：与 OQ5 主实验完全一致的 12 条边（K4 六条 + XAU/USD + R/Au_corner + 4 FRED）

关键对比项：
  1. Au/R 在 2007-09 激活且无背驰。当前窗口中 Au/R 是什么状态？
  2. R/$ 在所有窗口中是唯一趋势边。当前窗口中 R/$ 仍是唯一趋势边吗？
  3. 当前窗口的活跃边数/背驰边数 vs 2007-09 是什么格局？

认识论等级：L2（真实数据验证）
谱系引用：284号（OQ v2）、283号、279号、286号
"""

from __future__ import annotations

import io
import json
import sys
import time
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "src"))

import pandas as pd
import requests
import yfinance as yf

from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.events import DomainEvent
from newchan.types import Bar

RESULTS_DIR = ROOT / "tmp" / "oq5-current-window-results"

# ═══════════════════════════════════════════════════════════════
# 边定义（与 OQ5 主实验完全一致）
# ═══════════════════════════════════════════════════════════════

K4_TICKERS_YF = {
    "SPY": "SPY",
    "GLD": "GLD",
    "TLT": "TLT",
}

K4_VERTEX_EDGES = [
    ("E/$", "SPY", None),
    ("Au/$", "GLD", None),
    ("R/$", "TLT", None),
]

K4_RATIO_EDGES = [
    ("E/Au", "SPY", "GLD"),
    ("Au/R", "GLD", "TLT"),
    ("E/R", "SPY", "TLT"),
]

CASH_CORNER_YF = {
    "GCF": "GC=F",
}

CASH_CORNER_FRED = {
    "DEXJPUS": "DEXJPUS",
    "DEXUSUK": "DEXUSUK",
    "DEXSZUS": "DEXSZUS",
    "DEXUSEU": "DEXUSEU",
}

# 两个对等比较窗口
COMPARISON_WINDOWS = {
    "liquidity_crisis_2007_2009": ("2007-01-01", "2009-12-31"),
    "current_2020_2026": ("2020-01-01", "2026-03-01"),
}


# ═══════════════════════════════════════════════════════════════
# 数据拉取（复用 OQ5 主实验逻辑）
# ═══════════════════════════════════════════════════════════════


def fetch_yfinance(ticker: str, label: str) -> pd.DataFrame | None:
    """从 yfinance 拉取日线 OHLCV。"""
    print(f"  [yf] {label} ({ticker}) ...", end=" ", flush=True)
    try:
        df = yf.download(ticker, period="max", interval="1d", progress=False)
        if isinstance(df.columns, pd.MultiIndex):
            df.columns = df.columns.get_level_values(0)
        if df.empty:
            print("无数据")
            return None
        if df.index.tz is not None:
            df.index = df.index.tz_localize(None)
        print(f"{len(df)} bars ({df.index[0].date()} ~ {df.index[-1].date()})")
        return df
    except Exception as e:
        print(f"失败: {e}")
        return None


def fetch_fred(series_id: str, label: str) -> pd.DataFrame | None:
    """从 FRED CSV API 拉取日线汇率数据，构造伪 OHLCV。"""
    print(f"  [fred] {label} ({series_id}) ...", end=" ", flush=True)
    try:
        url = (
            f"https://fred.stlouisfed.org/graph/fredgraph.csv"
            f"?id={series_id}"
            f"&cosd=1970-01-01"
            f"&coed=2026-12-31"
        )
        resp = requests.get(url, timeout=30)
        resp.raise_for_status()

        df = pd.read_csv(
            io.StringIO(resp.text),
            parse_dates=["observation_date"],
            index_col="observation_date",
        )
        df = df.replace(".", float("nan"))
        df[series_id] = pd.to_numeric(df[series_id], errors="coerce")
        df = df.dropna()

        if df.empty:
            print("无数据")
            return None

        vals = df[series_id].values
        result = pd.DataFrame(
            {
                "Open": vals,
                "High": vals,
                "Low": vals,
                "Close": vals,
                "Volume": 0.0,
            },
            index=df.index,
        )
        if result.index.tz is not None:
            result.index = result.index.tz_localize(None)
        print(f"{len(result)} bars ({result.index[0].date()} ~ {result.index[-1].date()})")
        return result
    except Exception as e:
        print(f"失败: {e}")
        return None


def df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """DataFrame -> Bar list。"""
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


def make_ratio_bars_aligned(
    df_a: pd.DataFrame,
    df_b: pd.DataFrame,
) -> pd.DataFrame | None:
    """对齐两个 DataFrame 并构造比价 OHLC。"""
    common = df_a.index.intersection(df_b.index).sort_values()
    if len(common) == 0:
        return None

    a = df_a.loc[common]
    b = df_b.loc[common]

    ratio_open = a["Open"].values / b["Open"].values
    ratio_close = a["Close"].values / b["Close"].values
    ratio_hl1 = a["High"].values / b["Low"].values
    ratio_hl2 = a["Low"].values / b["High"].values

    all_ratios = pd.DataFrame({
        "r1": ratio_open,
        "r2": ratio_close,
        "r3": ratio_hl1,
        "r4": ratio_hl2,
    }, index=common)

    result = pd.DataFrame({
        "Open": ratio_open,
        "High": all_ratios.max(axis=1).values,
        "Low": all_ratios.min(axis=1).values,
        "Close": ratio_close,
        "Volume": 0.0,
    }, index=common)

    return result


# ═══════════════════════════════════════════════════════════════
# 单条边运行（与 OQ5 主实验一致）
# ═══════════════════════════════════════════════════════════════

STRUCTURAL_EVENT_TYPES = frozenset({
    "segment_settle",
    "zhongshu_candidate",
    "zhongshu_settle",
    "move_candidate",
    "move_settle",
    "bsp_candidate",
    "bsp_confirm",
    "bsp_settle",
})


@dataclass
class EdgeAnalysis:
    """单条边的分析结果。"""
    edge_name: str
    total_bars: int = 0
    date_range: str = ""

    move_count: int = 0
    settled_move_count: int = 0
    seg_count: int = 0
    zs_count: int = 0
    trend_count: int = 0
    consolidation_count: int = 0

    bsp_count: int = 0
    bsp_events_total: int = 0

    last_move_kind: str = ""
    last_move_direction: str = ""

    events: list[dict] = field(default_factory=list)
    moves_detail: list[dict] = field(default_factory=list)


def run_edge(edge_name: str, bars: list[Bar]) -> EdgeAnalysis:
    """对单条边运行 RecursiveOrchestrator。"""
    orch = RecursiveOrchestrator(stream_id=edge_name)
    last_snap: RecursiveOrchestratorSnapshot | None = None
    edge_events: list[dict] = []
    bsp_events_total = 0
    seen_event_ids: set[str] = set()

    for bar in bars:
        snap = orch.process_bar(bar)
        last_snap = snap

        for de in snap.all_events:
            if de.event_type not in STRUCTURAL_EVENT_TYPES:
                continue

            eid = de.event_id if de.event_id else f"{de.event_type}_{de.bar_idx}_{de.seq}"
            if eid in seen_event_ids:
                continue
            seen_event_ids.add(eid)

            if de.event_type in ("bsp_candidate", "bsp_confirm", "bsp_settle"):
                bsp_events_total += 1

            detail: dict = {"seq": de.seq}
            for attr in ("direction", "kind", "side", "level_id",
                         "move_id", "bsp_id", "price"):
                if hasattr(de, attr):
                    val = getattr(de, attr)
                    if val is not None:
                        detail[attr] = val

            edge_events.append({
                "bar_idx": snap.bar_idx,
                "ts": bar.ts.isoformat() if isinstance(bar.ts, datetime) else str(bar.ts),
                "event_type": de.event_type,
                "detail": detail,
            })

    result = EdgeAnalysis(
        edge_name=edge_name,
        total_bars=len(bars),
        bsp_events_total=bsp_events_total,
        events=edge_events,
    )

    if bars:
        result.date_range = f"{bars[0].ts.date()} ~ {bars[-1].ts.date()}"

    if last_snap is not None:
        moves = last_snap.move_snapshot.moves
        result.move_count = len(moves)
        result.settled_move_count = len([m for m in moves if m.settled])
        result.seg_count = len(last_snap.seg_snapshot.segments)
        result.zs_count = len(last_snap.zs_snapshot.zhongshus)
        result.bsp_count = len(last_snap.bsp_snapshot.buysellpoints)

        for m in moves:
            if m.kind == "trend":
                result.trend_count += 1
            elif m.kind == "consolidation":
                result.consolidation_count += 1

        if moves:
            result.last_move_kind = moves[-1].kind
            result.last_move_direction = moves[-1].direction

        for i, m in enumerate(moves):
            result.moves_detail.append({
                "idx": i,
                "kind": m.kind,
                "direction": m.direction,
                "settled": m.settled,
                "high": round(m.high, 6),
                "low": round(m.low, 6),
                "zs_count": m.zs_count,
            })

    return result


# ═══════════════════════════════════════════════════════════════
# 窗口切片 + 分析
# ═══════════════════════════════════════════════════════════════


def slice_bars(bars: list[Bar], start_date: str, end_date: str) -> list[Bar]:
    """按日期范围切片 bar 序列。"""
    start = pd.Timestamp(start_date)
    end = pd.Timestamp(end_date)
    return [b for b in bars if start <= pd.Timestamp(b.ts) <= end]


def analyze_edge_in_window(analysis: EdgeAnalysis, start_date: str, end_date: str) -> dict:
    """分析单条边在指定时间窗口内的事件。"""
    start = pd.Timestamp(start_date)
    end = pd.Timestamp(end_date)

    window_events = [
        ev for ev in analysis.events
        if start <= pd.Timestamp(ev["ts"]) <= end
    ]

    bsp_events = [ev for ev in window_events
                  if ev["event_type"] in ("bsp_candidate", "bsp_confirm", "bsp_settle")]
    move_events = [ev for ev in window_events
                   if ev["event_type"] in ("move_candidate", "move_settle")]
    zs_events = [ev for ev in window_events
                 if ev["event_type"] in ("zhongshu_candidate", "zhongshu_settle")]

    return {
        "total_events": len(window_events),
        "bsp_events": len(bsp_events),
        "move_events": len(move_events),
        "zs_events": len(zs_events),
        "is_active": len(move_events) > 0,
        "has_divergence": len(bsp_events) > 0,
    }


def run_window_comparison(
    edge_bar_streams: dict[str, list[Bar]],
    edge_full_analyses: dict[str, EdgeAnalysis],
) -> dict[str, dict]:
    """对两个窗口做对等比较。

    关键：每条边在每个窗口内独立跑 RecursiveOrchestrator。
    不是从全历史结果里截取事件（那样背驰判断跨越窗口边界，不对等）。
    """
    window_results: dict[str, dict] = {}

    for window_name, (start_date, end_date) in COMPARISON_WINDOWS.items():
        print(f"\n  === {window_name} ({start_date} ~ {end_date}) ===")

        edge_analyses: dict[str, EdgeAnalysis] = {}

        for edge_name, full_bars in edge_bar_streams.items():
            windowed_bars = slice_bars(full_bars, start_date, end_date)
            if not windowed_bars:
                print(f"    {edge_name}: 无数据（窗口外）")
                edge_analyses[edge_name] = EdgeAnalysis(edge_name=edge_name)
                continue

            print(f"    {edge_name} ({len(windowed_bars)} bars) ...", end=" ", flush=True)
            t_edge = time.time()
            analysis = run_edge(f"{edge_name}_{window_name}", windowed_bars)
            analysis.edge_name = edge_name  # 恢复原始边名
            elapsed = time.time() - t_edge
            edge_analyses[edge_name] = analysis
            print(f"{elapsed:.1f}s | "
                  f"moves={analysis.move_count} "
                  f"settled={analysis.settled_move_count} "
                  f"trend={analysis.trend_count} "
                  f"consol={analysis.consolidation_count} "
                  f"bsp={analysis.bsp_count} "
                  f"events={len(analysis.events)}")

        # 汇总
        pure_asset_edges = ["E/Au", "Au/R", "E/R"]
        cash_edges = ["E/$", "Au/$", "R/$"]

        active_edges = [n for n, a in edge_analyses.items() if a.move_count > 0]
        diverging_edges = [n for n, a in edge_analyses.items() if a.bsp_count > 0]
        silent_edges = [n for n, a in edge_analyses.items() if a.move_count == 0]

        pure_asset_active = [e for e in active_edges if e in pure_asset_edges]
        cash_active = [e for e in active_edges if e in cash_edges]
        corner_edges = [n for n in edge_analyses if n not in pure_asset_edges + cash_edges]
        corner_active = [e for e in active_edges if e in corner_edges]

        # Au/R 和 R/$ 的详细状态（关键对比项）
        au_r = edge_analyses.get("Au/R", EdgeAnalysis(edge_name="Au/R"))
        r_usd = edge_analyses.get("R/$", EdgeAnalysis(edge_name="R/$"))

        edge_detail_table = {}
        for name, a in edge_analyses.items():
            edge_detail_table[name] = {
                "bars": a.total_bars,
                "date_range": a.date_range,
                "moves": a.move_count,
                "settled": a.settled_move_count,
                "trend": a.trend_count,
                "consolidation": a.consolidation_count,
                "bsp": a.bsp_count,
                "events": len(a.events),
                "last_move_kind": a.last_move_kind,
                "last_move_direction": a.last_move_direction,
                "moves_detail": a.moves_detail,
            }

        window_results[window_name] = {
            "window_name": window_name,
            "date_range": f"{start_date} ~ {end_date}",
            "total_edges": len(edge_analyses),
            "active_edges": active_edges,
            "diverging_edges": diverging_edges,
            "silent_edges": silent_edges,
            "active_count": len(active_edges),
            "diverging_count": len(diverging_edges),
            "pure_asset_active": pure_asset_active,
            "cash_active": cash_active,
            "corner_active": corner_active,
            "narrative": {
                "pure_asset_vs_cash_divergence": (
                    len(pure_asset_active) > 0 and len(cash_active) > 0
                ),
                "corner_showing_structure": len(corner_active) > 0,
            },
            "key_edges": {
                "Au/R": {
                    "active": au_r.move_count > 0,
                    "has_divergence": au_r.bsp_count > 0,
                    "trend_count": au_r.trend_count,
                    "consolidation_count": au_r.consolidation_count,
                    "bsp_count": au_r.bsp_count,
                    "last_move_kind": au_r.last_move_kind,
                    "last_move_direction": au_r.last_move_direction,
                },
                "R/$": {
                    "active": r_usd.move_count > 0,
                    "has_divergence": r_usd.bsp_count > 0,
                    "trend_count": r_usd.trend_count,
                    "consolidation_count": r_usd.consolidation_count,
                    "bsp_count": r_usd.bsp_count,
                    "last_move_kind": r_usd.last_move_kind,
                    "last_move_direction": r_usd.last_move_direction,
                },
            },
            "edge_details": edge_detail_table,
        }

        # 打印摘要
        print(f"\n  活跃边: {len(active_edges)} / {len(edge_analyses)}")
        print(f"  背驰边: {len(diverging_edges)}")
        print(f"  Au/R: active={au_r.move_count > 0}, "
              f"divergence={au_r.bsp_count > 0}, "
              f"trend={au_r.trend_count}, consol={au_r.consolidation_count}")
        print(f"  R/$:  active={r_usd.move_count > 0}, "
              f"divergence={r_usd.bsp_count > 0}, "
              f"trend={r_usd.trend_count}, consol={r_usd.consolidation_count}")

    return window_results


# ═══════════════════════════════════════════════════════════════
# 报告生成
# ═══════════════════════════════════════════════════════════════


def generate_report(window_results: dict[str, dict]) -> str:
    """生成对比报告。"""
    lines: list[str] = []

    lines.append("# OQ5 补充实验——当前窗口 vs 2007-2009 对等比较")
    lines.append("")
    lines.append(f"生成时间: {datetime.now().isoformat()}")
    lines.append("认识论等级: L2（真实数据验证）")
    lines.append("谱系引用: 284号（OQ v2）、283号、279号、286号")
    lines.append("")

    lines.append("## 1. 实验设计")
    lines.append("")
    lines.append("每条边在每个窗口内**独立**跑 RecursiveOrchestrator。")
    lines.append("不是从全历史结果里截取事件——确保背驰判断不跨越窗口边界。")
    lines.append("两个窗口方法完全一致，可对等比较。")
    lines.append("")

    # 对比总表
    lines.append("## 2. 对比总表")
    lines.append("")
    lines.append("| 指标 | 流动性危机 2007-2009 | 当前周期 2020-2026 |")
    lines.append("|------|---------------------|-------------------|")

    w1 = window_results.get("liquidity_crisis_2007_2009", {})
    w2 = window_results.get("current_2020_2026", {})

    lines.append(f"| 活跃边数 | {w1.get('active_count', '?')} / {w1.get('total_edges', '?')} | "
                 f"{w2.get('active_count', '?')} / {w2.get('total_edges', '?')} |")
    lines.append(f"| 背驰边数 | {w1.get('diverging_count', '?')} | "
                 f"{w2.get('diverging_count', '?')} |")

    # 纯资产边 vs 现金边
    lines.append(f"| 纯资产边活跃 | {', '.join(w1.get('pure_asset_active', []))} | "
                 f"{', '.join(w2.get('pure_asset_active', []))} |")
    lines.append(f"| 现金边活跃 | {', '.join(w1.get('cash_active', []))} | "
                 f"{', '.join(w2.get('cash_active', []))} |")
    lines.append(f"| 叙事不一致 | {w1.get('narrative', {}).get('pure_asset_vs_cash_divergence', '?')} | "
                 f"{w2.get('narrative', {}).get('pure_asset_vs_cash_divergence', '?')} |")
    lines.append(f"| 现金角子图有结构 | {w1.get('narrative', {}).get('corner_showing_structure', '?')} | "
                 f"{w2.get('narrative', {}).get('corner_showing_structure', '?')} |")

    # 关键边
    for key_edge in ["Au/R", "R/$"]:
        k1 = w1.get("key_edges", {}).get(key_edge, {})
        k2 = w2.get("key_edges", {}).get(key_edge, {})
        lines.append(f"| {key_edge} 活跃 | {k1.get('active', '?')} | {k2.get('active', '?')} |")
        lines.append(f"| {key_edge} 背驰 | {k1.get('has_divergence', '?')} | {k2.get('has_divergence', '?')} |")
        lines.append(f"| {key_edge} 趋势/盘整 | {k1.get('trend_count', '?')}/{k1.get('consolidation_count', '?')} | "
                     f"{k2.get('trend_count', '?')}/{k2.get('consolidation_count', '?')} |")
    lines.append("")

    # 各窗口详情
    for window_name, wr in window_results.items():
        lines.append(f"## 3. {wr['window_name']} ({wr['date_range']})")
        lines.append("")
        lines.append(f"- 活跃边: {', '.join(wr['active_edges']) or '无'}")
        lines.append(f"- 背驰边: {', '.join(wr['diverging_edges']) or '无'}")
        lines.append(f"- 沉寂边: {', '.join(wr['silent_edges']) or '无'}")
        lines.append("")

        lines.append("| 边 | bars | 走势 | settled | 趋势 | 盘整 | BSP | 事件 | 末走势 |")
        lines.append("|---|---|---|---|---|---|---|---|---|")
        for edge_name, ed in wr["edge_details"].items():
            lines.append(
                f"| {edge_name} | {ed['bars']} | {ed['moves']} | "
                f"{ed['settled']} | {ed['trend']} | {ed['consolidation']} | "
                f"{ed['bsp']} | {ed['events']} | "
                f"{ed['last_move_kind']} {ed['last_move_direction']} |"
            )
        lines.append("")

    lines.append("## 4. 结果包")
    lines.append("")
    lines.append("1. **结论**: 见对比总表")
    lines.append("2. **定义依据**: 284号 OQ v2——扭曲形式的边界条件阅读")
    lines.append("3. **边界条件**:")
    lines.append("   - 每条边在窗口内独立跑 D 算子，窗口外的走势历史不参与判断")
    lines.append("   - 窗口长度不完全相等（2007-09 三年 vs 2020-26 六年）——更长窗口有更多结构产出机会")
    lines.append("   - FRED 汇率 O=H=L=C 退化数据的影响不变（边界条件继承自 OQ5 主实验）")
    lines.append("4. **下游推论**: Au/R 在当前窗口的状态——如果和 2007-09 不同——是该窗口的结构签名")
    lines.append("5. **谱系引用**: 284号、283号、279号、286号")
    lines.append("6. **影响声明**: 实验结果，不修改现有代码或定义")

    return "\n".join(lines)


# ═══════════════════════════════════════════════════════════════
# 主流程
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    print("=" * 70)
    print("OQ5 补充实验——当前窗口 vs 2007-2009 对等比较")
    print("284号谱系：D 算子 + 背驰判断的联合阅读")
    print("286号：跨实验交叉验证——Au/R 和 R/$ 关键对比")
    print("=" * 70)

    # ── 1. 拉取数据 ──
    print("\n[1/4] 拉取数据")
    raw_data: dict[str, pd.DataFrame] = {}

    for label, ticker in K4_TICKERS_YF.items():
        df = fetch_yfinance(ticker, label)
        if df is not None:
            raw_data[label] = df

    for label, ticker in CASH_CORNER_YF.items():
        df = fetch_yfinance(ticker, label)
        if df is not None:
            raw_data[label] = df

    for label, series_id in CASH_CORNER_FRED.items():
        df = fetch_fred(series_id, label)
        if df is not None:
            raw_data[label] = df

    print(f"\n  可用数据源: {list(raw_data.keys())} ({len(raw_data)} 个)")

    # ── 2. 构造全部边 ──
    print("\n[2/4] 构造全部边")

    edge_dfs: dict[str, pd.DataFrame] = {}

    # K4 顶点边
    for edge_label, ticker, _ in K4_VERTEX_EDGES:
        if ticker in raw_data:
            edge_dfs[edge_label] = raw_data[ticker]
            print(f"  {edge_label} = {ticker}: {len(raw_data[ticker])} bars")

    # K4 比价边
    for edge_label, num, den in K4_RATIO_EDGES:
        if num in raw_data and den in raw_data:
            ratio_df = make_ratio_bars_aligned(raw_data[num], raw_data[den])
            if ratio_df is not None:
                edge_dfs[edge_label] = ratio_df
                print(f"  {edge_label} = {num}/{den}: {len(ratio_df)} bars")

    # 现金角子图
    if "GCF" in raw_data:
        edge_dfs["XAU/USD"] = raw_data["GCF"]
        print(f"  XAU/USD = GC=F: {len(raw_data['GCF'])} bars")

    if "TLT" in raw_data and "GLD" in raw_data:
        ratio_df = make_ratio_bars_aligned(raw_data["TLT"], raw_data["GLD"])
        if ratio_df is not None:
            edge_dfs["R/Au_corner"] = ratio_df
            print(f"  R/Au_corner = TLT/GLD: {len(ratio_df)} bars")

    for fred_label in CASH_CORNER_FRED:
        if fred_label in raw_data:
            edge_dfs[fred_label] = raw_data[fred_label]
            print(f"  {fred_label}: {len(raw_data[fred_label])} bars")

    print(f"\n  总边数: {len(edge_dfs)}")

    # 预转换为 bars
    edge_bar_streams: dict[str, list[Bar]] = {}
    for name, df in edge_dfs.items():
        edge_bar_streams[name] = df_to_bars(df)

    # ── 3. 窗口内独立运行 ──
    print("\n[3/4] 窗口对比分析（每条边在每个窗口内独立跑 D 算子）")

    window_results = run_window_comparison(edge_bar_streams, {})

    # ── 4. 保存结果 ──
    print("\n[4/4] 保存结果")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    results_path = RESULTS_DIR / "oq5-current-window-results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(window_results, f, indent=2, ensure_ascii=False, default=str)
    print(f"  结果 -> {results_path}")

    report = generate_report(window_results)
    report_path = RESULTS_DIR / "oq5-current-window-report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"  报告 -> {report_path}")

    print("\n" + "=" * 70)
    print("OQ5 补充实验完成")
    print("=" * 70)


if __name__ == "__main__":
    main()
