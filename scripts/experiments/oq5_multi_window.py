"""OQ5-ext 多窗口扩展实验——Au/R 在不同危机窗口的行为验证。

287号下游推论1：Au/R 背驰状态是否区分'逼近边界'的不同阶段——
待更多历史窗口（如 1998 亚洲金融危机、2011 欧债危机）验证。

受 ETF 上市时间限制（GLD=2004-11, TLT=2002-07, SPY=1993-01），
K4 六条边 + XAU/USD + R/Au_corner 最早可比窗口从 2005 年开始。
FRED 汇率边可追溯到 1971 年，但不构成 Au/R 的直接验证。

窗口设计（6 个对等窗口，每个 2-3 年）：
  1. pre_crisis_2005_2006:    2005-01-01 ~ 2006-12-31  前危机平静期
  2. liquidity_crisis_2007_2009: 2007-01-01 ~ 2009-12-31  流动性危机（参照组）
  3. euro_debt_2010_2012:     2010-01-01 ~ 2012-12-31  欧债危机
  4. china_shock_2015_2016:   2015-01-01 ~ 2016-12-31  中国股灾/人民币贬值
  5. trade_war_2018_2019:     2018-01-01 ~ 2019-12-31  贸易战
  6. current_2020_2026:       2020-01-01 ~ 2026-03-01  当前周期（参照组）

核心对比项：
  - Au/R 在每个窗口是否激活（有走势产出）？
  - Au/R 是否有背驰（力度衰竭信号）？
  - 沉寂边位置是否因窗口不同而变化？
  - 全部边在各窗口内是否仍然全部盘整、0 趋势？

认识论等级：L2（真实数据验证——多窗口，但同一标的集）
谱系引用：287号（综合结论）、284号（OQ v2）、283号、279号、286号
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

RESULTS_DIR = ROOT / "tmp" / "oq5-multi-window-results"

# ═══════════════════════════════════════════════════════════════
# 边定义（与 OQ5 主实验 + 补充实验完全一致）
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

# 六个对等比较窗口
COMPARISON_WINDOWS = {
    "pre_crisis_2005_2006": ("2005-01-01", "2006-12-31"),
    "liquidity_crisis_2007_2009": ("2007-01-01", "2009-12-31"),
    "euro_debt_2010_2012": ("2010-01-01", "2012-12-31"),
    "china_shock_2015_2016": ("2015-01-01", "2016-12-31"),
    "trade_war_2018_2019": ("2018-01-01", "2019-12-31"),
    "current_2020_2026": ("2020-01-01", "2026-03-01"),
}

# 窗口的人类可读标签
WINDOW_LABELS = {
    "pre_crisis_2005_2006": "前危机平静期 2005-2006",
    "liquidity_crisis_2007_2009": "流动性危机 2007-2009",
    "euro_debt_2010_2012": "欧债危机 2010-2012",
    "china_shock_2015_2016": "中国股灾 2015-2016",
    "trade_war_2018_2019": "贸易战 2018-2019",
    "current_2020_2026": "当前周期 2020-2026",
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


def run_window_comparison(
    edge_bar_streams: dict[str, list[Bar]],
) -> dict[str, dict]:
    """对多个窗口做对等比较。

    关键：每条边在每个窗口内独立跑 RecursiveOrchestrator。
    不是从全历史结果里截取事件（那样背驰判断跨越窗口边界，不对等）。
    """
    window_results: dict[str, dict] = {}

    for window_name, (start_date, end_date) in COMPARISON_WINDOWS.items():
        label = WINDOW_LABELS.get(window_name, window_name)
        print(f"\n  === {label} ({start_date} ~ {end_date}) ===")

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
            analysis.edge_name = edge_name
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

        # 关键边状态
        au_r = edge_analyses.get("Au/R", EdgeAnalysis(edge_name="Au/R"))
        r_usd = edge_analyses.get("R/$", EdgeAnalysis(edge_name="R/$"))
        e_usd = edge_analyses.get("E/$", EdgeAnalysis(edge_name="E/$"))

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
            "label": label,
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
            "key_edges": {
                "Au/R": {
                    "active": au_r.move_count > 0,
                    "has_divergence": au_r.bsp_count > 0,
                    "trend_count": au_r.trend_count,
                    "consolidation_count": au_r.consolidation_count,
                    "bsp_count": au_r.bsp_count,
                    "move_count": au_r.move_count,
                    "last_move_kind": au_r.last_move_kind,
                    "last_move_direction": au_r.last_move_direction,
                },
                "R/$": {
                    "active": r_usd.move_count > 0,
                    "has_divergence": r_usd.bsp_count > 0,
                    "trend_count": r_usd.trend_count,
                    "consolidation_count": r_usd.consolidation_count,
                    "bsp_count": r_usd.bsp_count,
                    "move_count": r_usd.move_count,
                    "last_move_kind": r_usd.last_move_kind,
                    "last_move_direction": r_usd.last_move_direction,
                },
                "E/$": {
                    "active": e_usd.move_count > 0,
                    "has_divergence": e_usd.bsp_count > 0,
                    "trend_count": e_usd.trend_count,
                    "consolidation_count": e_usd.consolidation_count,
                    "bsp_count": e_usd.bsp_count,
                    "move_count": e_usd.move_count,
                    "last_move_kind": e_usd.last_move_kind,
                    "last_move_direction": e_usd.last_move_direction,
                },
            },
            "edge_details": edge_detail_table,
        }

        # 打印摘要
        print(f"\n  活跃边: {len(active_edges)} / {len(edge_analyses)}")
        print(f"  背驰边: {len(diverging_edges)}")
        print(f"  Au/R: active={au_r.move_count > 0}, "
              f"divergence={au_r.bsp_count > 0}, "
              f"moves={au_r.move_count}, "
              f"trend={au_r.trend_count}, consol={au_r.consolidation_count}, "
              f"bsp={au_r.bsp_count}")
        print(f"  R/$:  active={r_usd.move_count > 0}, "
              f"divergence={r_usd.bsp_count > 0}, "
              f"moves={r_usd.move_count}, "
              f"trend={r_usd.trend_count}, consol={r_usd.consolidation_count}")
        print(f"  E/$:  active={e_usd.move_count > 0}, "
              f"divergence={e_usd.bsp_count > 0}, "
              f"moves={e_usd.move_count}")

    return window_results


# ═══════════════════════════════════════════════════════════════
# 报告生成
# ═══════════════════════════════════════════════════════════════


def generate_report(window_results: dict[str, dict]) -> str:
    """生成多窗口对比报告。"""
    lines: list[str] = []

    lines.append("# OQ5-ext 多窗口扩展实验——Au/R 在不同危机窗口的行为验证")
    lines.append("")
    lines.append(f"生成时间: {datetime.now().isoformat()}")
    lines.append("认识论等级: L2（真实数据验证——多窗口，同一标的集）")
    lines.append("谱系引用: 287号（综合结论）、284号（OQ v2）、283号、279号、286号")
    lines.append("")

    # 实验设计
    lines.append("## 1. 实验设计")
    lines.append("")
    lines.append("验证 287号下游推论1：Au/R 背驰状态是否区分'逼近边界'的不同阶段。")
    lines.append("")
    lines.append("每条边在每个窗口内**独立**跑 RecursiveOrchestrator。")
    lines.append("不从全历史结果截取事件——确保背驰判断不跨越窗口边界。")
    lines.append("六个窗口方法完全一致，可对等比较。")
    lines.append("")
    lines.append("| 窗口 | 日期范围 | 背景 |")
    lines.append("|------|---------|------|")
    for wn, (s, e) in COMPARISON_WINDOWS.items():
        lines.append(f"| {WINDOW_LABELS[wn]} | {s} ~ {e} | |")
    lines.append("")

    # Au/R 对比总表（核心）
    lines.append("## 2. Au/R 多窗口对比（核心对比项）")
    lines.append("")
    lines.append("| 窗口 | Au/R 激活 | 走势数 | 背驰(BSP) | 趋势 | 盘整 | 末走势 |")
    lines.append("|------|---------|--------|----------|------|------|--------|")
    for wn in COMPARISON_WINDOWS:
        wr = window_results.get(wn, {})
        ke = wr.get("key_edges", {}).get("Au/R", {})
        lines.append(
            f"| {WINDOW_LABELS.get(wn, wn)} | "
            f"{'Yes' if ke.get('active') else 'No'} | "
            f"{ke.get('move_count', 0)} | "
            f"{ke.get('bsp_count', 0)} | "
            f"{ke.get('trend_count', 0)} | "
            f"{ke.get('consolidation_count', 0)} | "
            f"{ke.get('last_move_kind', '')} {ke.get('last_move_direction', '')} |"
        )
    lines.append("")

    # 沉寂边位置对比
    lines.append("## 3. 沉寂边位置对比")
    lines.append("")
    lines.append("| 窗口 | 沉寂边 | E/$ | Au/$ | R/$ |")
    lines.append("|------|--------|-----|------|-----|")
    for wn in COMPARISON_WINDOWS:
        wr = window_results.get(wn, {})
        silent = ", ".join(wr.get("silent_edges", [])) or "无"
        e_active = "active" if wr.get("key_edges", {}).get("E/$", {}).get("active") else "silent"
        # Au/$ 不在 key_edges 中，从 edge_details 获取
        au_detail = wr.get("edge_details", {}).get("Au/$", {})
        au_active = "active" if au_detail.get("moves", 0) > 0 else "silent"
        r_active = "active" if wr.get("key_edges", {}).get("R/$", {}).get("active") else "silent"
        lines.append(
            f"| {WINDOW_LABELS.get(wn, wn)} | {silent} | {e_active} | {au_active} | {r_active} |"
        )
    lines.append("")

    # 全景总表
    lines.append("## 4. 全景总表")
    lines.append("")
    lines.append("| 窗口 | 活跃边 | 背驰边 | 趋势边 | 全盘整 |")
    lines.append("|------|--------|--------|--------|--------|")
    for wn in COMPARISON_WINDOWS:
        wr = window_results.get(wn, {})
        total_trend = sum(
            ed.get("trend", 0)
            for ed in wr.get("edge_details", {}).values()
        )
        all_consol = total_trend == 0
        lines.append(
            f"| {WINDOW_LABELS.get(wn, wn)} | "
            f"{wr.get('active_count', 0)}/{wr.get('total_edges', 0)} | "
            f"{wr.get('diverging_count', 0)} | "
            f"{total_trend} | "
            f"{'Yes' if all_consol else 'No'} |"
        )
    lines.append("")

    # 各窗口详情
    for wn in COMPARISON_WINDOWS:
        wr = window_results.get(wn, {})
        if not wr:
            continue
        lines.append(f"## 5-{list(COMPARISON_WINDOWS.keys()).index(wn)+1}. {wr['label']} ({wr['date_range']})")
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

    # 结果包
    lines.append("## 6. 结果包")
    lines.append("")
    lines.append("1. **结论**: 见 Au/R 多窗口对比表（第2节）和全景总表（第4节）")
    lines.append("2. **定义依据**: 284号 OQ v2——扭曲形式的边界条件阅读；287号综合结论中 Au/R 假说")
    lines.append("3. **边界条件**:")
    lines.append("   - 每条边在窗口内独立跑 D 算子，窗口外的走势历史不参与判断")
    lines.append("   - 窗口长度不完全相等（2年/3年/6年混合）——更长窗口有更多结构产出机会")
    lines.append("   - GLD 上市时间 2004-11，前两个窗口的 Au 相关边（Au/$, E/Au, Au/R, XAU/USD, R/Au_corner）数据有效")
    lines.append("   - FRED 汇率 O=H=L=C 退化数据的影响不变")
    lines.append("4. **下游推论**: Au/R 的激活/背驰模式在六个窗口中的分布——如果只在危机窗口激活且仅在当前窗口有背驰，则 287号结论加固")
    lines.append("5. **谱系引用**: 287号、284号、283号、279号、286号")
    lines.append("6. **影响声明**: 实验结果，不修改现有代码或定义")

    return "\n".join(lines)


# ═══════════════════════════════════════════════════════════════
# 主流程
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    print("=" * 70)
    print("OQ5-ext 多窗口扩展实验——Au/R 在不同危机窗口的行为验证")
    print("287号下游推论1 验证")
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

    # ── 3. 多窗口独立运行 ──
    print(f"\n[3/4] 多窗口对比分析（{len(COMPARISON_WINDOWS)} 个窗口 x {len(edge_dfs)} 条边）")
    print("每条边在每个窗口内独立跑 D 算子")

    t_start = time.time()
    window_results = run_window_comparison(edge_bar_streams)
    t_total = time.time() - t_start
    print(f"\n  总耗时: {t_total:.1f}s")

    # ── 4. 保存结果 ──
    print("\n[4/4] 保存结果")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    results_path = RESULTS_DIR / "oq5-multi-window-results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(window_results, f, indent=2, ensure_ascii=False, default=str)
    print(f"  结果 -> {results_path}")

    report = generate_report(window_results)
    report_path = RESULTS_DIR / "oq5-multi-window-report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"  报告 -> {report_path}")

    print("\n" + "=" * 70)
    print("OQ5-ext 多窗口扩展实验完成")
    print(f"6个窗口 x 12条边 = 72次独立D算子运行")
    print("=" * 70)


if __name__ == "__main__":
    main()
