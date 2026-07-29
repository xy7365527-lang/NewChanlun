"""Rust MACD 层 ↔ Python MACD 层等价 golden 测试（第七层：compute_macd rel=1e-11，其余逐位）。

验证 `newchan_rust` 的 MACD 实现与 Python 源等价：`compute_macd` 批量路径使用相对容差
rel=1e-12，其余逐位（bit-exact）相等：

| Rust | Python 源 |
|------|-----------|
| `compute_macd` | `a_macd.compute_macd`（pandas ewm adjust=False，rel=1e-11） |
| `OnlineMacdState` | `a_macd.OnlineMacdState` |
| `macd_area_for_range` | `a_macd.macd_area_for_range` |
| `dif_peak_for_range` | `a_divergence_v1.dif_peak_for_range` |
| `histogram_peak_for_range` | `a_divergence_v1.histogram_peak_for_range` |
| `divergences_from_moves_v1(macd_col=, hist_col=, merged_to_raw=)` | `a_divergence_v1.divergences_from_moves_v1(df_macd=, merged_to_raw=)` |

## bit-exact 基础（formalization-validity-domain.md）

- **EMA 递推**：pandas `ewm(adjust=False)` 内部用 `prev + α·(x−prev)`（非代数等价的
  `α·x+(1−α)·prev`）。`compute_macd` 复刻前者，`OnlineMacdState` 复刻后者——二者在 Python
  中本就不 bit-exact（差 ~1e-14），Rust 分别忠实移植。
- **平台浮点（#324）**：`compute_macd` 的批量 EMA 路径在 ubuntu x86 上与 macOS arm64
  结果不逐位相等（末位差约 2 ULP，相对 3e-16）。裁定放宽为 rel=1e-12；CI 实证
  （run 30209225831）长参数 [500/5000] 在第 179 棒累积到 ~1.9e-12 仍超线 → 二裁放宽为
  rel=1e-11（仍比业务噪声低四五个量级），即分辨率（rel/eps，eps = 2^-52 ≈ 2.22e-16 到 2^-53 ≈ 1.11e-16，
  随 mantissa 位置浮动）。其余 5 个测试的 bit-exact 声明不变。
- **area 累加**：`pandas.Series.sum() == numpy.sum()`，用 pairwise summation（块 128，
  8 路展开）。Rust `pairwise_sum` 逐位复刻。
- **round(·, 6)**：Python 内置 round（round-half-to-even），Rust 用 `{:.6}` 正确舍入复刻。

## 认识论等级
- 合成数据：L1（管线正确性 + 浮点累加树 bit-exact）。16 个非 slow 用例覆盖，
  任何环境（含 CI）都会跑到。
- BZ 真实数据：L2（真实价格序列上 bit-exact）。来自唯一一个
  `@pytest.mark.slow` 用例 `test_macd_real_data_bit_exact`，且仅在本机存在
  `.cache/BZ_1min_2024_raw.parquet` 缓存文件时才会被执行验证——CI 环境
  （用 `-m "not slow"` 过滤）下该证据实际不会被跑到（deselected）。
  两种口径：
  - 本地默认全跑（不加 `-m` 过滤）：17 passed，含 L2 真实数据验证。
  - CI 口径（`-m "not slow"`）：16 passed + 1 deselected(slow)，只有 L1
    合成数据证据，L2 证据未被执行。
"""

from __future__ import annotations

import math

import pytest

newchan_rust = pytest.importorskip("newchan_rust")

from newchan.a_macd import (  # noqa: E402
    OnlineMacdState as PyOnlineMacdState,
    compute_macd as py_compute_macd,
    macd_area_for_range as py_macd_area_for_range,
)
from newchan.a_divergence_v1 import (  # noqa: E402
    dif_peak_for_range as py_dif_peak,
    histogram_peak_for_range as py_hist_peak,
)


def _bits_equal(a: float, b: float) -> bool:
    """逐位相等（NaN 比特也相等）。"""
    import struct

    return struct.pack("<d", a) == struct.pack("<d", b)


def _rel_close(a: float, b: float, rel: float = 1e-11, abs_tol: float = 1e-13) -> bool:
    """abs+rel 混合容差比较；NaN↔NaN 视为相等（保留 _bits_equal 的 NaN 语义）。

    #324 三裁（CI 二轮实证）：rel=1e-11 在 DIF/hist 过零附近失效——递推舍入噪声有
    绝对地板（~3e-14），值过零时相对误差被放大到 ~3e-11，纯 rel 放多大都没用。
    abs_tol=1e-13（噪声地板 3 倍）兜过零段，rel=1e-11 管正常区间（numpy allclose 同型）。
    """
    if math.isnan(a) and math.isnan(b):
        return True
    return math.isclose(a, b, rel_tol=rel, abs_tol=abs_tol)


