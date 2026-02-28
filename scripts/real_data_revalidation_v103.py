"""L78 修复后真实数据重新验证 — 249号谱系。

段引擎 L78 前置 reject → warning 修复（commit ca3958a）后，
用完全相同的验证流程重跑 v101 的 SPY 真实数据验证。

唯一变量：段引擎修复。其他条件不变（标的、TF、参数）。

概念溯源：
  - 249号：L78 修复后真实数据重新验证
  - 248号：段引擎 71→1 压缩诊断
  - 247号：真实数据多 TF 验证
  - 246号：多 TF pipeline 集成
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

from newchan.topology.multi_tf_adapter import (
    CrossLevelDivergence,
    LevelResult,
    TimeframeLevel,
    _detect_cross_level_divergence,
    _move_amplitude,
)
from newchan.topology.multi_tf_pipeline import (
    MultiTFPipelineAdapter,
    MultiTFPipelineResult,
    _directional_move_force,
    detect_cross_level_divergence_directional,
)
from newchan.types import Bar

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("real_data_revalidation_v103")


# ── 数据获取 ──────────────────────────────────────────────


def fetch_spy_daily_weekly(years: int = 2) -> tuple[list[Bar], list[Bar]]:
    """拉取 SPY 日线 + 周线数据。"""
    ticker = yf.Ticker("SPY")

    logger.info("拉取 SPY 日线数据 (%d 年)...", years)
    daily_df = ticker.history(period=f"{years}y", interval="1d")
    logger.info("日线 bar 数: %d", len(daily_df))

    logger.info("拉取 SPY 周线数据 (%d 年)...", years)
    weekly_df = ticker.history(period=f"{years}y", interval="1wk")
    logger.info("周线 bar 数: %d", len(weekly_df))

    daily_bars = _df_to_bars(daily_df)
    weekly_bars = _df_to_bars(weekly_df)

    return daily_bars, weekly_bars


def fetch_spy_60min_daily(months: int = 6) -> tuple[list[Bar], list[Bar]]:
    """拉取 SPY 60min + 日线数据。

    yfinance 对 intraday 数据有限制（60min 最多 ~730 天），
    取能获得的最大范围。
    """
    ticker = yf.Ticker("SPY")

    # yfinance 60min 数据限制：最多约 730 天
    logger.info("拉取 SPY 60min 数据 (%d 月)...", months)
    intraday_df = ticker.history(period=f"{months}mo", interval="60m")
    logger.info("60min bar 数: %d", len(intraday_df))

    logger.info("拉取 SPY 日线数据 (%d 月)...", months)
    daily_df = ticker.history(period=f"{months}mo", interval="1d")
    logger.info("日线 bar 数: %d", len(daily_df))

    intraday_bars = _df_to_bars(intraday_df)
    daily_bars = _df_to_bars(daily_df)

    return intraday_bars, daily_bars


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


# ── 验证逻辑 ─────────────────────────────────────────────


def validate_dataset(
    name: str,
    low_tf_name: str,
    high_tf_name: str,
    low_bars: list[Bar],
    high_bars: list[Bar],
) -> dict:
    """对一组 TF 数据运行 MultiTFPipelineAdapter 并收集结果。"""
    logger.info("=" * 60)
    logger.info("验证数据集: %s (%s + %s)", name, low_tf_name, high_tf_name)
    logger.info("低级别 bar 数: %d, 高级别 bar 数: %d", len(low_bars), len(high_bars))

    timeframes = [
        TimeframeLevel(tf_name=low_tf_name, bar_source="yfinance", level_index=0),
        TimeframeLevel(tf_name=high_tf_name, bar_source="yfinance", level_index=1),
    ]

    # ── 使用方向性力度 ──
    adapter_directional = MultiTFPipelineAdapter(
        timeframes=timeframes,
        stroke_mode="wide",
        max_levels=6,
        use_directional_force=True,
    )

    tf_bars = {
        low_tf_name: low_bars,
        high_tf_name: high_bars,
    }

    result_dir: MultiTFPipelineResult = adapter_directional.run(
        tf_bars,
        timestamp=low_bars[-1].ts.timestamp() if low_bars else 0.0,
    )

    # ── 使用振幅力度（对比） ──
    adapter_amplitude = MultiTFPipelineAdapter(
        timeframes=timeframes,
        stroke_mode="wide",
        max_levels=6,
        use_directional_force=False,
    )

    result_amp: MultiTFPipelineResult = adapter_amplitude.run(
        tf_bars,
        timestamp=low_bars[-1].ts.timestamp() if low_bars else 0.0,
    )

    # ── 收集诊断信息 ──
    diag = _diagnose_result(result_dir, result_amp, name, low_tf_name, high_tf_name)

    # ── 管线各阶段诊断（深度诊断 D2 压缩） ──
    pipeline_stages = {}
    for tf_name, bars in tf_bars.items():
        if not bars:
            continue
        lr = result_dir.multi_tf_result.levels.get(tf_name)
        if lr is None or lr.snapshot is None:
            pipeline_stages[tf_name] = {
                "bar_count": len(bars),
                "stroke_count": 0,
                "segment_count": 0,
                "zhongshu_count": 0,
                "move_count": 0,
                "snapshot_available": False,
            }
            continue

        snap = lr.snapshot
        strokes = snap.bi_snapshot.strokes
        segments = snap.seg_snapshot.segments
        zhongshus = snap.zs_snapshot.zhongshus
        moves = snap.move_snapshot.moves

        stage_info = {
            "bar_count": len(bars),
            "stroke_count": len(strokes),
            "segment_count": len(segments),
            "zhongshu_count": len(zhongshus),
            "move_count": len(moves),
            "snapshot_available": True,
            "d2_compression_ratio": (
                f"{len(strokes)} strokes → {len(segments)} segments"
                if len(strokes) > 0 else "no strokes"
            ),
        }

        # 线段详情
        seg_details = []
        for s in segments:
            seg_details.append({
                "s0": s.s0, "s1": s.s1,
                "direction": s.direction,
                "confirmed": s.confirmed,
                "high": s.high, "low": s.low,
                "stroke_span": s.s1 - s.s0,
            })
        stage_info["segments"] = seg_details

        # 中枢详情
        zs_details = []
        for z in zhongshus:
            zs_details.append({
                "seg_start": z.seg_start, "seg_end": z.seg_end,
                "zg": z.zg, "zd": z.zd,
                "gg": z.gg, "dd": z.dd,
            })
        stage_info["zhongshus"] = zs_details

        pipeline_stages[tf_name] = stage_info
        logger.info("[%s] TF=%s pipeline: %d bars → %d strokes → %d segs → %d zs → %d moves",
                    name, tf_name, len(bars), len(strokes), len(segments),
                    len(zhongshus), len(moves))

    diag["pipeline_stages"] = pipeline_stages
    return diag


def _diagnose_result(
    result_dir: MultiTFPipelineResult,
    result_amp: MultiTFPipelineResult,
    dataset_name: str,
    low_tf_name: str,
    high_tf_name: str,
) -> dict:
    """从两个结果中提取诊断信息。"""

    # T6 可达性
    active_levels_dir = result_dir.recursive_levels_equivalent
    t6_reachable_dir = result_dir.t6_reachable
    active_levels_amp = result_amp.recursive_levels_equivalent
    t6_reachable_amp = result_amp.t6_reachable

    logger.info("[%s] 方向性力度: active_levels=%d, t6_reachable=%s",
                dataset_name, active_levels_dir, t6_reachable_dir)
    logger.info("[%s] 振幅力度:   active_levels=%d, t6_reachable=%s",
                dataset_name, active_levels_amp, t6_reachable_amp)

    # 各 TF 层详情
    level_details = {}
    for tf_name, lr in result_dir.multi_tf_result.levels.items():
        detail = {
            "bar_count": lr.bar_count,
            "move_count": lr.move_count,
            "zhongshu_count": lr.zhongshu_count,
            "direction": lr.direction,
            "has_last_move": lr.last_move is not None,
        }
        if lr.last_move is not None:
            m = lr.last_move
            detail["last_move"] = {
                "kind": m.kind,
                "direction": m.direction,
                "seg_start": m.seg_start,
                "seg_end": m.seg_end,
                "zs_count": m.zs_count,
                "settled": m.settled,
                "high": m.high,
                "low": m.low,
            }
        logger.info("[%s] TF=%s: bars=%d, moves=%d, zhongshu=%d, dir=%s, has_move=%s",
                    dataset_name, tf_name, lr.bar_count, lr.move_count,
                    lr.zhongshu_count, lr.direction, lr.last_move is not None)
        level_details[tf_name] = detail

    # 方向性力度背驰
    dir_divs = result_dir.directional_divergences
    dir_div_details = [_divergence_to_dict(d) for d in dir_divs]
    logger.info("[%s] 方向性力度背驰数量: %d", dataset_name, len(dir_divs))
    for i, d in enumerate(dir_divs):
        logger.info("  背驰 %d: dir=%s, ratio=%.4f, confirmed=%s, "
                    "high_tf=%s, low_tf=%s, force_high=%.4f, force_low=%.4f",
                    i, d.direction, d.ratio, d.confirmed,
                    d.high_tf.tf_name, d.low_tf.tf_name,
                    d.force_high, d.force_low)

    # 振幅力度背驰（对比）
    amp_divs = result_amp.directional_divergences
    amp_div_details = [_divergence_to_dict(d) for d in amp_divs]
    logger.info("[%s] 振幅力度背驰数量: %d", dataset_name, len(amp_divs))
    for i, d in enumerate(amp_divs):
        logger.info("  背驰 %d: dir=%s, ratio=%.4f, confirmed=%s, "
                    "force_high=%.4f, force_low=%.4f",
                    i, d.direction, d.ratio, d.confirmed,
                    d.force_high, d.force_low)

    # 共振信号
    resonance_dir = result_dir.resonance_signals
    resonance_amp = result_amp.resonance_signals
    logger.info("[%s] 方向性共振信号数量: %d", dataset_name, len(resonance_dir))
    logger.info("[%s] 振幅共振信号数量: %d", dataset_name, len(resonance_amp))

    # 买卖点
    bsps_dir = result_dir.bsps
    bsps_amp = result_amp.bsps
    logger.info("[%s] 方向性买卖点数量: %d", dataset_name, len(bsps_dir))
    logger.info("[%s] 振幅买卖点数量: %d", dataset_name, len(bsps_amp))

    # ── 力度值对比分析 ──
    force_comparison = _compare_force_methods(result_dir, result_amp, low_tf_name, high_tf_name)

    return {
        "dataset": dataset_name,
        "low_tf": low_tf_name,
        "high_tf": high_tf_name,
        "t6_reachability": {
            "directional": {
                "active_levels": active_levels_dir,
                "t6_reachable": t6_reachable_dir,
            },
            "amplitude": {
                "active_levels": active_levels_amp,
                "t6_reachable": t6_reachable_amp,
            },
        },
        "level_details": level_details,
        "divergences": {
            "directional": {
                "count": len(dir_divs),
                "details": dir_div_details,
            },
            "amplitude": {
                "count": len(amp_divs),
                "details": amp_div_details,
            },
        },
        "resonance_signals": {
            "directional_count": len(resonance_dir),
            "amplitude_count": len(resonance_amp),
        },
        "bsps": {
            "directional_count": len(bsps_dir),
            "amplitude_count": len(bsps_amp),
        },
        "force_comparison": force_comparison,
    }


def _compare_force_methods(
    result_dir: MultiTFPipelineResult,
    result_amp: MultiTFPipelineResult,
    low_tf_name: str,
    high_tf_name: str,
) -> dict:
    """比较两种力度方法在相同数据上的力度值。"""
    comparison = {}

    for tf_name in [low_tf_name, high_tf_name]:
        lr_dir = result_dir.multi_tf_result.levels.get(tf_name)
        lr_amp = result_amp.multi_tf_result.levels.get(tf_name)

        if lr_dir is None or lr_amp is None:
            continue
        if lr_dir.last_move is None or lr_amp.last_move is None:
            continue

        move = lr_dir.last_move  # 同一个 move（数据一样）
        amp_force = _move_amplitude(move)
        dir_force = _directional_move_force(move, lr_dir)

        comparison[tf_name] = {
            "amplitude_force": amp_force,
            "directional_force": dir_force,
            "move_kind": move.kind,
            "move_direction": move.direction,
            "move_zs_count": move.zs_count,
            "move_seg_span": move.seg_end - move.seg_start + 1,
        }

    return comparison


def _divergence_to_dict(d: CrossLevelDivergence) -> dict:
    """将 CrossLevelDivergence 转换为可序列化的 dict。"""
    return {
        "direction": d.direction,
        "ratio": d.ratio,
        "confirmed": d.confirmed,
        "high_tf": d.high_tf.tf_name,
        "low_tf": d.low_tf.tf_name,
        "force_high": d.force_high,
        "force_low": d.force_low,
        "high_move": {
            "kind": d.high_move.kind,
            "direction": d.high_move.direction,
            "high": d.high_move.high,
            "low": d.high_move.low,
            "zs_count": d.high_move.zs_count,
            "settled": d.high_move.settled,
        },
        "low_move": {
            "kind": d.low_move.kind,
            "direction": d.low_move.direction,
            "high": d.low_move.high,
            "low": d.low_move.low,
            "zs_count": d.low_move.zs_count,
            "settled": d.low_move.settled,
        },
    }


# ── 主流程 ───────────────────────────────────────────────


def main() -> None:
    results = {}

    # ── 数据集 1：日线 + 周线（2年） ──
    try:
        daily_bars, weekly_bars = fetch_spy_daily_weekly(years=2)
        results["daily_weekly"] = validate_dataset(
            name="SPY日线+周线(2年)",
            low_tf_name="daily",
            high_tf_name="weekly",
            low_bars=daily_bars,
            high_bars=weekly_bars,
        )
    except Exception as e:
        logger.error("日线+周线数据集失败: %s", e, exc_info=True)
        results["daily_weekly"] = {"error": str(e)}

    # ── 数据集 2：60min + 日线（6个月） ──
    try:
        intraday_bars, daily_bars_2 = fetch_spy_60min_daily(months=6)
        results["60min_daily"] = validate_dataset(
            name="SPY60min+日线(6月)",
            low_tf_name="60min",
            high_tf_name="daily",
            low_bars=intraday_bars,
            high_bars=daily_bars_2,
        )
    except Exception as e:
        logger.error("60min+日线数据集失败: %s", e, exc_info=True)
        results["60min_daily"] = {"error": str(e)}

    # ── 汇总结论 ──
    summary = _build_summary(results)
    results["summary"] = summary

    # ── 输出 ──
    output_path = PROJECT_ROOT / "tmp" / "real-data-revalidation-v103.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False, default=str)

    logger.info("结果已写入: %s", output_path)

    # 打印汇总
    print("\n" + "=" * 60)
    print("v103 重新验证汇总（L78 修复后）")
    print("=" * 60)
    for key, val in summary.items():
        print(f"  {key}: {val}")


def _build_summary(results: dict) -> dict:
    """从两个数据集结果构建汇总。"""
    summary = {}

    # T6 可达性
    t6_datasets = []
    for ds_key in ["daily_weekly", "60min_daily"]:
        ds = results.get(ds_key, {})
        if "error" in ds:
            t6_datasets.append(f"{ds_key}: ERROR")
            continue
        t6_info = ds.get("t6_reachability", {})
        dir_t6 = t6_info.get("directional", {}).get("t6_reachable", False)
        amp_t6 = t6_info.get("amplitude", {}).get("t6_reachable", False)
        t6_datasets.append(f"{ds_key}: dir={dir_t6}, amp={amp_t6}")

    summary["t6_reachability"] = t6_datasets

    # 任何一个数据集上 T6 可达即可
    any_t6 = False
    for ds_key in ["daily_weekly", "60min_daily"]:
        ds = results.get(ds_key, {})
        if "error" in ds:
            continue
        t6_info = ds.get("t6_reachability", {})
        if t6_info.get("directional", {}).get("t6_reachable", False):
            any_t6 = True
    summary["t6_reachable_any"] = any_t6

    # 背驰信号统计
    total_dir_divs = 0
    total_amp_divs = 0
    for ds_key in ["daily_weekly", "60min_daily"]:
        ds = results.get(ds_key, {})
        if "error" in ds:
            continue
        divs = ds.get("divergences", {})
        total_dir_divs += divs.get("directional", {}).get("count", 0)
        total_amp_divs += divs.get("amplitude", {}).get("count", 0)

    summary["directional_divergences_total"] = total_dir_divs
    summary["amplitude_divergences_total"] = total_amp_divs
    summary["divergence_difference"] = total_dir_divs - total_amp_divs

    # ker(D) 操作意义判定
    if total_dir_divs != total_amp_divs:
        summary["ker_d_operational_significance"] = (
            f"有操作意义: 方向性力度检测到 {total_dir_divs} 个背驰, "
            f"振幅力度检测到 {total_amp_divs} 个背驰, "
            f"差异 {abs(total_dir_divs - total_amp_divs)} 个"
        )
    else:
        summary["ker_d_operational_significance"] = (
            f"本次验证中无差异: 两种力度均检测到 {total_dir_divs} 个背驰. "
            "可能原因: 数据量不足或市场阶段特殊"
        )

    # 诊断: 如果 T6 不可达，分析原因
    t6_diagnosis = []
    for ds_key in ["daily_weekly", "60min_daily"]:
        ds = results.get(ds_key, {})
        if "error" in ds:
            continue
        t6_info = ds.get("t6_reachability", {})
        if not t6_info.get("directional", {}).get("t6_reachable", False):
            level_details = ds.get("level_details", {})
            missing_moves = []
            for tf_name, detail in level_details.items():
                if not detail.get("has_last_move", False):
                    missing_moves.append(tf_name)
            if missing_moves:
                t6_diagnosis.append(
                    f"{ds_key}: T6不可达, 原因: TF {missing_moves} 未产生走势 "
                    f"(bar数不足或波动不够)"
                )
            else:
                t6_diagnosis.append(
                    f"{ds_key}: T6不可达但所有TF有走势——检查 active_levels 计算逻辑"
                )
    if t6_diagnosis:
        summary["t6_unreachable_diagnosis"] = t6_diagnosis

    # D2 压缩诊断
    d2_compression = []
    for ds_key in ["daily_weekly", "60min_daily"]:
        ds = results.get(ds_key, {})
        if "error" in ds:
            continue
        stages = ds.get("pipeline_stages", {})
        for tf_name, stage in stages.items():
            stroke_count = stage.get("stroke_count", 0)
            seg_count = stage.get("segment_count", 0)
            zs_count = stage.get("zhongshu_count", 0)
            move_count = stage.get("move_count", 0)
            d2_compression.append(
                f"{ds_key}/{tf_name}: {stage.get('bar_count', 0)} bars → "
                f"{stroke_count} strokes → {seg_count} segs → "
                f"{zs_count} zhongshu → {move_count} moves"
            )
    summary["d2_compression_pipeline"] = d2_compression

    return summary


if __name__ == "__main__":
    main()
