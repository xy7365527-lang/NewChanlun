"""60min 单层走势背驰检测——方向性力度 vs 振幅力度对比。

251号谱系：在 60min 层的 7 个真实走势上，比较方向性力度 vs 振幅力度
的背驰检测差异。这是 ker(D) 操作意义的最终检验——单层走势即可。

概念溯源：
  - 251号：60min 单层背驰对比
  - 250号：扩展数据窗口验证
  - 249号：L78 修复后真实数据重新验证
  - 233号：ker(D) 操作意义
  - 239号：方向性力度（∉ ker(D)）
"""

from __future__ import annotations

import json
import logging
import sys
from datetime import timezone
from pathlib import Path

import yfinance as yf

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "src"))

from newchan.a_move_v1 import Move
from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.types import Bar

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("divergence_comparison_v105")


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


def fetch_spy_60min(days: int = 730) -> list[Bar]:
    """拉取 SPY 60min 数据（yfinance, 730 天）。"""
    ticker = yf.Ticker("SPY")
    logger.info("拉取 SPY 60min 数据 (%d 天, yfinance)...", days)
    df = ticker.history(period=f"{days}d", interval="60m")
    logger.info("60min bar 数: %d", len(df))
    return _df_to_bars(df)


# ── 单 TF 管线（通过 RecursiveOrchestrator）──────────────────


def run_single_tf_pipeline(bars: list[Bar]) -> RecursiveOrchestratorSnapshot:
    """运行完整的单 TF 管线，返回最终快照。

    使用 RecursiveOrchestrator 逐 bar 驱动（与 v104 一致）。
    """
    orch = RecursiveOrchestrator(
        stream_id="divergence_v105_60min",
        max_levels=6,
        stroke_mode="wide",
    )

    snap: RecursiveOrchestratorSnapshot | None = None
    for bar in bars:
        snap = orch.process_bar(bar)

    if snap is None:
        raise RuntimeError("管线无输出：bar 数为 0")

    return snap


# ── 力度计算 ──────────────────────────────────────────────


def amplitude_force(move: Move) -> float:
    """振幅力度 = |high - low|。

    与 multi_tf_adapter._move_amplitude 一致。
    ∈ ker(D)：方向反转不改变力度值。
    """
    return abs(move.high - move.low)


def directional_force(move: Move, segments: list) -> float:
    """方向性力度——结构复杂度 + 方向持续性。

    与 multi_tf_pipeline._directional_move_force 逻辑一致，
    但直接接受 segments 列表而非 LevelResult。

    组件1：中枢密度倒数 = seg_span / zs_count（趋势性 = 中枢少段多）
    组件2：走势内线段方向一致性
    ∉ ker(D)：方向信息参与力度计算。
    """
    seg_span = move.seg_end - move.seg_start + 1
    if seg_span <= 0:
        return 0.0

    # 组件1：中枢密度倒数
    zs_count = max(move.zs_count, 1)
    density = zs_count / seg_span
    density_force = 1.0 / density if density > 0 else float(seg_span)

    # 组件2：方向持续性
    persistence = 0.5  # 默认中性
    if segments and move.seg_start < len(segments):
        end = min(move.seg_end, len(segments) - 1)
        start = move.seg_start
        total = end - start + 1
        if total > 0:
            same_dir = sum(
                1 for i in range(start, end + 1)
                if segments[i].direction == move.direction
            )
            persistence = same_dir / total

    return 0.5 * persistence + 0.5 * density_force


# ── 背驰检测 ──────────────────────────────────────────────


def detect_divergence_pairs(
    moves: list[Move],
    segments: list,
) -> list[dict]:
    """对每对相邻同向走势，分别用振幅力度和方向性力度检测背驰。

    背驰条件：后一个走势的力度 < 前一个走势的力度，且方向相同。

    返回每对比较的详细记录。
    """
    comparisons: list[dict] = []

    for i in range(len(moves) - 1):
        prev_move = moves[i]
        curr_move = moves[i + 1]

        # 只比较同向走势
        if prev_move.direction != curr_move.direction:
            continue

        # 振幅力度
        amp_prev = amplitude_force(prev_move)
        amp_curr = amplitude_force(curr_move)
        amp_divergence = amp_curr < amp_prev

        # 方向性力度
        dir_prev = directional_force(prev_move, segments)
        dir_curr = directional_force(curr_move, segments)
        dir_divergence = dir_curr < dir_prev

        judgment_differs = amp_divergence != dir_divergence

        comparisons.append({
            "move_pair": [i, i + 1],
            "direction": prev_move.direction,
            "amplitude_force_prev": round(amp_prev, 6),
            "amplitude_force_curr": round(amp_curr, 6),
            "amplitude_divergence": amp_divergence,
            "directional_force_prev": round(dir_prev, 6),
            "directional_force_curr": round(dir_curr, 6),
            "directional_divergence": dir_divergence,
            "judgment_differs": judgment_differs,
        })

    return comparisons


