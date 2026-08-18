"""#1041 修正规则测试锁——SIP consolidated 去重/过滤（consolidated vs 原样，行数差 = 重复率）。

只测确定性逻辑（合成 fixture），不依赖 Massive 真实数据。真实数据读数由
`scripts/probe_sip_dedup.py` 负责；数据落地后的对拍见 `test_consolidate_real_data_skips_without_data`。
"""

from __future__ import annotations

import math
from pathlib import Path

import pandas as pd
import pytest

from newchan.sip_consolidate import (
    DROP_CONDITIONS,
    DROP_CORRECTIONS,
    KEY_PARTICIPANT,
    KEY_SIP_SEQ,
    conditions_series,
    consolidate,
    dedup_by_key,
    drop_trf_prints,
    filter_trades,
    normalize_conditions,
)

# ---------------------------------------------------------------------------
# fixture：12 行，三个判据各司其职，数字可逐条核
# ---------------------------------------------------------------------------


def _make_fixture() -> pd.DataFrame:
    """12 行合成原样数据。

    行号 → 命运：
      A/B/C/D   干净主所打印，键互异            → 全留（4）
      E         与 A 同 (sip,seq) 键             → dedup 丢（1）
      F/G       trf_id 非空（TRF/暗池打印）      → venue 丢（2）
      H         correction=8（撤单原始）         → filter 丢
      I         conditions 含 46（Correction）   → filter 丢
      J         correction=11（错误记录）         → filter 丢
      K         correction=NaN（无修正）           → 留，键唯一（并入 A-D 的「干净组」）
      L         与 B 同 participant 键            → dedup 丢（participant 键下）
    合计 12 → filter 后 9 → venue 后 7 → sip_seq 键 dedup 后 6；participant 键 dedup 后 6。
    """
    rows = [
        # id, ticker, conditions, correction, exchange, pt, sip, price, seq, size, tape, trf_id
        ("A", "AAPL", [0], 0, 1, 1001, 2001, 150.0, 1, 100, 1, None),
        ("B", "AAPL", [12], 0, 3, 1002, 2002, 150.1, 2, 200, 3, None),
        ("C", "AAPL", [0, 37], 0, 2, 1003, 2003, 150.2, 3, 50, 1, None),
        ("D", "AAPL", [], 0, 1, 1004, 2004, 150.3, 4, 75, 1, None),
        ("E", "AAPL", [0], 0, 1, 1001, 2001, 150.0, 1, 100, 1, None),  # dup of A
        ("F", "AAPL", [0], 0, 4, 1005, 2005, 150.0, 5, 100, 3, 201),   # TRF
        ("G", "AAPL", [0], 0, 4, 1006, 2006, 150.1, 6, 200, 3, 202),   # TRF
        ("H", "AAPL", [0], 8, 1, 1007, 2007, 150.4, 7, 10, 1, None),   # cancel
        ("I", "AAPL", [46], 0, 1, 1008, 2008, 150.5, 8, 10, 1, None),  # correction cond
        ("J", "AAPL", [0], 11, 1, 1009, 2009, 150.6, 9, 10, 1, None),  # error
        ("K", "AAPL", [0], None, 1, 1010, 2010, 150.7, 10, 10, 1, None),
        ("L", "AAPL", [0], 0, 1, 1002, 2011, 150.1, 11, 200, 1, None),  # dup pt of B
    ]
    cols = [
        "id",
        "ticker",
        "conditions",
        "correction",
        "exchange",
        "participant_timestamp",
        "sip_timestamp",
        "price",
        "sequence_number",
        "size",
        "tape",
        "trf_id",
    ]
    return pd.DataFrame(rows, columns=cols)


# ---------------------------------------------------------------------------
# normalize_conditions
# ---------------------------------------------------------------------------


def test_normalize_conditions_all_shapes():
    import numpy as np

    assert normalize_conditions(None) == frozenset()
    assert normalize_conditions(float("nan")) == frozenset()
    assert normalize_conditions(12) == frozenset({12})
    assert normalize_conditions("12,37") == frozenset({12, 37})
    assert normalize_conditions(" 12 ") == frozenset({12})
    assert normalize_conditions("") == frozenset()
    assert normalize_conditions([12, 37]) == frozenset({12, 37})
    assert normalize_conditions((0, "12")) == frozenset({0, 12})
    # parquet 读回后是 numpy 标量：np.int64 整数、np.float64 NaN 均须归一。
    assert normalize_conditions(np.int64(12)) == frozenset({12})
    assert normalize_conditions(np.float64("nan")) == frozenset()
    assert normalize_conditions([np.int64(0), np.int64(37)]) == frozenset({0, 37})
    with pytest.raises(ValueError):
        normalize_conditions("not-an-int")


# ---------------------------------------------------------------------------
# 判据一：条件/修正过滤
# ---------------------------------------------------------------------------


