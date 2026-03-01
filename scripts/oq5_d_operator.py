"""OQ5 D 算子运行管线 + 三重联立检测（口径A：递归级别）。

277号谱系重写：
  从口径B（时间周期级别：在月线/年线K线上直接跑管线）
  改为口径A（递归级别：从日线K线出发，用 RecursiveOrchestrator 逐bar递归）。

  级别不再由TF名称标识，而由递归深度（level_id）自然决定。
  level_id=1 对应日线级别的笔→线段→中枢→走势，
  level_id=2 对应日线走势类型构成的更大级别中枢→走势，依此类推。

269号谱系验证：
  1. D 算子在汇率/黄金/美元指数上的走势方向读数（口径A递归）
  2. 三重联立拓扑指纹检测：中心失构 + 外围升构 + 法币/黄金共振
  3. 2002-2007 反例验证 + 1971 布雷顿森林崩溃验证

认识论等级：L2（真实数据验证）
"""

from __future__ import annotations

import json
import logging
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import pandas as pd

from newchan.a_move_v1 import Move
from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.types import Bar

logger = logging.getLogger(__name__)


# ── 多级别 D 算子数据结构 ──────────────────────────────────────


@dataclass(frozen=True, slots=True)
class LevelResult:
    """单个递归级别的走势产出。

    level_id=1: 日线级别（笔→线段→中枢→走势）
    level_id=2: 日线走势构成的更大级别
    level_id=N: 递归第N层
    """

    level_id: int
    move_count: int
    moves: list[Move]
    last_move_direction: str   # "up", "down", "none"
    last_move_kind: str        # "trend", "consolidation", "none"
    last_move_settled: bool
    structural_breakdown: bool  # 是否有结构性破坏（settled 下跌趋势）


@dataclass(frozen=True, slots=True)
class DOperatorResult:
    """D 算子在单品种日线上的完整输出（口径A：递归级别）。

    D = RecursiveOrchestrator 逐 bar 驱动：
      日线 bar → BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine
                                                               ↓
                                                         RecursiveStack
                                                               ↓
                                                     多级别走势产出

    级别由递归深度决定，不由时间周期名称决定。
    """

    symbol: str
    bar_count: int
    max_level_reached: int     # 递归达到的最高级别
    levels: list[LevelResult]  # 各级别产出，按 level_id 递增


def _extract_level_result(level_id: int, moves: list[Move]) -> LevelResult:
    """从走势列表提取单级别结果。"""
    if moves:
        last = moves[-1]
        last_dir = last.direction
        last_kind = last.kind
        last_settled = last.settled
    else:
        last_dir = "none"
        last_kind = "none"
        last_settled = False

    structural_breakdown = any(
        m.kind == "trend" and m.direction == "down" and m.settled
        for m in moves
    )

    return LevelResult(
        level_id=level_id,
        move_count=len(moves),
        moves=moves,
        last_move_direction=last_dir,
        last_move_kind=last_kind,
        last_move_settled=last_settled,
        structural_breakdown=structural_breakdown,
    )


