"""OQ5 D 算子运行管线 + 三重联立检测。

269号谱系验证：
  1. D 算子在汇率/黄金/美元指数年线/月线上的走势方向读数
  2. 三重联立拓扑指纹检测：中心失构 + 外围升构 + 法币/黄金共振
  3. 2002-2007 反例验证 + 1971 布雷顿森林崩溃验证

认识论等级：L2（真实数据验证）
"""

from __future__ import annotations

import json
import logging
from dataclasses import asdict, dataclass
from datetime import datetime
from pathlib import Path
from typing import Literal

import pandas as pd

from newchan.a_fractal import fractals_from_merged
from newchan.a_inclusion import merge_inclusion
from newchan.a_move_v1 import Move, moves_from_zhongshus
from newchan.a_segment_v0 import Segment
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_stroke import Stroke, strokes_from_fractals
from newchan.a_zhongshu_v1 import Zhongshu, zhongshu_from_segments

logger = logging.getLogger(__name__)


# ── D 算子运行管线 ──────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class DOperatorResult:
    """D 算子在单品种单周期上的完整输出。

    D = D3 . D2 . D1：
      D1: bar → 笔（方向编码）
      D2: 笔 → 线段（结构方向编码）
      D3: 线段 → 中枢 → 走势方向
    """

    symbol: str
    timeframe: str
    bar_count: int
    stroke_count: int
    segment_count: int
    zhongshu_count: int
    move_count: int
    moves: list[Move]
    last_move_direction: str  # "up", "down", "none"
    last_move_kind: str       # "trend", "consolidation", "none"
    last_move_settled: bool
    structural_breakdown: bool  # 是否有结构性破坏（趋势下跌）


def run_d_operator(
    df: pd.DataFrame,
    symbol: str,
    timeframe: str,
) -> DOperatorResult:
    """在单品种单周期数据上运行完整 D 算子管线。

    Parameters
    ----------
    df : pd.DataFrame
        OHLCV 数据（columns: Open/open, High/high, Low/low, Close/close）。
    symbol : str
        品种名称。
    timeframe : str
        时间周期标识（'yearly', 'quarterly', 'monthly'）。

    Returns
    -------
    DOperatorResult
    """
    # 标准化列名
    df_std = _standardize_columns(df)

    if len(df_std) < 5:
        return DOperatorResult(
            symbol=symbol,
            timeframe=timeframe,
            bar_count=len(df_std),
            stroke_count=0,
            segment_count=0,
            zhongshu_count=0,
            move_count=0,
            moves=[],
            last_move_direction="none",
            last_move_kind="none",
            last_move_settled=False,
            structural_breakdown=False,
        )

    # D1: bar → 笔
    df_merged, merged_to_raw = merge_inclusion(df_std)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(
        df_merged, fractals, mode="wide", merged_to_raw=merged_to_raw,
    )

    # D2: 笔 → 线段
    segments = segments_from_strokes_v1(strokes)

    # D3: 线段 → 中枢 → 走势
    zhongshus = zhongshu_from_segments(segments)
    moves = moves_from_zhongshus(zhongshus, num_segments=len(segments))

    # 提取最后走势
    if moves:
        last = moves[-1]
        last_dir = last.direction
        last_kind = last.kind
        last_settled = last.settled
    else:
        last_dir = "none"
        last_kind = "none"
        last_settled = False

    # 结构破坏检测：是否存在 settled 的下跌趋势
    structural_breakdown = any(
        m.kind == "trend" and m.direction == "down" and m.settled
        for m in moves
    )

    return DOperatorResult(
        symbol=symbol,
        timeframe=timeframe,
        bar_count=len(df_std),
        stroke_count=len(strokes),
        segment_count=len(segments),
        zhongshu_count=len(zhongshus),
        move_count=len(moves),
        moves=moves,
        last_move_direction=last_dir,
        last_move_kind=last_kind,
        last_move_settled=last_settled,
        structural_breakdown=structural_breakdown,
    )


def _standardize_columns(df: pd.DataFrame) -> pd.DataFrame:
    """标准化列名为小写 ohlcv。"""
    col_map = {
        "Open": "open",
        "High": "high",
        "Low": "low",
        "Close": "close",
        "Volume": "volume",
    }
    result = df.rename(columns=col_map)
    # 确保必要列存在
    for col in ("open", "high", "low", "close"):
        if col not in result.columns:
            raise ValueError(f"Missing column: {col}")
    return result