def test_filter_drops_corrections_and_conditions():
    df = _make_fixture()
    out, stats = filter_trades(df)
    # 丢 H(corr=8)、I(cond=46)、J(corr=11)，其余 9 行留。
    assert len(out) == 9
    assert stats["filter_in"] == 12
    assert stats["filter_out"] == 9
    assert stats["dropped_correction"] == 2  # H + J
    assert stats["dropped_condition"] == 1  # I
    kept_ids = set(out["id"])
    assert kept_ids == {"A", "B", "C", "D", "E", "F", "G", "K", "L"}


def test_filter_keeps_null_correction():
    df = _make_fixture()
    out, _ = filter_trades(df)
    assert "K" in set(out["id"]), "correction=NaN 是『无修正』，须保留"


# ---------------------------------------------------------------------------
# 判据二：主所判定
# ---------------------------------------------------------------------------


def test_drop_trf_prints_drops_nonnull_trf_id():
    df = _make_fixture()
    filtered, _ = filter_trades(df)
    out, stats = drop_trf_prints(filtered)
    assert len(out) == 7  # 9 - F - G
    assert stats["dropped_trf"] == 2
    kept_ids = set(out["id"])
    assert "F" not in kept_ids and "G" not in kept_ids


def test_drop_trf_prints_without_column_is_noop():
    df = pd.DataFrame({"id": ["x"], "price": [1.0]})
    out, stats = drop_trf_prints(df)
    assert len(out) == 1 and stats["dropped_trf"] == 0


# ---------------------------------------------------------------------------
# 判据三：同笔多报去重
# ---------------------------------------------------------------------------


def test_dedup_by_sip_seq_keeps_first_per_key():
    df = _make_fixture()
    filtered, _ = filter_trades(df)
    venue, _ = drop_trf_prints(filtered)
    out, stats = dedup_by_key(venue, KEY_SIP_SEQ)
    # A/E 同键 (2001,1) → 留 A；其余键互异。
    assert stats["dedup_in"] == 7 and stats["dedup_out"] == 6
    assert stats["dedup_dropped"] == 1
    assert "E" not in set(out["id"])


def test_dedup_by_participant_collapses_same_pt():
    df = _make_fixture()
    filtered, _ = filter_trades(df)
    venue, _ = drop_trf_prints(filtered)
    out, stats = dedup_by_key(venue, KEY_PARTICIPANT)
    # B/L 同 participant_timestamp=1002 → 留 B；A/E 同 pt=1001 → 留 A。
    assert stats["dedup_in"] == 7 and stats["dedup_out"] == 5
    assert "L" not in set(out["id"]) and "E" not in set(out["id"])


def test_dedup_missing_key_column_fails_loud():
    df = _make_fixture().drop(columns=["sequence_number"])
    with pytest.raises(ValueError):
        dedup_by_key(df, KEY_SIP_SEQ)


# ---------------------------------------------------------------------------
# 端到端：修正规则测试锁（consolidated vs 原样，行数差 = 重复率读数）
# ---------------------------------------------------------------------------


def test_consolidate_sip_seq_row_diff_equals_duplicate_rate():
    df = _make_fixture()
    out, stats = consolidate(df, KEY_SIP_SEQ)
    assert len(out) == 6
    assert stats["consolidate_in"] == 12
    assert stats["consolidate_out"] == 6
    expected_rate = 1.0 - 6 / 12
    assert math.isclose(stats["duplicate_rate"], expected_rate)
    assert stats["duplicate_rate"] == 0.5


def test_consolidate_participant_key():
    df = _make_fixture()
    out, stats = consolidate(df, KEY_PARTICIPANT)
    assert len(out) == 5
    assert math.isclose(stats["duplicate_rate"], 1.0 - 5 / 12)


def test_consolidate_no_dual_track():
    """#799：判据单一——consolidate 只走一个键，键列缺失即炸，不换键兜底。"""
    df = _make_fixture().drop(columns=["sequence_number"])
    with pytest.raises(ValueError):
        consolidate(df, KEY_SIP_SEQ)


# ---------------------------------------------------------------------------
# 真实数据对拍（数据未落地时显式 skip，不静默、不编数）
# ---------------------------------------------------------------------------


def test_consolidate_real_data_skips_without_data():
    root = Path("analysis/data_cache/massive_tick")
    if not root.exists() or not any(root.glob("*/dt=*/trades.parquet")):
        pytest.skip("Massive 原样数据未落地（#1040），真实对拍读数待数据后跑")
    # 数据落地后：对最近一个分区跑 consolidate，断言行数差 = 重复率读数（非空）。
    paths = sorted(root.glob("*/dt=*/trades.parquet"))
    df = pd.read_parquet(paths[-1])
    out, stats = consolidate(df, KEY_SIP_SEQ)
    assert len(out) > 0
    assert 0.0 <= stats["duplicate_rate"] <= 1.0