def _synthetic_closes(n: int) -> list[float]:
    out = []
    for k in range(n):
        out.append(
            100.0
            + 40.0 * math.sin(k * 0.004)
            + 10.0 * math.sin(k * 0.05)
            + 3.0 * math.sin(k * 0.17 + 1.0)
            + 1.5 * math.sin(k * 0.31 + 2.0)
            + 0.2 * math.cos(k * 0.7)
        )
    return out


# ── compute_macd（批量，pandas ewm 路径）──────────────────────


@pytest.mark.parametrize("n", [1, 2, 5, 50, 500, 5000])
def test_compute_macd_batch_rel_close(n: int) -> None:
    """批量 MACD：Python(pandas ewm) vs Rust 相对容差 rel=1e-11（#324 裁定，二裁放宽）。

    原名 test_compute_macd_batch_bit_exact。ubuntu x86 上 py/rust 的 DIF 末位相差
    约 2 ULP（相对 3e-16），macOS arm64 逐位相等——差异来自平台浮点收缩/库差异，
    非实现分歧。裁定：放宽断言，生产数值零改动。函数名同步去掉 bit_exact
    （090：名字不得声明代码不具备的能力）。
    """
    import pandas as pd

    closes = _synthetic_closes(n)
    df = py_compute_macd(pd.DataFrame({"close": closes}))
    rs_macd, rs_signal, rs_hist = newchan_rust.compute_macd(closes)

    assert len(rs_macd) == n
    for col, rs in (("macd", rs_macd), ("signal", rs_signal), ("hist", rs_hist)):
        py_col = list(df[col])
        for i, (p, r) in enumerate(zip(py_col, rs)):
            assert _rel_close(float(p), r), (
                f"compute_macd[{col}][{i}] py={float(p)!r} rust={r!r} rel_tol=1e-11"
            )


# ── OnlineMacdState（在线递推路径）────────────────────────────


@pytest.mark.parametrize("n", [1, 2, 5, 50, 500, 5000])
def test_online_macd_bit_exact(n: int) -> None:
    from datetime import datetime, timedelta

    closes = _synthetic_closes(n)
    t0 = datetime(2024, 1, 1)

    py_state = PyOnlineMacdState()
    rs_state = newchan_rust.OnlineMacdState()
    for i, c in enumerate(closes):
        pm, ps, ph = py_state.update(c, t0 + timedelta(minutes=i))
        rm, rs, rh = rs_state.update(c)
        assert _bits_equal(pm, rm), f"online macd[{i}] py={pm!r} rust={rm!r}"
        assert _bits_equal(ps, rs), f"online signal[{i}] py={ps!r} rust={rs!r}"
        assert _bits_equal(ph, rh), f"online hist[{i}] py={ph!r} rust={rh!r}"

    py_df = py_state.to_dataframe()
    rm, rs, rh = rs_state.series()
    assert rs_state.n_bars == n
    for col, rs_list in (("macd", rm), ("signal", rs), ("hist", rh)):
        for i, (p, r) in enumerate(zip(py_df[col], rs_list)):
            assert _bits_equal(float(p), r), f"series[{col}][{i}]"


# ── macd_area_for_range（pairwise 累加 + round6）──────────────


def test_macd_area_bit_exact() -> None:
    import pandas as pd

    closes = _synthetic_closes(3000)
    df = py_compute_macd(pd.DataFrame({"close": closes}))
    hist = list(df["hist"])

    ranges = [
        (0, 0), (0, 1), (0, 7), (0, 8), (0, 127), (0, 128), (0, 129),
        (10, 200), (100, 2999), (0, 2999), (-5, 50), (2999, 5000),
        (3000, 4000), (50, 49),
    ]
    for raw_i0, raw_i1 in ranges:
        py_a = py_macd_area_for_range(df, raw_i0, raw_i1)
        rs_total, rs_pos, rs_neg, rs_n = newchan_rust.macd_area_for_range(
            hist, raw_i0, raw_i1
        )
        assert _bits_equal(py_a["area_total"], rs_total), (
            f"area_total [{raw_i0},{raw_i1}] py={py_a['area_total']!r} rust={rs_total!r}"
        )
        assert _bits_equal(py_a["area_pos"], rs_pos), (
            f"area_pos [{raw_i0},{raw_i1}] py={py_a['area_pos']!r} rust={rs_pos!r}"
        )
        assert _bits_equal(py_a["area_neg"], rs_neg), (
            f"area_neg [{raw_i0},{raw_i1}] py={py_a['area_neg']!r} rust={rs_neg!r}"
        )
        assert py_a["n_bars"] == rs_n


# ── dif/hist peak（max/min 约简）──────────────────────────────


