"""#1048 consolidated 正本产出步测试锁——离线合成，不碰网络、不读 key、不依赖真实数据。

锁四类行为：
1. 单分区可复现：raw → consolidate → 正本分区（行数折叠 + schema 保留 + 幂等重跑 + 确定性）；
2. 消歧键标定：双键读数 + 推荐规则（participant 折叠同笔多报 / 平手条款 / 缺样本不定稿）；
3. raw → transient：只移动「正本已产出且过行数校验」的分区，不删除；
4. 键列校验：缺列/空值 fail-loud；无数据时 plan/calibrate/consolidate 明确报错不编数。

不依赖真实 Massive 数据（沙盒无 raw staging、无 MASSIVE_API_KEY，见 issue #1048 评论）。
"""

from __future__ import annotations

from pathlib import Path

import pandas as pd
import pytest

import consolidate_massive_tick as cmt
from newchan.sip_consolidate import KEY_PARTICIPANT, KEY_SIP_SEQ


# ---------------------------------------------------------------------------
# fixture：9 行原样数据，两笔成交 × 3 主所 + TRF + 撤单 + 修正条件码
# ---------------------------------------------------------------------------


def _raw_df() -> pd.DataFrame:
    """9 行原样数据，数字可逐条核。

    行 → 命运：
      t1a/t1b/t1c  成交 1 经 3 个主所打印（participant_timestamp 同=1001）→ 留
      t2a/t2b/t2c  成交 2 经 3 个主所打印（participant_timestamp 同=1002）→ 留
      trf          exchange=4 且 trf_id=201（TRF/暗池打印）               → venue 丢
      corr         correction=8（撤单原始）                               → filter 丢
      cond         conditions 含 46（Correction）                         → filter 丢
    合计 9 → filter 后 7 → venue 后 6 → participant 键 dedup 后 2；sip_seq 键 dedup 后 6。
    """
    n = 9
    df = pd.DataFrame(
        {
            "id": ["t1a", "t1b", "t1c", "t2a", "t2b", "t2c", "trf", "corr", "cond"],
            "ticker": ["COST"] * n,
            "conditions": [[0], [0], [0], [0], [0], [0], [0], [0], [46]],
            "exchange": [1, 2, 3, 1, 2, 3, 4, 1, 1],
            "participant_timestamp": [1001, 1001, 1001, 1002, 1002, 1002, 1001, 1003, 1004],
            "sip_timestamp": [2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009],
            "trf_timestamp": pd.array([None] * n, dtype="Int64"),
            "price": [150.0, 150.0, 150.0, 150.5, 150.5, 150.5, 150.0, 150.6, 150.7],
            "sequence_number": [1, 2, 3, 4, 5, 6, 7, 8, 9],
            "size": [100] * n,
            "tape": [1] * n,
            "trf_id": pd.array([None, None, None, None, None, None, 201, None, None], dtype="Int64"),
            "decimal_size": [100.0] * n,
            "correction": [0] * n,
        }
    )
    df.loc[7, "correction"] = 8
    return df


def _raw_path(tmp_path: Path, ticker: str = "COST", date: str = "2024-01-02") -> Path:
    raw = tmp_path / "massive_tick"
    cmt.write_partition(raw, ticker, date, _raw_df())
    return raw / ticker / f"dt={date}" / "trades.parquet"


# ---------------------------------------------------------------------------
# 分区扫描 / 路径 / 断点续跑
# ---------------------------------------------------------------------------


def test_collect_partitions_and_paths(tmp_path):
    cmt.write_partition(tmp_path, "COST", "2024-01-02", pd.DataFrame({"id": ["a"]}))
    cmt.write_partition(tmp_path, "COST", "2024-01-03", pd.DataFrame({"id": ["b"]}))
    cmt.write_partition(tmp_path, "AAPL", "2024-01-02", pd.DataFrame({"id": ["c"]}))
    found = cmt.collect_partitions(tmp_path)
    assert list(found) == ["AAPL", "COST"]
    assert [d for d, _ in found["COST"]] == ["2024-01-02", "2024-01-03"]
    assert cmt.consolidated_path(tmp_path, "COST", "2024-01-02") == (
        tmp_path / "COST" / "dt=2024-01-02" / "trades.parquet"
    )


def test_partition_done_true_empty_and_corrupt(tmp_path):
    p = cmt.write_partition(tmp_path, "COST", "2024-01-02", pd.DataFrame({"id": ["a"]}))
    assert cmt.partition_done(p) is True
    empty = tmp_path / "AAPL" / "dt=2024-01-02" / "trades.parquet"
    empty.parent.mkdir(parents=True)
    empty.write_bytes(b"not a parquet")
    assert cmt.partition_done(empty) is False
    assert cmt.partition_done(tmp_path / "MSFT" / "dt=2024-01-02" / "trades.parquet") is False


# ---------------------------------------------------------------------------
# 键列校验
# ---------------------------------------------------------------------------


