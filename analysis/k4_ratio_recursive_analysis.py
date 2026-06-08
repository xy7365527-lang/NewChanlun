#!/usr/bin/env python3
"""K4 六边比价递归分析 — 完全图的每条边跑缠论递归。

四节点 K4 = {E(股), Au(金), Oil(油), $(美元)}
六条边：
  独立边（连接$）：E/$=SPY, Au/$=GLD, Oil/$=USO
  派生边（纯资产间）：E/Au=SPY/GLD, E/Oil=SPY/USO, Au/Oil=GLD/USO(=ω)

对每条边构造 Bar 序列 → RecursiveOrchestrator 增量处理 → 提取走势结构。
配置 Γ=(σ(E/$), σ(Au/$), σ(Oil/$))，极性指数 S=Σσ。

认识论等级：L2（真实 1min 数据，非合成）。
"""

from __future__ import annotations

import json
import sys
import time
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Literal

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator, RecursiveOrchestratorSnapshot
from newchan.types import Bar

DATA_DIR = ROOT / "analysis" / "data_cache"
REPORT_PATH = ROOT / "analysis" / "k4_ratio_recursive_report.md"


# ════════════════════════════════════════════════════════════
# 1. 数据加载
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class PriceSeries:
    """一个品种的 OHLC 序列，带时间戳。"""
    symbol: str
    dates: list[str]
    opens: list[float]
    highs: list[float]
    lows: list[float]
    closes: list[float]


def load_series(filename: str) -> PriceSeries:
    """从 columnar JSON 加载价格序列。"""
    path = DATA_DIR / filename
    with open(path) as f:
        data = json.load(f)
    return PriceSeries(
        symbol=data["symbol"],
        dates=data["dates"],
        opens=data["opens"],
        highs=data["highs"],
        lows=data["lows"],
        closes=data["closes"],
    )


# ════════════════════════════════════════════════════════════
# 2. 比价 K 线构造
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class AlignedRatioBars:
    """两个品种按时间对齐后的比价 K 线。"""
    edge_name: str
    dates: list[str]
    opens: list[float]
    highs: list[float]
    lows: list[float]
    closes: list[float]


def build_ratio_bars(
    a: PriceSeries,
    b: PriceSeries,
    edge_name: str,
) -> AlignedRatioBars:
    """对齐两个品种的时间戳，构造 ratio = A/B 的 OHLC K 线。

    ratio OHLC = (a.open/b.open, a.high/b.high, a.low/b.low, a.close/b.close)
    然后对 high/low 做 max/min 修正，保证 high >= max(open, close), low <= min(open, close)。
    """
    b_index: dict[str, int] = {}
    for i, d in enumerate(b.dates):
        b_index[d] = i

    dates: list[str] = []
    opens: list[float] = []
    highs: list[float] = []
    lows: list[float] = []
    closes: list[float] = []

    for i, d in enumerate(a.dates):
        j = b_index.get(d)
        if j is None:
            continue
        bo, bh, bl, bc = b.opens[j], b.highs[j], b.lows[j], b.closes[j]
        if bo <= 0 or bh <= 0 or bl <= 0 or bc <= 0:
            continue

        ro = a.opens[i] / bo
        rh = a.highs[i] / bh
        rl = a.lows[i] / bl
        rc = a.closes[i] / bc

        actual_high = max(ro, rh, rl, rc)
        actual_low = min(ro, rh, rl, rc)

        dates.append(d)
        opens.append(ro)
        highs.append(actual_high)
        lows.append(actual_low)
        closes.append(rc)

    return AlignedRatioBars(
        edge_name=edge_name,
        dates=dates,
        opens=opens,
        highs=highs,
        lows=lows,
        closes=closes,
    )


# ════════════════════════════════════════════════════════════
# 3. 递归分析
# ════════════════════════════════════════════════════════════

