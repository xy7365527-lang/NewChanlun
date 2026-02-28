"""SPY 1min 递归深度验证 — 252号谱系。

v105 方向：从 1min 数据直接递归，级别是递归的产物，不是预设的时间框架。
拉 SPY 1min 数据，用 RecursiveOrchestrator 从底层递归，验证能达到多少级别。

概念溯源：
  - 252号：递归 1min 深度验证
  - 250号：扩展数据窗口验证
  - 249号：L78 修复后真实数据重新验证
  - 246号：多 TF pipeline 集成
  - 229号：T6 可达性
"""

from __future__ import annotations

import json
import logging
import sys
from datetime import datetime, timezone
from pathlib import Path

import yfinance as yf

# 确保项目在 path 上
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("recursive_1min_v105")


# ── 数据获取 ──────────────────────────────────────────────


def _df_to_bars(df) -> list[Bar]:
    """将 yfinance DataFrame 转换为 Bar 列表。"""
    bars: list[Bar] = []
    for idx, row in df.iterrows():
        ts = idx.to_pydatetime()
        if ts.tzinfo is None:
            ts = ts.replace(tzinfo=timezone.utc)
        bars.append(
            Bar(
                ts=ts,
                open=float(row["Open"]),
                high=float(row["High"]),
                low=float(row["Low"]),
                close=float(row["Close"]),
                volume=float(row["Volume"]) if "Volume" in row else None,
            )
        )
    return bars


def fetch_spy_1min_yf() -> list[Bar]:
    """拉取 SPY 1min 数据（yfinance, 分段拉取最近约 30 天）。

    yfinance 对 1min 数据限制为每次请求最多 8 天。
    分段拉取 4 个 7 天窗口覆盖约 28 天。
    """
    import time as _time
    from datetime import timedelta

    import pandas as pd

    ticker = yf.Ticker("SPY")
    now = datetime.now(tz=timezone.utc)
    all_dfs = []

    # 分 4 段拉取，每段 7 天，从最远到最近
    for i in range(4, 0, -1):
        end = now - timedelta(days=7 * (i - 1))
        start = now - timedelta(days=7 * i)
        logger.info("拉取 SPY 1min 数据: %s ~ %s ...", start.date(), end.date())
        df = ticker.history(start=start, end=end, interval="1m")
        if df is not None and len(df) > 0:
            logger.info("  本段 bar 数: %d", len(df))
            all_dfs.append(df)
        else:
            logger.warning("  本段无数据")
        _time.sleep(1)  # 避免触发 rate limit

    if not all_dfs:
        raise RuntimeError("yfinance 未返回任何 1min 数据")

    combined = pd.concat(all_dfs)
    combined = combined[~combined.index.duplicated(keep="first")]
    combined = combined.sort_index()
    logger.info("合并后 1min bar 总数: %d", len(combined))
    return _df_to_bars(combined)


# ── 序列化辅助 ────────────────────────────────────────────


def _serialize_segment(seg) -> dict:
    """序列化 Segment 到 dict。"""
    return {
        "s0": seg.s0,
        "s1": seg.s1,
        "direction": seg.direction,
        "high": seg.high,
        "low": seg.low,
        "confirmed": seg.confirmed,
        "settled": getattr(seg, "settled", None),
    }


def _serialize_zhongshu(zs) -> dict:
    """序列化 Zhongshu 到 dict。"""
    return {
        "zd": zs.zd,
        "zg": zs.zg,
        "seg_start": zs.seg_start,
        "seg_end": zs.seg_end,
        "seg_count": zs.seg_count,
        "settled": zs.settled,
        "break_direction": zs.break_direction,
    }


def _serialize_move(m) -> dict:
    """序列化 Move 到 dict。"""
    return {
        "kind": m.kind,
        "direction": m.direction,
        "seg_start": m.seg_start,
        "seg_end": m.seg_end,
        "zs_start": m.zs_start,
        "zs_end": m.zs_end,
        "zs_count": m.zs_count,
        "settled": m.settled,
        "high": m.high,
        "low": m.low,
    }


def _serialize_bsp(bsp) -> dict:
    """序列化 BuySellPoint 到 dict。"""
    return {
        "kind": bsp.kind,
        "side": bsp.side,
        "level_id": bsp.level_id,
        "seg_idx": bsp.seg_idx,
        "price": bsp.price,
        "confirmed": bsp.confirmed,
    }


