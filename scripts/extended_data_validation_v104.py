"""扩展数据窗口验证 — 250号谱系。

v103 确认段引擎 L78 修复后 60min 恢复到 8 段 2 中枢 1 走势。
但 T6 仍不可达（active_levels=1），瓶颈是 daily 层数据不足。

本脚本扩展数据窗口：
  - 数据集1: SPY daily 3年 + weekly 3年（yfinance）
  - 数据集2: SPY 60min 730天 + daily 对应时段（yfinance）
  - 数据集3: SPY daily 2年+（Alpha Vantage）

概念溯源：
  - 250号：扩展数据窗口验证
  - 249号：L78 修复后真实数据重新验证
  - 248号：段引擎 71→1 压缩诊断
  - 246号：多 TF pipeline 集成
"""

from __future__ import annotations

import json
import logging
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import requests
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
logger = logging.getLogger("extended_data_validation_v104")


# ── 数据获取 ──────────────────────────────────────────────


def fetch_spy_daily_weekly_yf(years: int = 3) -> tuple[list[Bar], list[Bar]]:
    """拉取 SPY 日线 + 周线数据（yfinance, 扩展到 3 年）。"""
    ticker = yf.Ticker("SPY")

    logger.info("拉取 SPY 日线数据 (%d 年, yfinance)...", years)
    daily_df = ticker.history(period=f"{years}y", interval="1d")
    logger.info("日线 bar 数: %d", len(daily_df))

    logger.info("拉取 SPY 周线数据 (%d 年, yfinance)...", years)
    weekly_df = ticker.history(period=f"{years}y", interval="1wk")
    logger.info("周线 bar 数: %d", len(weekly_df))

    daily_bars = _df_to_bars(daily_df)
    weekly_bars = _df_to_bars(weekly_df)

    return daily_bars, weekly_bars


def fetch_spy_60min_daily_yf(days: int = 730) -> tuple[list[Bar], list[Bar]]:
    """拉取 SPY 60min + 日线数据（yfinance, 扩展到 730 天）。

    yfinance 对 intraday 60min 数据最多支持约 730 天。
    """
    ticker = yf.Ticker("SPY")

    logger.info("拉取 SPY 60min 数据 (%d 天, yfinance)...", days)
    intraday_df = ticker.history(period=f"{days}d", interval="60m")
    logger.info("60min bar 数: %d", len(intraday_df))

    logger.info("拉取 SPY 日线数据 (%d 天, yfinance)...", days)
    daily_df = ticker.history(period=f"{days}d", interval="1d")
    logger.info("日线 bar 数: %d", len(daily_df))

    intraday_bars = _df_to_bars(intraday_df)
    daily_bars = _df_to_bars(daily_df)

    return intraday_bars, daily_bars


def fetch_spy_daily_av() -> list[Bar]:
    """拉取 SPY 日线数据（Alpha Vantage, full outputsize = 20年+）。"""
    import os as _os
    api_key = _os.environ.get("ALPHA_VANTAGE_API_KEY")
    if not api_key:
        raise RuntimeError("ALPHA_VANTAGE_API_KEY 环境变量未设置，拒绝执行")
    url = (
        "https://www.alphavantage.co/query"
        f"?function=TIME_SERIES_DAILY&symbol=SPY"
        f"&outputsize=full&apikey={api_key}&datatype=json"
    )

    logger.info("拉取 SPY 日线数据 (Alpha Vantage, full)...")
    resp = requests.get(url, timeout=60)
    data = resp.json()

    # Rate limit check
    if "Note" in data or "Information" in data:
        msg = data.get("Note", data.get("Information", ""))
        logger.warning("Alpha Vantage rate limited: %s — 等待 65 秒后重试...", msg[:100])
        time.sleep(65)
        resp = requests.get(url, timeout=60)
        data = resp.json()
        if "Note" in data or "Information" in data:
            raise RuntimeError(
                f"Alpha Vantage 仍然 rate limited: {data.get('Note', data.get('Information', ''))}"
            )

    ts_data = data.get("Time Series (Daily)", {})
    if not ts_data:
        raise RuntimeError(f"Alpha Vantage 无数据, keys: {list(data.keys())}")

    logger.info("Alpha Vantage 返回 %d 个日线 bar", len(ts_data))

    bars: list[Bar] = []
    for dt_str in sorted(ts_data.keys()):
        vals = ts_data[dt_str]
        ts = datetime.strptime(dt_str, "%Y-%m-%d").replace(tzinfo=timezone.utc)
        bars.append(
            Bar(
                ts=ts,
                open=float(vals["1. open"]),
                high=float(vals["2. high"]),
                low=float(vals["3. low"]),
                close=float(vals["4. close"]),
                volume=float(vals.get("5. volume", 0)),
            )
        )

    # 只取最近 3 年（~750 bar），与 yfinance 数据集对齐
    if len(bars) > 750:
        bars = bars[-750:]
        logger.info("截取最近 750 bar（约 3 年）")

    return bars


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