@dataclass
class EdgeResult:
    """一条边的递归分析结果。"""
    edge_name: str
    bar_count: int
    date_range: tuple[str, str]
    elapsed: float

    # L1 结构
    stroke_count: int
    segment_count: int
    zhongshu_count: int
    move_count: int

    # 最高级别走势
    max_level: int
    current_move_kind: str
    current_move_direction: str
    current_move_zs_count: int
    current_move_settled: bool

    # 递归层摘要
    recursive_levels: list[dict]

    # 最新买卖点
    latest_bsp: list[dict]

    # 当前价格
    last_price: float


def parse_datetime(date_str: str) -> datetime:
    """解析日期字符串。"""
    for fmt in ("%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M:%S%z", "%Y-%m-%d"):
        try:
            return datetime.strptime(date_str.split("+")[0].split("-0")[0].rstrip(), fmt)
        except ValueError:
            continue
    return datetime.strptime(date_str[:19], "%Y-%m-%d %H:%M:%S")


def run_recursive_on_edge(
    edge_name: str,
    dates: list[str],
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
    progress_interval: int = 100_000,
) -> EdgeResult:
    """对一条边跑 RecursiveOrchestrator，返回结构摘要。"""
    t0 = time.time()
    n = len(closes)

    orch = RecursiveOrchestrator(
        stream_id=edge_name,
        max_levels=6,
        stroke_mode="wide",
    )

    snap: RecursiveOrchestratorSnapshot | None = None
    for i in range(n):
        ts = parse_datetime(dates[i])
        bar = Bar(
            ts=ts,
            open=opens[i],
            high=highs[i],
            low=lows[i],
            close=closes[i],
        )
        snap = orch.process_bar(bar)
        if (i + 1) % progress_interval == 0:
            print(f"    {edge_name}: {i+1:,}/{n:,} bars processed ...")

    elapsed = time.time() - t0
    assert snap is not None

    # 提取 L1 结构
    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves
    bsps = snap.bsp_snapshot.buysellpoints

    # 当前走势（最后一个 move）
    if moves:
        last_move = moves[-1]
        current_kind = last_move.kind
        current_dir = last_move.direction
        current_zs_count = last_move.zs_count
        current_settled = last_move.settled
    else:
        current_kind = "none"
        current_dir = "none"
        current_zs_count = 0
        current_settled = False

    # 递归层
    recursive_levels: list[dict] = []
    max_level = 1
    for rs in snap.recursive_snapshots:
        max_level = max(max_level, rs.level_id)
        recursive_levels.append({
            "level": rs.level_id,
            "zhongshu_count": len(rs.zhongshus),
            "move_count": len(rs.moves),
            "last_move_kind": rs.moves[-1].kind if rs.moves else "none",
            "last_move_dir": rs.moves[-1].direction if rs.moves else "none",
            "last_move_settled": rs.moves[-1].settled if rs.moves else False,
        })

    # 最新买卖点（取最后 3 个）
    latest_bsp: list[dict] = []
    for bp in bsps[-3:]:
        latest_bsp.append({
            "kind": bp.kind,
            "side": bp.side,
            "price": bp.price,
            "confirmed": bp.confirmed,
            "settled": bp.settled,
            "seg_idx": bp.seg_idx,
        })

    return EdgeResult(
        edge_name=edge_name,
        bar_count=n,
        date_range=(dates[0][:19], dates[-1][:19]),
        elapsed=elapsed,
        stroke_count=len(strokes),
        segment_count=len(segments),
        zhongshu_count=len(zhongshus),
        move_count=len(moves),
        max_level=max_level,
        current_move_kind=current_kind,
        current_move_direction=current_dir,
        current_move_zs_count=current_zs_count,
        current_move_settled=current_settled,
        recursive_levels=recursive_levels,
        latest_bsp=latest_bsp,
        last_price=closes[-1],
    )


# ════════════════════════════════════════════════════════════
# 4. 配置空间判定
# ════════════════════════════════════════════════════════════

def direction_to_sigma(direction: str, kind: str) -> int:
    """走势方向 → 极性符号 σ。

    趋势上 → +1, 趋势下 → -1, 盘整 → 0。
    """
    if kind == "consolidation":
        return 0
    if direction == "up":
        return 1
    if direction == "down":
        return -1
    return 0