def run_d_operator(
    df: pd.DataFrame,
    symbol: str,
) -> DOperatorResult:
    """在单品种日线数据上运行完整 D 算子管线（口径A）。

    Parameters
    ----------
    df : pd.DataFrame
        日线 OHLCV 数据（columns: Open/open, High/high, Low/low, Close/close）。
    symbol : str
        品种名称。

    Returns
    -------
    DOperatorResult
    """
    df_std = _standardize_columns(df)

    if len(df_std) < 5:
        return DOperatorResult(
            symbol=symbol,
            bar_count=len(df_std),
            max_level_reached=0,
            levels=[],
        )

    # 构建 RecursiveOrchestrator 并逐 bar 驱动
    orchestrator = RecursiveOrchestrator(
        stream_id=symbol,
        max_levels=6,
        stroke_mode="wide",
    )

    last_snapshot: RecursiveOrchestratorSnapshot | None = None

    for row in df_std.itertuples():
        bar = Bar(
            ts=row.Index.to_pydatetime() if hasattr(row.Index, 'to_pydatetime') else row.Index,
            open=float(row.open),
            high=float(row.high),
            low=float(row.low),
            close=float(row.close),
            volume=float(row.volume) if hasattr(row, 'volume') and pd.notna(row.volume) else None,
        )
        last_snapshot = orchestrator.process_bar(bar)

    if last_snapshot is None:
        return DOperatorResult(
            symbol=symbol,
            bar_count=len(df_std),
            max_level_reached=0,
            levels=[],
        )

    # 提取各级别产出
    levels: list[LevelResult] = []

    # Level 1: 来自 move_snapshot
    level1 = _extract_level_result(1, list(last_snapshot.move_snapshot.moves))
    levels.append(level1)

    # Level 2+: 来自 recursive_snapshots
    for rs in last_snapshot.recursive_snapshots:
        level_result = _extract_level_result(rs.level_id, list(rs.moves))
        levels.append(level_result)

    max_level = max(lr.level_id for lr in levels) if levels else 0

    return DOperatorResult(
        symbol=symbol,
        bar_count=len(df_std),
        max_level_reached=max_level,
        levels=levels,
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
    for col in ("open", "high", "low", "close"):
        if col not in result.columns:
            raise ValueError(f"Missing column: {col}")
    return result


# ── 辅助：按 level_id 查找 ──────────────────────────────────────


def get_level(result: DOperatorResult, level_id: int) -> LevelResult | None:
    """从 DOperatorResult 中按 level_id 取出对应级别结果。"""
    for lr in result.levels:
        if lr.level_id == level_id:
            return lr
    return None


def get_highest_level(result: DOperatorResult) -> LevelResult | None:
    """取递归达到的最高级别结果。"""
    if not result.levels:
        return None
    return result.levels[-1]


# ── 三重联立检测 ──────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class TripleSynchronyResult:
    """三重联立拓扑指纹检测结果。

    三个条件（269号）：
      1. 中心失构：美元指数出现结构性下跌趋势
      2. 外围升构：外围货币对美元出现结构性上升趋势
      3. 法币/黄金共振：黄金对美元出现结构性上升趋势

    每个条件在指定递归级别达到缠论结构完备时联立判断。
    """

    check_level_id: int                 # 检测使用的递归级别
    center_breakdown: bool              # 中心失构（DXY 下跌趋势）
    periphery_construction: bool        # 外围升构
    gold_resonance: bool                # 法币/黄金共振
    triple_met: bool                    # 三重联立是否满足
    center_detail: str
    periphery_detail: str
    gold_detail: str


def detect_triple_synchrony(
    dxy_result: DOperatorResult,
    periphery_results: list[DOperatorResult],
    gold_result: DOperatorResult | None,
    check_level_id: int | None = None,
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
    check_level_id : int | None
        指定检测的递归级别。None 时使用 DXY 达到的最高级别。

    Returns
    -------
    TripleSynchronyResult
    """
    # 确定检测级别
    if check_level_id is None:
        check_level_id = dxy_result.max_level_reached
    if check_level_id < 1:
        check_level_id = 1

    # 条件1：中心失构——DXY 在指定级别存在 settled 下跌趋势
    dxy_level = get_level(dxy_result, check_level_id)
    center_breakdown = dxy_level.structural_breakdown if dxy_level else False
    center_detail = _describe_level_moves(dxy_result, "DXY", check_level_id)

    # 条件2：外围升构——至少一个外围货币对在指定级别有 settled 上升趋势
    periphery_up: list[str] = []
    for pr in periphery_results:
        pr_level = get_level(pr, check_level_id)
        if pr_level is not None:
            has_up_trend = any(
                m.kind == "trend" and m.direction == "up" and m.settled
                for m in pr_level.moves
            )
            if has_up_trend:
                periphery_up.append(pr.symbol)

    periphery_construction = len(periphery_up) > 0
    periphery_detail = (
        f"外围升构品种 (level {check_level_id}): {periphery_up}"
        if periphery_up
        else f"无外围品种在 level {check_level_id} 出现 settled 上升趋势"
    )

    # 条件3：法币/黄金共振——黄金在指定级别有 settled 上升趋势
    if gold_result is not None:
        gold_level = get_level(gold_result, check_level_id)
        if gold_level is not None:
            gold_resonance = any(
                m.kind == "trend" and m.direction == "up" and m.settled
                for m in gold_level.moves
            )
            gold_detail = _describe_level_moves(gold_result, "Gold", check_level_id)
        else:
            gold_resonance = False
            gold_detail = f"Gold 未达到 level {check_level_id}"
    else:
        gold_resonance = False
        gold_detail = "无黄金数据"

    triple_met = center_breakdown and periphery_construction and gold_resonance

    return TripleSynchronyResult(
        check_level_id=check_level_id,
        center_breakdown=center_breakdown,
        periphery_construction=periphery_construction,
        gold_resonance=gold_resonance,
        triple_met=triple_met,
        center_detail=center_detail,
        periphery_detail=periphery_detail,
        gold_detail=gold_detail,
    )


def _describe_level_moves(
    result: DOperatorResult, label: str, level_id: int,
) -> str:
    """生成指定级别的走势摘要描述。"""
    lr = get_level(result, level_id)
    if lr is None:
        return f"{label}: 未达到 level {level_id}"
    if not lr.moves:
        return f"{label} (level {level_id}): 无走势（数据不足或无结构）"
    parts = []
    for i, m in enumerate(lr.moves):
        settled_str = "settled" if m.settled else "active"
        parts.append(
            f"  M{i}: {m.kind} {m.direction} "
            f"(segs {m.seg_start}-{m.seg_end}, "
            f"{m.zs_count} ZS, {settled_str}, "
            f"range [{m.low:.2f}, {m.high:.2f}])"
        )
    header = (
        f"{label} ({result.bar_count} bars, "
        f"max_level={result.max_level_reached}, "
        f"showing level {level_id}):"
    )
    return header + "\n" + "\n".join(parts)


# ── 窗口验证 ──────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class WindowVerification:
    """单个验证窗口的完整结果。"""

    window_name: str
    window_range: str
    check_level_ids: list[int]
    d_results: dict[str, list[dict]]   # symbol -> [level dicts]
    synchrony_results: list[dict]      # per check_level_id
    conclusion: str


def verify_window(
    window_name: str,
    window_start: int,
    window_end: int,
    cache_dir: Path,
    check_level_ids: list[int] | None = None,
) -> WindowVerification:
    """对指定时间窗口运行完整验证（口径A：从日线递归）。

    Parameters
    ----------
    window_name : str
        窗口名称。
    window_start, window_end : int
        窗口年份范围。
    cache_dir : Path
        数据缓存目录。
    check_level_ids : list[int] | None
        要检测三重联立的递归级别列表。None 时自动使用 [1, 2, 3]。

    Returns
    -------
    WindowVerification
    """
    from oq5_data_fetch import SYMBOLS, load_cached

    if check_level_ids is None:
        check_level_ids = [1, 2, 3]

    # 所有品种用日线数据跑 D 算子
    d_results_all: dict[str, list[dict]] = {}
    dxy_result: DOperatorResult | None = None
    periphery_results: list[DOperatorResult] = []
    gold_result: DOperatorResult | None = None

    for symbol, meta in SYMBOLS.items():
        df = load_cached(symbol, "daily", cache_dir)
        if df is None:
            logger.warning("No cached daily data for %s", symbol)
            continue

        # 去除时区信息
        if df.index.tz is not None:
            df.index = df.index.tz_localize(None)

        # 过滤到窗口范围（保留窗口前全部数据以保证结构完整）
        df_window = df[df.index.year <= window_end]

        if df_window.empty:
            continue

        result = run_d_operator(df_window, symbol)

        # 记录各级别产出
        level_dicts = []
        for lr in result.levels:
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
                for m in lr.moves
            ]
            level_dicts.append({
                "level_id": lr.level_id,
                "move_count": lr.move_count,
                "last_direction": lr.last_move_direction,
                "last_kind": lr.last_move_kind,
                "last_settled": lr.last_move_settled,
                "structural_breakdown": lr.structural_breakdown,
                "moves": move_dicts,
            })

        d_results_all[symbol] = [{
            "bar_count": result.bar_count,
            "max_level_reached": result.max_level_reached,
            "levels": level_dicts,
        }]

        # 分类
        if meta["role"] == "center":
            dxy_result = result
        elif meta["role"] == "gold":
            gold_result = result
        elif meta["role"] == "periphery":
            periphery_results.append(result)

    # 三重联立检测（在多个递归级别上）
    synchrony_results: list[dict] = []
    if dxy_result is not None:
        for level_id in check_level_ids:
            sync = detect_triple_synchrony(
                dxy_result, periphery_results, gold_result,
                check_level_id=level_id,
            )
            synchrony_results.append({
                "check_level_id": sync.check_level_id,
                "center_breakdown": sync.center_breakdown,
                "periphery_construction": sync.periphery_construction,
                "gold_resonance": sync.gold_resonance,
                "triple_met": sync.triple_met,
                "center_detail": sync.center_detail,
                "periphery_detail": sync.periphery_detail,
                "gold_detail": sync.gold_detail,
            })

    conclusion = _generate_conclusion(
        window_name, synchrony_results, check_level_ids,
    )

    return WindowVerification(
        window_name=window_name,
        window_range=f"{window_start}-{window_end}",
        check_level_ids=check_level_ids,
        d_results=d_results_all,
        synchrony_results=synchrony_results,
        conclusion=conclusion,
    )


def _generate_conclusion(
    window_name: str,
    synchrony_results: list[dict],
    check_level_ids: list[int],
) -> str:
    """从验证结果生成结论文本。"""
    if not synchrony_results:
        return f"{window_name}: 无有效数据，无法验证"

    parts = [f"=== {window_name} 验证结论 ===\n"]

    for sr in synchrony_results:
        level_id = sr["check_level_id"]
        triple = sr["triple_met"]
        parts.append(f"\n递归级别 {level_id}:")
        parts.append(f"  中心失构 (DXY下跌趋势): {'是' if sr['center_breakdown'] else '否'}")
        parts.append(f"  外围升构: {'是' if sr['periphery_construction'] else '否'}")
        parts.append(f"  法币/黄金共振: {'是' if sr['gold_resonance'] else '否'}")
        parts.append(f"  三重联立: {'满足' if triple else '不满足'}")

    # 关键判断：用最高检测级别和level 1做对比
    highest_sync = next(
        (sr for sr in reversed(synchrony_results) if sr["check_level_id"] == max(check_level_ids)),
        None,
    )
    level1_sync = next(
        (sr for sr in synchrony_results if sr["check_level_id"] == 1),
        None,
    )

    parts.append("\n--- 269号验证结论 ---")

    if highest_sync and level1_sync:
        if level1_sync["triple_met"] and not highest_sync["triple_met"]:
            parts.append(
                f"level 1 三重联立满足但 level {max(check_level_ids)} 不满足 → "
                "支持级别匹配化解论证（次级别波动）"
            )
        elif highest_sync["triple_met"] and level1_sync["triple_met"]:
            parts.append(
                f"level 1 和 level {max(check_level_ids)} 均满足三重联立 → "
                "级别匹配化解失败，回退到'必要非充分+基本面确认'"
            )
        elif not level1_sync["triple_met"]:
            parts.append(
                "level 1 三重联立不满足 → "
                "三重联立判据在此窗口不适用"
            )
    elif level1_sync:
        if level1_sync["triple_met"]:
            parts.append(
                "仅有 level 1 数据可判断，level 1 三重联立满足 → "
                "无法验证级别匹配（需更高递归级别数据）"
            )

    return "\n".join(parts)


# ── 主入口 ──────────────────────────────────────────────────


def main() -> None:
    """运行 OQ5 完整验证（口径A：从日线递归）。"""
    logging.basicConfig(level=logging.INFO, format="%(message)s")

    cache_dir = Path(__file__).resolve().parents[1] / ".cache" / "oq5"

    if not cache_dir.exists():
        print("请先运行 scripts/oq5_data_fetch.py 获取数据")
        return

    print("=" * 70)
    print("OQ5 D 算子验证管线（口径A：递归级别）")
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

    # 输出 DXY 各级别走势详情
    print("\n--- DXY 各级别走势详情 ---")
    dxy_data = w1.d_results.get("DX-Y.NYB", [])
    for entry in dxy_data:
        print(f"\n  max_level_reached={entry['max_level_reached']}, "
              f"bar_count={entry['bar_count']}")
        for ld in entry["levels"]:
            print(f"\n  Level {ld['level_id']} ({ld['move_count']} moves):")
            for m in ld["moves"]:
                print(f"    Move: {m['kind']} {m['direction']} "
                      f"(segs {m['seg_range']}, {m['zs_count']} ZS, "
                      f"settled={m['settled']}, "
                      f"range [{m['low']:.2f}, {m['high']:.2f}])")

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
    print("\n--- DXY 各级别走势详情 ---")
    dxy_data2 = w2.d_results.get("DX-Y.NYB", [])
    for entry in dxy_data2:
        print(f"\n  max_level_reached={entry['max_level_reached']}, "
              f"bar_count={entry['bar_count']}")
        for ld in entry["levels"]:
            print(f"\n  Level {ld['level_id']} ({ld['move_count']} moves):")
            for m in ld["moves"]:
                print(f"    Move: {m['kind']} {m['direction']} "
                      f"(segs {m['seg_range']}, {m['zs_count']} ZS, "
                      f"settled={m['settled']}, "
                      f"range [{m['low']:.2f}, {m['high']:.2f}])")

    # 保存结果
    output = {
        "verification_time": datetime.now().isoformat(),
        "pipeline": "口径A（递归级别）",
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
