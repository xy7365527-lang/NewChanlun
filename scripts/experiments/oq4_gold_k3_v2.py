"""OQ4 v2 K3 截面下的操作完备性实验。

284号谱系（OQ v2）：D 算子 + 背驰判断的联合阅读。

K4 退化为 K3：黄金既是顶点又是度量基准，丢失一个自由度。
三条边（黄金计价）各自跑 RecursiveOrchestrator + 背驰判断。
和美元截面下的 K4 产出对比。

核心问题：
- D 算子在三条边上能否产出完整的走势结构？
- 背驰判断在三条边上是否仍有意义？（尺子从美元变成黄金）
- K3 截面下的选股和操作能独立运行，还是必须和 K4 截面交叉验证？

数据源：yfinance，最长最纯净原则。
认识论等级：L2（真实数据验证）
谱系引用：284号（OQ v2）、283号、279号
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

from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.types import Bar

RESULTS_DIR = ROOT / "tmp" / "oq4-v2-results"

# ═══════════════════════════════════════════════════════════════
# 数据源
# ═══════════════════════════════════════════════════════════════

K3_TICKERS = {
    "GSPC": "^GSPC",   # S&P 500 index
    "GCF": "GC=F",     # Gold futures
    "SPY": "SPY",
    "TLT": "TLT",
    "GLD": "GLD",
}

# K3 三条边（黄金计价）
K3_EDGE_DEFS = [
    ("E/Au", "E", "Au", "GSPC", "GCF"),    # ^GSPC / GC=F
    ("R/Au", "R", "Au", "TLT", "GLD"),      # TLT / GLD
    ("E/R", "E", "R", "SPY", "TLT"),        # SPY / TLT
]

# K4 六条边（美元截面参照）
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


# ═══════════════════════════════════════════════════════════════
# 数据拉取
# ═══════════════════════════════════════════════════════════════


def fetch_daily(ticker: str, label: str) -> pd.DataFrame | None:
    """拉取日线数据。"""
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
    """DataFrame -> (Bar list, timestamp list)。"""
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
    """构造比价 Bar 序列。"""
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
        all_ratios = [ratio_open, ratio_close, a.high / b.low, a.low / b.high]
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


# ═══════════════════════════════════════════════════════════════
# 单条边运行
# ═══════════════════════════════════════════════════════════════


@dataclass
class EdgeResult:
    """单条边的分析结果。"""
    edge_label: str
    vertex_from: str
    vertex_to: str
    total_bars: int = 0
    date_range: str = ""

    # 走势结构
    move_count: int = 0
    seg_count: int = 0
    zs_count: int = 0
    trend_count: int = 0
    consolidation_count: int = 0
    settled_move_count: int = 0

    # 背驰/买卖点
    bsp_count: int = 0
    bsp_events_total: int = 0

    # 最新走势状态
    last_move_kind: str = ""
    last_move_direction: str = ""
    last_move_settled: bool = False
    has_bsp: bool = False

    # 走势详情
    moves_detail: list[dict] = field(default_factory=list)

    # 递归级别摘要
    recursive_levels: list[dict] = field(default_factory=list)

    timestamps: list[datetime] = field(default_factory=list)


def run_edge(
    edge_label: str,
    vertex_from: str,
    vertex_to: str,
    bars: list[Bar],
) -> EdgeResult:
    """对单条边运行 RecursiveOrchestrator（D算子+背驰+BSP一体）。"""
    orch = RecursiveOrchestrator(stream_id=edge_label)
    ts_list: list[datetime] = []
    last_snap: RecursiveOrchestratorSnapshot | None = None
    bsp_events_total = 0

    for bar in bars:
        snap = orch.process_bar(bar)
        ts_list.append(bar.ts)
        last_snap = snap

        for ev in snap.all_events:
            if ev.event_type in ("bsp_candidate", "bsp_confirm", "bsp_settle"):
                bsp_events_total += 1

    result = EdgeResult(
        edge_label=edge_label,
        vertex_from=vertex_from,
        vertex_to=vertex_to,
        total_bars=len(bars),
        timestamps=ts_list,
        bsp_events_total=bsp_events_total,
    )

    if ts_list:
        result.date_range = f"{ts_list[0].date()} ~ {ts_list[-1].date()}"

    if last_snap is not None:
        moves = last_snap.move_snapshot.moves
        result.move_count = len(moves)
        result.seg_count = len(last_snap.seg_snapshot.segments)
        result.zs_count = len(last_snap.zs_snapshot.zhongshus)
        result.settled_move_count = len([m for m in moves if m.settled])
        result.bsp_count = len(last_snap.bsp_snapshot.buysellpoints)
        result.has_bsp = result.bsp_count > 0

        for m in moves:
            if m.kind == "trend":
                result.trend_count += 1
            elif m.kind == "consolidation":
                result.consolidation_count += 1

        if moves:
            result.last_move_kind = moves[-1].kind
            result.last_move_direction = moves[-1].direction
            result.last_move_settled = moves[-1].settled

        for i, m in enumerate(moves):
            result.moves_detail.append({
                "idx": i,
                "kind": m.kind,
                "direction": m.direction,
                "settled": m.settled,
                "high": round(m.high, 4),
                "low": round(m.low, 4),
                "zs_count": m.zs_count,
            })

        for rs in last_snap.recursive_snapshots:
            result.recursive_levels.append({
                "level_id": rs.level_id,
                "move_count": len(rs.moves),
                "zhongshu_count": len(rs.zhongshus),
                "trend_count": sum(1 for m in rs.moves if m.kind == "trend"),
                "consolidation_count": sum(1 for m in rs.moves if m.kind == "consolidation"),
            })

    return result


# ═══════════════════════════════════════════════════════════════
# K3 vs K4 对比
# ═══════════════════════════════════════════════════════════════


def compare_k3_k4(
    k3_results: dict[str, EdgeResult],
    k4_results: dict[str, EdgeResult],
) -> dict:
    """K3 三条边 vs K4 六条边结构产出对比。"""
    k3_summary: dict[str, dict] = {}
    for label, r in k3_results.items():
        k3_summary[label] = {
            "total_bars": r.total_bars,
            "date_range": r.date_range,
            "structure": {
                "moves": r.move_count,
                "settled_moves": r.settled_move_count,
                "segments": r.seg_count,
                "zhongshus": r.zs_count,
                "trends": r.trend_count,
                "consolidations": r.consolidation_count,
            },
            "divergence": {
                "bsp_count": r.bsp_count,
                "bsp_events_total": r.bsp_events_total,
                "has_bsp": r.has_bsp,
            },
            "last_move": {
                "kind": r.last_move_kind,
                "direction": r.last_move_direction,
                "settled": r.last_move_settled,
            },
            "moves_detail": r.moves_detail,
            "recursive_levels": r.recursive_levels,
        }

    k4_summary: dict[str, dict] = {}
    for label, r in k4_results.items():
        k4_summary[label] = {
            "total_bars": r.total_bars,
            "date_range": r.date_range,
            "structure": {
                "moves": r.move_count,
                "settled_moves": r.settled_move_count,
                "segments": r.seg_count,
                "zhongshus": r.zs_count,
                "trends": r.trend_count,
                "consolidations": r.consolidation_count,
            },
            "divergence": {
                "bsp_count": r.bsp_count,
                "bsp_events_total": r.bsp_events_total,
                "has_bsp": r.has_bsp,
            },
            "last_move": {
                "kind": r.last_move_kind,
                "direction": r.last_move_direction,
                "settled": r.last_move_settled,
            },
            "moves_detail": r.moves_detail,
            "recursive_levels": r.recursive_levels,
        }

    # K3 完备性评估
    k3_has_structure = all(
        k3_results[label].move_count > 0
        for label in ["E/Au", "R/Au", "E/R"]
        if label in k3_results
    )
    k3_has_divergence = any(
        k3_results[label].has_bsp
        for label in ["E/Au", "R/Au", "E/R"]
        if label in k3_results
    )

    return {
        "k3_edges": k3_summary,
        "k4_edges": k4_summary,
        "completeness": {
            "k3_all_edges_have_structure": k3_has_structure,
            "k3_any_edge_has_divergence_bsp": k3_has_divergence,
            "k3_edge_count": len(k3_results),
            "k4_edge_count": len(k4_results),
            "assessment": (
                "K3 截面可产出完整走势结构"
                if k3_has_structure
                else "K3 截面部分边未产出走势结构"
            ),
        },
    }


# ═══════════════════════════════════════════════════════════════
# 报告生成
# ═══════════════════════════════════════════════════════════════


def generate_report(comparison: dict) -> str:
    """生成 Markdown 报告。"""
    lines: list[str] = []

    lines.append("# OQ4 v2 K3 截面操作完备性实验报告")
    lines.append("")
    lines.append("认识论等级：L2（真实数据验证）")
    lines.append("谱系引用：284号（OQ v2）、283号、279号")
    lines.append("")

    lines.append("## 1. 284号变更")
    lines.append("")
    lines.append("- D 算子 + 背驰判断一起跑在每条边上")
    lines.append("- 核心问题：K3 截面三条边能否产出完整的走势结构 + 有意义的背驰判断？")
    lines.append("- 尺子从美元变成黄金，背驰判断的参照系变了")
    lines.append("- OQ4 和 OQ5 不独立——OQ5 读扭曲边界，OQ4 读退化后还能做什么")
    lines.append("")

    lines.append("## 2. K3 截面结构")
    lines.append("")
    k3_edges = comparison.get("k3_edges", {})
    for label in ["E/Au", "R/Au", "E/R"]:
        info = k3_edges.get(label)
        if not info:
            continue
        s = info["structure"]
        d = info["divergence"]
        lines.append(f"### {label} ({info['date_range']})")
        lines.append("")
        lines.append(f"- 总 bars: {info['total_bars']}")
        lines.append(f"- 走势: {s['moves']}（趋势 {s['trends']} / 盘整 {s['consolidations']}）")
        lines.append(f"- 已结算走势: {s['settled_moves']}")
        lines.append(f"- 线段: {s['segments']}, 中枢: {s['zhongshus']}")
        lines.append(f"- 买卖点: {d['bsp_count']}（BSP事件总计 {d['bsp_events_total']}）")
        lines.append("")

        if info.get("moves_detail"):
            lines.append("- 走势列表:")
            lines.append("")
            lines.append("| # | 类型 | 方向 | settled | 价格区间 | 中枢数 |")
            lines.append("|---|------|------|---------|---------|--------|")
            for m in info["moves_detail"]:
                lines.append(
                    f"| M{m['idx']} | {m['kind']} | {m['direction']} | "
                    f"{'Y' if m['settled'] else 'N'} | [{m['low']}, {m['high']}] | "
                    f"{m['zs_count']} |"
                )
            lines.append("")

    lines.append("## 3. K4 对照组结构")
    lines.append("")
    k4_edges = comparison.get("k4_edges", {})
    for label in ["E/$", "Au/$", "R/$", "E/Au", "Au/R", "E/R"]:
        info = k4_edges.get(label)
        if not info:
            continue
        s = info["structure"]
        d = info["divergence"]
        tag = "（顶点边）" if "$" in label else "（比价边）"
        lines.append(
            f"- **{label}** {tag}: {info['total_bars']} bars | "
            f"走势 {s['moves']}（趋势 {s['trends']}/盘整 {s['consolidations']}）| "
            f"已结算 {s['settled_moves']} | BSP {d['bsp_count']}"
        )
    lines.append("")

    lines.append("## 4. 完备性评估")
    lines.append("")
    comp = comparison.get("completeness", {})
    lines.append(f"- K3 三条边全部有走势结构: {'是' if comp.get('k3_all_edges_have_structure') else '否'}")
    lines.append(f"- K3 存在背驰/BSP: {'是' if comp.get('k3_any_edge_has_divergence_bsp') else '否'}")
    lines.append(f"- 评估: {comp.get('assessment', '')}")
    lines.append("")

    lines.append("## 5. 结果包")
    lines.append("")
    lines.append("1. **结论**: K3 截面结构产出和背驰判断有效性（见上）")
    lines.append("2. **定义依据**: 284号 OQ v2——D算子+背驰判断的联合阅读")
    lines.append("3. **边界条件**: GLD/TLT 上市日期限制公共窗口；GC=F 期货数据覆盖取决于 yfinance")
    lines.append("4. **下游推论**: K3 截面若可独立产出结构+背驰，可作为 K4 的快速筛选工具")
    lines.append("5. **谱系引用**: 284号、283号、279号")
    lines.append("6. **影响声明**: 实验结果，不修改现有代码或定义")

    return "\n".join(lines)


# ═══════════════════════════════════════════════════════════════
# 主流程
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    print("=" * 70)
    print("OQ4 v2 K3 截面操作完备性实验")
    print("284号谱系：D 算子 + 背驰判断的联合阅读")
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
    k3_bar_data: dict[str, tuple[list[Bar], str, str]] = {}

    gspc_bars, gspc_ts = raw_bars["GSPC"]
    gcf_bars, gcf_ts = raw_bars["GCF"]
    ratio_bars, _ = make_ratio_bars(gspc_bars, gcf_bars, gspc_ts, gcf_ts)
    k3_bar_data["E/Au"] = (ratio_bars, "E", "Au")
    print(f"  E/Au (^GSPC/GC=F, K3): {len(ratio_bars)} bars")

    tlt_bars, tlt_ts = raw_bars["TLT"]
    gld_bars, gld_ts = raw_bars["GLD"]
    ratio_bars, _ = make_ratio_bars(tlt_bars, gld_bars, tlt_ts, gld_ts)
    k3_bar_data["R/Au"] = (ratio_bars, "R", "Au")
    print(f"  R/Au (TLT/GLD, K3): {len(ratio_bars)} bars")

    spy_bars, spy_ts = raw_bars["SPY"]
    ratio_bars, _ = make_ratio_bars(spy_bars, tlt_bars, spy_ts, tlt_ts)
    k3_bar_data["E/R"] = (ratio_bars, "E", "R")
    print(f"  E/R (SPY/TLT, K3): {len(ratio_bars)} bars")

    # K4 Bar 数据
    k4_bar_data: dict[str, tuple[list[Bar], str, str]] = {}
    k4_bar_data["E/$"] = (spy_bars, "E", "$")
    k4_bar_data["Au/$"] = (gld_bars, "Au", "$")
    k4_bar_data["R/$"] = (tlt_bars, "R", "$")

    ratio_bars, _ = make_ratio_bars(spy_bars, gld_bars, spy_ts, gld_ts)
    k4_bar_data["E/Au"] = (ratio_bars, "E", "Au")

    ratio_bars, _ = make_ratio_bars(gld_bars, tlt_bars, gld_ts, tlt_ts)
    k4_bar_data["Au/R"] = (ratio_bars, "Au", "R")

    ratio_bars, _ = make_ratio_bars(spy_bars, tlt_bars, spy_ts, tlt_ts)
    k4_bar_data["E/R"] = (ratio_bars, "E", "R")

    # ── 4. 运行 RecursiveOrchestrator ──
    print("\n[4/6] 运行 RecursiveOrchestrator（D算子+背驰+BSP一体）")

    print("\n  --- K3 截面（黄金计价三条边）---")
    k3_results: dict[str, EdgeResult] = {}
    t0 = time.time()
    for edge_label in ["E/Au", "R/Au", "E/R"]:
        bars_for_edge, vf, vt = k3_bar_data[edge_label]
        print(f"  {edge_label} ({len(bars_for_edge)} bars) ...", end=" ", flush=True)
        t_edge = time.time()
        result = run_edge(f"K3_{edge_label}", vf, vt, bars_for_edge)
        elapsed = time.time() - t_edge
        k3_results[edge_label] = result
        print(f"{elapsed:.1f}s | "
              f"moves={result.move_count} "
              f"(trend={result.trend_count} cons={result.consolidation_count}) "
              f"settled={result.settled_move_count} "
              f"bsp={result.bsp_count}")
    k3_time = time.time() - t0
    print(f"  K3 完成: {k3_time:.1f}s")

    print("\n  --- K4 对照组（美元计价六条边）---")
    k4_results: dict[str, EdgeResult] = {}
    t0 = time.time()
    for edge_label in ["E/$", "Au/$", "R/$", "E/Au", "Au/R", "E/R"]:
        bars_for_edge, vf, vt = k4_bar_data[edge_label]
        print(f"  {edge_label} ({len(bars_for_edge)} bars) ...", end=" ", flush=True)
        t_edge = time.time()
        result = run_edge(f"K4_{edge_label}", vf, vt, bars_for_edge)
        elapsed = time.time() - t_edge
        k4_results[edge_label] = result
        print(f"{elapsed:.1f}s | "
              f"moves={result.move_count} "
              f"(trend={result.trend_count} cons={result.consolidation_count}) "
              f"settled={result.settled_move_count} "
              f"bsp={result.bsp_count}")
    k4_time = time.time() - t0
    print(f"  K4 完成: {k4_time:.1f}s")

    # ── 5. 对比分析 ──
    print("\n[5/6] 对比分析")
    comparison = compare_k3_k4(k3_results, k4_results)
    comp = comparison["completeness"]
    print(f"  K3 全部有结构: {comp['k3_all_edges_have_structure']}")
    print(f"  K3 有背驰/BSP: {comp['k3_any_edge_has_divergence_bsp']}")
    print(f"  评估: {comp['assessment']}")

    # ── 6. 保存结果 ──
    print("\n[6/6] 保存结果")
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    results_path = RESULTS_DIR / "oq4-v2-results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(comparison, f, indent=2, ensure_ascii=False, default=str)
    print(f"  结果 JSON -> {results_path}")

    report = generate_report(comparison)
    report_path = RESULTS_DIR / "oq4-v2-report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"  报告 -> {report_path}")

    print("\n" + "=" * 70)
    print("OQ4 v2 K3 截面操作完备性实验完成")
    print(f"  K3 耗时: {k3_time:.1f}s | K4 耗时: {k4_time:.1f}s")
    print("=" * 70)


if __name__ == "__main__":
    main()
