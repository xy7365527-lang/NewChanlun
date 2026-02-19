"""026 谱系真实数据验证 — 现金边信号消歧在真实 ETF 数据上的表现。

结算条件 #2: 在真实数据上验证：找到度量变动 vs 真实流转的实际案例

验证方式:
  1. 从 Alpha Vantage 获取 SPY / GLD / IYR 日线数据
  2. 计算每日六条边的资本流转方向（比价上涨/下跌/持平）
  3. 调用 disambiguate_cash_signal 消歧
  4. 检验信号类型分布：应同时存在 genuine_flow / metric_shift / mixed

运行: pytest tests/test_026_real_data_validation.py -v -s
"""

from __future__ import annotations

import os
from datetime import datetime

import pytest

# 跳过条件：需要 AV API key
_AV_KEY = os.environ.get("ALPHAVANTAGE_API_KEY") or ""
if not _AV_KEY:
    try:
        from dotenv import load_dotenv
        load_dotenv()
        _AV_KEY = os.environ.get("ALPHAVANTAGE_API_KEY") or ""
    except ImportError:
        pass

skip_no_av_key = pytest.mark.skipif(
    not _AV_KEY,
    reason="ALPHAVANTAGE_API_KEY not set",
)


def _direction_from_change(change: float) -> str:
    """价格变动 → 流转方向字符串。"""
    if change > 0:
        return "up"
    elif change < 0:
        return "down"
    return "flat"


def _build_edge_inputs_for_day(
    spy_prev: float, spy_curr: float,
    gld_prev: float, gld_curr: float,
    iyr_prev: float, iyr_curr: float,
):
    """从三个 ETF 的前后收盘价构建六条边的 EdgeFlowInput。

    四顶点映射：
      CASH = USD（隐含）
      EQUITY = SPY
      COMMODITY = GLD
      REAL_ESTATE = IYR

    六条边：
      CASH↔EQUITY: SPY 涨 → A_TO_B（CASH→EQUITY）
      CASH↔COMMODITY: GLD 涨 → A_TO_B（CASH→COMMODITY）
      CASH↔REAL_ESTATE: IYR 涨 → A_TO_B（CASH→REAL_ESTATE）
      EQUITY↔COMMODITY: GLD/SPY 比值涨 → A_TO_B（EQUITY→COMMODITY）
      EQUITY↔REAL_ESTATE: IYR/SPY 比值涨 → A_TO_B（EQUITY→REAL_ESTATE）
      COMMODITY↔REAL_ESTATE: IYR/GLD 比值涨 → A_TO_B（COMMODITY→REAL_ESTATE）
    """
    from newchan.matrix_topology import AssetVertex
    from newchan.capital_flow import FlowDirection
    from newchan.flow_relation import EdgeFlowInput

    def _dir(prev_val: float, curr_val: float) -> FlowDirection:
        """值上涨 → A_TO_B, 下跌 → B_TO_A, 持平 → EQUILIBRIUM."""
        if curr_val > prev_val:
            return FlowDirection.A_TO_B
        elif curr_val < prev_val:
            return FlowDirection.B_TO_A
        return FlowDirection.EQUILIBRIUM

    # 现金边：价格直接比较
    spy_dir = _dir(spy_prev, spy_curr)
    gld_dir = _dir(gld_prev, gld_curr)
    iyr_dir = _dir(iyr_prev, iyr_curr)

    # 纯资产边：比值比较
    ratio_gld_spy_prev = gld_prev / spy_prev
    ratio_gld_spy_curr = gld_curr / spy_curr
    eq_com_dir = _dir(ratio_gld_spy_prev, ratio_gld_spy_curr)

    ratio_iyr_spy_prev = iyr_prev / spy_prev
    ratio_iyr_spy_curr = iyr_curr / spy_curr
    eq_re_dir = _dir(ratio_iyr_spy_prev, ratio_iyr_spy_curr)

    ratio_iyr_gld_prev = iyr_prev / gld_prev
    ratio_iyr_gld_curr = iyr_curr / gld_curr
    com_re_dir = _dir(ratio_iyr_gld_prev, ratio_iyr_gld_curr)

    return [
        EdgeFlowInput(AssetVertex.CASH, AssetVertex.EQUITY, spy_dir),
        EdgeFlowInput(AssetVertex.CASH, AssetVertex.COMMODITY, gld_dir),
        EdgeFlowInput(AssetVertex.CASH, AssetVertex.REAL_ESTATE, iyr_dir),
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.COMMODITY, eq_com_dir),
        EdgeFlowInput(AssetVertex.EQUITY, AssetVertex.REAL_ESTATE, eq_re_dir),
        EdgeFlowInput(AssetVertex.COMMODITY, AssetVertex.REAL_ESTATE, com_re_dir),
    ]


