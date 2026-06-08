"""递归引擎效率验证脚本。

修复后验证：
  - a_move_v1.py: zg_max / zd_min
  - a_level_protocol.py: ZG/ZD 递归组件范围
  - a_zhongshu_level.py: 方向判断修正

数据集：
  1. QQQ 1min  (21K bars, 3 months)
  2. QQQ 5min  (96K bars, 2 years) — optional, slow
  3. HK700 日线 (491 bars)

用法：
  PYTHONPATH=src python analysis/run_recursive_validation.py
  PYTHONPATH=src python analysis/run_recursive_validation.py --dataset 1min
  PYTHONPATH=src python analysis/run_recursive_validation.py --dataset 5min --max-bars 30000
  PYTHONPATH=src python analysis/run_recursive_validation.py --dataset hk700
  PYTHONPATH=src python analysis/run_recursive_validation.py --dataset all
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from datetime import datetime
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.types import Bar  # noqa: E402

DATA_DIR = PROJECT_ROOT / "analysis" / "data_cache"
REPORT_PATH = PROJECT_ROOT / "analysis" / "recursive_efficiency_validation.md"

DATASETS = {
    "1min": {
        "path": DATA_DIR / "qqq_1m_3mo.json",
        "symbol": "QQQ",
        "tf": "1min",
        "desc": "QQQ 1分钟 (3个月)",
    },
    "5min": {
        "path": DATA_DIR / "qqq_5m_2y.json",
        "symbol": "QQQ",
        "tf": "5min",
        "desc": "QQQ 5分钟 (2年)",
    },
    "daily": {
        "path": DATA_DIR / "QQQ_1d_max.json",
        "symbol": "QQQ",
        "tf": "1d",
        "desc": "QQQ 日线 (max, 1999至今)",
    },
    "hk700": {
        "path": DATA_DIR / "HK700_1d_2y.json",
        "symbol": "700.HK",
        "tf": "1d",
        "desc": "美团 日线 (2年)",
    },
}


def load_bars(path: Path, max_bars: int | None = None) -> tuple[list[Bar], dict]:
    with open(path) as f:
        raw = json.load(f)

    bars = []

    if isinstance(raw, dict) and "bars" in raw:
        bars_data = raw["bars"]
        if max_bars:
            bars_data = bars_data[:max_bars]
        for b in bars_data:
            ts_str = b.get("ts") or b.get("time") or b.get("date", "")
            try:
                ts = datetime.fromisoformat(ts_str)
            except (ValueError, TypeError):
                ts = datetime.now()
            bars.append(
                Bar(
                    ts=ts,
                    open=float(b["open"]),
                    high=float(b["high"]),
                    low=float(b["low"]),
                    close=float(b["close"]),
                    volume=b.get("volume"),
                )
            )
    elif isinstance(raw, dict) and "opens" in raw:
        n = len(raw["closes"])
        if max_bars:
            n = min(n, max_bars)
        dates = raw.get("dates", [f"bar_{i}" for i in range(n)])
        for i in range(n):
            ts_str = dates[i] if i < len(dates) else ""
            try:
                ts = datetime.fromisoformat(ts_str)
            except (ValueError, TypeError):
                try:
                    ts = datetime.strptime(ts_str, "%Y-%m-%d")
                except (ValueError, TypeError):
                    ts = datetime.now()
            bars.append(
                Bar(
                    ts=ts,
                    open=float(raw["opens"][i]),
                    high=float(raw["highs"][i]),
                    low=float(raw["lows"][i]),
                    close=float(raw["closes"][i]),
                    volume=None,
                )
            )
    elif isinstance(raw, list):
        if max_bars:
            raw = raw[:max_bars]
        for b in raw:
            ts_str = b.get("ts") or b.get("time") or b.get("date", "")
            try:
                ts = datetime.fromisoformat(ts_str)
            except (ValueError, TypeError):
                ts = datetime.now()
            bars.append(
                Bar(
                    ts=ts,
                    open=float(b["open"]),
                    high=float(b["high"]),
                    low=float(b["low"]),
                    close=float(b["close"]),
                    volume=b.get("volume"),
                )
            )

    meta = raw if isinstance(raw, dict) else {"bar_count": len(bars)}
    return bars, meta


def run_engine(
    bars: list[Bar], stream_id: str, use_stroke_zhongshu: bool = False,
) -> tuple:
    orch = RecursiveOrchestrator(
        stream_id=stream_id,
        max_levels=8,
        stroke_mode="new",
        min_strict_sep=5,
        enable_macd_divergence=True,
    )
    t0 = time.time()
    snap = None
    n = len(bars)
    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)
        if (i + 1) % 5000 == 0:
            elapsed = time.time() - t0
            bi = len(snap.bi_snapshot.strokes)
            sg = len(snap.seg_snapshot.segments)
            zs = len(snap.zs_snapshot.zhongshus)
            mv = len(snap.move_snapshot.moves)
            rc = len(snap.recursive_snapshots)
            ms_per = elapsed / (i + 1) * 1000
            print(
                f"  [{i+1}/{n}] {elapsed:.1f}s ({ms_per:.1f}ms/bar) | "
                f"bi={bi} seg={sg} zs={zs} mv={mv} rec_levels={rc}",
                flush=True,
            )
    elapsed = time.time() - t0
    print(f"  Done: {elapsed:.1f}s ({elapsed/n*1000:.1f}ms/bar)", flush=True)
    return snap, elapsed


def extract_level_stats(snap, bars: list[Bar]) -> list[dict]:
    levels = []

    def safe_ts(idx: int) -> str:
        return bars[min(idx, len(bars) - 1)].ts.isoformat()

    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves
    bsps = snap.bsp_snapshot.buysellpoints

    zs_details = []
    for i, z in enumerate(zhongshus):
        zs_details.append({
            "n": i,
            "zg": round(z.zg, 2),
            "zd": round(z.zd, 2),
            "gg": round(z.gg, 2),
            "dd": round(z.dd, 2),
            "seg_range": f"seg[{z.seg_start}..{z.seg_end}]",
            "seg_count": z.seg_count,
            "settled": z.settled,
        })

    mv_details = []
    for i, m in enumerate(moves):
        mv_details.append({
            "n": i,
            "kind": m.kind,
            "dir": m.direction,
            "zs_count": m.zs_count,
            "settled": m.settled,
            "high": round(m.high, 2),
            "low": round(m.low, 2),
            "zg_max": round(m.zg_max, 2) if hasattr(m, "zg_max") else None,
            "zd_min": round(m.zd_min, 2) if hasattr(m, "zd_min") else None,
            "seg_range": f"seg[{m.seg_start}..{m.seg_end}]",
        })

    bsp_details = []
    for b in bsps:
        bsp_details.append({
            "kind": b.kind,
            "side": b.side,
            "price": round(b.price, 2),
            "ts": safe_ts(b.bar_idx),
            "confirmed": b.confirmed,
            "settled": b.settled,
        })

    levels.append({
        "level": "L1 (base)",
        "strokes": len(strokes),
        "segments": len(segments),
        "zhongshus": len(zhongshus),
        "moves": len(moves),
        "settled_moves": sum(1 for m in moves if m.settled),
        "bsp": len(bsps),
        "zs_details": zs_details,
        "mv_details": mv_details,
        "bsp_details": bsp_details,
    })

    for rs in snap.recursive_snapshots:
        zs_d = []
        for i, z in enumerate(rs.zhongshus):
            d = {
                "n": i,
                "zg": round(z.zg, 2),
                "zd": round(z.zd, 2),
                "settled": z.settled,
            }
            if hasattr(z, "gg"):
                d["gg"] = round(z.gg, 2)
            if hasattr(z, "dd"):
                d["dd"] = round(z.dd, 2)
            if hasattr(z, "seg_count"):
                d["seg_count"] = z.seg_count
            elif hasattr(z, "comp_count"):
                d["seg_count"] = z.comp_count
            zs_d.append(d)

        mv_d = []
        for i, m in enumerate(rs.moves):
            d = {
                "n": i,
                "dir": m.direction,
                "kind": m.kind,
                "zs_count": m.zs_count,
                "settled": m.settled,
                "high": round(m.high, 2),
                "low": round(m.low, 2),
            }
            if hasattr(m, "zg_max"):
                d["zg_max"] = round(m.zg_max, 2)
            if hasattr(m, "zd_min"):
                d["zd_min"] = round(m.zd_min, 2)
            mv_d.append(d)

        levels.append({
            "level": f"L{rs.level_id}",
            "zhongshus": len(rs.zhongshus),
            "moves": len(rs.moves),
            "settled_moves": sum(1 for m in rs.moves if m.settled),
            "zs_details": zs_d,
            "mv_details": mv_d,
        })

    return levels


def format_report_section(
    dataset_name: str, desc: str, bars_count: int,
    elapsed: float, levels: list[dict],
) -> str:
    lines = []
    lines.append(f"## {desc}")
    lines.append("")
    lines.append(f"- **bars**: {bars_count}")
    lines.append(f"- **耗时**: {elapsed:.1f}s ({elapsed/bars_count*1000:.1f}ms/bar)")
    lines.append(f"- **递归深度**: {len(levels)} 层")
    lines.append("")

    lines.append("### 各层统计")
    lines.append("")
    lines.append("| 层级 | 笔 | 线段 | 中枢 | 走势 | settled走势 | 买卖点 |")
    lines.append("|------|-----|------|------|------|------------|--------|")
    for lv in levels:
        bi = lv.get("strokes", "-")
        seg = lv.get("segments", "-")
        zs = lv["zhongshus"]
        mv = lv["moves"]
        sm = lv["settled_moves"]
        bsp = lv.get("bsp", "-")
        lines.append(f"| {lv['level']} | {bi} | {seg} | {zs} | {mv} | {sm} | {bsp} |")
    lines.append("")

    for lv in levels:
        if lv["zhongshus"] == 0 and lv["moves"] == 0:
            continue
        lines.append(f"### {lv['level']} 详情")
        lines.append("")

        if lv["zs_details"]:
            lines.append("**中枢：**")
            lines.append("")
            for z in lv["zs_details"]:
                parts = [f"ZS#{z['n']}: [{z['zd']}~{z['zg']}]"]
                if "gg" in z:
                    parts.append(f"GG={z['gg']} DD={z['dd']}")
                if "seg_count" in z:
                    parts.append(f"{z['seg_count']}段")
                if "seg_range" in z:
                    parts.append(z["seg_range"])
                parts.append(f"settled={z['settled']}")
                lines.append(f"- {' | '.join(parts)}")
            lines.append("")

        if lv["mv_details"]:
            lines.append("**走势：**")
            lines.append("")
            for m in lv["mv_details"]:
                parts = [
                    f"Move#{m['n']}: {m['kind']} {m['dir']}",
                    f"zs={m['zs_count']}",
                    f"[{m['low']}~{m['high']}]",
                ]
                if m.get("zg_max") is not None:
                    parts.append(f"zg_max={m['zg_max']} zd_min={m['zd_min']}")
                if "seg_range" in m:
                    parts.append(m["seg_range"])
                parts.append(f"settled={m['settled']}")
                lines.append(f"- {' | '.join(parts)}")
            lines.append("")

        if lv.get("bsp_details"):
            lines.append("**买卖点（最近10个）：**")
            lines.append("")
            for b in lv["bsp_details"][-10:]:
                lines.append(
                    f"- {b['kind']} {b['side']} @{b['price']} "
                    f"({b['ts'][:16]}) conf={b['confirmed']} settled={b['settled']}"
                )
            lines.append("")

    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--dataset", default="all",
        choices=["1min", "5min", "daily", "hk700", "all"],
    )
    parser.add_argument("--max-bars", type=int, default=None)
    args = parser.parse_args()

    datasets_to_run = (
        list(DATASETS.keys()) if args.dataset == "all"
        else [args.dataset]
    )

    report_sections = []
    report_sections.append("# 递归引擎效率验证报告")
    report_sections.append("")
    report_sections.append(f"**生成时间**: {datetime.now().isoformat()[:19]}")
    report_sections.append("")
    report_sections.append("**修复内容**:")
    report_sections.append("- `a_move_v1.py`: 新增 `zg_max`/`zd_min` 走势边界追踪")
    report_sections.append("- `a_level_protocol.py`: 递归组件范围使用 ZG/ZD（非 GG/DD）")
    report_sections.append("- `a_zhongshu_level.py`: 修正走势方向判断逻辑")
    report_sections.append("")

    all_results = {}

    for ds_key in datasets_to_run:
        ds = DATASETS[ds_key]
        if not ds["path"].exists():
            print(f"SKIP {ds_key}: {ds['path']} not found")
            continue

        max_bars = args.max_bars
        if ds_key == "5min" and max_bars is None:
            max_bars = 30000
            print(f"NOTE: 5min defaulting to --max-bars={max_bars}")

        print(f"\n{'='*60}")
        print(f"Running: {ds['desc']} (max_bars={max_bars or 'all'})")
        print(f"{'='*60}")

        bars, meta = load_bars(ds["path"], max_bars=max_bars)
        print(f"  Loaded {len(bars)} bars")

        snap, elapsed = run_engine(bars, stream_id=f"{ds['symbol']}_{ds['tf']}")
        levels = extract_level_stats(snap, bars)

        section = format_report_section(
            ds_key, ds["desc"], len(bars), elapsed, levels,
        )
        report_sections.append(section)

        all_results[ds_key] = {
            "bars": len(bars),
            "elapsed": round(elapsed, 1),
            "ms_per_bar": round(elapsed / len(bars) * 1000, 1),
            "levels": len(levels),
            "l1_strokes": levels[0].get("strokes", 0),
            "l1_segments": levels[0].get("segments", 0),
            "l1_zhongshus": levels[0]["zhongshus"],
            "l1_moves": levels[0]["moves"],
        }

    summary_lines = []
    summary_lines.append("## 汇总")
    summary_lines.append("")
    summary_lines.append("| 数据集 | bars | 耗时 | ms/bar | 递归层数 | L1笔 | L1段 | L1中枢 | L1走势 |")
    summary_lines.append("|--------|------|------|--------|----------|------|------|--------|--------|")
    for k, r in all_results.items():
        summary_lines.append(
            f"| {k} | {r['bars']} | {r['elapsed']}s | {r['ms_per_bar']} | "
            f"{r['levels']} | {r['l1_strokes']} | {r['l1_segments']} | "
            f"{r['l1_zhongshus']} | {r['l1_moves']} |"
        )
    summary_lines.append("")

    report_sections.insert(4, "\n".join(summary_lines))

    with open(REPORT_PATH, "w") as f:
        f.write("\n".join(report_sections))
    print(f"\nReport saved to {REPORT_PATH}")


if __name__ == "__main__":
    main()
