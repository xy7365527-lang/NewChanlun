"""诊断：为什么 nested_divergence_search 在日线5年数据上返回空？

检查：
1. max_levels=4 vs 6 的递归层实际形成情况
2. 各级别的 moves/zhongshus/divergences 数量
3. nested_divergence_search 的触发条件
"""
from __future__ import annotations

import json
import logging
import sys
from datetime import datetime
from pathlib import Path

_project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_project_root / "src"))

from newchan.a_nested_divergence import (
    NestedDivergence,
    divergences_from_level_snapshot,
    nested_divergence_search,
    _get_moves_at_level,
)
from newchan.convergence import convergence_tightness
from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(message)s")
logger = logging.getLogger("diagnostic")

DATA_DIR = _project_root / "data" / "scanner_l2"

# Pick a few diverse symbols
DIAG_SYMBOLS = {
    "000002": "万科A",      # proxy=1.80 (highest proxy)
    "600036": "招商银行",    # proxy=1.80
    "600547": "山东黄金",    # proxy=0.80
    "601398": "工商银行",    # proxy=0.30
}


def _load_bars(symbol: str) -> list[Bar]:
    path = DATA_DIR / f"{symbol}_daily.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    bars = [
        Bar(
            ts=datetime.fromisoformat(d["ts"]),
            open=d["open"],
            high=d["high"],
            low=d["low"],
            close=d["close"],
            volume=d["volume"],
        )
        for d in data
    ]
    bars.sort(key=lambda b: b.ts)
    return bars


def diagnose_symbol(symbol: str, name: str) -> None:
    bars = _load_bars(symbol)
    logger.info("=== %s (%s) — %d bars, max_levels=6 ===", symbol, name, len(bars))

    orch = RecursiveOrchestrator(stream_id=symbol, max_levels=6)

    # Track max state across all bars
    max_recursive_levels = 0
    max_level_moves: dict[int, int] = {}  # level -> max_moves_count
    max_level_zhongshus: dict[int, int] = {}
    total_nds_found = 0
    best_nd: NestedDivergence | None = None
    best_nd_score = 0.0
    any_level2_divs = 0

    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)

        n_rec = len(snap.recursive_snapshots)
        if n_rec > max_recursive_levels:
            max_recursive_levels = n_rec

        # Level 1 stats
        n_moves_l1 = len(snap.move_snapshot.moves)
        n_zs_l1 = len(snap.zs_snapshot.zhongshus)
        max_level_moves[1] = max(max_level_moves.get(1, 0), n_moves_l1)
        max_level_zhongshus[1] = max(max_level_zhongshus.get(1, 0), n_zs_l1)

        # Recursive levels
        for rs in snap.recursive_snapshots:
            lid = rs.level_id
            max_level_moves[lid] = max(max_level_moves.get(lid, 0), len(rs.moves))
            max_level_zhongshus[lid] = max(max_level_zhongshus.get(lid, 0), len(rs.zhongshus))

            # Check if level2+ has divergences
            components = _get_moves_at_level(lid - 1, snap)
            settled_components = [m for m in components if m.settled]
            if settled_components:
                divs = divergences_from_level_snapshot(rs, settled_components)
                if divs:
                    any_level2_divs += len(divs)

        # Check nested divergence search
        nds = nested_divergence_search(snap)
        if nds:
            total_nds_found += len(nds)
            for nd in nds:
                ct = convergence_tightness(nd)
                if ct.score > best_nd_score:
                    best_nd_score = ct.score
                    best_nd = nd

    print(f"\n--- {symbol} ({name}) ---")
    print(f"  bars: {len(bars)}")
    print(f"  max recursive levels formed: {max_recursive_levels}")
    for lid in sorted(max_level_moves.keys()):
        print(f"  Level {lid}: max_moves={max_level_moves.get(lid, 0)}, "
              f"max_zhongshus={max_level_zhongshus.get(lid, 0)}")
    print(f"  Level 2+ divergences found (total across all bars): {any_level2_divs}")
    print(f"  Nested divergence chains found (total across all bars): {total_nds_found}")
    if best_nd:
        ct = convergence_tightness(best_nd)
        print(f"  Best T(S): {ct.score} (D={ct.depth}, L={ct.level_max}, C={ct.consistency})")
        print(f"  Best chain: {[(lvl, div.direction if div else None) for lvl, div in best_nd.chain]}")
    else:
        print(f"  Best T(S): 0 (no nested divergence found)")


def main() -> None:
    print("=" * 70)
    print("Tightness Diagnostic: Why nested_divergence_search returns empty?")
    print("=" * 70)

    for symbol, name in DIAG_SYMBOLS.items():
        diagnose_symbol(symbol, name)

    print("\n" + "=" * 70)
    print("诊断完成")
    print("=" * 70)


if __name__ == "__main__":
    main()
