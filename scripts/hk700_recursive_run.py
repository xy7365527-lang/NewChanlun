"""700.HK 1min TWS 数据递归分析。

读取 analysis/data_cache/hk700_1m_tws.json，用 RecursiveOrchestrator 跑全量递归，
输出每层笔/线段/中枢/走势数量。
"""

from __future__ import annotations

import json
import logging
import sys
from datetime import datetime, timezone
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("hk700_recursive")

DATA_PATH = PROJECT_ROOT / "analysis" / "data_cache" / "hk700_1m_tws.json"


def load_bars() -> list[Bar]:
    with open(DATA_PATH) as f:
        raw = json.load(f)
    bars: list[Bar] = []
    for r in raw:
        dt_str = r["date"]
        if "+" in dt_str:
            dt_str = dt_str.rsplit("+", 1)[0]
        elif dt_str.endswith("Z"):
            dt_str = dt_str[:-1]
        ts = datetime.strptime(dt_str, "%Y-%m-%d %H:%M:%S").replace(tzinfo=timezone.utc)
        bars.append(Bar(
            ts=ts,
            open=float(r["open"]),
            high=float(r["high"]),
            low=float(r["low"]),
            close=float(r["close"]),
            volume=float(r["volume"]),
        ))
    return bars


def run() -> None:
    bars = load_bars()
    logger.info("加载 %d 根 bar，范围: %s ~ %s", len(bars), bars[0].ts, bars[-1].ts)

    orch = RecursiveOrchestrator(
        stream_id="HK700_1min",
        max_levels=6,
        stroke_mode="wide",
    )

    snap = None
    for bar in bars:
        snap = orch.process_bar(bar)

    if snap is None:
        raise RuntimeError("process_bar 未返回快照")

    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves
    bsps = snap.bsp_snapshot.buysellpoints

    print(f"\n{'='*60}")
    print(f"700.HK 1min 递归分析结果")
    print(f"{'='*60}")
    print(f"数据: {len(bars)} 根 bar, {bars[0].ts.date()} ~ {bars[-1].ts.date()}")
    print(f"\n--- Level 1 (1min base) ---")
    print(f"  笔 (strokes):  {len(strokes)}")
    print(f"  线段 (segments): {len(segments)}")
    print(f"  中枢 (zhongshu): {len(zhongshus)}")
    print(f"  走势 (moves):    {len(moves)}")
    print(f"  买卖点 (BSP):    {len(bsps)}")

    for rs in snap.recursive_snapshots:
        print(f"\n--- Level {rs.level_id} ---")
        print(f"  中枢 (zhongshu): {len(rs.zhongshus)}")
        print(f"  走势 (moves):    {len(rs.moves)}")

    print(f"\n{'='*60}")
    pipeline = (
        f"{len(bars)} bars → {len(strokes)} strokes → {len(segments)} segs"
        f" → {len(zhongshus)} zs → {len(moves)} moves"
    )
    max_level = 1
    for rs in snap.recursive_snapshots:
        if rs.level_id > max_level:
            max_level = rs.level_id
        pipeline += f" → L{rs.level_id}({len(rs.zhongshus)}zs/{len(rs.moves)}mv)"
    print(f"Pipeline: {pipeline}")
    print(f"最大递归深度: {max_level}")
    print(f"{'='*60}")


if __name__ == "__main__":
    run()
