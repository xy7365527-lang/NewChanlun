"""multi_tf_prototype.py — 多 TF 输入架构验证脚本。

用 yfinance 获取 SPY 的日线和周线数据，分别构建两个级别，
检查跨级别背驰是否存在，与单 TF 递归（229号方式）的结果做对比。

输出 JSON 到 tmp/multi-tf-prototype.json。

依赖: pip install yfinance
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

# 确保 src 在 path 中
src_path = str(Path(__file__).resolve().parent.parent / "src")
if src_path not in sys.path:
    sys.path.insert(0, src_path)

from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.topology.multi_tf_adapter import (  # noqa: E402
    MultiTFOrchestrator,
    TimeframeLevel,
)
from newchan.types import Bar  # noqa: E402


def _fetch_bars(ticker: str, period: str, interval: str) -> list[Bar]:
    """用 yfinance 获取 bar 数据。"""
    try:
        import yfinance as yf
    except ImportError:
        print("yfinance not installed. Install with: pip install yfinance")
        sys.exit(1)

    data = yf.download(ticker, period=period, interval=interval, progress=False)
    if data.empty:
        print(f"No data returned for {ticker} period={period} interval={interval}")
        return []

    # yfinance 可能返回多级列索引（MultiIndex），需要扁平化
    if hasattr(data.columns, "nlevels") and data.columns.nlevels > 1:
        data.columns = data.columns.droplevel(1)

    bars: list[Bar] = []
    for idx, row in data.iterrows():
        ts = idx.to_pydatetime()  # type: ignore[union-attr]
        if ts.tzinfo is None:
            ts = ts.replace(tzinfo=timezone.utc)
        bars.append(
            Bar(
                ts=ts,
                open=float(row["Open"]),
                high=float(row["High"]),
                low=float(row["Low"]),
                close=float(row["Close"]),
                volume=float(row["Volume"]) if "Volume" in row.index else None,
            ),
        )
    return bars


def _single_tf_recursive(bars: list[Bar], label: str) -> dict:
    """单 TF 递归（229号方式）。"""
    orch = RecursiveOrchestrator(
        stream_id=f"single_{label}",
        max_levels=6,
        stroke_mode="wide",
    )
    snap = None
    for bar in bars:
        snap = orch.process_bar(bar)

    if snap is None:
        return {
            "label": label,
            "bar_count": 0,
            "l1_strokes": 0,
            "l1_segments": 0,
            "l1_zhongshus": 0,
            "l1_moves": 0,
            "recursive_levels": 0,
        }

    return {
        "label": label,
        "bar_count": len(bars),
        "l1_strokes": len(snap.bi_snapshot.strokes),
        "l1_segments": len(snap.seg_snapshot.segments),
        "l1_zhongshus": len(snap.zs_snapshot.zhongshus),
        "l1_moves": len(snap.move_snapshot.moves),
        "recursive_levels": len(snap.recursive_snapshots),
    }


def main() -> None:
    ticker = "SPY"
    print(f"Fetching {ticker} daily and weekly data...")

    daily_bars = _fetch_bars(ticker, period="2y", interval="1d")
    weekly_bars = _fetch_bars(ticker, period="5y", interval="1wk")

    print(f"Daily bars: {len(daily_bars)}")
    print(f"Weekly bars: {len(weekly_bars)}")

    # 1. 单 TF 递归（对比基准）
    print("\n=== Single TF Recursive (229 baseline) ===")
    daily_recursive = _single_tf_recursive(daily_bars, "daily")
    weekly_recursive = _single_tf_recursive(weekly_bars, "weekly")

    for result in [daily_recursive, weekly_recursive]:
        print(
            f"  {result['label']}: "
            f"{result['bar_count']} bars → "
            f"{result['l1_strokes']} strokes → "
            f"{result['l1_segments']} segs → "
            f"{result['l1_zhongshus']} zs → "
            f"{result['l1_moves']} moves → "
            f"recursive_levels={result['recursive_levels']}",
        )

    # 2. 多 TF 输入
    print("\n=== Multi TF Input (238 architecture) ===")
    tfs = [
        TimeframeLevel(tf_name="daily", bar_source="yfinance", level_index=0),
        TimeframeLevel(tf_name="weekly", bar_source="yfinance", level_index=1),
    ]
    multi_orch = MultiTFOrchestrator(tfs, stroke_mode="wide")
    multi_result = multi_orch.run({"daily": daily_bars, "weekly": weekly_bars})

    for tf_name, level_result in multi_result.levels.items():
        print(
            f"  {tf_name}: "
            f"{level_result.bar_count} bars → "
            f"{level_result.move_count} moves → "
            f"{level_result.zhongshu_count} zs → "
            f"direction={level_result.direction}",
        )

    # 3. 跨级别背驰
    print(f"\nCross-level divergences: {len(multi_result.cross_level_divergences)}")
    for div in multi_result.cross_level_divergences:
        print(
            f"  {div.high_tf.tf_name}→{div.low_tf.tf_name}: "
            f"direction={div.direction}, "
            f"force_ratio={div.ratio:.3f}, "
            f"confirmed={div.confirmed}",
        )

    # 4. 买卖点
    print(f"\nBuy/Sell points: {len(multi_result.buysellpoints)}")
    for bsp in multi_result.buysellpoints:
        print(
            f"  {bsp.kind} {bsp.side} @ {bsp.price:.2f} "
            f"(tf={bsp.tf.tf_name})",
        )

    # 5. 对比总结
    print("\n=== Comparison Summary ===")
    print(f"Single TF daily recursive_levels: {daily_recursive['recursive_levels']}")
    print(f"Single TF weekly recursive_levels: {weekly_recursive['recursive_levels']}")
    print(f"Multi TF cross-level divergences: {len(multi_result.cross_level_divergences)}")
    print(f"Multi TF buy/sell points: {len(multi_result.buysellpoints)}")

    has_cross_divergence = len(multi_result.cross_level_divergences) > 0
    single_tf_stuck = (
        daily_recursive["recursive_levels"] <= 1
        and weekly_recursive["recursive_levels"] <= 1
    )
    print(
        f"\n229号瓶颈确认: single TF recursive stuck at level 1 = {single_tf_stuck}",
    )
    print(
        f"238号绕过验证: multi TF cross-level divergence found = {has_cross_divergence}",
    )

    # 6. JSON 输出
    output = {
        "ticker": ticker,
        "single_tf_recursive": {
            "daily": daily_recursive,
            "weekly": weekly_recursive,
        },
        "multi_tf": {
            "levels": {
                tf_name: {
                    "bar_count": lr.bar_count,
                    "move_count": lr.move_count,
                    "zhongshu_count": lr.zhongshu_count,
                    "direction": lr.direction,
                    "has_move": lr.last_move is not None,
                }
                for tf_name, lr in multi_result.levels.items()
            },
            "cross_level_divergences": [
                {
                    "high_tf": d.high_tf.tf_name,
                    "low_tf": d.low_tf.tf_name,
                    "direction": d.direction,
                    "force_high": d.force_high,
                    "force_low": d.force_low,
                    "ratio": d.ratio,
                    "confirmed": d.confirmed,
                }
                for d in multi_result.cross_level_divergences
            ],
            "buysellpoints": [
                {
                    "kind": b.kind,
                    "side": b.side,
                    "tf": b.tf.tf_name,
                    "price": b.price,
                }
                for b in multi_result.buysellpoints
            ],
        },
        "comparison": {
            "single_tf_stuck_at_level_1": single_tf_stuck,
            "multi_tf_cross_divergence_found": has_cross_divergence,
            "multi_tf_buysellpoints_count": len(multi_result.buysellpoints),
        },
    }

    out_path = Path(__file__).resolve().parent.parent / "tmp" / "multi-tf-prototype.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)
    print(f"\nJSON output written to {out_path}")


if __name__ == "__main__":
    main()
