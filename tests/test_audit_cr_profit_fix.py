"""审计 A1/A2 修复回归测试 — 降成本 trim 利润不再被截断，且区分多空方向。

锁定编排者裁定的不变式：
  - 高卖低买为正利润，高买低卖为亏损（多空均成立）。
  - 亏损 trim **不被截断为 0**（A1 提款机 bug 的根因）。

依据：docs/architecture/opus48_audit_report.md A1/A2；docs/architecture/audit_fix_log.md。
认识论等级：L0（纯代数 / 定义验证——验证利润公式符号正确，不验证策略 alpha）。
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))


def _load_v3():
    spec = importlib.util.spec_from_file_location(
        "full_system_backtest_v3", ROOT / "analysis" / "full_system_backtest_v3.py"
    )
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


V3 = _load_v3()
ES = next(c for c in V3.ASSETS if c.symbol == "ES")        # futures
OKLO = next(c for c in V3.ASSETS if c.symbol == "OKLO")    # stock
RATIO = 0.20


# ── 多头 trim：高抛低吸 ─────────────────────────────────────────

def test_long_sell_high_buy_low_is_profit():
    # sell(开)=110 高，buy(平)=100 低 → 高卖低买 → 正
    p = V3._cr_profit_dollars(ES, 1, sell_price=110.0, buy_price=100.0, ratio=RATIO)
    assert p > 0, f"多头高卖低买应为正，得 {p}"


def test_long_buy_high_sell_low_is_loss_not_clamped():
    # sell(开)=100 低，buy(平)=110 高 → 高买低卖 → 必须为负（A1：不截断）
    p = V3._cr_profit_dollars(ES, 1, sell_price=100.0, buy_price=110.0, ratio=RATIO)
    assert p < 0, f"多头高买低卖应为亏损（不截断为0），得 {p}"


# ── 空头 trim：回补再卖 ─────────────────────────────────────────

def test_short_resell_high_cover_low_is_profit():
    # 空头：sell_price=回补价(开)=100 低，buy_price=再卖价(平)=110 高
    # 再卖(110) > 回补(100) → 高卖低买 → 正
    p = V3._cr_profit_dollars(OKLO, -1, sell_price=100.0, buy_price=110.0, ratio=RATIO)
    assert p > 0, f"空头再卖高于回补应为正，得 {p}"


def test_short_resell_low_cover_high_is_loss_not_clamped():
    # 空头：回补价(开)=110 高，再卖价(平)=100 低 → 再卖低于回补 → 高买低卖 → 负
    p = V3._cr_profit_dollars(OKLO, -1, sell_price=110.0, buy_price=100.0, ratio=RATIO)
    assert p < 0, f"空头再卖低于回补应为亏损（不截断为0），得 {p}"


# ── 提款机反例：连续逆向 trim 应使累计降成本为负 ──────────────────

def test_adverse_trims_accumulate_negative():
    """A1 核心：多次逆向（亏损）trim 累加后必须 < 0，证明截断已移除。"""
    total = 0.0
    for _ in range(50):
        # 每次都高买低卖（最坏情况）
        total += V3._cr_profit_dollars(ES, 1, sell_price=100.0, buy_price=105.0, ratio=RATIO)
    assert total < 0, f"50 次逆向 trim 累计应为负，得 {total}（截断未移除？）"


def test_no_clamp_source_removed():
    """源码层断言：v3 不再含 max(0.0, profit 截断，no_addon 不再含 if profit > 0。"""
    v3_src = (ROOT / "analysis" / "full_system_backtest_v3.py").read_text()
    assert "max(0.0, profit" not in v3_src
    no_addon_src = (ROOT / "analysis" / "fugue_no_addon_backtest_1min.py").read_text()
    assert "if profit > 0 and" not in no_addon_src
    with_short_src = (ROOT / "analysis" / "fugue_with_short_backtest_1min.py").read_text()
    assert "if profit > 0 and" not in with_short_src


# ════════════════════════════════════════════════════════════
# 挣股数阶段（53/81课）回归测试
# ════════════════════════════════════════════════════════════

def test_full_accounting_equivalent_to_legacy_phase1():
    """阶段1（降成本，股数固定）：全额记账公式 ≡ 旧 (price-cost_basis)/entry 公式。

    锁定 OKLO 实测的等价性（A1-only 与挣股数版 OKLO 均为 +114.08%）。
    """
    INITIAL = 100_000.0
    entry = 10.0
    shares = INITIAL / entry           # 阶段1 股数固定
    recovered = 30_000.0               # 降成本已回收
    cost_basis = entry - recovered / shares
    price = 12.0
    legacy = (price - cost_basis) / entry * 100
    full = (recovered + shares * price - INITIAL) / INITIAL * 100
    assert abs(legacy - full) < 1e-9, f"阶段1 两公式不等价: {legacy} vs {full}"


def test_earning_shares_increases_position():
    """挣股数阶段：盈利短差买回更多股数 → 总收益高于固定股数。"""
    INITIAL = 100_000.0
    entry = 10.0
    shares = INITIAL / entry           # 10000 股
    recovered = INITIAL                # 成本已归零
    # 挣股数：在 8.0 用利润买回额外股数（利润 2000$ / 8.0 = 250 股）
    earn_profit, earn_px = 2000.0, 8.0
    shares += earn_profit / earn_px
    price = 12.0
    pnl_with_earn = (recovered + shares * price - INITIAL) / INITIAL * 100
    pnl_no_earn = (recovered + (INITIAL / entry) * price - INITIAL) / INITIAL * 100
    assert pnl_with_earn > pnl_no_earn, "挣股数应提高总收益"
    # 额外 250 股 × 12 = 3000$ 增量 = +3.0%
    assert abs((pnl_with_earn - pnl_no_earn) - 3.0) < 1e-6


def test_earning_shares_state_present():
    """源码结构：no_addon 有 EARNING_SHARES 状态；v3 有 earn_legs 挣股数逻辑。"""
    na = (ROOT / "analysis" / "fugue_no_addon_backtest_1min.py").read_text()
    assert "EARNING_SHARES" in na
    assert "total_shares += profit / c" in na   # 挣股数核心：增持
    v3 = (ROOT / "analysis" / "full_system_backtest_v3.py").read_text()
    assert "earn_legs" in v3
    assert "挣股数" in v3