def test_validate_key_columns_missing_fails_loud():
    df = _raw_df().drop(columns=["sequence_number"])
    with pytest.raises(ValueError, match="缺失"):
        cmt._validate_key_columns(df, KEY_SIP_SEQ)


def test_validate_key_columns_nan_fails_loud():
    df = pd.DataFrame({"sip_timestamp": [1.0, float("nan")], "sequence_number": [1, 2]})
    with pytest.raises(ValueError, match="空值"):
        cmt._validate_key_columns(df, KEY_SIP_SEQ)


# ---------------------------------------------------------------------------
# 单分区可复现（raw → consolidate → 正本分区）
# ---------------------------------------------------------------------------


def test_consolidate_partition_participant_collapses_venue_dups(tmp_path):
    raw_path = _raw_path(tmp_path)
    out_root = tmp_path / "consolidated"
    stats = cmt.consolidate_partition(raw_path, out_root, "COST", "2024-01-02", KEY_PARTICIPANT)
    assert stats["consolidate_in"] == 9
    assert stats["consolidate_out"] == 2
    out_path = cmt.consolidated_path(out_root, "COST", "2024-01-02")
    assert cmt.partition_done(out_path)
    out = pd.read_parquet(out_path)
    # 列序/列集与 raw 一致（纯行过滤，不删字段）
    raw = pd.read_parquet(raw_path)
    assert list(out.columns) == list(raw.columns)
    assert set(out["id"]) == {"t1a", "t2a"}  # keep first per participant_timestamp


def test_consolidate_partition_sip_seq_keeps_venue_dups(tmp_path):
    raw_path = _raw_path(tmp_path)
    out_root = tmp_path / "consolidated"
    stats = cmt.consolidate_partition(raw_path, out_root, "COST", "2024-01-02", KEY_SIP_SEQ)
    assert stats["consolidate_out"] == 6  # (sip,seq) 不折叠主所多报


def test_consolidate_all_end_to_end_reproducible(tmp_path):
    raw = tmp_path / "massive_tick"
    out = tmp_path / "consolidated"
    cmt.write_partition(raw, "COST", "2024-01-02", _raw_df())
    t1 = cmt.consolidate_all(raw, out, ["COST"], KEY_PARTICIPANT)
    assert t1["consolidated"] == 1 and t1["skipped"] == 0
    assert t1["out_rows"] == 2
    out_path = cmt.consolidated_path(out, "COST", "2024-01-02")
    first = pd.read_parquet(out_path)
    # 幂等重跑：已有正本分区 → 跳过
    t2 = cmt.consolidate_all(raw, out, ["COST"], KEY_PARTICIPANT)
    assert t2["consolidated"] == 0 and t2["skipped"] == 1
    # 删除正本后重跑 → 行集逐位一致（确定性）
    out_path.unlink()
    t3 = cmt.consolidate_all(raw, out, ["COST"], KEY_PARTICIPANT)
    assert t3["consolidated"] == 1
    second = pd.read_parquet(out_path)
    assert list(first["id"]) == list(second["id"])
    assert list(first["participant_timestamp"]) == list(second["participant_timestamp"])
    assert first.shape == second.shape


def test_write_partition_empty_returns_none(tmp_path):
    assert cmt.write_partition(tmp_path, "COST", "2024-01-02", pd.DataFrame()) is None


# ---------------------------------------------------------------------------
# 消歧键标定：双键读数 + 推荐规则
# ---------------------------------------------------------------------------


def _post_filter_venue_df() -> pd.DataFrame:
    """filter + venue 之后的帧（6 行：两笔成交 × 3 主所，无 TRF/撤单/修正）。"""
    from newchan.sip_consolidate import drop_trf_prints, filter_trades

    f, _ = filter_trades(_raw_df())
    v, _ = drop_trf_prints(f)
    return v


def test_key_readings_participant_collapses_sip_seq_does_not():
    df = _post_filter_venue_df()
    r = cmt.key_readings(df, KEY_PARTICIPANT)
    assert r["n_rows"] == 6 and r["n_unique_keys"] == 2
    assert r["dup_rate"] == round(1.0 - 2 / 6, 6)
    assert r["price_agree_rate"] == 1.0  # 组内价格一致（同笔多报同价）
    r2 = cmt.key_readings(df, KEY_SIP_SEQ)
    assert r2["n_rows"] == 6 and r2["n_unique_keys"] == 6
    assert r2["dup_rate"] == 0.0
    assert r2["price_agree_rate"] == 1.0  # 每组成单行，平凡一致


def test_recommend_key_picks_participant_when_readings_differ():
    df = _post_filter_venue_df()
    summary = cmt._summarize_readings([{
        "dup_by_key": [cmt.key_readings(df, KEY_SIP_SEQ), cmt.key_readings(df, KEY_PARTICIPANT)],
    }])
    rec, reason = cmt.recommend_key(summary)
    assert tuple(rec) == KEY_PARTICIPANT
    assert "重复率" in reason


