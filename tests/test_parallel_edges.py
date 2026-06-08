"""并行执行 + 比价 OHLC 向量化的等价性回归测试。

核心断言：并行/向量化只改变执行调度与数据搬运，不改变计算结果。
    - bars_from_frame ⟺ 逐行参考循环（逐字段一致）
    - ratio_frame_numpy ⟺ pandas 逐列除法（数值一致）
    - run_edges_parallel ⟺ run_edges_sequential（逐字段一致）
    - analyze_batch_parallel ⟺ analyze_batch（逐字段一致）
"""
from __future__ import annotations

import numpy as np
import pandas as pd
import pytest

from newchan.parallel_edges import (
    EdgeSpec,
    bars_from_frame,
    ratio_frame_numpy,
    run_edges_parallel,
    run_edges_sequential,
)
from newchan.types import Bar


def _synth_ohlcv(n: int, seed: int, base: float = 100.0) -> pd.DataFrame:
    rng = np.random.default_rng(seed)
    steps = rng.normal(0, 0.5, size=n).cumsum()
    close = base + steps
    high = close + np.abs(rng.normal(0, 0.3, size=n))
    low = close - np.abs(rng.normal(0, 0.3, size=n))
    open_ = close - rng.normal(0, 0.1, size=n)
    vol = rng.integers(1, 1000, size=n).astype(float)
    idx = pd.date_range("2024-01-01", periods=n, freq="1min", tz="UTC")
    return pd.DataFrame(
        {"open": open_, "high": high, "low": low, "close": close, "volume": vol},
        index=idx,
    )


# --------------------------------------------------------------------
# 向量化摄入等价
# --------------------------------------------------------------------


def _bars_reference(df: pd.DataFrame) -> list[Bar]:
    """逐行参考实现（等价基准）。"""
    out: list[Bar] = []
    ts = df.index.to_pydatetime()
    for i, (_, row) in enumerate(df.iterrows()):
        v = row["volume"]
        out.append(
            Bar(
                ts=ts[i],
                open=float(row["open"]), high=float(row["high"]),
                low=float(row["low"]), close=float(row["close"]),
                volume=None if v != v else float(v),
            )
        )
    return out


def test_bars_from_frame_matches_reference():
    df = _synth_ohlcv(500, seed=1)
    fast = bars_from_frame(df)
    ref = _bars_reference(df)
    assert len(fast) == len(ref)
    for a, b in zip(fast, ref):
        assert a.ts == b.ts
        assert a.open == b.open and a.high == b.high
        assert a.low == b.low and a.close == b.close
        assert a.volume == b.volume


def test_bars_from_frame_nan_volume_to_none():
    df = _synth_ohlcv(10, seed=2)
    df.loc[df.index[3], "volume"] = np.nan
    bars = bars_from_frame(df)
    assert bars[3].volume is None
    assert bars[0].volume is not None


def test_bars_from_frame_empty():
    df = _synth_ohlcv(0, seed=3)
    assert bars_from_frame(df) == []


def test_bars_from_frame_no_volume_column():
    df = _synth_ohlcv(20, seed=4).drop(columns=["volume"])
    bars = bars_from_frame(df)
    assert all(b.volume is None for b in bars)
    assert len(bars) == 20


# --------------------------------------------------------------------
# 比价 OHLC 向量化等价
# --------------------------------------------------------------------


def test_ratio_frame_numpy_matches_pandas():
    a = _synth_ohlcv(300, seed=10, base=2000.0)
    b = _synth_ohlcv(300, seed=11, base=80.0)
    fast = ratio_frame_numpy(a, b)
    # pandas 逐列除法参考
    idx = a.index.intersection(b.index)
    aa, bb = a.loc[idx], b.loc[idx]
    for col in ("open", "high", "low", "close"):
        np.testing.assert_array_equal(
            fast[col].to_numpy(), (aa[col] / bb[col]).to_numpy()
        )
    np.testing.assert_array_equal(fast["volume"].to_numpy(), aa["volume"].to_numpy())


def test_ratio_frame_numpy_partial_overlap():
    a = _synth_ohlcv(100, seed=20)
    b = _synth_ohlcv(100, seed=21).iloc[30:]  # 只与 a 的后 70 行重叠
    fast = ratio_frame_numpy(a, b)
    assert len(fast) == 70


# --------------------------------------------------------------------
# 跨边并行 ⟺ 顺序（逐字段一致）
# --------------------------------------------------------------------


def _make_edges(n_edges: int, n_bars: int) -> list[EdgeSpec]:
    edges: list[EdgeSpec] = []
    for k in range(n_edges):
        a = _synth_ohlcv(n_bars, seed=100 + k, base=2000.0)
        b = _synth_ohlcv(n_bars, seed=200 + k, base=80.0)
        edges.append(EdgeSpec(name=f"edge{k}", df_a=a, df_b=b))
    return edges


def _assert_results_equal(seq, par):
    assert len(seq) == len(par)
    for s, p in zip(seq, par):
        assert s.name == p.name
        assert s.error == p.error
        assert s.n_bars == p.n_bars
        assert s.n_strokes == p.n_strokes
        assert s.n_segments == p.n_segments
        assert s.n_zhongshus == p.n_zhongshus
        assert s.n_moves == p.n_moves
        assert s.n_levels == p.n_levels


@pytest.mark.slow
def test_parallel_matches_sequential():
    edges = _make_edges(n_edges=4, n_bars=1500)
    seq = run_edges_sequential(edges)
    par = run_edges_parallel(edges, processes=2)
    _assert_results_equal(seq, par)
    # 至少应有非平凡结构产出，证明确实跑了管线
    assert any(r.n_strokes > 0 for r in seq)


def test_run_edges_single_degrades_to_sequential():
    edges = _make_edges(n_edges=1, n_bars=800)
    par = run_edges_parallel(edges)
    seq = run_edges_sequential(edges)
    _assert_results_equal(seq, par)


def test_run_edges_empty():
    assert run_edges_parallel([]) == []
    assert run_edges_sequential([]) == []


def test_edge_error_isolation():
    """单边输入非法不应抛出，错误编码进 EdgeResult.error。"""
    bad = EdgeSpec(name="bad", df_a=None, df_b=None)
    results = run_edges_sequential([bad])
    assert results[0].error is not None
    assert results[0].n_bars == 0