@pytest.mark.slow
@skip_no_av_key
class TestCashDisambiguationRealData:
    """026 谱系结算条件 #2: 真实数据验证。"""

    @pytest.fixture(scope="class")
    def daily_data(self):
        """从 AV 获取 SPY / GLD / IYR 日线数据（compact = 100 个交易日）。"""
        from newchan.data_av import AlphaVantageProvider

        prov = AlphaVantageProvider(api_key=_AV_KEY, rate_limit=13.0)
        spy_bars = prov.fetch_daily("SPY", outputsize="compact")
        gld_bars = prov.fetch_daily("GLD", outputsize="compact")
        iyr_bars = prov.fetch_daily("IYR", outputsize="compact")
        return spy_bars, gld_bars, iyr_bars

    @pytest.fixture(scope="class")
    def aligned_closes(self, daily_data):
        """对齐三个 ETF 的交易日（按日期取交集），返回对齐的收盘价序列。"""
        spy_bars, gld_bars, iyr_bars = daily_data

        # 转为 {date: close} 字典
        spy_map = {b.ts.date(): b.close for b in spy_bars}
        gld_map = {b.ts.date(): b.close for b in gld_bars}
        iyr_map = {b.ts.date(): b.close for b in iyr_bars}

        # 取三者交集日期
        common_dates = sorted(
            set(spy_map.keys()) & set(gld_map.keys()) & set(iyr_map.keys())
        )
        assert len(common_dates) >= 20, f"对齐后交易日不足: {len(common_dates)}"

        return [
            (d, spy_map[d], gld_map[d], iyr_map[d])
            for d in common_dates
        ]

    @pytest.fixture(scope="class")
    def daily_signals(self, aligned_closes):
        """对每个交易日计算消歧信号。"""
        from newchan.flow_relation import disambiguate_cash_signal

        signals = []
        for i in range(1, len(aligned_closes)):
            _, spy_prev, gld_prev, iyr_prev = aligned_closes[i - 1]
            date, spy_curr, gld_curr, iyr_curr = aligned_closes[i]

            edges = _build_edge_inputs_for_day(
                spy_prev, spy_curr,
                gld_prev, gld_curr,
                iyr_prev, iyr_curr,
            )
            analysis = disambiguate_cash_signal(edges)
            signals.append((date, analysis))

        return signals

    def test_data_fetched(self, daily_data):
        """验证三个 ETF 数据获取成功。"""
        spy, gld, iyr = daily_data
        assert len(spy) >= 50, f"SPY 数据不足: {len(spy)}"
        assert len(gld) >= 50, f"GLD 数据不足: {len(gld)}"
        assert len(iyr) >= 50, f"IYR 数据不足: {len(iyr)}"
        print(f"\n[026] SPY: {len(spy)} bars, GLD: {len(gld)} bars, IYR: {len(iyr)} bars")

    def test_dates_aligned(self, aligned_closes):
        """验证日期对齐。"""
        assert len(aligned_closes) >= 20
        print(f"\n[026] 对齐交易日: {len(aligned_closes)}")
        print(f"[026] 范围: {aligned_closes[0][0]} ~ {aligned_closes[-1][0]}")

    def test_signal_types_distribution(self, daily_signals):
        """核心验证：信号类型分布应覆盖多种情况。"""
        type_counts: dict[str, int] = {}
        for date, analysis in daily_signals:
            t = analysis.signal_type
            type_counts[t] = type_counts.get(t, 0) + 1

        total = len(daily_signals)
        print(f"\n{'='*60}")
        print(f"[026] 消歧信号分布 ({total} 个交易日)")
        print(f"{'='*60}")
        for t, c in sorted(type_counts.items()):
            print(f"  {t:20s}: {c:3d} ({c/total:.1%})")
        print(f"{'='*60}")

        # 核心断言：至少应出现两种非 neutral 信号类型
        non_neutral_types = {t for t in type_counts if t != "neutral"}
        assert len(non_neutral_types) >= 2, (
            f"信号类型过于单一: {type_counts}。"
            f"消歧函数应能在真实数据上区分不同情况。"
        )

    def test_genuine_flow_exists(self, daily_signals):
        """应存在 genuine_flow 信号（纯资产子图有分化时的真实流转）。"""
        genuine = [
            (d, a) for d, a in daily_signals
            if a.signal_type == "genuine_flow"
        ]
        print(f"\n[026] genuine_flow 样例: {len(genuine)} 个")
        if genuine:
            d, a = genuine[0]
            print(f"  首例: {d}, cash_net={a.cash_net_flow}, "
                  f"var={a.asset_subgraph_variance:.4f}, conf={a.confidence:.2f}")
        assert len(genuine) > 0, "真实数据中未找到 genuine_flow 信号"

    def test_metric_shift_or_mixed_exists(self, daily_signals):
        """应存在 metric_shift 或 mixed 信号。"""
        ms_or_mixed = [
            (d, a) for d, a in daily_signals
            if a.signal_type in ("metric_shift", "mixed")
        ]
        print(f"\n[026] metric_shift/mixed 样例: {len(ms_or_mixed)} 个")
        if ms_or_mixed:
            for d, a in ms_or_mixed[:3]:
                print(f"  {d}: type={a.signal_type}, cash_net={a.cash_net_flow}, "
                      f"var={a.asset_subgraph_variance:.4f}, conf={a.confidence:.2f}")
        assert len(ms_or_mixed) > 0, (
            "真实数据中未找到 metric_shift 或 mixed 信号。"
            "这可能意味着消歧阈值需要调整，或度量基准变动在此时间段不明显。"
        )

    def test_high_confidence_signals(self, daily_signals):
        """高置信度信号应占合理比例。"""
        high_conf = [
            (d, a) for d, a in daily_signals
            if a.confidence >= 0.8 and a.signal_type != "neutral"
        ]
        total_non_neutral = sum(
            1 for _, a in daily_signals if a.signal_type != "neutral"
        )
        if total_non_neutral == 0:
            pytest.skip("无非 neutral 信号")

        ratio = len(high_conf) / total_non_neutral
        print(f"\n[026] 高置信度(≥0.8): {len(high_conf)}/{total_non_neutral} = {ratio:.1%}")
        # 不强制比例，仅报告

    def test_daily_signal_report(self, daily_signals):
        """输出逐日信号报告（最近 10 天）。"""
        recent = daily_signals[-10:]
        print(f"\n{'='*70}")
        print(f"[026] 最近 {len(recent)} 天逐日消歧报告")
        print(f"{'='*70}")
        print(f"  {'日期':12s} {'类型':15s} {'cash_net':>8s} {'方差':>8s} {'置信度':>6s}")
        print(f"  {'-'*12} {'-'*15} {'-'*8} {'-'*8} {'-'*6}")
        for d, a in recent:
            print(f"  {str(d):12s} {a.signal_type:15s} "
                  f"{a.cash_net_flow:>8d} {a.asset_subgraph_variance:>8.4f} "
                  f"{a.confidence:>6.2f}")
        print(f"{'='*70}")