def test_recommend_key_tie_goes_sip_seq():
    summary = {
        "sip_seq": {"n_rows": 10, "n_unique_keys": 10, "dup_rate": 0.0,
                    "price_agree_groups": 10, "price_agree_rate": 1.0},
        "participant": {"n_rows": 10, "n_unique_keys": 10, "dup_rate": 0.0,
                        "price_agree_groups": 10, "price_agree_rate": 1.0},
    }
    rec, reason = cmt.recommend_key(summary)
    assert tuple(rec) == KEY_SIP_SEQ


def test_recommend_key_none_when_both_merge_different_prices():
    summary = {
        "sip_seq": {"n_rows": 10, "n_unique_keys": 5, "dup_rate": 0.5,
                    "price_agree_groups": 2, "price_agree_rate": 0.4},
        "participant": {"n_rows": 10, "n_unique_keys": 5, "dup_rate": 0.5,
                        "price_agree_groups": 2, "price_agree_rate": 0.4},
    }
    rec, _ = cmt.recommend_key(summary)
    assert rec is None


def test_recommend_key_none_when_no_samples():
    rec, _ = cmt.recommend_key({})
    assert rec is None


def test_calibrate_no_data_errors(tmp_path):
    report = cmt.calibrate(tmp_path / "none", ["COST"], 5)
    assert "error" in report
    assert "records" not in report


def test_calibrate_skips_malformed_partition(tmp_path):
    # 分区缺消歧键列（sequence_number）⟹ 跳过该分区、不拖垮整轮标定（与
    # consolidate_all 逐分区失败跳过同口径）。
    raw = tmp_path / "raw"
    cmt.write_partition(raw, "COST", "2024-01-02", _raw_df().drop(columns=["sequence_number"]))
    report = cmt.calibrate(raw, ["COST"], 5)
    assert "error" not in report
    assert report["n_partitions"] == 0
    assert report["recommended_key"] is None
    assert report["summary"]["sip_seq"]["n_rows"] == 0


# ---------------------------------------------------------------------------
# raw → transient（只移动已产出正本且过行数校验的分区）
# ---------------------------------------------------------------------------


def test_retire_raw_moves_verified_keeps_unverified(tmp_path):
    raw = tmp_path / "massive_tick"
    out = tmp_path / "consolidated"
    transient = tmp_path / "transient"
    cmt.write_partition(raw, "COST", "2024-01-02", _raw_df())
    raw_path = raw / "COST" / "dt=2024-01-02" / "trades.parquet"
    # 无正本 → 校验未过，保留 raw
    totals = cmt.retire_raw(raw, out, transient, ["COST"])
    assert totals["moved"] == 0 and totals["skipped_verify"] == 1
    assert raw_path.exists()
    # 产出正本 → retire 移动 raw 到 transient（不删除）
    cmt.consolidate_partition(raw_path, out, "COST", "2024-01-02", KEY_PARTICIPANT)
    totals = cmt.retire_raw(raw, out, transient, ["COST"])
    assert totals["moved"] == 1
    assert not raw_path.exists()
    assert (transient / "COST" / "dt=2024-01-02" / "trades.parquet").exists()


def test_verify_partition_rejects_out_larger_than_raw(tmp_path):
    raw = tmp_path / "raw"
    out = tmp_path / "out"
    cmt.write_partition(raw, "COST", "2024-01-02", _raw_df())
    cmt.write_partition(out, "COST", "2024-01-02", pd.concat([_raw_df(), _raw_df()]))
    raw_path = raw / "COST" / "dt=2024-01-02" / "trades.parquet"
    out_path = out / "COST" / "dt=2024-01-02" / "trades.parquet"
    ok, reason = cmt.verify_partition(raw_path, out_path)
    assert ok is False and "行数校验失败" in reason


# ---------------------------------------------------------------------------
# 无数据时 CLI 明确报错（不编数）
# ---------------------------------------------------------------------------


def test_main_plan_no_data(capsys):
    rc = cmt.main(["--plan", "--symbols", "COST", "--root", "/nonexistent/raw"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "COST" in out and "0 天" in out


def test_main_consolidate_no_data(tmp_path):
    rc = cmt.main(["--symbols", "COST", "--root", str(tmp_path / "none"), "--key", "sip_seq"])
    assert rc == 1


def test_main_consolidate_requires_key(tmp_path):
    # 产出正本不指定 --key → 拒绝替操作者选键（fail-loud）
    with pytest.raises(SystemExit):
        cmt.main(["--symbols", "COST", "--root", str(tmp_path / "none")])


def test_main_calibrate_no_data(tmp_path):
    rc = cmt.main(["--calibrate-key", "--symbols", "COST", "--root", str(tmp_path / "none")])
    assert rc == 1


def test_main_mutually_exclusive_modes(tmp_path):
    with pytest.raises(SystemExit):
        cmt.main(["--plan", "--retire-raw", "--root", str(tmp_path)])
