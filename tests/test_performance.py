"""性能基准测试的验证测试。

验证：
  - 基准测试脚本可运行
  - 分层计时和复杂度分析正确
  - 报告结构完整
"""

from __future__ import annotations

import pytest

from scripts.performance_benchmark import (
    benchmark_layer_breakdown,
    benchmark_orchestrator,
    estimate_scaling_exponent,
    generate_multiscale_bars,
    generate_zigzag_bars,
)


class TestDataGeneration:
    """合成数据生成器测试。"""

    def test_zigzag_generates_correct_count(self) -> None:
        bars = generate_zigzag_bars(100)
        assert len(bars) == 100

    def test_zigzag_prices_positive(self) -> None:
        bars = generate_zigzag_bars(500)
        for bar in bars:
            assert bar.high > bar.low
            assert bar.high > 0

    def test_multiscale_generates_correct_count(self) -> None:
        bars = generate_multiscale_bars(100)
        assert len(bars) == 100

    def test_multiscale_prices_positive(self) -> None:
        bars = generate_multiscale_bars(500)
        for bar in bars:
            assert bar.high > bar.low
            assert bar.high > 0

    def test_timestamps_monotonic(self) -> None:
        bars = generate_zigzag_bars(100)
        for i in range(1, len(bars)):
            assert bars[i].ts > bars[i - 1].ts


class TestBenchmarkOrchestrator:
    """基准测试核心函数验证。"""

    def test_small_run_completes(self) -> None:
        """小数据集能正常跑完。"""
        bars = generate_zigzag_bars(200)
        report = benchmark_orchestrator(bars, label="test-200")
        assert report["label"] == "test-200"
        assert report["total_seconds"] > 0
        assert report["bars_per_second"] > 0

    def test_report_structure(self) -> None:
        """报告包含所有必要字段。"""
        bars = generate_zigzag_bars(200)
        report = benchmark_orchestrator(bars, label="test-struct")

        assert "total_seconds" in report
        assert "bars_per_second" in report
        assert "structure_stats" in report

        stat_keys = {
            "n_bars", "n_strokes", "n_segments",
            "n_zhongshu", "n_moves", "n_recursive_levels",
        }
        assert stat_keys == set(report["structure_stats"].keys())

    def test_layer_breakdown_keys(self) -> None:
        """分层计时包含所有引擎。"""
        bars = generate_zigzag_bars(200)
        report = benchmark_orchestrator(bars, label="test-layers", layer_breakdown=True)
        assert "layer_times" in report
        layer_keys = {
            "bi_engine", "segment_engine", "zhongshu_engine",
            "move_engine", "buysellpoint_engine", "recursive_stack",
        }
        assert layer_keys == set(report["layer_times"].keys())

    def test_structure_stats_nonnegative(self) -> None:
        """结构统计值非负。"""
        bars = generate_zigzag_bars(300)
        report = benchmark_orchestrator(bars, label="test-stats")
        for key, val in report["structure_stats"].items():
            assert val >= 0, f"{key} = {val}"


class TestScalingAnalysis:
    """复杂度分析测试。"""

    def test_estimate_linear(self) -> None:
        """线性数据 → α ≈ 1.0。"""
        sizes = [100, 200, 400, 800]
        times = [1.0, 2.0, 4.0, 8.0]
        alpha = estimate_scaling_exponent(sizes, times)
        assert 0.9 < alpha < 1.1

    def test_estimate_quadratic(self) -> None:
        """二次数据 → α ≈ 2.0。"""
        sizes = [100, 200, 400, 800]
        times = [1.0, 4.0, 16.0, 64.0]
        alpha = estimate_scaling_exponent(sizes, times)
        assert 1.9 < alpha < 2.1

    def test_estimate_single_point(self) -> None:
        """单点 → α = 0。"""
        alpha = estimate_scaling_exponent([100], [1.0])
        assert alpha == 0.0

    @pytest.mark.slow
    def test_bi_engine_is_dominant_bottleneck(self) -> None:
        """BiEngine 占全链路 >80% 耗时。"""
        bars = generate_zigzag_bars(500)
        layer_times = benchmark_layer_breakdown(bars)
        total = sum(layer_times.values())
        bi_pct = layer_times["bi_engine"] / total if total > 0 else 0
        assert bi_pct > 0.8, (
            f"BiEngine = {bi_pct:.1%} of total, expected >80%. "
            f"Layer times: {layer_times}"
        )

    @pytest.mark.slow
    def test_scaling_superlinear(self) -> None:
        """验证当前 BiEngine 的 O(n²) 特征（α > 1.5）。"""
        sizes = [200, 500, 1000]
        times = []
        for n in sizes:
            bars = generate_zigzag_bars(n)
            r = benchmark_orchestrator(bars, label=f"scale-{n}")
            times.append(r["total_seconds"])
        alpha = estimate_scaling_exponent(sizes, times)
        assert alpha > 1.5, (
            f"Scaling exponent α = {alpha:.2f}, expected >1.5 (O(n²) bottleneck). "
            f"sizes={sizes}, times={times}"
        )