def _serialize_level_zhongshu(zs) -> dict:
    """序列化 LevelZhongshu 到 dict。"""
    return {
        "zd": zs.zd,
        "zg": zs.zg,
        "comp_start": zs.comp_start,
        "comp_end": zs.comp_end,
        "comp_count": zs.comp_count,
        "settled": zs.settled,
        "break_direction": zs.break_direction,
        "level_id": zs.level_id,
    }


def _head_tail(items: list, n: int = 5) -> list:
    """取前 n 个和后 n 个（不足则全取）。"""
    if len(items) <= 2 * n:
        return items
    return items[:n] + items[-n:]


# ── 主逻辑 ────────────────────────────────────────────────


def run() -> dict:
    """执行 1min 递归深度验证，返回结果 dict。"""
    bars = fetch_spy_1min_yf()
    if not bars:
        raise RuntimeError("未获取到任何 1min bar 数据")

    logger.info("开始递归处理 %d 根 1min bar...", len(bars))

    orch = RecursiveOrchestrator(
        stream_id="SPY_1min",
        max_levels=6,
        stroke_mode="wide",
    )

    snap = None
    for bar in bars:
        snap = orch.process_bar(bar)

    if snap is None:
        raise RuntimeError("process_bar 未返回快照")

    logger.info("递归处理完成。构建结果...")

    # ── Level 1 ──
    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves
    bsps = snap.bsp_snapshot.buysellpoints

    level_1 = {
        "stroke_count": len(strokes),
        "segment_count": len(segments),
        "zhongshu_count": len(zhongshus),
        "move_count": len(moves),
        "bsp_count": len(bsps),
        "segments_detail": [_serialize_segment(s) for s in _head_tail(segments)],
        "zhongshus_detail": [_serialize_zhongshu(z) for z in _head_tail(zhongshus)],
        "moves_detail": [_serialize_move(m) for m in _head_tail(moves)],
        "bsps_detail": [_serialize_bsp(b) for b in _head_tail(bsps)],
    }

    # ── 递归层 ──
    recursive_levels = []
    max_level_reached = 1
    total_zhongshus = len(zhongshus)
    total_moves = len(moves)
    total_bsps = len(bsps)
    levels_with_moves = 0

    for rs in snap.recursive_snapshots:
        level_data = {
            "level_id": rs.level_id,
            "zhongshu_count": len(rs.zhongshus),
            "move_count": len(rs.moves),
            "zhongshus_detail": [_serialize_level_zhongshu(z) for z in _head_tail(rs.zhongshus)],
            "moves_detail": [_serialize_move(m) for m in _head_tail(rs.moves)],
        }
        recursive_levels.append(level_data)
        total_zhongshus += len(rs.zhongshus)
        total_moves += len(rs.moves)
        if rs.moves:
            levels_with_moves += 1
        if rs.level_id > max_level_reached:
            max_level_reached = rs.level_id

    # levels_with_moves 应加上 level 1（如果有 moves）
    if moves:
        levels_with_moves += 1

    pipeline_str = (
        f"{len(bars)} bars → {len(strokes)} strokes → {len(segments)} segments"
        f" → {len(zhongshus)} zhongshu → {len(moves)} moves"
        f" → recursive {max_level_reached} levels"
    )

    result = {
        "data_source": "yfinance",
        "symbol": "SPY",
        "interval": "1min",
        "bar_count": len(bars),
        "bar_range": {
            "first": bars[0].ts.isoformat(),
            "last": bars[-1].ts.isoformat(),
        },
        "level_1": level_1,
        "recursive_levels": recursive_levels,
        "max_level_reached": max_level_reached,
        "t6_equivalent": {
            "recursive_levels_with_moves": levels_with_moves,
            "t6_reachable_equivalent": levels_with_moves >= 2,
        },
        "summary": {
            "total_levels": max_level_reached,
            "total_zhongshus": total_zhongshus,
            "total_moves": total_moves,
            "total_bsps": total_bsps,
            "pipeline": pipeline_str,
        },
    }

    logger.info("Pipeline: %s", pipeline_str)
    logger.info("最大递归深度: %d, T6 等价可达: %s",
                max_level_reached, levels_with_moves >= 2)

    return result


def main() -> None:
    result = run()
    out_path = PROJECT_ROOT / "tmp" / "recursive-1min-v105.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    logger.info("结果已写入 %s", out_path)


if __name__ == "__main__":
    main()