# ── 三重联立检测 ──────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class TripleSynchronyResult:
    """三重联立拓扑指纹检测结果。

    三个条件（269号）：
      1. 中心失构：美元指数出现结构性下跌趋势
      2. 外围升构：外围货币对美元出现结构性上升趋势
      3. 法币/黄金共振：黄金对美元出现结构性上升趋势

    每个条件在指定级别达到缠论结构完备时联立判断。
    """

    timeframe: str
    center_breakdown: bool          # 中心失构（DXY 下跌趋势）
    periphery_construction: bool    # 外围升构
    gold_resonance: bool            # 法币/黄金共振
    triple_met: bool                # 三重联立是否满足
    center_detail: str
    periphery_detail: str
    gold_detail: str


def detect_triple_synchrony(
    dxy_result: DOperatorResult,
    periphery_results: list[DOperatorResult],
    gold_result: DOperatorResult | None,
    timeframe: str,
) -> TripleSynchronyResult:
    """检测三重联立拓扑指纹。

    Parameters
    ----------
    dxy_result : DOperatorResult
        美元指数 D 算子结果。
    periphery_results : list[DOperatorResult]
        外围货币对 D 算子结果列表。
    gold_result : DOperatorResult | None
        黄金 D 算子结果（可能无数据）。
    timeframe : str
        检测级别。

    Returns
    -------
    TripleSynchronyResult
    """
    # 条件1：中心失构——DXY 存在 settled 下跌趋势
    center_breakdown = dxy_result.structural_breakdown
    center_detail = _describe_moves(dxy_result, "DXY")

    # 条件2：外围升构——至少一个外围货币对有 settled 上升趋势
    periphery_up = []
    for pr in periphery_results:
        has_up_trend = any(
            m.kind == "trend" and m.direction == "up" and m.settled
            for m in pr.moves
        )
        if has_up_trend:
            periphery_up.append(pr.symbol)

    periphery_construction = len(periphery_up) > 0
    periphery_detail = (
        f"外围升构品种: {periphery_up}"
        if periphery_up
        else "无外围品种出现 settled 上升趋势"
    )

    # 条件3：法币/黄金共振——黄金有 settled 上升趋势
    if gold_result is not None:
        gold_resonance = any(
            m.kind == "trend" and m.direction == "up" and m.settled
            for m in gold_result.moves
        )
        gold_detail = _describe_moves(gold_result, "Gold")
    else:
        gold_resonance = False
        gold_detail = "无黄金数据"

    triple_met = center_breakdown and periphery_construction and gold_resonance

    return TripleSynchronyResult(
        timeframe=timeframe,
        center_breakdown=center_breakdown,
        periphery_construction=periphery_construction,
        gold_resonance=gold_resonance,
        triple_met=triple_met,
        center_detail=center_detail,
        periphery_detail=periphery_detail,
        gold_detail=gold_detail,
    )


def _describe_moves(result: DOperatorResult, label: str) -> str:
    """生成走势摘要描述。"""
    if not result.moves:
        return f"{label}: 无走势（数据不足或无结构）"
    parts = []
    for i, m in enumerate(result.moves):
        settled_str = "settled" if m.settled else "active"
        parts.append(
            f"  M{i}: {m.kind} {m.direction} "
            f"(segs {m.seg_start}-{m.seg_end}, "
            f"{m.zs_count} ZS, {settled_str}, "
            f"range [{m.low:.2f}, {m.high:.2f}])"
        )
    return f"{label} ({result.bar_count} bars, {result.timeframe}):\n" + "\n".join(parts)


# ── 窗口验证 ──────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class WindowVerification:
    """单个验证窗口的完整结果。"""

    window_name: str
    window_range: str
    timeframes_checked: list[str]
    d_results: dict[str, list[dict]]   # symbol -> [move dicts per timeframe]
    synchrony_results: list[dict]      # per timeframe
    conclusion: str