def validate_single_tf(
    name: str,
    tf_name: str,
    bars: list[Bar],
) -> dict:
    """对单一 TF 数据运行 MultiTFPipelineAdapter（无高级别配对）。

    用于 Alpha Vantage 数据集——只有 daily，没有配对的 weekly。
    通过与自身配对（只看 daily 层结果）来跑管线。
    """
    logger.info("=" * 60)
    logger.info("验证数据集: %s (单 TF: %s)", name, tf_name)
    logger.info("bar 数: %d", len(bars))

    # 单 TF 配置——只设一层
    timeframes = [
        TimeframeLevel(tf_name=tf_name, bar_source="alphavantage", level_index=0),
    ]

    # ── 方向性力度 ──
    adapter_directional = MultiTFPipelineAdapter(
        timeframes=timeframes,
        stroke_mode="wide",
        max_levels=6,
        use_directional_force=True,
    )

    tf_bars = {tf_name: bars}

    result_dir: MultiTFPipelineResult = adapter_directional.run(
        tf_bars,
        timestamp=bars[-1].ts.timestamp() if bars else 0.0,
    )

    # ── 振幅力度 ──
    adapter_amplitude = MultiTFPipelineAdapter(
        timeframes=timeframes,
        stroke_mode="wide",
        max_levels=6,
        use_directional_force=False,
    )

    result_amp: MultiTFPipelineResult = adapter_amplitude.run(
        tf_bars,
        timestamp=bars[-1].ts.timestamp() if bars else 0.0,
    )

    # 收集诊断
    diag = {
        "dataset": name,
        "tf": tf_name,
        "bar_count": len(bars),
        "t6_reachability": {
            "directional": {
                "active_levels": result_dir.recursive_levels_equivalent,
                "t6_reachable": result_dir.t6_reachable,
            },
            "amplitude": {
                "active_levels": result_amp.recursive_levels_equivalent,
                "t6_reachable": result_amp.t6_reachable,
            },
        },
        "note": "单 TF 数据集（Alpha Vantage daily），T6 需要 >=2 active_levels，"
                "此数据集最多 1 层——用于 pipeline 管线诊断对比",
    }

    # 管线诊断
    lr = result_dir.multi_tf_result.levels.get(tf_name)
    if lr is not None and lr.snapshot is not None:
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

        zs_details = []
        for z in zhongshus:
            zs_details.append({
                "seg_start": z.seg_start, "seg_end": z.seg_end,
                "zg": z.zg, "zd": z.zd,
                "gg": z.gg, "dd": z.dd,
            })
        stage_info["zhongshus"] = zs_details

        diag["pipeline_stages"] = {tf_name: stage_info}

        # level details
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
        diag["level_details"] = {tf_name: detail}

        # 力度对比
        if lr.last_move is not None:
            move = lr.last_move
            amp_force = _move_amplitude(move)
            dir_force = _directional_move_force(move, lr)
            diag["force_comparison"] = {
                tf_name: {
                    "amplitude_force": amp_force,
                    "directional_force": dir_force,
                    "move_kind": move.kind,
                    "move_direction": move.direction,
                    "move_zs_count": move.zs_count,
                    "move_seg_span": move.seg_end - move.seg_start + 1,
                }
            }

        logger.info("[%s] TF=%s pipeline: %d bars → %d strokes → %d segs → %d zs → %d moves",
                    name, tf_name, len(bars), len(strokes), len(segments),
                    len(zhongshus), len(moves))
    else:
        diag["pipeline_stages"] = {tf_name: {
            "bar_count": len(bars), "snapshot_available": False,
        }}

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

        move = lr_dir.last_move
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


# ── v103 对比 ─────────────────────────────────────────────


def _load_v103_results() -> dict | None:
    """加载 v103 结果用于对比。"""
    v103_path = PROJECT_ROOT / "tmp" / "real-data-revalidation-v103.json"
    if not v103_path.exists():
        logger.warning("v103 结果文件不存在: %s", v103_path)
        return None
    with open(v103_path, encoding="utf-8") as f:
        return json.load(f)


