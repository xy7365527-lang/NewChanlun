"""Tests for a_macd module — compute_macd + compute_log_macd."""

from __future__ import annotations

import numpy as np
import pandas as pd
import pytest

from newchan.a_macd import compute_log_macd, compute_macd, macd_area_for_range


# ── compute_macd ──────────────────────────────────────────────


class TestComputeMacd:
    def test_output_columns(self):
        df = pd.DataFrame({"close": np.random.default_rng(0).random(50) + 1})
        result = compute_macd(df)
        assert list(result.columns) == ["macd", "signal", "hist"]
        assert len(result) == len(df)

    def test_hist_equals_macd_minus_signal(self):
        df = pd.DataFrame({"close": np.random.default_rng(1).random(100) + 1})
        result = compute_macd(df)
        np.testing.assert_allclose(
            result["hist"].values,
            (result["macd"] - result["signal"]).values,
            atol=1e-15,
        )


# ── compute_log_macd ─────────────────────────────────────────


class TestComputeLogMacd:
    def test_output_columns(self):
        df = pd.DataFrame({"close": np.random.default_rng(0).random(50) + 1})
        result = compute_log_macd(df)
        assert list(result.columns) == ["macd", "signal", "hist"]
        assert len(result) == len(df)

    def test_hist_equals_macd_minus_signal(self):
        df = pd.DataFrame({"close": np.random.default_rng(1).random(100) + 1})
        result = compute_log_macd(df)
        np.testing.assert_allclose(
            result["hist"].values,
            (result["macd"] - result["signal"]).values,
            atol=1e-15,
        )

    def test_log_macd_additivity(self):
        """log(A/B) + log(B/C) = log(A/C) => hist 可加。"""
        n = 200
        rng = np.random.default_rng(42)
        price_a = np.cumprod(1 + rng.normal(0, 0.01, n))
        price_b = np.cumprod(1 + rng.normal(0, 0.01, n))
        price_c = np.cumprod(1 + rng.normal(0, 0.01, n))
        df_ab = pd.DataFrame({"close": price_a / price_b})
        df_bc = pd.DataFrame({"close": price_b / price_c})
        df_ac = pd.DataFrame({"close": price_a / price_c})
        macd_ab = compute_log_macd(df_ab)["hist"]
        macd_bc = compute_log_macd(df_bc)["hist"]
        macd_ac = compute_log_macd(df_ac)["hist"]
        # 跳过前50根（EMA 初始化效应）
        np.testing.assert_allclose(
            macd_ac.iloc[50:].values,
            (macd_ab + macd_bc).iloc[50:].values,
            rtol=1e-10,
        )

    def test_single_price_matches_regular_macd_shape(self):
        """对单一价格序列，log_macd 和 macd 输出形状一致。"""
        df = pd.DataFrame({"close": np.random.default_rng(7).random(60) + 1})
        r1 = compute_macd(df)
        r2 = compute_log_macd(df)
        assert r1.shape == r2.shape


# ── macd_area_for_range ──────────────────────────────────────


class TestMacdAreaForRange:
    def test_basic_area(self):
        df = pd.DataFrame({"close": np.arange(1.0, 51.0)})
        macd_df = compute_macd(df)
        result = macd_area_for_range(macd_df, 0, 49)
        assert result["n_bars"] == 50
        assert result["area_total"] == pytest.approx(
            result["area_pos"] + result["area_neg"], abs=1e-6
        )

    def test_empty_range(self):
        df = pd.DataFrame({"close": np.arange(1.0, 11.0)})
        macd_df = compute_macd(df)
        result = macd_area_for_range(macd_df, 5, 3)
        assert result["n_bars"] == 0
        assert result["area_total"] == 0.0

    def test_clamp_bounds(self):
        df = pd.DataFrame({"close": np.arange(1.0, 11.0)})
        macd_df = compute_macd(df)
        result = macd_area_for_range(macd_df, -5, 100)
        assert result["n_bars"] == 10


# ── OnlineMacdState 等价性 ────────────────────────────────────────


class TestOnlineMacdState:
    def test_equivalence_with_compute_macd(self):
        """OnlineMacdState 逐根更新结果 ≈ compute_macd 批量结果（容差等价，非 bit-exact）。

        两路径浮点末位偏差 ~1e-13（朴素递推 vs pandas ewm 的 FMA 收缩），
        故用 atol=1e-10 容差断言而非 exact。详见 a_macd.OnlineMacdState docstring。
        """
        from datetime import datetime, timezone
        from newchan.a_macd import OnlineMacdState

        rng = np.random.default_rng(42)
        closes = (rng.random(80) * 100 + 200).tolist()
        dates = [datetime(2020, 1, 1, tzinfo=timezone.utc)] * len(closes)

        state = OnlineMacdState()
        for c, d in zip(closes, dates):
            state.update(c, d)

        df_batch = compute_macd(pd.DataFrame({"close": closes}))
        df_online = state.to_dataframe()

        np.testing.assert_allclose(
            df_online["macd"].values, df_batch["macd"].values, atol=1e-10,
        )
        np.testing.assert_allclose(
            df_online["signal"].values, df_batch["signal"].values, atol=1e-10,
        )
        np.testing.assert_allclose(
            df_online["hist"].values, df_batch["hist"].values, atol=1e-10,
        )

    def test_reset_clears_state(self):
        """reset() 后 n_bars=0 且再次 update 从头开始。"""
        from datetime import datetime, timezone
        from newchan.a_macd import OnlineMacdState

        state = OnlineMacdState()
        ts = datetime(2020, 1, 1, tzinfo=timezone.utc)
        for c in [100.0, 101.0, 102.0]:
            state.update(c, ts)
        assert state.n_bars == 3
        state.reset()
        assert state.n_bars == 0
        state.update(100.0, ts)
        assert state.n_bars == 1

    def test_incremental_matches_stepwise(self):
        """逐步追加 vs 一次性批量：结果相同。"""
        from datetime import datetime, timezone
        from newchan.a_macd import OnlineMacdState

        closes = [float(i) for i in range(1, 31)]
        ts = datetime(2020, 1, 1, tzinfo=timezone.utc)

        # 一次性
        state_all = OnlineMacdState()
        for c in closes:
            state_all.update(c, ts)

        # 分两批
        state_step = OnlineMacdState()
        for c in closes[:15]:
            state_step.update(c, ts)
        for c in closes[15:]:
            state_step.update(c, ts)

        np.testing.assert_allclose(
            state_all.to_dataframe()["hist"].values,
            state_step.to_dataframe()["hist"].values,
            atol=1e-12,
        )
