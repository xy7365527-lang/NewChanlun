"""#1208 ①件 turn_class 列接线最小解析测试。

面（与 Rust parity 测试分工）：本文件只锁 Python 列——`OrganicTape.from_columns`
的 `turn_class` 参数解析与 fail-fast；不传 = None = 零行为变化（capability guard，
Rust 侧 `SignalTape.turn_class_rows`）。行语义/判据对拍在 Rust 侧
（`turn_class_production_bridge_parity_xzd_positive`）。

磁带组装点 `analysis/organic_fugue_rust_check.py::pack_tape` 的 `turn_class`
收集器透传（与 dir_flips/trend_flips 同构的调用约定）已接线：pack_tape 依赖
analysis 全链（numpy/matplotlib），不在本最小测试内 import——列解析契约以
`from_columns` 直接对拍。

用法：PYTHONPATH=src .venv/bin/python -m pytest tests/test_turn_class_column.py -q
"""

from __future__ import annotations

import pytest

import newchan_rust

# fugue_version_i.MAX_LADDER（磁带 ladder 上限，from_columns 校验同款）。
MAX_LADDER = 11
N_BARS = 5


def _columns(n: int = N_BARS) -> dict:
    """最小列集：n 根无事件 bar（布尔行全零、无 bsp/div 事件、无 D3 行）。"""
    return dict(
        closes=[100.0 + i for i in range(n)],
        buy1=[0] * n,
        sell1=[0] * n,
        sell_any=[0] * n,
        buy_any=[0] * n,
        up_settled=[0] * n,
        max_ladder=[2] * n,
        type2_buy=[False] * n,
        bsp_flat=[],
        div_flat=[],
    )


def _build(turn_class=None) -> newchan_rust.OrganicTape:
    cols = _columns()
    return newchan_rust.OrganicTape.from_columns(
        cols["closes"], cols["buy1"], cols["sell1"], cols["sell_any"],
        cols["buy_any"], cols["up_settled"], cols["max_ladder"],
        cols["type2_buy"], cols["bsp_flat"], cols["div_flat"],
        turn_class=turn_class,
    )


def _xzd_row(bar: int, third_src: int, second: int | None, extreme: int) -> tuple:
    return (bar, 2, "XiaozhuandaCandidate", (third_src, second, extreme))


def test_turn_class_column_absent_none_zero_change():
    # 不传 ⟹ None ⟹ 零行为变化：磁带照常构造，bar 数正确。
    tape = _build()
    assert tape.n_bars() == N_BARS
    tape2 = _build(turn_class=None)
    assert tape2.n_bars() == N_BARS


def test_turn_class_column_parses_minimal_rows():
    # 行 bar 升序：NestedConfirmed（无 evidence）+ XiaozhuandaCandidate
    # （evidence=(third_src, second_class, turn_extreme) 三元组）。
    rows = [
        (0, 2, "NestedConfirmed", None),
        _xzd_row(1, 3, None, 200),
        (2, 3, "ExecEvidenceOnly", None),
        (3, 4, "DeferOrphan", None),
        _xzd_row(4, 3, 4, 200),
    ]
    tape = _build(turn_class=rows)
    assert tape.n_bars() == N_BARS


def test_turn_class_column_rejects_disorder():
    rows = [_xzd_row(3, 3, None, 200), _xzd_row(1, 3, None, 200)]
    with pytest.raises(ValueError, match="turn_class"):
        _build(turn_class=rows)


def test_turn_class_column_rejects_bad_class():
    with pytest.raises(ValueError, match="turn_class"):
        _build(turn_class=[(0, 2, "NotAClass", None)])


def test_turn_class_column_rejects_out_of_range():
    # bar 越界（≥n）。
    with pytest.raises(ValueError, match="turn_class"):
        _build(turn_class=[(N_BARS, 2, "NestedConfirmed", None)])
    # ladder 越界（≥MAX_LADDER）。
    with pytest.raises(ValueError, match="turn_class"):
        _build(turn_class=[(0, MAX_LADDER, "NestedConfirmed", None)])
    # evidence third_src 越界。
    with pytest.raises(ValueError, match="turn_class"):
        _build(turn_class=[_xzd_row(0, N_BARS, None, 200)])
    # evidence second_class 越界。
    with pytest.raises(ValueError, match="turn_class"):
        _build(turn_class=[_xzd_row(0, 1, N_BARS, 200)])