def _compare_with_v103(results: dict, v103: dict) -> dict:
    """将 v104 结果与 v103 结果进行对比。"""
    comparison = {}

    # daily_weekly 对比
    ds1_v104 = results.get("daily_weekly_3y", {})
    ds1_v103 = v103.get("daily_weekly", {})
    if ds1_v104 and ds1_v103 and "error" not in ds1_v104 and "error" not in ds1_v103:
        comparison["daily_weekly"] = _compare_pipeline_stages(
            ds1_v104.get("pipeline_stages", {}),
            ds1_v103.get("pipeline_stages", {}),
            "daily",
        )

    # 60min_daily 对比
    ds2_v104 = results.get("60min_daily_730d", {})
    ds2_v103 = v103.get("60min_daily", {})
    if ds2_v104 and ds2_v103 and "error" not in ds2_v104 and "error" not in ds2_v103:
        comparison["60min_daily"] = {
            "60min": _compare_pipeline_stages(
                ds2_v104.get("pipeline_stages", {}),
                ds2_v103.get("pipeline_stages", {}),
                "60min",
            ),
            "daily": _compare_pipeline_stages(
                ds2_v104.get("pipeline_stages", {}),
                ds2_v103.get("pipeline_stages", {}),
                "daily",
            ),
        }

    return comparison


def _compare_pipeline_stages(v104_stages: dict, v103_stages: dict, tf_name: str) -> dict:
    """比较两个版本在同一 TF 上的管线指标。"""
    s104 = v104_stages.get(tf_name, {})
    s103 = v103_stages.get(tf_name, {})

    return {
        "v103": {
            "bar_count": s103.get("bar_count", 0),
            "stroke_count": s103.get("stroke_count", 0),
            "segment_count": s103.get("segment_count", 0),
            "zhongshu_count": s103.get("zhongshu_count", 0),
            "move_count": s103.get("move_count", 0),
        },
        "v104": {
            "bar_count": s104.get("bar_count", 0),
            "stroke_count": s104.get("stroke_count", 0),
            "segment_count": s104.get("segment_count", 0),
            "zhongshu_count": s104.get("zhongshu_count", 0),
            "move_count": s104.get("move_count", 0),
        },
        "delta": {
            "bar_count": s104.get("bar_count", 0) - s103.get("bar_count", 0),
            "stroke_count": s104.get("stroke_count", 0) - s103.get("stroke_count", 0),
            "segment_count": s104.get("segment_count", 0) - s103.get("segment_count", 0),
            "zhongshu_count": s104.get("zhongshu_count", 0) - s103.get("zhongshu_count", 0),
            "move_count": s104.get("move_count", 0) - s103.get("move_count", 0),
        },
    }


# ── 主流程 ───────────────────────────────────────────────


def main() -> None:
    results = {}

    # ── 数据集 1：日线 + 周线（3年, yfinance） ──
    try:
        daily_bars, weekly_bars = fetch_spy_daily_weekly_yf(years=3)
        results["daily_weekly_3y"] = validate_dataset(
            name="SPY日线+周线(3年,yfinance)",
            low_tf_name="daily",
            high_tf_name="weekly",
            low_bars=daily_bars,
            high_bars=weekly_bars,
        )
    except Exception as e:
        logger.error("数据集1 (daily_weekly_3y) 失败: %s", e, exc_info=True)
        results["daily_weekly_3y"] = {"error": str(e)}

    # ── 数据集 2：60min + 日线（730天, yfinance） ──
    try:
        intraday_bars, daily_bars_2 = fetch_spy_60min_daily_yf(days=730)
        results["60min_daily_730d"] = validate_dataset(
            name="SPY60min+日线(730天,yfinance)",
            low_tf_name="60min",
            high_tf_name="daily",
            low_bars=intraday_bars,
            high_bars=daily_bars_2,
        )
    except Exception as e:
        logger.error("数据集2 (60min_daily_730d) 失败: %s", e, exc_info=True)
        results["60min_daily_730d"] = {"error": str(e)}

    # ── 数据集 3：Alpha Vantage SPY daily（补充验证） ──
    try:
        av_bars = fetch_spy_daily_av()
        results["av_daily"] = validate_single_tf(
            name="SPY日线(Alpha Vantage,~3年)",
            tf_name="daily",
            bars=av_bars,
        )
    except Exception as e:
        logger.error("数据集3 (av_daily) 失败: %s", e, exc_info=True)
        results["av_daily"] = {"error": str(e)}

    # ── 与 v103 对比 ──
    v103 = _load_v103_results()
    if v103 is not None:
        results["v103_comparison"] = _compare_with_v103(results, v103)

    # ── 汇总结论 ──
    summary = _build_summary(results)
    results["summary"] = summary

    # ── 输出 ──
    output_path = PROJECT_ROOT / "tmp" / "extended-data-validation-v104.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False, default=str)

    logger.info("结果已写入: %s", output_path)

    # 打印汇总
    print("\n" + "=" * 60)
    print("v104 扩展数据窗口验证汇总")
    print("=" * 60)
    for key, val in summary.items():
        if isinstance(val, list):
            print(f"  {key}:")
            for item in val:
                print(f"    - {item}")
        else:
            print(f"  {key}: {val}")