def verify_window(
    window_name: str,
    window_start: int,
    window_end: int,
    cache_dir: Path,
    timeframes: list[str] = ("yearly", "quarterly", "monthly"),
) -> WindowVerification:
    """对指定时间窗口运行完整验证。

    Parameters
    ----------
    window_name : str
        窗口名称。
    window_start, window_end : int
        窗口年份范围。
    cache_dir : Path
        数据缓存目录。
    timeframes : list[str]
        要检查的时间周期列表。

    Returns
    -------
    WindowVerification
    """
    from oq5_data_fetch import SYMBOLS, load_cached

    d_results_all: dict[str, list[dict]] = {}
    synchrony_results: list[dict] = []

    for tf in timeframes:
        dxy_result = None
        periphery_results = []
        gold_result = None

        for symbol, meta in SYMBOLS.items():
            df = load_cached(symbol, tf, cache_dir)
            if df is None:
                logger.warning("No cached data for %s/%s", symbol, tf)
                continue

            # 过滤到窗口范围（取窗口前后适当扩展以保证结构完整）
            # 扩展策略：窗口前取全部数据（结构需要历史上下文）
            if df.index.tz is not None:
                df.index = df.index.tz_localize(None)

            df_window = df[df.index.year <= window_end]

            if df_window.empty:
                continue

            result = run_d_operator(df_window, symbol, tf)

            # 记录
            move_dicts = [
                {
                    "kind": m.kind,
                    "direction": m.direction,
                    "seg_range": f"{m.seg_start}-{m.seg_end}",
                    "zs_count": m.zs_count,
                    "settled": m.settled,
                    "high": m.high,
                    "low": m.low,
                }
                for m in result.moves
            ]

            if symbol not in d_results_all:
                d_results_all[symbol] = []
            d_results_all[symbol].append({
                "timeframe": tf,
                "bar_count": result.bar_count,
                "strokes": result.stroke_count,
                "segments": result.segment_count,
                "zhongshus": result.zhongshu_count,
                "moves": move_dicts,
                "last_direction": result.last_move_direction,
                "structural_breakdown": result.structural_breakdown,
            })

            # 分类
            if meta["role"] == "center":
                dxy_result = result
            elif meta["role"] == "gold":
                gold_result = result
            elif meta["role"] == "periphery":
                periphery_results.append(result)

        # 三重联立检测
        if dxy_result is not None:
            sync = detect_triple_synchrony(
                dxy_result, periphery_results, gold_result, tf,
            )
            synchrony_results.append({
                "timeframe": tf,
                "center_breakdown": sync.center_breakdown,
                "periphery_construction": sync.periphery_construction,
                "gold_resonance": sync.gold_resonance,
                "triple_met": sync.triple_met,
                "center_detail": sync.center_detail,
                "periphery_detail": sync.periphery_detail,
                "gold_detail": sync.gold_detail,
            })

    # 生成结论
    conclusion = _generate_conclusion(
        window_name, synchrony_results, timeframes,
    )

    return WindowVerification(
        window_name=window_name,
        window_range=f"{window_start}-{window_end}",
        timeframes_checked=list(timeframes),
        d_results=d_results_all,
        synchrony_results=synchrony_results,
        conclusion=conclusion,
    )


def _generate_conclusion(
    window_name: str,
    synchrony_results: list[dict],
    timeframes: list[str],
) -> str:
    """从验证结果生成结论文本。"""
    if not synchrony_results:
        return f"{window_name}: 无有效数据，无法验证"

    parts = [f"=== {window_name} 验证结论 ===\n"]

    for sr in synchrony_results:
        tf = sr["timeframe"]
        triple = sr["triple_met"]
        parts.append(f"\n{tf} 级别:")
        parts.append(f"  中心失构 (DXY下跌趋势): {'是' if sr['center_breakdown'] else '否'}")
        parts.append(f"  外围升构: {'是' if sr['periphery_construction'] else '否'}")
        parts.append(f"  法币/黄金共振: {'是' if sr['gold_resonance'] else '否'}")
        parts.append(f"  三重联立: {'满足' if triple else '不满足'}")

    # 关键判断
    yearly_sync = next(
        (sr for sr in synchrony_results if sr["timeframe"] == "yearly"),
        None,
    )
    monthly_sync = next(
        (sr for sr in synchrony_results if sr["timeframe"] == "monthly"),
        None,
    )

    parts.append("\n--- 269号验证结论 ---")

    if yearly_sync and monthly_sync:
        if monthly_sync["triple_met"] and not yearly_sync["triple_met"]:
            parts.append(
                "月线三重联立满足但年线不满足 → "
                "支持级别匹配化解论证（次级别波动）"
            )
        elif yearly_sync["triple_met"] and monthly_sync["triple_met"]:
            parts.append(
                "年线和月线均满足三重联立 → "
                "级别匹配化解失败，回退到'必要非充分+基本面确认'"
            )
        elif not monthly_sync["triple_met"]:
            parts.append(
                "月线三重联立不满足 → "
                "三重联立判据在此窗口不适用"
            )
    elif monthly_sync:
        if monthly_sync["triple_met"]:
            parts.append(
                "仅有月线数据，月线三重联立满足 → "
                "无法验证级别匹配（缺年线数据）"
            )

    return "\n".join(parts)