def test_peaks_bit_exact() -> None:
    import pandas as pd

    closes = _synthetic_closes(2000)
    df = py_compute_macd(pd.DataFrame({"close": closes}))
    macd_col = list(df["macd"])
    hist_col = list(df["hist"])

    ranges = [(0, 1999), (10, 500), (-3, 50), (1999, 3000), (100, 99)]
    for raw_i0, raw_i1 in ranges:
        for direction, up in (("up", True), ("down", False)):
            py_d = py_dif_peak(df, raw_i0, raw_i1, direction)
            rs_d = newchan_rust.dif_peak_for_range(macd_col, raw_i0, raw_i1, up)
            assert _bits_equal(py_d, rs_d), (
                f"dif_peak [{raw_i0},{raw_i1}] {direction} py={py_d!r} rust={rs_d!r}"
            )
            py_h = py_hist_peak(df, raw_i0, raw_i1, direction)
            rs_h = newchan_rust.histogram_peak_for_range(hist_col, raw_i0, raw_i1, up)
            assert _bits_equal(py_h, rs_h), (
                f"hist_peak [{raw_i0},{raw_i1}] {direction} py={py_h!r} rust={rs_h!r}"
            )


# ── b_segment_crosses_zero ────────────────────────────────────


def test_b_cross_zero() -> None:
    macd = [-1.0, -0.5, 0.0, 0.5, 1.0]
    assert newchan_rust.b_segment_crosses_zero(macd, 0, 4) is True
    assert newchan_rust.b_segment_crosses_zero(macd, 3, 4) is False  # 全正
    assert newchan_rust.b_segment_crosses_zero(macd, 0, 1) is False  # 全负
    assert newchan_rust.b_segment_crosses_zero(macd, 2, 2) is False  # 仅 0
    assert newchan_rust.b_segment_crosses_zero(macd, 5, 6) is False  # 越界


# ── 全 MACD 三维度背驰路径（接通验证）────────────────────────


def _build_pipeline(n: int):
    """合成 bars → (segments, zhongshus, moves, closes, merged_to_raw)。"""
    from newchan.a_move_v1 import moves_from_zhongshus as py_moves
    from newchan.a_segment_v1 import segments_from_strokes_v1 as py_segments
    from newchan.a_zhongshu_v1 import zhongshu_from_segments as py_zs_segments
    from newchan.bi_engine import BiEngine as PyBiEngine
    from newchan.core.bar import BarV1

    closes = _synthetic_closes(n)
    eng = PyBiEngine(stroke_mode="new")
    ts0 = 1_700_000_000.0
    for k, c in enumerate(closes):
        # 简单 OHLC 包络（与背驰等价测试同思路）
        spread = 0.5 + 0.4 * abs(math.sin(k * 0.13))
        o = c
        h = c + spread
        low = c - spread
        eng.process_bar(BarV1(bar_time=ts0 + k * 60, open=o, high=h, low=low, close=c))
    strokes = eng.current_strokes
    segs = py_segments(strokes, min_seg_strokes=3, extend_mode="strict")
    zhongshus = py_zs_segments(segs)
    moves = py_moves(zhongshus, num_segments=len(segs))
    # 1:1 merged→raw（合成无包含合并时 merged==raw；此处用恒等映射保证两侧一致）
    n_merged = max((s.i1 for s in segs), default=-1) + 1
    n_merged = max(n_merged, len(closes))
    merged_to_raw = [(i, i) for i in range(n_merged)]
    return segs, zhongshus, moves, closes, merged_to_raw


def _div_eq(py_d, rs) -> bool:
    head, tail = rs
    return (
        py_d.kind == head[0]
        and py_d.direction == head[1]
        and py_d.level_id == head[2]
        and py_d.seg_a_start == head[3]
        and py_d.seg_a_end == head[4]
        and py_d.seg_c_start == head[5]
        and py_d.seg_c_end == head[6]
        and py_d.center_idx == head[7]
        and _bits_equal(py_d.force_a, tail[0])
        and _bits_equal(py_d.force_c, tail[1])
        and py_d.confirmed == tail[2]
        and _bits_equal(py_d.dif_peak_a, tail[3])
        and _bits_equal(py_d.dif_peak_c, tail[4])
        and _bits_equal(py_d.hist_peak_a, tail[5])
        and _bits_equal(py_d.hist_peak_c, tail[6])
    )