def _build_summary(results: dict) -> dict:
    """从三个数据集结果构建汇总。"""
    summary = {}

    ds_keys = ["daily_weekly_3y", "60min_daily_730d", "av_daily"]

    # T6 可达性
    t6_datasets = []
    for ds_key in ds_keys:
        ds = results.get(ds_key, {})
        if "error" in ds:
            t6_datasets.append(f"{ds_key}: ERROR — {ds['error'][:80]}")
            continue
        t6_info = ds.get("t6_reachability", {})
        dir_t6 = t6_info.get("directional", {}).get("t6_reachable", False)
        amp_t6 = t6_info.get("amplitude", {}).get("t6_reachable", False)
        dir_levels = t6_info.get("directional", {}).get("active_levels", 0)
        amp_levels = t6_info.get("amplitude", {}).get("active_levels", 0)
        t6_datasets.append(
            f"{ds_key}: dir_t6={dir_t6} (levels={dir_levels}), "
            f"amp_t6={amp_t6} (levels={amp_levels})"
        )

    summary["t6_reachability"] = t6_datasets

    # 任何一个数据集上 T6 可达即可
    any_t6 = False
    for ds_key in ds_keys:
        ds = results.get(ds_key, {})
        if "error" in ds:
            continue
        t6_info = ds.get("t6_reachability", {})
        if t6_info.get("directional", {}).get("t6_reachable", False):
            any_t6 = True
        if t6_info.get("amplitude", {}).get("t6_reachable", False):
            any_t6 = True
    summary["t6_reachable_any"] = any_t6

    # 背驰信号统计
    total_dir_divs = 0
    total_amp_divs = 0
    for ds_key in ds_keys:
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
            f"本次验证中无差异: 两种力度均检测到 {total_dir_divs} 个背驰"
        )

    # 诊断: 如果 T6 不可达，分析原因
    t6_diagnosis = []
    for ds_key in ds_keys:
        ds = results.get(ds_key, {})
        if "error" in ds:
            continue
        t6_info = ds.get("t6_reachability", {})
        if not t6_info.get("directional", {}).get("t6_reachable", False):
            level_details = ds.get("level_details", {})
            stages = ds.get("pipeline_stages", {})
            missing_moves = []
            for tf_name, detail in level_details.items():
                if not detail.get("has_last_move", False):
                    stage = stages.get(tf_name, {})
                    seg_count = stage.get("segment_count", 0)
                    zs_count = stage.get("zhongshu_count", 0)
                    missing_moves.append(
                        f"{tf_name}(segs={seg_count},zs={zs_count})"
                    )
            if missing_moves:
                t6_diagnosis.append(
                    f"{ds_key}: T6不可达, 无走势的TF: {missing_moves}"
                )
            else:
                t6_diagnosis.append(
                    f"{ds_key}: T6不可达但所有TF有走势——检查 active_levels 计算逻辑"
                )
    if t6_diagnosis:
        summary["t6_unreachable_diagnosis"] = t6_diagnosis

    # D2 压缩诊断
    d2_compression = []
    for ds_key in ds_keys:
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

    # v103 对比摘要
    v103_comp = {}
    comparison = {}
    # pull from results if available
    raw_comp = results.get("v103_comparison", {})
    if raw_comp:
        for section_key, section_val in raw_comp.items():
            if isinstance(section_val, dict) and "delta" in section_val:
                v103_comp[section_key] = section_val["delta"]
            elif isinstance(section_val, dict):
                for sub_key, sub_val in section_val.items():
                    if isinstance(sub_val, dict) and "delta" in sub_val:
                        v103_comp[f"{section_key}/{sub_key}"] = sub_val["delta"]
    if v103_comp:
        summary["v103_delta"] = v103_comp

    return summary


if __name__ == "__main__":
    main()
