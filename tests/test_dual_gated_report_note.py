"""#1315 第二轮 driver 报告的「无门读数 bar」记档行（`_no_reading_note`）行为锁。

这条记档行是 §3 门 sanity 表的口径附注：占比表的分子里混着口径差重建臂填的
fail-closed 值，扣除量只能从这行读。故三种形态各锁一把：

  1. 有非 0 ⇒ 逐标点名根数；
  2. 全 0 且全部标的都有该读数 ⇒ 可以断言"门列与驱动 bar 集逐根一致"；
  3. 有标的缺该读数（归档 JSON 出自 #1315 之前的跑批、resume 未重算）⇒ **不得**并进
     "全为 0"（缺键不是 0），须单独点名——`m1_e_futures_dual_gated_backtest` 的 resume
     正是"只重算 DX、另 6 标的吃旧档"的常规路径，这一形态是常态而非边角。

driver 模块顶部 import 信号层（`fugue_alpha_diagnosis` → `newchan`），沙盒缺件时
importorskip 跳过（不静默 skip：pytest 会打出原因）。
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

M = pytest.importorskip("m1_e_futures_dual_gated_backtest")


def _results(**per_symbol) -> dict:
    """{sym: no_reading_bars 值或 None（None = 该键缺失，模拟旧档）} → results 骨架。"""
    out = {}
    for sym, k in per_symbol.items():
        rates = {"n_bars": 100}
        if k is not None:
            rates["no_reading_bars"] = k
        out[sym] = {"gate_sanity": {"gate_open_rates": rates}}
    return out


def test_note_names_symbols_with_no_reading_bars():
    res = _results(ES=0, DX=2)
    note = M._no_reading_note(res, ["ES", "DX"])
    assert "DX 2 根" in note
    assert "ES" not in note.split("。")[0]  # 0 根的不点名
    assert "占比读数须按此扣除" in note


def test_note_asserts_all_zero_only_when_every_symbol_has_the_reading():
    res = _results(ES=0, GC=0, DX=0)
    note = M._no_reading_note(res, ["ES", "GC", "DX"])
    assert "3 标的全为 0（门列与驱动 bar 集逐根一致）" in note
    assert "无此读数" not in note


def test_note_does_not_count_a_missing_reading_as_zero():
    """旧档（resume 未重算）缺 `no_reading_bars` ⇒ 单独点名，不并进"全为 0"的断言。"""
    res = _results(ES=None, GC=None, DX=0)
    note = M._no_reading_note(res, ["ES", "GC", "DX"])
    assert "ES、GC" in note and "无此读数" in note
    assert "3 标的全为 0" not in note
    assert "门列与驱动 bar 集逐根一致" not in note


def test_note_reports_hits_and_missing_readings_together():
    res = _results(ES=None, DX=2)
    note = M._no_reading_note(res, ["ES", "DX"])
    assert "DX 2 根" in note
    assert "ES 的归档 JSON 无此读数" in note