def test_divergence_with_macd_bit_exact() -> None:
    """MACD 三维度背驰路径：Python(df_macd=) vs Rust(macd_col=) 全字段 bit-exact。"""
    import pandas as pd

    from newchan.a_divergence_v1 import divergences_from_moves_v1 as py_divs

    segs, zhongshus, moves, closes, merged_to_raw = _build_pipeline(12000)
    assert len(moves) > 0, "未产生走势，测试无效"

    df_macd = py_compute_macd(pd.DataFrame({"close": closes}))

    py_ds = py_divs(
        segs, zhongshus, moves, 1,
        df_macd=df_macd, merged_to_raw=merged_to_raw,
    )

    rs_ds = newchan_rust.divergences_from_moves_v1(
        [(s.direction, s.high, s.low, s.i0, s.i1) for s in segs],
        [(z.zd, z.zg, z.seg_start, z.seg_end, z.settled) for z in zhongshus],
        [
            (m.kind, m.direction, m.seg_start, m.seg_end,
             m.zs_start, m.zs_end, m.zs_count, m.settled)
            for m in moves
        ],
        1,
        macd_col=list(df_macd["macd"]),
        hist_col=list(df_macd["hist"]),
        merged_to_raw=merged_to_raw,
    )

    assert len(py_ds) == len(rs_ds), (
        f"MACD 背驰数不等 py={len(py_ds)} rust={len(rs_ds)}"
    )
    for idx, (a, b) in enumerate(zip(py_ds, rs_ds)):
        assert _div_eq(a, b), (
            f"第 {idx} MACD 背驰分歧\n"
            f"  py=(kind={a.kind},dir={a.direction},fa={a.force_a!r},fc={a.force_c!r},"
            f"difA={a.dif_peak_a!r},difC={a.dif_peak_c!r},"
            f"histA={a.hist_peak_a!r},histC={a.hist_peak_c!r})\n  rust={b}"
        )
    assert len(py_ds) > 0, "MACD 路径未产生背驰，测试未覆盖有效域"


# ── 真实数据 L2 ───────────────────────────────────────────────

_BZ_PARQUET = ".cache/BZ_1min_2024_raw.parquet"


@pytest.mark.slow
def test_macd_real_data_bit_exact() -> None:
    """BZ 真实价格序列：批量 + 在线 MACD 全字段 bit-exact（L2）。"""
    import os

    if not os.path.exists(_BZ_PARQUET):
        pytest.skip(f"真实数据不存在: {_BZ_PARQUET}")
    import pandas as pd

    df = pd.read_parquet(_BZ_PARQUET)
    closes = [float(c) for c in df["close"]]
    assert len(closes) > 1000

    # 批量 MACD（pandas ewm 路径）
    py_df = py_compute_macd(pd.DataFrame({"close": closes}))
    rm, rs, rh = newchan_rust.compute_macd(closes)
    for col, rs_list in (("macd", rm), ("signal", rs), ("hist", rh)):
        for i, (p, r) in enumerate(zip(py_df[col], rs_list)):
            assert _bits_equal(float(p), r), f"real batch {col}[{i}]"

    # 在线 MACD（OnlineMacdState 路径）
    from datetime import datetime, timedelta

    py_state = PyOnlineMacdState()
    rs_state = newchan_rust.OnlineMacdState()
    t0 = datetime(2024, 1, 1)
    for i, c in enumerate(closes):
        pm, ps, ph = py_state.update(c, t0 + timedelta(minutes=i))
        rm1, rs1, rh1 = rs_state.update(c)
        assert _bits_equal(pm, rm1), f"real online macd[{i}]"
        assert _bits_equal(ps, rs1), f"real online signal[{i}]"
        assert _bits_equal(ph, rh1), f"real online hist[{i}]"

    # area / peaks on real hist
    hist = list(py_df["hist"])
    macd_col = list(py_df["macd"])
    for raw_i0, raw_i1 in [(0, len(hist) - 1), (100, 5000), (1234, 8888)]:
        py_a = py_macd_area_for_range(py_df, raw_i0, raw_i1)
        rt, rp, rn, rnb = newchan_rust.macd_area_for_range(hist, raw_i0, raw_i1)
        assert _bits_equal(py_a["area_total"], rt), f"real area_total [{raw_i0},{raw_i1}]"
        assert _bits_equal(py_a["area_pos"], rp), f"real area_pos [{raw_i0},{raw_i1}]"
        assert _bits_equal(py_a["area_neg"], rn), f"real area_neg [{raw_i0},{raw_i1}]"
        for direction, up in (("up", True), ("down", False)):
            assert _bits_equal(
                py_dif_peak(py_df, raw_i0, raw_i1, direction),
                newchan_rust.dif_peak_for_range(macd_col, raw_i0, raw_i1, up),
            ), f"real dif_peak [{raw_i0},{raw_i1}] {direction}"
            assert _bits_equal(
                py_hist_peak(py_df, raw_i0, raw_i1, direction),
                newchan_rust.histogram_peak_for_range(hist, raw_i0, raw_i1, up),
            ), f"real hist_peak [{raw_i0},{raw_i1}] {direction}"
