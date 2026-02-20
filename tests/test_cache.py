"""Tests for newchan.cache — Parquet 本地缓存模块。"""

from __future__ import annotations

from pathlib import Path

import pandas as pd
import pytest

from newchan import cache


# ── fixtures ──


@pytest.fixture()
def cache_dir(tmp_path, monkeypatch):
    """将 CACHE_DIR 指向临时目录，避免污染真实缓存。"""
    monkeypatch.setattr(cache, "_cache_dir", lambda: tmp_path)
    return tmp_path


def _make_df(values: list[float], index_start: str = "2024-01-01") -> pd.DataFrame:
    """构造简单的测试 DataFrame。"""
    idx = pd.date_range(index_start, periods=len(values), freq="min")
    return pd.DataFrame({"close": values}, index=idx)


# ── save_df / load_df ──


class TestSaveLoadDf:
    def test_save_and_load_roundtrip(self, cache_dir):
        df = _make_df([1.0, 2.0, 3.0])
        path = cache.save_df("test_data", df)

        assert path.exists()
        assert path.suffix == ".parquet"

        loaded = cache.load_df("test_data")
        assert loaded is not None
        # parquet 不保留 DatetimeIndex.freq，用 check_freq=False
        pd.testing.assert_frame_equal(loaded, df, check_freq=False)

    def test_load_nonexistent_returns_none(self, cache_dir):
        result = cache.load_df("does_not_exist")
        assert result is None

    def test_save_overwrites_existing(self, cache_dir):
        df1 = _make_df([1.0, 2.0])
        df2 = _make_df([10.0, 20.0, 30.0])

        cache.save_df("overwrite_test", df1)
        cache.save_df("overwrite_test", df2)

        loaded = cache.load_df("overwrite_test")
        assert loaded is not None
        assert len(loaded) == 3
        pd.testing.assert_frame_equal(loaded, df2, check_freq=False)


# ── _normalize_tz ──


class TestNormalizeTz:
    def test_tz_aware_becomes_naive(self):
        idx = pd.date_range("2024-01-01", periods=3, freq="min", tz="US/Eastern")
        df = pd.DataFrame({"v": [1, 2, 3]}, index=idx)

        result = cache._normalize_tz(df)
        assert result.index.tz is None

    def test_tz_naive_unchanged(self):
        idx = pd.date_range("2024-01-01", periods=3, freq="min")
        df = pd.DataFrame({"v": [1, 2, 3]}, index=idx)

        result = cache._normalize_tz(df)
        assert result.index.tz is None
        pd.testing.assert_frame_equal(result, df)

    def test_does_not_mutate_original(self):
        idx = pd.date_range("2024-01-01", periods=3, freq="min", tz="UTC")
        df = pd.DataFrame({"v": [1, 2, 3]}, index=idx)

        cache._normalize_tz(df)
        assert df.index.tz is not None  # original unchanged


# ── append_df ──


class TestAppendDf:
    def test_append_to_nonexistent_creates_file(self, cache_dir):
        df = _make_df([1.0, 2.0])
        path = cache.append_df("new_cache", df)

        assert path.exists()
        loaded = cache.load_df("new_cache")
        assert loaded is not None
        assert len(loaded) == 2

    def test_append_merges_and_deduplicates(self, cache_dir):
        # 第一批: 3 条
        df1 = _make_df([1.0, 2.0, 3.0], "2024-01-01 00:00")
        cache.save_df("merge_test", df1)

        # 第二批: 重叠 1 条 + 新增 2 条
        idx2 = pd.date_range("2024-01-01 00:02", periods=3, freq="min")
        df2 = pd.DataFrame({"close": [30.0, 4.0, 5.0]}, index=idx2)

        cache.append_df("merge_test", df2)
        loaded = cache.load_df("merge_test")

        assert loaded is not None
        assert len(loaded) == 5  # 3 old + 2 new (1 deduped)
        # 重叠行保留最新值 (30.0 而非 3.0)
        assert loaded.iloc[2]["close"] == 30.0

    def test_append_sorts_by_index(self, cache_dir):
        # 乱序插入
        idx = pd.to_datetime(["2024-01-03", "2024-01-01", "2024-01-02"])
        df = pd.DataFrame({"close": [3.0, 1.0, 2.0]}, index=idx)

        cache.append_df("sort_test", df)
        loaded = cache.load_df("sort_test")

        assert loaded is not None
        assert list(loaded["close"]) == [1.0, 2.0, 3.0]

    def test_append_normalizes_tz(self, cache_dir):
        idx = pd.date_range("2024-01-01", periods=2, freq="min", tz="UTC")
        df = pd.DataFrame({"close": [1.0, 2.0]}, index=idx)

        cache.append_df("tz_test", df)
        loaded = cache.load_df("tz_test")

        assert loaded is not None
        assert loaded.index.tz is None


# ── list_cached ──


class TestListCached:
    def test_empty_dir(self, cache_dir):
        assert cache.list_cached() == []

    def test_lists_standard_files(self, cache_dir):
        df = _make_df([1.0])
        cache.save_df("CL_1min_raw", df)
        cache.save_df("GC_5min_raw", df)

        result = cache.list_cached()
        assert len(result) == 2

        symbols = {r["symbol"] for r in result}
        assert symbols == {"CL", "GC"}

        intervals = {r["interval"] for r in result}
        assert intervals == {"1min", "5min"}

    def test_composite_symbol(self, cache_dir):
        """合成品种名含下划线时应正确解析。"""
        df = _make_df([1.0])
        cache.save_df("CL_GC_spread_1min_raw", df)

        result = cache.list_cached()
        assert len(result) == 1
        assert result[0]["symbol"] == "CL_GC_spread"
        assert result[0]["interval"] == "1min"

    def test_ignores_non_matching_files(self, cache_dir):
        # 不符合命名规范的文件应被忽略
        df = _make_df([1.0])
        df.to_parquet(cache_dir / "random_file.parquet")

        assert cache.list_cached() == []