# ── 主流程 ───────────────────────────────────────────────


def main() -> None:
    # 1. 获取数据
    bars = fetch_spy_60min(days=730)

    # 2. 运行管线
    snap = run_single_tf_pipeline(bars)

    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves

    logger.info("管线结果: %d strokes → %d segments → %d zhongshu → %d moves",
                len(strokes), len(segments), len(zhongshus), len(moves))

    # 3. 构建管线统计
    pipeline_info = {
        "bar_count": len(bars),
        "stroke_count": len(strokes),
        "segment_count": len(segments),
        "zhongshu_count": len(zhongshus),
        "move_count": len(moves),
    }

    # 4. 构建每个走势的详细信息
    moves_info = []
    for idx, m in enumerate(moves):
        amp = amplitude_force(m)
        dir_f = directional_force(m, segments)
        moves_info.append({
            "index": idx,
            "kind": m.kind,
            "direction": m.direction,
            "seg_start": m.seg_start,
            "seg_end": m.seg_end,
            "zs_count": m.zs_count,
            "settled": m.settled,
            "high": round(m.high, 6),
            "low": round(m.low, 6),
            "amplitude_force": round(amp, 6),
            "directional_force": round(dir_f, 6),
        })

    # 5. 背驰检测对比
    comparisons = detect_divergence_pairs(moves, segments)

    # 6. 汇总
    total_pairs = len(comparisons)
    amp_divs = sum(1 for c in comparisons if c["amplitude_divergence"])
    dir_divs = sum(1 for c in comparisons if c["directional_divergence"])
    differs_count = sum(1 for c in comparisons if c["judgment_differs"])

    if differs_count > 0:
        differ_details = [c for c in comparisons if c["judgment_differs"]]
        ker_d_desc = (
            f"有操作意义: {differs_count}/{total_pairs} 对同向走势上两种力度给出不同背驰判断。"
            f" 振幅力度检测到 {amp_divs} 个背驰, 方向性力度检测到 {dir_divs} 个背驰。"
            f" 差异走势对: {[d['move_pair'] for d in differ_details]}"
        )
    else:
        ker_d_desc = (
            f"本次 60min 单层验证中两种力度在所有 {total_pairs} 对同向走势上"
            f"给出相同的背驰判断（均检测到 {amp_divs} 个背驰）。"
            f" ker(D) 在此数据窗口内无操作差异。"
        )

    summary = {
        "total_move_pairs_compared": total_pairs,
        "amplitude_divergences": amp_divs,
        "directional_divergences": dir_divs,
        "judgment_differs_count": differs_count,
        "ker_d_operational_significance": ker_d_desc,
    }

    # 7. 输出 JSON
    result = {
        "pipeline": pipeline_info,
        "moves": moves_info,
        "divergence_comparisons": comparisons,
        "summary": summary,
    }

    output_path = PROJECT_ROOT / "tmp" / "divergence-comparison-v105.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False, default=str)

    logger.info("结果已写入: %s", output_path)

    # 打印汇总
    print("\n" + "=" * 60)
    print("v105 60min 单层背驰检测对比汇总")
    print("=" * 60)
    print(f"  管线: {pipeline_info['bar_count']} bars → "
          f"{pipeline_info['stroke_count']} strokes → "
          f"{pipeline_info['segment_count']} segments → "
          f"{pipeline_info['zhongshu_count']} zhongshu → "
          f"{pipeline_info['move_count']} moves")
    print(f"  同向走势对数: {total_pairs}")
    print(f"  振幅力度背驰: {amp_divs}")
    print(f"  方向性力度背驰: {dir_divs}")
    print(f"  判断差异数: {differs_count}")
    print(f"  ker(D) 操作意义: {ker_d_desc}")

    # 打印每个走势的力度
    print("\n走势力度详情:")
    for m in moves_info:
        print(f"  Move[{m['index']}] {m['kind']:15s} {m['direction']:4s} "
              f"segs={m['seg_start']}-{m['seg_end']} zs={m['zs_count']} "
              f"{'settled' if m['settled'] else 'UNSETTLED':9s} "
              f"amp={m['amplitude_force']:10.4f} dir={m['directional_force']:.6f}")

    if comparisons:
        print("\n背驰检测对比:")
        for c in comparisons:
            diff_mark = " *** DIFFERS ***" if c["judgment_differs"] else ""
            print(f"  Move[{c['move_pair'][0]}]↔Move[{c['move_pair'][1]}] "
                  f"({c['direction']}) "
                  f"amp: {c['amplitude_force_prev']:.4f}→{c['amplitude_force_curr']:.4f} "
                  f"({'背驰' if c['amplitude_divergence'] else '无'}) | "
                  f"dir: {c['directional_force_prev']:.6f}→{c['directional_force_curr']:.6f} "
                  f"({'背驰' if c['directional_divergence'] else '无'})"
                  f"{diff_mark}")


if __name__ == "__main__":
    main()
