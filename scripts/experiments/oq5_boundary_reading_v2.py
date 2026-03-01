"""OQ5 v2 当前扭曲形式的边界条件阅读。

284号谱系（OQ v2）：D 算子 + 背驰判断的联合阅读。

K4 完全图永远是扭曲的——某个顶点同时是度量基准。
当前特殊性是美元霸权：$ 同时是 K4 的顶点和尺子。
六条边分为纯资产边（E/Au, E/R, Au/R）和现金边（E/$, Au/$, R/$）。
$ 内部有现金角子图（XAU/USD、Treasury/Gold 等锚定物比价）。

OQ5 要读的是：当前这个特殊扭曲形式是否在逼近自身的边界条件。
不是"断了/没断"的二元检测。是阅读扭曲形式的生命周期。

验证窗口：1971（布雷顿森林）、2002-2007（美元贬值周期）、2008（流动性危机）、全历史扫描。

数据源：yfinance + FRED 组合。
认识论等级：L2（真实数据验证）
谱系引用：284号（OQ v2）、283号、279号
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

RESULTS_DIR = ROOT / "tmp" / "oq5-v2-results"


# ═══════════════════════════════════════════════════════════════
# 边定义
# ═══════════════════════════════════════════════════════════════

# K4 六条边
K4_TICKERS_YF = {
    "SPY": "SPY",       # E/$
    "GLD": "GLD",       # Au/$
    "TLT": "TLT",       # R/$
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

# 现金角子图（XAU/USD 和 国债/黄金）
# XAU/USD 用 GC=F（黄金期货）
# 外汇用 FRED 长历史（1971年起）
CASH_CORNER_YF = {
    "GCF": "GC=F",  # XAU/USD (黄金期货)
}

CASH_CORNER_FRED = {
    "DEXJPUS": "DEXJPUS",   # USD/JPY, 1971年起
    "DEXUSUK": "DEXUSUK",   # GBP/USD, 1971年起
    "DEXSZUS": "DEXSZUS",   # USD/CHF, 1971年起
    "DEXUSEU": "DEXUSEU",   # USD/EUR, 1999年起
}

# 验证窗口
VALIDATION_WINDOWS = {
    "bretton_woods_1971": (1971, 1975),
    "usd_weakness_2002_2007": (2002, 2007),
    "liquidity_crisis_2008": (2007, 2009),
    "full_history": None,
}


# ═══════════════════════════════════════════════════════════════
# 数据拉取
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
    """从 FRED CSV API 拉取日线汇率数据，构造伪 OHLCV。

    FRED 汇率只有每日收盘价，构造 O=H=L=C=value, Volume=0。
    """
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
# 单条边运行
# ═══════════════════════════════════════════════════════════════


# 关注的事件类型
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

    # 结构统计
    move_count: int = 0
    settled_move_count: int = 0
    seg_count: int = 0
    zs_count: int = 0
    trend_count: int = 0
    consolidation_count: int = 0

    # 背驰/BSP
    bsp_count: int = 0
    bsp_events_total: int = 0

    # 最新走势状态
    last_move_kind: str = ""
    last_move_direction: str = ""

    # 结构事件时间线
    events: list[dict] = field(default_factory=list)

    # 走势详情
    moves_detail: list[dict] = field(default_factory=list)


def run_edge(edge_name: str, bars: list[Bar]) -> EdgeAnalysis:
    """对单条边运行 RecursiveOrchestrator（D算子+背驰+BSP一体）。"""
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
# 窗口分析——叙事自洽形态
# ═══════════════════════════════════════════════════════════════


def analyze_window(
    edge_analyses: dict[str, EdgeAnalysis],
    window_name: str,
    year_start: int | None,
    year_end: int | None,
) -> dict:
    """分析指定时间窗口内六条边 + 现金角子图的叙事自洽形态。

    记录：
    - 哪几条边有结构事件（活跃）
    - 哪几条边的结构事件包含背驰/BSP
    - 边之间的矛盾（同时期不同方向的走势）
    - 矛盾在什么级别
    """
    window_results: dict[str, dict] = {}

    for edge_name, analysis in edge_analyses.items():
        if year_start is not None and year_end is not None:
            window_events = [
                ev for ev in analysis.events
                if year_start <= datetime.fromisoformat(ev["ts"]).year <= year_end
            ]
        else:
            window_events = analysis.events

        bsp_events = [ev for ev in window_events
                      if ev["event_type"] in ("bsp_candidate", "bsp_confirm", "bsp_settle")]
        move_events = [ev for ev in window_events
                       if ev["event_type"] in ("move_candidate", "move_settle")]
        zs_events = [ev for ev in window_events
                     if ev["event_type"] in ("zhongshu_candidate", "zhongshu_settle")]

        window_results[edge_name] = {
            "total_events": len(window_events),
            "bsp_events": len(bsp_events),
            "move_events": len(move_events),
            "zs_events": len(zs_events),
            "is_active": len(move_events) > 0,
            "has_divergence": len(bsp_events) > 0,
        }

    # 分类
    active_edges = [name for name, r in window_results.items() if r["is_active"]]
    silent_edges = [name for name, r in window_results.items() if not r["is_active"]]
    diverging_edges = [name for name, r in window_results.items() if r["has_divergence"]]

    # 纯资产边 vs 现金边分离
    pure_asset_edges = ["E/Au", "Au/R", "E/R"]
    cash_edges = ["E/$", "Au/$", "R/$"]
    corner_edges = [name for name in edge_analyses if name not in pure_asset_edges + cash_edges]

    pure_asset_active = [e for e in active_edges if e in pure_asset_edges]
    cash_active = [e for e in active_edges if e in cash_edges]
    corner_active = [e for e in active_edges if e in corner_edges]

    return {
        "window_name": window_name,
        "year_range": f"{year_start}-{year_end}" if year_start else "全历史",
        "edge_details": window_results,
        "active_edges": active_edges,
        "silent_edges": silent_edges,
        "diverging_edges": diverging_edges,
        "pure_asset_active": pure_asset_active,
        "cash_active": cash_active,
        "corner_active": corner_active,
        "narrative": {
            "pure_asset_vs_cash_divergence": (
                len(pure_asset_active) > 0 and len(cash_active) > 0
                and set(pure_asset_active) != set(cash_active)
            ),
            "corner_showing_structure": len(corner_active) > 0,
        },
    }


# ═══════════════════════════════════════════════════════════════
# 报告生成
# ═══════════════════════════════════════════════════════════════


def generate_report(
    edge_analyses: dict[str, EdgeAnalysis],
    window_analyses: dict[str, dict],
) -> str:
    """生成 Markdown 报告。"""
    lines: list[str] = []

    lines.append("# OQ5 v2 当前扭曲形式的边界条件阅读报告")
    lines.append("")
    lines.append(f"生成时间: {datetime.now().isoformat()}")
    lines.append("认识论等级: L2（真实数据验证）")
    lines.append("谱系引用: 284号（OQ v2）、283号、279号")
    lines.append("")

    lines.append("## 1. 284号框架")
    lines.append("")
    lines.append("- 扭曲是普遍的（任何结算体制都有一个顶点同时充当尺子）")
    lines.append("- 每一个扭曲都是特殊的（美元霸权是这个具体的扭曲形式）")
    lines.append("- 不是'断了/没断'的二元检测")
    lines.append("- 是阅读这个特殊扭曲形式的生命周期：")
    lines.append("  以什么形态存在、以什么方式逼近边界、征兆在六条边的图上呈现什么结构")
    lines.append("- D 算子 + 背驰判断一起跑在每条边上")
    lines.append("")

    lines.append("## 2. 各边结构产出")
    lines.append("")
    lines.append("| 边 | bars | 日期范围 | 走势 | settled | 趋势 | 盘整 | 线段 | 中枢 | BSP | 结构事件 |")
    lines.append("|---|---|---|---|---|---|---|---|---|---|---|")
    for name, a in edge_analyses.items():
        lines.append(
            f"| {name} | {a.total_bars} | {a.date_range} | "
            f"{a.move_count} | {a.settled_move_count} | "
            f"{a.trend_count} | {a.consolidation_count} | "
            f"{a.seg_count} | {a.zs_count} | {a.bsp_count} | {len(a.events)} |"
        )
    lines.append("")

    lines.append("## 3. 验证窗口分析")
    lines.append("")
    for window_name, wa in window_analyses.items():
        lines.append(f"### {window_name} ({wa['year_range']})")
        lines.append("")
        lines.append(f"- 活跃边: {', '.join(wa['active_edges']) or '无'}")
        lines.append(f"- 沉寂边: {', '.join(wa['silent_edges']) or '无'}")
        lines.append(f"- 有背驰/BSP的边: {', '.join(wa['diverging_edges']) or '无'}")
        lines.append(f"- 纯资产边活跃: {', '.join(wa['pure_asset_active']) or '无'}")
        lines.append(f"- 现金边活跃: {', '.join(wa['cash_active']) or '无'}")
        lines.append(f"- 现金角子图活跃: {', '.join(wa['corner_active']) or '无'}")
        lines.append("")

        narr = wa["narrative"]
        if narr["pure_asset_vs_cash_divergence"]:
            lines.append("- **纯资产边和现金边叙事不一致** ← 扭曲的直接显影")
        if narr["corner_showing_structure"]:
            lines.append("- **现金角子图出现走势结构** ← 可能是边界条件逼近的征兆")
        lines.append("")

        lines.append("| 边 | 事件总数 | BSP事件 | 走势事件 | 中枢事件 | 活跃 | 背驰 |")
        lines.append("|---|--------|--------|---------|---------|------|------|")
        for edge_name, details in wa["edge_details"].items():
            lines.append(
                f"| {edge_name} | {details['total_events']} | "
                f"{details['bsp_events']} | {details['move_events']} | "
                f"{details['zs_events']} | "
                f"{'Y' if details['is_active'] else 'N'} | "
                f"{'Y' if details['has_divergence'] else 'N'} |"
            )
        lines.append("")

    lines.append("## 4. 叙事形态分类")
    lines.append("")
    lines.append("分类依据不是外在阈值，是形态本身的结构性质：")
    lines.append("")
    lines.append("- **正常震荡**: 纯资产边和现金边叙事一致，现金角子图沉寂")
    lines.append("- **逼近征兆**: 纯资产边和现金边叙事分离，或现金角子图出现大级别结构事件")
    lines.append("- **历史异常**: 窗口内出现历史未见的不自洽模式")
    lines.append("")

    lines.append("## 5. 结果包")
    lines.append("")
    lines.append("1. **结论**: 各窗口的叙事自洽形态分析（见上）")
    lines.append("2. **定义依据**: 284号 OQ v2——扭曲形式的边界条件阅读")
    lines.append("3. **边界条件**:")
    lines.append("   - FRED 汇率数据仅有日收盘价（O=H=L=C），缺少日内波动")
    lines.append("   - yfinance 的 GC=F 和 ETF 有真实 OHLCV，与 FRED 数据结构不对等")
    lines.append("   - ETF 上市日期限制部分窗口的覆盖范围")
    lines.append("4. **下游推论**: 纯资产边和现金边叙事分离 + 现金角子图出现结构 = 边界条件逼近的候选信号")
    lines.append("5. **谱系引用**: 284号、283号、279号")
    lines.append("6. **影响声明**: 实验结果，不修改现有代码或定义")

    return "\n".join(lines)


# ═══════════════════════════════════════════════════════════════
# 主流程
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    print("=" * 70)
    print("OQ5 v2 当前扭曲形式的边界条件阅读")
    print("284号谱系：D 算子 + 背驰判断的联合阅读")
    print("数据源: yfinance + FRED (1971年起)")
    print("=" * 70)

    # ── 1. 拉取数据 ──
    print("\n[1/5] 拉取数据")
    raw_data: dict[str, pd.DataFrame] = {}

    # K4 顶点 ETF
    for label, ticker in K4_TICKERS_YF.items():
        df = fetch_yfinance(ticker, label)
        if df is not None:
            raw_data[label] = df

    # 现金角子图 yfinance
    for label, ticker in CASH_CORNER_YF.items():
        df = fetch_yfinance(ticker, label)
        if df is not None:
            raw_data[label] = df

    # FRED 外汇
    for label, series_id in CASH_CORNER_FRED.items():
        df = fetch_fred(series_id, label)
        if df is not None:
            raw_data[label] = df

    print(f"\n  可用数据源: {list(raw_data.keys())} ({len(raw_data)} 个)")

    # ── 2. 构造全部边 ──
    print("\n[2/5] 构造全部边")

    edge_bar_streams: dict[str, list[Bar]] = {}

    # K4 顶点边（原始品种）
    for edge_label, ticker, _ in K4_VERTEX_EDGES:
        if ticker in raw_data:
            edge_bar_streams[edge_label] = df_to_bars(raw_data[ticker])
            print(f"  {edge_label} = {ticker}: {len(edge_bar_streams[edge_label])} bars")

    # K4 比价边
    for edge_label, num, den in K4_RATIO_EDGES:
        if num in raw_data and den in raw_data:
            ratio_df = make_ratio_bars_aligned(raw_data[num], raw_data[den])
            if ratio_df is not None:
                edge_bar_streams[edge_label] = df_to_bars(ratio_df)
                print(f"  {edge_label} = {num}/{den}: {len(edge_bar_streams[edge_label])} bars")

    # 现金角子图边
    # XAU/USD = GC=F（直接使用）
    if "GCF" in raw_data:
        edge_bar_streams["XAU/USD"] = df_to_bars(raw_data["GCF"])
        print(f"  XAU/USD = GC=F: {len(edge_bar_streams['XAU/USD'])} bars")

    # Treasury/Gold = TLT/GLD
    if "TLT" in raw_data and "GLD" in raw_data:
        ratio_df = make_ratio_bars_aligned(raw_data["TLT"], raw_data["GLD"])
        if ratio_df is not None:
            edge_bar_streams["R/Au_corner"] = df_to_bars(ratio_df)
            print(f"  R/Au_corner = TLT/GLD: {len(edge_bar_streams['R/Au_corner'])} bars")

    # FRED 外汇对
    for fred_label in CASH_CORNER_FRED:
        if fred_label in raw_data:
            edge_bar_streams[fred_label] = df_to_bars(raw_data[fred_label])
            print(f"  {fred_label}: {len(edge_bar_streams[fred_label])} bars")

    print(f"\n  总边数: {len(edge_bar_streams)}")

    # ── 3. 逐边运行 RecursiveOrchestrator ──
    print("\n[3/5] 运行 RecursiveOrchestrator（D算子+背驰+BSP一体）")
    edge_analyses: dict[str, EdgeAnalysis] = {}
    t0 = time.time()

    for i, (edge_name, bars) in enumerate(edge_bar_streams.items()):
        print(f"  [{i+1}/{len(edge_bar_streams)}] {edge_name} ({len(bars)} bars) ...",
              end=" ", flush=True)
        t_edge = time.time()
        analysis = run_edge(edge_name, bars)
        elapsed = time.time() - t_edge
        edge_analyses[edge_name] = analysis
        print(f"{elapsed:.1f}s | "
              f"moves={analysis.move_count} "
              f"settled={analysis.settled_move_count} "
              f"bsp={analysis.bsp_count} "
              f"events={len(analysis.events)}")

    total_elapsed = time.time() - t0
    print(f"\n  全部边完成: {total_elapsed:.1f}s")

    # ── 4. 窗口分析 ──
    print("\n[4/5] 窗口分析")
    window_analyses: dict[str, dict] = {}

    for window_name, year_range in VALIDATION_WINDOWS.items():
        if year_range is not None:
            y_start, y_end = year_range
        else:
            y_start, y_end = None, None

        wa = analyze_window(edge_analyses, window_name, y_start, y_end)
        window_analyses[window_name] = wa

        print(f"\n  === {window_name} ({wa['year_range']}) ===")
        print(f"  活跃边: {len(wa['active_edges'])} / {len(edge_analyses)}")
        print(f"  有背驰: {len(wa['diverging_edges'])}")
        if wa["narrative"]["pure_asset_vs_cash_divergence"]:
            print(f"  *** 纯资产边和现金边叙事不一致 ***")
        if wa["narrative"]["corner_showing_structure"]:
            print(f"  *** 现金角子图出现走势结构 ***")

    # ── 5. 保存结果 ──
    print("\n[5/5] 保存结果")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    # 边摘要
    edge_summary = {}
    for name, analysis in edge_analyses.items():
        edge_summary[name] = {
            "total_bars": analysis.total_bars,
            "date_range": analysis.date_range,
            "move_count": analysis.move_count,
            "settled_move_count": analysis.settled_move_count,
            "seg_count": analysis.seg_count,
            "zs_count": analysis.zs_count,
            "trend_count": analysis.trend_count,
            "consolidation_count": analysis.consolidation_count,
            "bsp_count": analysis.bsp_count,
            "bsp_events_total": analysis.bsp_events_total,
            "last_move_kind": analysis.last_move_kind,
            "last_move_direction": analysis.last_move_direction,
            "total_events": len(analysis.events),
            "moves_detail": analysis.moves_detail,
        }

    result_package = {
        "metadata": {
            "experiment": "OQ5 v2 当前扭曲形式的边界条件阅读",
            "pipeline": "RecursiveOrchestrator（D算子+背驰+BSP一体）",
            "epistemological_level": "L2（真实数据验证）",
            "genealogy_ref": ["284号（OQ v2）", "283号", "279号"],
            "generated_at": datetime.now().isoformat(),
            "data_sources": {
                "yfinance": list(K4_TICKERS_YF.values()) + list(CASH_CORNER_YF.values()),
                "fred": list(CASH_CORNER_FRED.keys()),
            },
            "total_edges": len(edge_analyses),
        },
        "edge_summary": edge_summary,
        "window_analyses": window_analyses,
        "validation_windows": {
            name: {"start": rng[0], "end": rng[1]} if rng else {"start": None, "end": None}
            for name, rng in VALIDATION_WINDOWS.items()
        },
    }

    results_path = RESULTS_DIR / "oq5-v2-results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(result_package, f, indent=2, ensure_ascii=False, default=str)
    print(f"  结果 -> {results_path}")

    report = generate_report(edge_analyses, window_analyses)
    report_path = RESULTS_DIR / "oq5-v2-report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"  报告 -> {report_path}")

    print("\n" + "=" * 70)
    print("OQ5 v2 完成")
    print("=" * 70)


if __name__ == "__main__":
    main()
