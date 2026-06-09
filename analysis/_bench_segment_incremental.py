"""性能基准：线段层增量续算 vs 全量重算（主链 O(N²) 瓶颈修复量化）。

测量两项：
  1. orchestrator-only 时序（新增量路径，生产成本）——跑全 447K bars。
  2. 旧路径线段层单次成本——在不同 stroke 规模快照上计 full
     `segments_from_strokes_v1` 单次耗时，× 实测线段变化次数 ⟹ 旧总成本下界。

旧路径每次笔尾变化调 full（O(strokes)），变化次数 ~O(B)，故旧总 ~O(B·S)；
新路径每次 O(tail)。终点 stroke 规模越大，单次差距越大——3M+ 后明显变慢的根因。
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

import newchan_rust  # noqa: E402

DATA = ROOT / "analysis" / "data_cache" / "oklo_1m_databento.json"
_MAX_LEVELS = 6


def load():
    raw = json.loads(DATA.read_text())
    bars = raw["bars"]
    return (
        [float(b["open"]) for b in bars],
        [float(b["high"]) for b in bars],
        [float(b["low"]) for b in bars],
        [float(b["close"]) for b in bars],
    )


def main() -> None:
    opens, highs, lows, closes = load()
    n = len(opens)
    print(f"OKLO {n} bars")

    # ── 1. orchestrator-only（新增量路径，生产全链）──
    orch = newchan_rust.RecursiveOrchestrator(max_levels=_MAX_LEVELS)
    t0 = time.time()
    for i in range(n):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
    t_inc = time.time() - t0
    print(f"\n[新] orchestrator 全链 447K（增量线段层）: {t_inc:.1f}s  ({n / t_inc:,.0f} bar/s)")
    print(f"     最终段数={len(orch.current_segments())}  strokes={orch.stroke_count()}")

    # ── 2. 旧路径线段层单次成本（不同 stroke 规模）──
    # 重放至各阶段，取笔快照，计 full segments_from_strokes_v1 单次耗时。
    print("\n[旧] full segments_from_strokes_v1 单次耗时 vs stroke 规模：")
    probe = newchan_rust.RecursiveOrchestrator(max_levels=_MAX_LEVELS)
    targets = [int(n * f) for f in (0.25, 0.5, 0.75, 1.0)]
    ti = 0
    per_call = []
    for cut in targets:
        while ti < cut:
            probe.process_bar(opens[ti], highs[ti], lows[ti], closes[ti])
            ti += 1
        strokes = probe.current_strokes()
        # 计 full 单次（多跑取最小，排除噪声）
        best = min(_timed_full(strokes) for _ in range(3))
        per_call.append((len(strokes), best))
        print(f"   strokes={len(strokes):>6}  full 单次={best * 1000:.1f}ms")

    # 旧总成本下界估计：用平均单次 × 实测线段变化次数（来自验证：52593）。
    n_changes = 52593
    avg_call = sum(c for _, c in per_call) / len(per_call)
    old_est = avg_call * n_changes
    print(
        f"\n旧路径线段层估算：{n_changes} 次笔尾变化 × 平均 {avg_call * 1000:.1f}ms/次 "
        f"≈ {old_est:.0f}s（下界，尾部单次成本更高）"
    )
    print(f"新路径 orchestrator 全链总计：{t_inc:.1f}s")
    print(f"加速比（线段层估算 / 全链实测）≈ {old_est / t_inc:.1f}×（仅线段层下界）")


def _timed_full(strokes) -> float:
    t0 = time.time()
    newchan_rust.segments_from_strokes_v1(strokes, 3, "strict")
    return time.time() - t0


if __name__ == "__main__":
    main()
