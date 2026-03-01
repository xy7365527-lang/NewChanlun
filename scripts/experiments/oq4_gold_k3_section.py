"""OQ4 黄金截面 K3 实验 — ^GSPC/GC=F/TLT 三条边 vs K4 六条边。

K4 退化为 K3：三个顶点 E、R、Au/$，三条边。
黄金作为尺子，消除美元噪声。

K3 三条边（黄金计价）：
  1. E/Au  = ^GSPC ÷ GC=F — S&P500 用黄金量
  2. R/Au  = TLT ÷ GLD   — 债券用黄金量（yfinance, 2004年起）
  3. E/R   = SPY/TLT      — 不经过黄金，保留

K4 六条边（美元计价参照）：
  顶点边：E/$=SPY, Au/$=GLD, R/$=TLT
  比价边：E/Au=SPY/GLD, Au/R=GLD/TLT, E/R=SPY/TLT

数据源：yfinance。最长最纯净原则。
口径A管线：RecursiveOrchestrator 从日线递归。

认识论等级：L2（真实数据验证）
谱系引用：279号（口径A验证）、277号（级别口径修正）
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "src"))

import pandas as pd
import yfinance as yf

from newchan.orchestrator.recursive import RecursiveOrchestrator, RecursiveOrchestratorSnapshot
from newchan.topology.k4_scanner import walk_direction_from_snapshot
from newchan.topology.config_space import WalkDirection
from newchan.backtest.types import extract_d_reading, DOperatorReading, Direction
from newchan.types import Bar


# ===================================================================
# 常量
# ===================================================================

# K3 截面数据源（黄金计价）
K3_TICKERS = {
    "GSPC": "^GSPC",   # S&P 500 index
    "GCF": "GC=F",     # Gold futures
    "SPY": "SPY",       # E/R 边的分子
    "TLT": "TLT",       # R（债券）
    "GLD": "GLD",       # R/Au 边的分母
}

# K3 三条边定义: (edge_label, vertex_from, vertex_to, numerator_key, denominator_key)
K3_EDGE_DEFS = [
    ("E/Au", "E", "Au", "GSPC", "GCF"),    # ^GSPC / GC=F
    ("R/Au", "R", "Au", "TLT", "GLD"),      # TLT / GLD
    ("E/R", "E", "R", "SPY", "TLT"),        # SPY / TLT
]

# K4 六条边定义（美元截面参照）
K4_VERTEX_EDGES = [
    ("E/$", "E", "$", "SPY", None),
    ("Au/$", "Au", "$", "GLD", None),
    ("R/$", "R", "$", "TLT", None),
]
K4_RATIO_EDGES = [
    ("E/Au", "E", "Au", "SPY", "GLD"),
    ("Au/R", "Au", "R", "GLD", "TLT"),
    ("E/R", "E", "R", "SPY", "TLT"),
]
K4_ALL_EDGES = K4_VERTEX_EDGES + K4_RATIO_EDGES

RESULTS_JSON = ROOT / "tmp" / "oq4-gold-k3-results.json"
REPORT_MD = ROOT / "tmp" / "oq4-gold-k3-report.md"


# ===================================================================
# 数据拉取
# ===================================================================


def fetch_daily(ticker: str, label: str) -> pd.DataFrame | None:
    """拉取日线数据。失败返回 None。"""
    print(f"  [{label}] 拉取 {ticker} ...", end=" ", flush=True)
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


def df_to_bars(df: pd.DataFrame) -> tuple[list[Bar], list[datetime]]:
    """DataFrame -> (Bar list, timestamp list)."""
    bars: list[Bar] = []
    timestamps: list[datetime] = []
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
        timestamps.append(ts)
    return bars, timestamps


def make_ratio_bars(
    bars_a: list[Bar],
    bars_b: list[Bar],
    ts_a: list[datetime],
    ts_b: list[datetime],
) -> tuple[list[Bar], list[datetime]]:
    """构造比价 Bar 序列。

    ratio_open  = a.open / b.open
    ratio_close = a.close / b.close
    all_ratios  = [ratio_open, ratio_close, a.high/b.low, a.low/b.high]
    high = max(all_ratios), low = min(all_ratios)
    """
    idx_b = {ts: i for i, ts in enumerate(ts_b)}
    result_bars: list[Bar] = []
    result_ts: list[datetime] = []

    for i, ts in enumerate(ts_a):
        if ts not in idx_b:
            continue
        j = idx_b[ts]
        a = bars_a[i]
        b = bars_b[j]

        if b.open == 0 or b.close == 0 or b.high == 0 or b.low == 0:
            continue

        ratio_open = a.open / b.open
        ratio_close = a.close / b.close
        all_ratios = [
            ratio_open,
            ratio_close,
            a.high / b.low,
            a.low / b.high,
        ]
        result_bars.append(Bar(
            ts=ts,
            open=ratio_open,
            high=max(all_ratios),
            low=min(all_ratios),
            close=ratio_close,
            volume=None,
        ))
        result_ts.append(ts)

    return result_bars, result_ts


# ===================================================================
# D 算子运行
# ===================================================================


@dataclass
class EdgeResult:
    """单条边的多级别走势分析结果。"""
    edge_label: str
    vertex_from: str
    vertex_to: str
    total_bars: int = 0
    date_range: str = ""

    # Level 0 (基础层)
    move_count_l0: int = 0
    seg_count_l0: int = 0
    zs_count_l0: int = 0
    trend_count_l0: int = 0       # 趋势走势数量
    consolidation_count_l0: int = 0  # 盘整走势数量

    # 方向态时间序列（逐 bar，level 0）
    directions_l0: list[WalkDirection] = field(default_factory=list)
    # D 算子读数时间序列
    d_directions: list[Direction] = field(default_factory=list)
    d_amplitudes: list[float] = field(default_factory=list)
    d_absorptions: list[bool] = field(default_factory=list)

    # 递归级别摘要
    recursive_levels: list[dict] = field(default_factory=list)

    # 走势列表（完整）
    moves_detail: list[dict] = field(default_factory=list)

    timestamps: list[datetime] = field(default_factory=list)


def run_edge(
    edge_label: str,
    vertex_from: str,
    vertex_to: str,
    bars: list[Bar],
) -> EdgeResult:
    """对单条边运行 RecursiveOrchestrator 并提取多级别分析。"""
    orch = RecursiveOrchestrator(stream_id=edge_label)
    ts_list: list[datetime] = []
    dir_l0_list: list[WalkDirection] = []
    d_dir_list: list[Direction] = []
    d_amp_list: list[float] = []
    d_abs_list: list[bool] = []
    last_snap: RecursiveOrchestratorSnapshot | None = None

    for bar in bars:
        snap = orch.process_bar(bar)
        # Level 0 方向态
        dir_l0 = walk_direction_from_snapshot(snap, level=0)
        dir_l0_list.append(dir_l0)
        # D 算子读数
        d_reading = extract_d_reading(snap)
        d_dir_list.append(d_reading.direction)
        d_amp_list.append(d_reading.amplitude)
        d_abs_list.append(d_reading.absorption)

        ts_list.append(bar.ts)
        last_snap = snap

    result = EdgeResult(
        edge_label=edge_label,
        vertex_from=vertex_from,
        vertex_to=vertex_to,
        total_bars=len(bars),
        directions_l0=dir_l0_list,
        d_directions=d_dir_list,
        d_amplitudes=d_amp_list,
        d_absorptions=d_abs_list,
        timestamps=ts_list,
    )

    if ts_list:
        result.date_range = f"{ts_list[0].date()} ~ {ts_list[-1].date()}"

    if last_snap is not None:
        # Level 0 统计
        moves = last_snap.move_snapshot.moves
        result.move_count_l0 = len(moves)
        result.seg_count_l0 = len(last_snap.seg_snapshot.segments)
        result.zs_count_l0 = len(last_snap.zs_snapshot.zhongshus)

        for m in moves:
            if m.kind == "trend":
                result.trend_count_l0 += 1
            elif m.kind == "consolidation":
                result.consolidation_count_l0 += 1

        # 走势详情
        for i, m in enumerate(moves):
            result.moves_detail.append({
                "idx": i,
                "kind": m.kind,
                "direction": m.direction,
                "settled": m.settled,
                "high": round(m.high, 4),
                "low": round(m.low, 4),
                "zs_count": getattr(m, "zs_count", None),
            })

        # 递归级别
        for rs in last_snap.recursive_snapshots:
            level_info = {
                "level_id": rs.level_id,
                "move_count": len(rs.moves),
                "zhongshu_count": len(rs.zhongshus),
                "trend_count": sum(1 for m in rs.moves if m.kind == "trend"),
                "consolidation_count": sum(1 for m in rs.moves if m.kind == "consolidation"),
            }
            result.recursive_levels.append(level_info)

    return result


# ===================================================================
# 统计工具
# ===================================================================


def direction_distribution(directions: list) -> dict[str, int]:
    """方向态分布统计。接受 WalkDirection 或 Direction。"""
    counts: dict[str, int] = {}
    for d in directions:
        name = d.name if hasattr(d, "name") else str(d)
        counts[name] = counts.get(name, 0) + 1
    return counts


def direction_transitions(directions: list) -> int:
    """方向态变化次数。"""
    n = 0
    for i in range(1, len(directions)):
        if directions[i] != directions[i - 1]:
            n += 1
    return n


def transition_events(
    directions: list,
    timestamps: list[datetime],
) -> list[dict]:
    """提取方向态变化事件（日期+前后方向）。"""
    events: list[dict] = []
    for i in range(1, len(directions)):
        if directions[i] != directions[i - 1]:
            from_name = directions[i - 1].name if hasattr(directions[i - 1], "name") else str(directions[i - 1])
            to_name = directions[i].name if hasattr(directions[i], "name") else str(directions[i])
            events.append({
                "date": timestamps[i].strftime("%Y-%m-%d"),
                "from": from_name,
                "to": to_name,
            })
    return events


# ===================================================================
# K3 vs K4 对比
# ===================================================================


def compare_edges(
    k3_results: dict[str, EdgeResult],
    k4_results: dict[str, EdgeResult],
    common_dates: list[datetime],
) -> dict:
    """K3 三条边 vs K4 六条边逐 bar 对比。"""

    # 1. K3 三条边各自的 D 算子方向态分布和变化
    k3_summary: dict[str, dict] = {}
    for label, r in k3_results.items():
        d_dist = direction_distribution(r.d_directions)
        d_trans = direction_transitions(r.d_directions)
        d_events = transition_events(r.d_directions, r.timestamps)
        l0_dist = direction_distribution(r.directions_l0)
        l0_trans = direction_transitions(r.directions_l0)

        k3_summary[label] = {
            "total_bars": r.total_bars,
            "date_range": r.date_range,
            "d_operator": {
                "direction_distribution": d_dist,
                "transitions": d_trans,
                "transition_events": d_events[:50],
            },
            "walk_direction_l0": {
                "distribution": l0_dist,
                "transitions": l0_trans,
            },
            "structure": {
                "moves": r.move_count_l0,
                "segments": r.seg_count_l0,
                "zhongshus": r.zs_count_l0,
                "trends": r.trend_count_l0,
                "consolidations": r.consolidation_count_l0,
            },
            "recursive_levels": r.recursive_levels,
            "moves_detail": r.moves_detail,
        }

    # 2. K4 六条边摘要
    k4_summary: dict[str, dict] = {}
    for label, r in k4_results.items():
        d_dist = direction_distribution(r.d_directions)
        d_trans = direction_transitions(r.d_directions)
        d_events = transition_events(r.d_directions, r.timestamps)

        k4_summary[label] = {
            "total_bars": r.total_bars,
            "date_range": r.date_range,
            "d_operator": {
                "direction_distribution": d_dist,
                "transitions": d_trans,
                "transition_events": d_events[:50],
            },
            "structure": {
                "moves": r.move_count_l0,
                "segments": r.seg_count_l0,
                "zhongshus": r.zs_count_l0,
                "trends": r.trend_count_l0,
                "consolidations": r.consolidation_count_l0,
            },
            "recursive_levels": r.recursive_levels,
            "moves_detail": r.moves_detail,
        }

    # 3. 共享边 E/R（SPY/TLT）一致性比较
    # K3 的 E/R 和 K4 的 E/R 用同样的数据源
    k3_er = k3_results.get("E/R")
    k4_er = k4_results.get("E/R")
    er_agreement = {"available": False}
    if k3_er and k4_er:
        k3_idx = {ts: i for i, ts in enumerate(k3_er.timestamps)}
        k4_idx = {ts: i for i, ts in enumerate(k4_er.timestamps)}
        agree = 0
        disagree = 0
        total = 0
        samples: list[dict] = []
        for dt in common_dates:
            if dt in k3_idx and dt in k4_idx:
                total += 1
                d_k3 = k3_er.d_directions[k3_idx[dt]]
                d_k4 = k4_er.d_directions[k4_idx[dt]]
                if d_k3 == d_k4:
                    agree += 1
                else:
                    disagree += 1
                    if len(samples) < 20:
                        samples.append({
                            "date": dt.strftime("%Y-%m-%d"),
                            "k3": d_k3.name,
                            "k4": d_k4.name,
                        })
        er_agreement = {
            "available": True,
            "agree": agree,
            "disagree": disagree,
            "total": total,
            "agreement_pct": round(agree / total * 100, 2) if total > 0 else 0,
            "samples": samples,
        }

    # 4. K3 配置分布（三条边的 D 算子联合方向态）
    k3_config_counts: dict[str, int] = {}
    k3_edge_labels = ["E/Au", "R/Au", "E/R"]
    k3_idx_maps: dict[str, dict[datetime, int]] = {}
    for label in k3_edge_labels:
        if label in k3_results:
            k3_idx_maps[label] = {ts: i for i, ts in enumerate(k3_results[label].timestamps)}

    for dt in common_dates:
        dirs: list[str] = []
        all_found = True
        for label in k3_edge_labels:
            if label not in k3_idx_maps or dt not in k3_idx_maps[label]:
                all_found = False
                break
            idx = k3_idx_maps[label][dt]
            dirs.append(k3_results[label].d_directions[idx].name)
        if all_found:
            key = "/".join(dirs)
            k3_config_counts[key] = k3_config_counts.get(key, 0) + 1

    total_configs = sum(k3_config_counts.values())
    sorted_configs = sorted(k3_config_counts.items(), key=lambda x: -x[1])
    k3_config_distribution = [
        {
            "config": k,
            "E/Au": k.split("/")[0],
            "R/Au": k.split("/")[1],
            "E/R": k.split("/")[2],
            "count": v,
            "pct": round(v / total_configs * 100, 2) if total_configs > 0 else 0,
        }
        for k, v in sorted_configs
    ]

    # 5. K4 配置分布（六条边的 D 算子联合方向态）
    k4_edge_labels = ["E/$", "Au/$", "R/$", "E/Au", "Au/R", "E/R"]
    k4_idx_maps: dict[str, dict[datetime, int]] = {}
    for label in k4_edge_labels:
        if label in k4_results:
            k4_idx_maps[label] = {ts: i for i, ts in enumerate(k4_results[label].timestamps)}

    k4_config_counts: dict[str, int] = {}
    for dt in common_dates:
        dirs_k4: list[str] = []
        all_found = True
        for label in k4_edge_labels:
            if label not in k4_idx_maps or dt not in k4_idx_maps[label]:
                all_found = False
                break
            idx = k4_idx_maps[label][dt]
            dirs_k4.append(k4_results[label].d_directions[idx].name)
        if all_found:
            key = "/".join(dirs_k4)
            k4_config_counts[key] = k4_config_counts.get(key, 0) + 1

    total_k4 = sum(k4_config_counts.values())
    sorted_k4 = sorted(k4_config_counts.items(), key=lambda x: -x[1])
    k4_config_distribution = [
        {
            "config": k,
            "count": v,
            "pct": round(v / total_k4 * 100, 2) if total_k4 > 0 else 0,
        }
        for k, v in sorted_k4[:30]
    ]

    # 6. K3 vs K4 方向态比较：在每个 bar 看 K3 三条边 vs K4 六条边的活跃边数
    k3_active_counts: list[int] = []
    k4_active_counts: list[int] = []
    for dt in common_dates:
        k3_active = 0
        for label in k3_edge_labels:
            if label in k3_idx_maps and dt in k3_idx_maps[label]:
                idx = k3_idx_maps[label][dt]
                if k3_results[label].d_directions[idx] != Direction.FLAT:
                    k3_active += 1
        k3_active_counts.append(k3_active)

        k4_active = 0
        for label in k4_edge_labels:
            if label in k4_idx_maps and dt in k4_idx_maps[label]:
                idx = k4_idx_maps[label][dt]
                if k4_results[label].d_directions[idx] != Direction.FLAT:
                    k4_active += 1
        k4_active_counts.append(k4_active)

    active_stats = {
        "k3_mean_active": round(sum(k3_active_counts) / max(len(k3_active_counts), 1), 3),
        "k4_mean_active": round(sum(k4_active_counts) / max(len(k4_active_counts), 1), 3),
        "k3_max_active": max(k3_active_counts) if k3_active_counts else 0,
        "k4_max_active": max(k4_active_counts) if k4_active_counts else 0,
    }

    return {
        "k3_edges": k3_summary,
        "k4_edges": k4_summary,
        "shared_edge_E_R_agreement": er_agreement,
        "k3_config_distribution": k3_config_distribution,
        "k4_config_distribution": k4_config_distribution,
        "active_edge_stats": active_stats,
        "common_dates_count": len(common_dates),
        "common_date_range": (
            f"{common_dates[0].date()} ~ {common_dates[-1].date()}"
            if common_dates else "N/A"
        ),
    }


# ===================================================================
# 报告生成
# ===================================================================


def generate_report(comparison: dict) -> str:
    """生成 markdown 分析报告。"""
    lines: list[str] = []

    lines.append("# OQ4 黄金截面 K3 实验报告")
    lines.append("")
    lines.append("## 实验设计")
    lines.append("")
    lines.append("K4 退化为 K3：去掉美元顶点 $，保留 E（权益）、R（债券）、Au（黄金）三个顶点。")
    lines.append("三条边构成完备图 K3。")
    lines.append("")
    lines.append("### K3 三条边（黄金截面）")
    lines.append("")
    lines.append("| 边 | 定义 | 数据源 | 含义 |")
    lines.append("|---|------|--------|------|")
    lines.append("| E/Au | ^GSPC / GC=F | S&P500指数/黄金期货 | 权益的黄金购买力 |")
    lines.append("| R/Au | TLT / GLD | 长期国债ETF/黄金ETF | 债券的黄金购买力 |")
    lines.append("| E/R | SPY / TLT | 权益ETF/债券ETF | 权益相对债券的强弱 |")
    lines.append("")
    lines.append("### K4 对照组（美元截面，六条边）")
    lines.append("")
    lines.append("| 类型 | 边 | 数据源 |")
    lines.append("|------|---|--------|")
    lines.append("| 顶点边 | E/$, Au/$, R/$ | SPY, GLD, TLT |")
    lines.append("| 比价边 | E/Au, Au/R, E/R | SPY/GLD, GLD/TLT, SPY/TLT |")
    lines.append("")
    lines.append(f"- 公共交易日数：{comparison['common_dates_count']}")
    lines.append(f"- 数据范围：{comparison['common_date_range']}")
    lines.append("")

    # K3 各边结构
    lines.append("## K3 截面结构")
    lines.append("")
    k3_edges = comparison.get("k3_edges", {})
    for label in ["E/Au", "R/Au", "E/R"]:
        info = k3_edges.get(label)
        if not info:
            continue
        lines.append(f"### {label} ({info['date_range']})")
        lines.append("")
        s = info["structure"]
        lines.append(f"- 总 bars: {info['total_bars']}")
        lines.append(f"- 走势: {s['moves']}（趋势 {s['trends']} / 盘整 {s['consolidations']}）")
        lines.append(f"- 线段: {s['segments']}, 中枢: {s['zhongshus']}")
        lines.append("")

        # D 算子方向态
        d = info["d_operator"]
        lines.append(f"- D 算子方向态分布: {d['direction_distribution']}")
        lines.append(f"- 方向态变化次数: {d['transitions']}")
        if d["transition_events"]:
            lines.append("- 方向态变化事件（前20个）:")
            for ev in d["transition_events"][:20]:
                lines.append(f"  - {ev['date']}: {ev['from']} → {ev['to']}")
        lines.append("")

        # 走势详情
        if info.get("moves_detail"):
            lines.append("- 走势列表:")
            lines.append("")
            lines.append("| # | 类型 | 方向 | settled | 价格区间 |")
            lines.append("|---|------|------|---------|---------|")
            for m in info["moves_detail"]:
                lines.append(
                    f"| M{m['idx']} | {m['kind']} | {m['direction']} | "
                    f"{'Y' if m['settled'] else 'N'} | [{m['low']}, {m['high']}] |"
                )
            lines.append("")

        # 递归级别
        if info.get("recursive_levels"):
            lines.append("- 递归级别:")
            for rl in info["recursive_levels"]:
                lines.append(
                    f"  - Level {rl['level_id']}: "
                    f"{rl['move_count']}走势（趋势{rl['trend_count']}/盘整{rl['consolidation_count']}）"
                    f", {rl['zhongshu_count']}中枢"
                )
            lines.append("")

    # K4 对照组结构
    lines.append("## K4 对照组结构")
    lines.append("")
    k4_edges = comparison.get("k4_edges", {})
    for label in ["E/$", "Au/$", "R/$", "E/Au", "Au/R", "E/R"]:
        info = k4_edges.get(label)
        if not info:
            continue
        s = info["structure"]
        d = info["d_operator"]
        tag = "（顶点边）" if "$" in label else "（比价边）"
        lines.append(
            f"- **{label}** {tag}: {info['total_bars']} bars | "
            f"走势 {s['moves']}（趋势 {s['trends']}/盘整 {s['consolidations']}）| "
            f"D方向变化 {d['transitions']}次 | "
            f"方向分布 {d['direction_distribution']}"
        )
    lines.append("")

    # 共享边一致性
    lines.append("## 共享边 E/R 一致性")
    lines.append("")
    er = comparison.get("shared_edge_E_R_agreement", {})
    if er.get("available"):
        lines.append(
            f"K3 的 E/R（SPY/TLT）与 K4 的 E/R（SPY/TLT）使用相同数据源。"
        )
        lines.append(
            f"一致性: {er['agreement_pct']}% ({er['agree']}/{er['total']})"
        )
        if er.get("samples"):
            lines.append("")
            lines.append("不一致样本:")
            for s in er["samples"]:
                lines.append(f"  - {s['date']}: K3={s['k3']} K4={s['k4']}")
    else:
        lines.append("（无可比较数据）")
    lines.append("")

    # K3 配置分布
    lines.append("## K3 截面配置分布")
    lines.append("")
    lines.append("每条边的 D 算子方向态 {UP, DOWN, FLAT}，理论配置空间 3^3 = 27。")
    lines.append("")
    k3_configs = comparison.get("k3_config_distribution", [])
    if k3_configs:
        lines.append("| E/Au | R/Au | E/R | 频次 | 占比 |")
        lines.append("|------|------|-----|------|------|")
        for c in k3_configs:
            lines.append(
                f"| {c['E/Au']:4s} | {c['R/Au']:4s} | {c['E/R']:4s} | "
                f"{c['count']:5d} | {c['pct']:5.1f}% |"
            )
    lines.append("")

    # K3 vs K4 活跃边
    lines.append("## 活跃边对比")
    lines.append("")
    stats = comparison.get("active_edge_stats", {})
    lines.append(f"- K3 平均活跃边数: {stats.get('k3_mean_active', 0)}/3")
    lines.append(f"- K4 平均活跃边数: {stats.get('k4_mean_active', 0)}/6")
    lines.append(f"- K3 最大活跃边数: {stats.get('k3_max_active', 0)}/3")
    lines.append(f"- K4 最大活跃边数: {stats.get('k4_max_active', 0)}/6")
    lines.append("")

    # K3 少了/多了什么
    lines.append("## 分析：K3 少了什么")
    lines.append("")
    lines.append("K4 有六条边，K3 只有三条。K3 缺失的三条顶点边（E/$, Au/$, R/$）各自携带：")
    lines.append("")
    for label in ["E/$", "Au/$", "R/$"]:
        info = k4_edges.get(label)
        if info:
            d = info["d_operator"]
            s = info["structure"]
            lines.append(
                f"- **{label}**: D方向变化 {d['transitions']}次 | "
                f"走势 {s['moves']}（趋势 {s['trends']}/盘整 {s['consolidations']}）"
            )
    lines.append("")
    lines.append("顶点边提供了各资产相对美元的绝对方向信息。")
    lines.append("K3 截面仅保留相对比价信息，丢失了美元自身作为度量单位的波动影响。")
    lines.append("")

    lines.append("## 分析：K3 多了什么")
    lines.append("")
    lines.append("黄金截面的优势：")
    lines.append("1. **E/Au 使用 ^GSPC/GC=F**（指数/期货），比 SPY/GLD（ETF/ETF）覆盖更长历史")
    lines.append("2. **消除美元噪声**：在 K4 中，SPY/$ 和 GLD/$ 可能同时被美元波动驱动（假方向态），")
    lines.append("   但 ^GSPC/GC=F 直接反映权益相对黄金的真实购买力变化")
    lines.append("3. **K3 配置空间更小**（27 vs K4 的 729 理论上限），分类规则更简洁")
    lines.append("")

    # K3 分类规则初稿
    lines.append("## K3 截面完全分类规则初稿")
    lines.append("")
    lines.append("### 配置空间")
    lines.append("")
    lines.append("三条边各 {UP, DOWN, FLAT}，理论 3^3 = 27 种配置。")
    lines.append("")
    lines.append("### 传递性约束")
    lines.append("")
    lines.append("E/Au * Au/R ≈ E/R（传递性），但 D 算子独立运行在每条边上，")
    lines.append("不保证代数传递性。违反传递性的配置可能在实际中出现。")
    lines.append("")
    lines.append("### 语义分类")
    lines.append("")
    lines.append("| 配置族 | E/Au | R/Au | E/R | 语义 | 操作含义 |")
    lines.append("|--------|------|------|-----|------|---------|")
    lines.append("| α | UP | DOWN | UP | E > Au > R | 权益最强，做多E |")
    lines.append("| β | DOWN | UP | DOWN | R > Au > E | 债券最强，做多R |")
    lines.append("| γ | DOWN | DOWN | FLAT | E≈R < Au | 避险（黄金强势） |")
    lines.append("| δ | UP | UP | FLAT | E≈R > Au | 风险偏好（黄金弱势） |")
    lines.append("| ε | FLAT | FLAT | FLAT | 三者均衡 | 观望 |")
    lines.append("| ζ | UP | UP | UP | E > R, 但两者都弱于Au？ | 传递性违反待查 |")
    lines.append("")
    lines.append("### 区间套收敛与独立选股")
    lines.append("")
    lines.append("K3 截面能否独立选股取决于：")
    lines.append("1. 配置态非 ε（三者不均衡时才有方向性判断）")
    lines.append("2. 存在 α 或 β 配置（有明确的强弱排序）")
    lines.append("3. 至少两条边的 D 算子 amplitude > 1（趋势力度在增强）")
    lines.append("")
    lines.append("K3 独立选股的限制：")
    lines.append("- 无法区分'E 对 Au 上涨'是 E 上涨还是 Au 下跌（需要顶点边辅助）")
    lines.append("- 无美元维度信息，无法判断整体流动性环境")
    lines.append("")

    # 结果包
    lines.append("## 结果包")
    lines.append("")
    lines.append("1. **结论**: K3 截面三条边的走势结构、D算子读数、配置分布（见上）")
    lines.append("2. **定义依据**: OQ4 黄金截面假说——用黄金消除美元噪声；K4→K3 退化=去掉$顶点")
    lines.append(f"3. **边界条件**: GLD/TLT 数据起始于 2002/2004 限制了公共窗口；"
                 f"GC=F 期货数据覆盖取决于 yfinance")
    lines.append("4. **下游推论**: K3 截面配置分布是 K4 的投影——如果 K3 配置空间中非ε配置频率足够高，")
    lines.append("   可以作为快速筛选工具（先看 K3 三条边，再查 K4 确认）")
    lines.append("5. **谱系引用**: 279号（口径A验证）、277号（级别口径修正）")
    lines.append("6. **影响声明**: 实验结果，不修改仓库代码")
    lines.append("")
    lines.append("## 认识论等级")
    lines.append("")
    lines.append("L2（真实数据验证）")

    return "\n".join(lines)


# ===================================================================
# 主流程
# ===================================================================


def main() -> None:
    print("=" * 70)
    print("OQ4 黄金截面 K3 实验")
    print("口径 A (RecursiveOrchestrator 从日线递归)")
    print("数据源: yfinance (^GSPC, GC=F, SPY, TLT, GLD)")
    print("=" * 70)

    # ── 1. 拉取数据 ──
    print("\n[1/6] 拉取数据")
    raw_data: dict[str, pd.DataFrame] = {}
    for label, ticker in K3_TICKERS.items():
        df = fetch_daily(ticker, label)
        if df is not None:
            raw_data[label] = df

    missing = [k for k in K3_TICKERS if k not in raw_data]
    if missing:
        print(f"\n  缺失数据: {missing}，无法继续。")
        sys.exit(1)

    # ── 2. 对齐公共交易日 ──
    print("\n[2/6] 对齐公共交易日")
    # 所有品种取交集
    common_idx = raw_data["GSPC"].index
    for label in ("GCF", "SPY", "TLT", "GLD"):
        common_idx = common_idx.intersection(raw_data[label].index)
    common_idx = common_idx.sort_values()
    print(f"  公共交易日: {len(common_idx)} "
          f"({common_idx[0].date()} ~ {common_idx[-1].date()})")

    aligned: dict[str, pd.DataFrame] = {}
    for label in K3_TICKERS:
        aligned[label] = raw_data[label].loc[common_idx]

    # ── 3. 构造 Bar 流 ──
    print("\n[3/6] 构造 Bar 流")

    raw_bars: dict[str, tuple[list[Bar], list[datetime]]] = {}
    for label in K3_TICKERS:
        bars, ts = df_to_bars(aligned[label])
        raw_bars[label] = (bars, ts)
        print(f"  {label}: {len(bars)} bars")

    # K3 比价 Bars
    k3_bar_data: dict[str, tuple[list[Bar], list[datetime], str, str]] = {}

    # E/Au = ^GSPC / GC=F
    gspc_bars, gspc_ts = raw_bars["GSPC"]
    gcf_bars, gcf_ts = raw_bars["GCF"]
    ratio_bars, ratio_ts = make_ratio_bars(gspc_bars, gcf_bars, gspc_ts, gcf_ts)
    k3_bar_data["E/Au"] = (ratio_bars, ratio_ts, "E", "Au")
    print(f"  E/Au (^GSPC/GC=F, K3): {len(ratio_bars)} bars")

    # R/Au = TLT / GLD
    tlt_bars, tlt_ts = raw_bars["TLT"]
    gld_bars, gld_ts = raw_bars["GLD"]
    ratio_bars, ratio_ts = make_ratio_bars(tlt_bars, gld_bars, tlt_ts, gld_ts)
    k3_bar_data["R/Au"] = (ratio_bars, ratio_ts, "R", "Au")
    print(f"  R/Au (TLT/GLD, K3): {len(ratio_bars)} bars")

    # E/R = SPY / TLT
    spy_bars, spy_ts = raw_bars["SPY"]
    ratio_bars, ratio_ts = make_ratio_bars(spy_bars, tlt_bars, spy_ts, tlt_ts)
    k3_bar_data["E/R"] = (ratio_bars, ratio_ts, "E", "R")
    print(f"  E/R (SPY/TLT, K3): {len(ratio_bars)} bars")

    # K4 Bar 数据
    k4_bar_data: dict[str, tuple[list[Bar], list[datetime], str, str]] = {}

    # 顶点边（原始品种）
    k4_bar_data["E/$"] = (spy_bars, spy_ts, "E", "$")
    k4_bar_data["Au/$"] = (gld_bars, gld_ts, "Au", "$")
    k4_bar_data["R/$"] = (tlt_bars, tlt_ts, "R", "$")

    # 比价边
    ratio_bars, ratio_ts = make_ratio_bars(spy_bars, gld_bars, spy_ts, gld_ts)
    k4_bar_data["E/Au"] = (ratio_bars, ratio_ts, "E", "Au")

    ratio_bars, ratio_ts = make_ratio_bars(gld_bars, tlt_bars, gld_ts, tlt_ts)
    k4_bar_data["Au/R"] = (ratio_bars, ratio_ts, "Au", "R")

    ratio_bars, ratio_ts = make_ratio_bars(spy_bars, tlt_bars, spy_ts, tlt_ts)
    k4_bar_data["E/R"] = (ratio_bars, ratio_ts, "E", "R")

    # ── 4. 运行 D 算子 ──
    print("\n[4/6] 运行 D 算子")

    # K3 三条边
    print("\n  --- K3 截面（黄金计价三条边）---")
    k3_results: dict[str, EdgeResult] = {}
    t0 = time.time()
    for edge_label in ["E/Au", "R/Au", "E/R"]:
        bars_for_edge, _, vf, vt = k3_bar_data[edge_label]
        print(f"  {edge_label} ({len(bars_for_edge)} bars) ...", end=" ", flush=True)
        t_edge = time.time()
        result = run_edge(f"K3_{edge_label}", vf, vt, bars_for_edge)
        elapsed = time.time() - t_edge
        k3_results[edge_label] = result
        print(f"{elapsed:.1f}s | "
              f"moves={result.move_count_l0} "
              f"(trend={result.trend_count_l0} cons={result.consolidation_count_l0}) "
              f"segs={result.seg_count_l0} zs={result.zs_count_l0}")
    k3_time = time.time() - t0
    print(f"  K3 完成: {k3_time:.1f}s")

    # K4 六条边
    print("\n  --- K4 对照组（美元计价六条边）---")
    k4_results: dict[str, EdgeResult] = {}
    t0 = time.time()
    for edge_label in ["E/$", "Au/$", "R/$", "E/Au", "Au/R", "E/R"]:
        bars_for_edge, _, vf, vt = k4_bar_data[edge_label]
        print(f"  {edge_label} ({len(bars_for_edge)} bars) ...", end=" ", flush=True)
        t_edge = time.time()
        result = run_edge(f"K4_{edge_label}", vf, vt, bars_for_edge)
        elapsed = time.time() - t_edge
        k4_results[edge_label] = result
        print(f"{elapsed:.1f}s | "
              f"moves={result.move_count_l0} "
              f"(trend={result.trend_count_l0} cons={result.consolidation_count_l0}) "
              f"segs={result.seg_count_l0} zs={result.zs_count_l0}")
    k4_time = time.time() - t0
    print(f"  K4 完成: {k4_time:.1f}s")

    # ── 5. 对比分析 ──
    print("\n[5/6] 对比分析")

    # 取所有边的公共日期
    all_ts_sets: list[set[datetime]] = []
    for r in k3_results.values():
        all_ts_sets.append(set(r.timestamps))
    for r in k4_results.values():
        all_ts_sets.append(set(r.timestamps))
    common_ts = sorted(set.intersection(*all_ts_sets)) if all_ts_sets else []
    print(f"  对比窗口: {len(common_ts)} 个交易日")
    if common_ts:
        print(f"  范围: {common_ts[0].date()} ~ {common_ts[-1].date()}")

    comparison = compare_edges(k3_results, k4_results, common_ts)

    # 打印 K3 各边 D 算子方向态
    print("\n  K3 各边 D 算子方向态:")
    for label in ["E/Au", "R/Au", "E/R"]:
        info = comparison["k3_edges"].get(label, {})
        d = info.get("d_operator", {})
        print(f"    {label}: {d.get('direction_distribution', {})} "
              f"transitions={d.get('transitions', 0)}")

    # 打印 K4 对照
    print("\n  K4 各边 D 算子方向态:")
    for label in ["E/$", "Au/$", "R/$", "E/Au", "Au/R", "E/R"]:
        info = comparison["k4_edges"].get(label, {})
        d = info.get("d_operator", {})
        print(f"    {label}: {d.get('direction_distribution', {})} "
              f"transitions={d.get('transitions', 0)}")

    # 配置分布
    print("\n  K3 配置分布 (前10):")
    for c in comparison["k3_config_distribution"][:10]:
        print(f"    {c['E/Au']:4s}/{c['R/Au']:4s}/{c['E/R']:4s}: "
              f"{c['count']:5d} ({c['pct']:5.1f}%)")

    # 活跃边
    stats = comparison["active_edge_stats"]
    print(f"\n  活跃边平均数: K3={stats['k3_mean_active']}/3 "
          f"K4={stats['k4_mean_active']}/6")

    # ── 6. 保存结果 ──
    print("\n[6/6] 保存结果")

    # JSON 结果
    with open(RESULTS_JSON, "w", encoding="utf-8") as f:
        json.dump(comparison, f, indent=2, ensure_ascii=False, default=str)
    print(f"  结果 JSON -> {RESULTS_JSON}")

    # Markdown 报告
    report = generate_report(comparison)
    with open(REPORT_MD, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"  分析报告 -> {REPORT_MD}")

    # ── 完成 ──
    print("\n" + "=" * 70)
    print("OQ4 黄金截面 K3 实验完成")
    print("=" * 70)
    print(f"  K3 耗时: {k3_time:.1f}s")
    print(f"  K4 耗时: {k4_time:.1f}s")
    print(f"  结果: {RESULTS_JSON}")
    print(f"  报告: {REPORT_MD}")
    print("\n完成。")


if __name__ == "__main__":
    main()