# ── 主入口 ──────────────────────────────────────────────────


def main() -> None:
    """运行 OQ5 完整验证。"""
    logging.basicConfig(level=logging.INFO, format="%(message)s")

    cache_dir = Path(__file__).resolve().parents[1] / ".cache" / "oq5"

    if not cache_dir.exists():
        print("请先运行 scripts/oq5_data_fetch.py 获取数据")
        return

    print("=" * 70)
    print("OQ5 D 算子验证管线")
    print("=" * 70)

    # 验证窗口1：2002-2007 反例
    print("\n" + "=" * 70)
    print("窗口1：2002-2007 反例验证")
    print("=" * 70)
    w1 = verify_window(
        "2002-2007 反例",
        window_start=2002,
        window_end=2007,
        cache_dir=cache_dir,
    )
    print(w1.conclusion)

    # 输出 DXY 走势详情
    print("\n--- DXY 走势详情 ---")
    for tf_data in w1.d_results.get("DX-Y.NYB", []):
        print(f"\n{tf_data['timeframe']}:")
        print(f"  bars={tf_data['bar_count']}, strokes={tf_data['strokes']}, "
              f"segments={tf_data['segments']}, zhongshus={tf_data['zhongshus']}")
        for m in tf_data["moves"]:
            print(f"  Move: {m['kind']} {m['direction']} "
                  f"(segs {m['seg_range']}, {m['zs_count']} ZS, "
                  f"settled={m['settled']}, range [{m['low']:.2f}, {m['high']:.2f}])")

    # 验证窗口2：1971 布雷顿森林崩溃
    print("\n" + "=" * 70)
    print("窗口2：1971 布雷顿森林崩溃验证")
    print("=" * 70)
    w2 = verify_window(
        "1971 布雷顿森林",
        window_start=1968,
        window_end=1975,
        cache_dir=cache_dir,
    )
    print(w2.conclusion)

    # 输出详情
    print("\n--- DXY 走势详情 ---")
    for tf_data in w2.d_results.get("DX-Y.NYB", []):
        print(f"\n{tf_data['timeframe']}:")
        print(f"  bars={tf_data['bar_count']}, strokes={tf_data['strokes']}, "
              f"segments={tf_data['segments']}, zhongshus={tf_data['zhongshus']}")
        for m in tf_data["moves"]:
            print(f"  Move: {m['kind']} {m['direction']} "
                  f"(segs {m['seg_range']}, {m['zs_count']} ZS, "
                  f"settled={m['settled']}, range [{m['low']:.2f}, {m['high']:.2f}])")

    # 保存结果
    output = {
        "verification_time": datetime.now().isoformat(),
        "window_1_2002_2007": {
            "conclusion": w1.conclusion,
            "d_results": w1.d_results,
            "synchrony": w1.synchrony_results,
        },
        "window_2_1971": {
            "conclusion": w2.conclusion,
            "d_results": w2.d_results,
            "synchrony": w2.synchrony_results,
        },
    }

    output_path = cache_dir / "oq5_verification_result.json"
    output_path.write_text(
        json.dumps(output, indent=2, ensure_ascii=False, default=str),
    )
    print(f"\n结果已保存: {output_path}")


if __name__ == "__main__":
    main()
