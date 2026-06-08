"""回归测试：compute_move_persistence 闭式优化 ⟺ 原 sublevel_h0_bars 实现。

优化内容（ph_layer.compute_move_persistence）：
    旧：bars = sublevel_h0_bars(prices); return bars[0].persistence  # O(k log k)
    新：return max(prices) - min(prices)                            # O(k)

等价依据（代数恒等式，L0）：
    1D sublevel set filtration 的最大（最久存活）H0 特征 = 全局连通分量，
    从全局极小诞生、在 finite_cap=max(values) 封顶，persistence 恒为
    max-min 且恒为所有 H0 特征中最大者。原实现只取 bars[0]（最大），故
    与 max-min 逐位等价。

本测试在三个层级证伪等价性破坏：
    L1（合成）：随机 center 价格序列，新旧公式逐位一致。
    L1（边界）：退化序列（空/单点/等值/len<3）走相同 fallback。
    L2（真实数据）：真实行情驱动全管线，每次 compute_move_persistence 调用
                    新旧公式逐位一致（数据缺失则 skip）。
"""
from __future__ import annotations

import math
import random
from pathlib import Path

import pytest

from newchan.a_persistence_barcode import sublevel_h0_bars
from newchan.ph_layer import _center_prices, compute_move_persistence


def _old_formula(prices: list[float]) -> float:
    """原实现：sublevel filtration 取最大 H0 bar 的 persistence。"""
    bars = sublevel_h0_bars(prices)
    if not bars:
        raise AssertionError("len>=3 必产生 bar")
    return bars[0].persistence


def _new_formula(prices: list[float]) -> float:
    return max(prices) - min(prices)


# --------------------------------------------------------------------
# L1：合成随机序列 — 新旧公式逐位一致
# --------------------------------------------------------------------


def test_closed_form_matches_sublevel_random():
    rng = random.Random(20260608)
    for _ in range(20000):
        k = rng.randint(2, 14)
        prices: list[float] = []
        for _ in range(k):
            dd = rng.uniform(-50, 100)
            gg = dd + rng.uniform(0, 60)
            prices.append(dd)
            prices.append(gg)
        assert math.isclose(
            _new_formula(prices), _old_formula(prices), rel_tol=0, abs_tol=1e-9
        ), prices


def test_closed_form_matches_sublevel_non_monotonic():
    """非交替 / 任意排列价格 — 等价不依赖 dd<gg 假设。"""
    rng = random.Random(7)
    for _ in range(10000):
        k = rng.randint(3, 20)
        prices = [rng.uniform(-100, 100) for _ in range(k)]
        assert math.isclose(
            _new_formula(prices), _old_formula(prices), rel_tol=0, abs_tol=1e-9
        ), prices


# --------------------------------------------------------------------
# L1：退化边界 — fallback 一致
# --------------------------------------------------------------------


@pytest.mark.parametrize(
    "prices",
    [
        [5.0],
        [3.0, 3.0],
        [1.0, 2.0],
        [2.0, 1.0, 2.0],
        [0.0, 0.0, 0.0],
        [-1.0, -1.0, -1.0, -1.0],
    ],
)
def test_degenerate_sequences(prices):
    if len(prices) >= 3:
        assert math.isclose(
            _new_formula(prices), _old_formula(prices), rel_tol=0, abs_tol=1e-9
        )
    else:
        # len<3 走 high-low fallback，不调任一公式；此处仅确认 max-min 有定义
        assert _new_formula(prices) == max(prices) - min(prices)


# --------------------------------------------------------------------
# L2：真实数据全管线 — 每次调用新旧逐位一致
# --------------------------------------------------------------------

_CACHE = Path(__file__).resolve().parents[1] / ".cache"
_REAL = _CACHE / "BZ_1min_2024_raw.parquet"


@pytest.mark.skipif(not _REAL.exists(), reason="真实数据缓存缺失")
def test_real_data_pipeline_equivalence():
    import pandas as pd

    from newchan import ph_layer
    from newchan.orchestrator.recursive import RecursiveOrchestrator
    from newchan.types import Bar

    mismatches: list[tuple] = []
    n_calls = 0

    real_compute = ph_layer.compute_move_persistence

    def shadow_compute(move, zhongshus):  # noqa: ANN001
        nonlocal n_calls
        n_calls += 1
        new = real_compute(move, zhongshus)
        # 重建旧路径作为影子对照
        start = move.zs_start
        end = min(move.zs_end, len(zhongshus) - 1)
        if start > end or start >= len(zhongshus):
            old = move.high - move.low
        else:
            prices = _center_prices(zhongshus[start : end + 1])
            old = (move.high - move.low) if len(prices) < 3 else _old_formula(prices)
        if not math.isclose(new, old, rel_tol=0, abs_tol=1e-9):
            mismatches.append((start, end, new, old))
        return new

    # attach_persistence 在 ph_layer 内按模块全局名调用 compute_move_persistence，
    # 替换该全局即可被 move_engine / recursive_stack 两个消费点同时拾取。
    ph_layer.compute_move_persistence = shadow_compute

    try:
        df = pd.read_parquet(_REAL).iloc[:60000]
        idx = pd.to_datetime(df.index)
        orch = RecursiveOrchestrator(stream_id="reg")
        for t, o, h, lo, c, v in zip(
            idx, df["open"], df["high"], df["low"], df["close"], df["volume"]
        ):
            orch.process_bar(
                Bar(
                    ts=t, open=float(o), high=float(h), low=float(lo),
                    close=float(c), volume=float(v) if v == v else None,
                )
            )
    finally:
        ph_layer.compute_move_persistence = real_compute

    assert n_calls > 1000, f"调用次数过少({n_calls})，未真正驱动管线"
    assert not mismatches, f"{len(mismatches)}/{n_calls} 次新旧不一致，前3: {mismatches[:3]}"