def sigma_label(sigma: int) -> str:
    if sigma == 1:
        return "↑"
    if sigma == -1:
        return "↓"
    return "─"


# ════════════════════════════════════════════════════════════
# 5. 报告生成
# ════════════════════════════════════════════════════════════

def write_report(
    results: dict[str, EdgeResult],
    config_gamma: dict[str, int],
    polarity_s: int,
    total_elapsed: float,
) -> None:
    L: list[str] = []
    L.append("# K4 六边比价递归分析报告\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}")
    L.append(f"总耗时：{total_elapsed:.1f}s\n")

    # 数据概览
    L.append("## 1. 数据概览\n")
    L.append("| 边 | 类型 | Bar 数 | 时间范围 | 最新价/比值 | 耗时 |")
    L.append("|-----|------|--------|---------|------------|------|")
    for name, r in results.items():
        etype = "独立" if "/" not in name or name.count("/") == 1 and "$" in name else "派生"
        if name in ("E/$", "Au/$", "Oil/$"):
            etype = "独立"
        else:
            etype = "派生"
        L.append(
            f"| {name} | {etype} | {r.bar_count:,}"
            f" | {r.date_range[0][:10]}→{r.date_range[1][:10]}"
            f" | {r.last_price:.4f} | {r.elapsed:.1f}s |"
        )
    L.append("")

    # 各边递归结构
    L.append("## 2. 各边递归结构\n")
    for name, r in results.items():
        L.append(f"### {name}\n")
        L.append(f"- L1 笔：{r.stroke_count}，线段：{r.segment_count}，"
                 f"中枢：{r.zhongshu_count}，走势：{r.move_count}")
        L.append(f"- **当前走势**：{r.current_move_kind} "
                 f"{r.current_move_direction} "
                 f"(中枢×{r.current_move_zs_count}，"
                 f"{'已结算' if r.current_move_settled else '未结算'})")
        L.append(f"- 最高递归级别：L{r.max_level}")

        if r.recursive_levels:
            L.append("\n| 级别 | 中枢数 | 走势数 | 当前走势 | 方向 | 已结算 |")
            L.append("|------|--------|--------|---------|------|--------|")
            for rl in r.recursive_levels:
                L.append(
                    f"| L{rl['level']} | {rl['zhongshu_count']}"
                    f" | {rl['move_count']}"
                    f" | {rl['last_move_kind']}"
                    f" | {rl['last_move_dir']}"
                    f" | {'✓' if rl['last_move_settled'] else '✗'} |"
                )

        if r.latest_bsp:
            L.append(f"\n最近买卖点：")
            for bp in r.latest_bsp:
                status = "confirmed" if bp["confirmed"] else "candidate"
                L.append(
                    f"- {bp['kind']} {bp['side']} @ {bp['price']:.4f}"
                    f" (seg={bp['seg_idx']}, {status})"
                )
        else:
            L.append(f"\n无买卖点。")
        L.append("")

    # 配置空间
    L.append("## 3. 配置空间 Γ\n")
    edges_for_gamma = ["E/$", "Au/$", "Oil/$"]
    gamma_str = "("
    parts = []
    for e in edges_for_gamma:
        s = config_gamma.get(e, 0)
        r = results[e]
        parts.append(f"σ({e})={sigma_label(s)} [{r.current_move_kind[:5]}.{r.current_move_direction}]")
    gamma_str += ", ".join(parts) + ")"
    L.append(f"**Γ** = {gamma_str}\n")
    L.append(f"**极性指数 S** = {polarity_s}")
    L.append("")

    # S 的解读
    L.append("### 极性指数解读\n")
    L.append("| S | 含义 | 市场状态 |")
    L.append("|---|------|---------|")
    L.append("| +3 | 全面上涨 | 股/金/油同涨（通胀扩张？） |")
    L.append("| +2 | 两涨一跌 | 部分同向 |")
    L.append("| +1 | 一涨两跌/一涨两盘 | 分化 |")
    L.append("| 0  | 全盘整/对冲 | 方向不明 |")
    L.append("| -1 | 一跌两涨/一跌两盘 | 分化 |")
    L.append("| -2 | 两跌一涨 | 部分同向下跌 |")
    L.append("| -3 | 全面下跌 | 通缩/避险 |")
    L.append(f"\n当前 S={polarity_s}：", )

    if polarity_s >= 2:
        L.append("多数资产处于上涨趋势，信用环境偏宽松。")
    elif polarity_s <= -2:
        L.append("多数资产处于下跌趋势，信用环境偏紧缩或避险。")
    elif polarity_s == 0:
        L.append("方向分化或盘整，市场处于转折区间。")
    else:
        L.append("资产间方向不一致，需要结合派生边分析内部关系。")
    L.append("")

    # 派生边洞察
    L.append("## 4. 派生边洞察\n")
    derived_edges = ["E/Au", "E/Oil", "Au/Oil(ω)"]
    for name in derived_edges:
        if name in results:
            r = results[name]
            L.append(f"### {name}\n")
            L.append(f"- 当前走势：{r.current_move_kind} {r.current_move_direction}")
            L.append(f"- 最新比值：{r.last_price:.4f}")
            if name == "Au/Oil(ω)":
                L.append("- ω 上升 = 金强于油 = 信用收缩信号")
                L.append("- ω 下降 = 油强于金 = 信用扩张信号")
                L.append(f"- 当前 ω 走势方向 → {'信用收缩' if r.current_move_direction == 'up' else '信用扩张' if r.current_move_direction == 'down' else '方向不明'}")
            L.append("")

    # 综合建议
    L.append("## 5. 综合研判\n")
    omega_r = results.get("Au/Oil(ω)")
    spy_r = results.get("E/$")
    if omega_r and spy_r:
        omega_dir = omega_r.current_move_direction
        spy_dir = spy_r.current_move_direction
        if omega_dir == "down" and spy_dir == "up":
            L.append("**ω 下降 + E/$ 上涨**：经典的信用扩张期。股、油受益。")
        elif omega_dir == "up" and spy_dir == "down":
            L.append("**ω 上升 + E/$ 下跌**：信用收缩期。黄金避险，股、油承压。")
        elif omega_dir == "up" and spy_dir == "up":
            L.append("**ω 上升 + E/$ 上涨**：滞胀或防御性配置。金涨但股也涨，需注意分化。")
        elif omega_dir == "down" and spy_dir == "down":
            L.append("**ω 下降 + E/$ 下跌**：流动性危机信号。油跌更快于金。")
        else:
            L.append(f"ω={omega_dir}, E/$={spy_dir}，方向不明确，观望。")
    L.append("")

    # 结果包六要素
    L.append("## 结果包\n")
    L.append("**结论**：六条边递归结构及配置空间判定（见上表）。\n")
    L.append("**定义依据**：")
    L.append("- 走势类型：a_move_v1.py — 盘整(1中枢)/趋势(2+同向中枢)")
    L.append("- 中枢：a_zhongshu_v1.py — 至少3段重叠")
    L.append("- 递归：recursive_stack.py — settled move 作为上一级别线段输入")
    L.append("- 配置 Γ：三条独立边的最高级别走势方向\n")
    L.append("**边界条件**：")
    L.append("- 比价 OHLC 用 ETF 价格代理，含管理费/溢价噪声")
    L.append("- 不同品种交易时段不同，非交易时段的对齐 bar 被跳过")
    L.append("- 1min 级别递归，最高涌现级别受数据长度限制\n")
    L.append("**下游推论**：")
    L.append("- Γ 的变化轨迹（历史配置序列）可揭示 regime 切换")
    L.append("- 派生边走势与独立边走势的一致/矛盾关系构成交叉验证\n")
    L.append("**谱系引用**：")
    L.append("- 用户交易方向记忆：押注金油比下降（油涨）")
    L.append("- 卢麒元框架：L=M×ω\n")
    L.append("**影响声明**：新建分析脚本和报告，不修改引擎代码。\n")
    L.append("**认识论等级**：L2（真实 1min 数据，4品种 ETF）。")

    REPORT_PATH.write_text("\n".join(L))
    print(f"\n报告已写入：{REPORT_PATH}")


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    t0 = time.time()

    print("=" * 60)
    print("  K4 六边比价递归分析")
    print("=" * 60)

    # ── 加载数据 ──
    print("\n[1/4] 加载 1min 数据 ...")
    spy = load_series("spy_1m_full.json")
    gld = load_series("gld_1m_full.json")
    uso = load_series("uso_1m_full.json")
    uup = load_series("uup_1m_full.json")
    print(f"  SPY: {len(spy.closes):,} bars")
    print(f"  GLD: {len(gld.closes):,} bars")
    print(f"  USO: {len(uso.closes):,} bars")
    print(f"  UUP: {len(uup.closes):,} bars")

    # ── 构造比价 K 线 ──
    print("\n[2/4] 构造比价 K 线 ...")
    e_au = build_ratio_bars(spy, gld, "E/Au")
    e_oil = build_ratio_bars(spy, uso, "E/Oil")
    au_oil = build_ratio_bars(gld, uso, "Au/Oil(ω)")
    print(f"  E/Au:     {len(e_au.closes):,} aligned bars")
    print(f"  E/Oil:    {len(e_oil.closes):,} aligned bars")
    print(f"  Au/Oil(ω):{len(au_oil.closes):,} aligned bars")

    # ── 递归分析 ──
    print("\n[3/4] 递归分析（6条边）...\n")

    edges: list[tuple[str, list[str], list[float], list[float], list[float], list[float]]] = [
        ("E/$", spy.dates, spy.opens, spy.highs, spy.lows, spy.closes),
        ("Au/$", gld.dates, gld.opens, gld.highs, gld.lows, gld.closes),
        ("Oil/$", uso.dates, uso.opens, uso.highs, uso.lows, uso.closes),
        ("E/Au", e_au.dates, e_au.opens, e_au.highs, e_au.lows, e_au.closes),
        ("E/Oil", e_oil.dates, e_oil.opens, e_oil.highs, e_oil.lows, e_oil.closes),
        ("Au/Oil(ω)", au_oil.dates, au_oil.opens, au_oil.highs, au_oil.lows, au_oil.closes),
    ]

    results: dict[str, EdgeResult] = {}
    for name, dates, opens, highs, lows, closes in edges:
        print(f"  ── {name} ({len(closes):,} bars) ──")
        result = run_recursive_on_edge(
            name, dates, opens, highs, lows, closes,
            progress_interval=200_000,
        )
        results[name] = result
        print(f"    完成：{result.stroke_count} 笔, {result.segment_count} 段, "
              f"{result.zhongshu_count} 中枢, {result.move_count} 走势, "
              f"L{result.max_level}, "
              f"当前={result.current_move_kind}.{result.current_move_direction} "
              f"({result.elapsed:.1f}s)\n")

    # ── 配置空间 ──
    print("\n[4/4] 配置空间判定 ...")
    config_gamma: dict[str, int] = {}
    for edge_name in ("E/$", "Au/$", "Oil/$"):
        r = results[edge_name]
        sigma = direction_to_sigma(r.current_move_direction, r.current_move_kind)
        config_gamma[edge_name] = sigma
        print(f"  σ({edge_name}) = {sigma_label(sigma)} "
              f"({r.current_move_kind}.{r.current_move_direction})")

    polarity_s = sum(config_gamma.values())
    print(f"  Γ = ({', '.join(sigma_label(config_gamma[e]) for e in ('E/$', 'Au/$', 'Oil/$'))})")
    print(f"  S = {polarity_s}")

    total_elapsed = time.time() - t0
    print(f"\n总耗时：{total_elapsed:.1f}s")

    # ── 生成报告 ──
    write_report(results, config_gamma, polarity_s, total_elapsed)


if __name__ == "__main__":
    main()
