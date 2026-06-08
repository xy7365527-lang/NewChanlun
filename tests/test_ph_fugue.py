"""ph_fugue 引擎单测 —— band 分层 / 账本 MTM / 声部进出 / 短差 / 缠论分类 / 零前视。

认识论：本测试验证**管线正确性（L0/L1）**——band 分层、资金核算、信号产出的算法
确定性。不验证"赋格 > baseline"经验断言（那是回测脚本的 L2 工作）。
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.trading.ph_fugue import (  # noqa: E402
    ChanBspType,
    ChanlunBspClassifier,
    DualTreeStream,
    FugueVoice,
    PersistenceBands,
    Side,
    Signal,
    VoiceLedger,
    rolling_atr,
)


# ── band 分层 ──────────────────────────────────────────────────


def test_band_classify_thresholds():
    b = PersistenceBands(k0=1.0, k1=3.0, k2=8.0)
    atr = 2.0
    assert b.classify(0.5, atr) is None      # < τ₀=2
    assert b.classify(2.5, atr) == 0          # [τ₀,τ₁)=[2,6)
    assert b.classify(7.0, atr) == 1          # [τ₁,τ₂)=[6,16)
    assert b.classify(20.0, atr) == 2         # ≥ τ₂=16
    assert b.classify(100.0, 0.0) is None     # ATR=0 → 无级别


# ── 因果 ATR ───────────────────────────────────────────────────


def test_rolling_atr_causal_prefix_stability():
    """atr[i] 只依赖 [0..i]：扩展序列不改变已算前缀。"""
    h = [10, 11, 12, 11, 13, 14, 12, 15]
    lo = [9, 10, 11, 10, 12, 13, 11, 14]
    c = [9.5, 10.5, 11.5, 10.5, 12.5, 13.5, 11.5, 14.5]
    full = rolling_atr(h, lo, c, window=3)
    pref = rolling_atr(h[:5], lo[:5], c[:5], window=3)
    assert full[:5] == pref               # 前缀逐位相同 → 因果


# ── 账本 MTM ───────────────────────────────────────────────────


def test_ledger_buy_sell_realized():
    led = VoiceLedger(cash=1000.0)
    led.invest_all(price=10.0)             # 买 100 股，cash→0
    assert led.held_shares == 100.0
    assert led.equity(10.0) == 1000.0
    assert led.equity(12.0) == 1200.0      # MTM 浮盈
    pnl = led.sell(price=12.0, shares=50.0)
    assert pnl == 100.0                    # (12-10)*50
    assert led.realized == 100.0
    assert led.cash == 600.0               # 50*12
    assert led.held_shares == 50.0


def test_ledger_sell_capped_at_held():
    led = VoiceLedger(cash=100.0)
    led.invest_all(price=10.0)             # 10 股
    pnl = led.sell(price=11.0, shares=999.0)
    assert led.held_shares == 0.0
    assert pnl == 10.0                     # 只卖 10 股 (11-10)*10


# ── 单声部进出周期 ─────────────────────────────────────────────


def _sig(bar, side, band, pers, price, birth_idx=0, birth_price=0.0):
    return Signal(bar, side, band, pers, price, birth_idx, birth_price)


def test_voice_open_close_compounds():
    v = FugueVoice.create("L2", main_band=2, sub_band=1, capital=1000.0)
    v.on_signal(_sig(0, Side.BUY, 2, 99.0, 10.0))      # 主带买 → 满仓
    assert v.holding
    assert v.ledger.held_shares == 100.0
    v.on_signal(_sig(10, Side.SELL, 2, 99.0, 12.0))    # 主带卖 → 清仓 + RESET
    assert not v.holding
    assert abs(v.equity(12.0) - 1200.0) < 1e-6         # 全部变现
    assert abs(v.fsm.own_capital - 1200.0) < 1e-6      # 复利：新本金=现金


def test_voice_short_diff_reduces_and_rebuys():
    v = FugueVoice.create("L2", main_band=2, sub_band=1, capital=1000.0, sub_ratio=0.5)
    v.on_signal(_sig(0, Side.BUY, 2, 99.0, 10.0))      # 满仓 100 股
    v.on_signal(_sig(5, Side.SELL, 1, 50.0, 12.0))     # 子带卖 → 减 50% = 50 股@12
    assert v.has_open_short
    assert v.ledger.held_shares == 50.0
    assert abs(v.ledger.realized - 100.0) < 1e-6       # (12-10)*50
    v.on_signal(_sig(8, Side.BUY, 1, 50.0, 11.0))      # 子带买回 50 股@11
    assert not v.has_open_short
    assert abs(v.ledger.held_shares - 100.0) < 1e-6
    # 短差净赚：卖50@12买回50@11 → 比一直持有多赚 (12-11)*50=50
    assert v.equity(11.0) > 1000.0


def test_l0_voice_no_subband_ignores_sub_signals():
    v = FugueVoice.create("L0", main_band=0, sub_band=None, capital=1000.0)
    v.on_signal(_sig(0, Side.BUY, 0, 5.0, 10.0))
    held = v.ledger.held_shares
    # 无子带 → 任何非主带信号被忽略
    v.on_signal(_sig(2, Side.SELL, 1, 5.0, 11.0))
    assert v.ledger.held_shares == held
    assert v.holding


# ── 双树信号产出（buy=底确认 / sell=顶确认）─────────────────────


def test_dual_tree_emits_buy_on_bottom_confirmation():
    """谷A→峰P→谷B→升过P：younger 谷在 P 处 settle = BUY 信号（底确认）。

    单 V 只有一个全局极小分量，不产生 merge → 无 settle。必须有第二个分量
    在屏障峰处被吞并才 settle（因果合并事件）。
    """
    bands = PersistenceBands(k0=0.5, k1=3.0, k2=8.0)
    stream = DualTreeStream(bands)
    prices = [10, 6, 9, 7, 12]   # 谷6→峰9→谷7→升过9 → 谷7@峰9 settle, pers=2
    sides = [s.side for p in prices for s in stream.step(float(p), atr=1.0)]
    assert Side.BUY in sides


def test_dual_tree_emits_sell_on_top_confirmation():
    """峰A→谷T→峰B→跌破T：superlevel(喂-价) settle = SELL 信号（顶确认）。"""
    bands = PersistenceBands(k0=0.5, k1=3.0, k2=8.0)
    stream = DualTreeStream(bands)
    prices = [10, 14, 11, 13, 8]   # 峰14→谷11→峰13→跌破11 → 顶13@谷11 settle
    sides = [s.side for p in prices for s in stream.step(float(p), atr=1.0)]
    assert Side.SELL in sides


def test_dual_tree_zero_lookahead_prefix_stability():
    """喂第 i 根不改变 < i 根已发出的信号 —— 零前视核心不变量。"""
    bands = PersistenceBands(k0=0.5, k1=2.0, k2=5.0)
    prices = [10, 8, 6, 9, 7, 5, 11, 13, 10, 14, 8, 6]

    def run(seq):
        st = DualTreeStream(bands)
        log = []
        for i, p in enumerate(seq):
            for s in st.step(float(p), atr=1.0):
                log.append((s.bar_idx, s.side, s.band))
        return log

    full = run(prices)
    pref = run(prices[:7])
    # prefix run 发出的信号必是 full run 在 bar<7 的信号子集（settle 只增不改）
    pref_set = set(pref)
    full_early = {x for x in full if x[0] < 7}
    assert pref_set <= full_early


# ── 缠论买点分类（幅度代理）─────────────────────────────────────


def test_chanlun_classifier_type1_then_type2():
    clf = ChanlunBspClassifier()
    # 首底
    t0 = clf.classify(_sig(0, Side.BUY, 1, 10.0, 100.0, birth_price=100.0))
    assert t0 == ChanBspType.TYPE1
    # 创新低 + 幅度衰减 → 一买（背驰代理）
    t1 = clf.classify(_sig(5, Side.BUY, 1, 6.0, 90.0, birth_price=90.0))
    assert t1 == ChanBspType.TYPE1
    # 不创新低（higher-low） → 二买
    t2 = clf.classify(_sig(9, Side.BUY, 1, 5.0, 95.0, birth_price=95.0))
    assert t2 == ChanBspType.TYPE2


def test_chanlun_classifier_new_low_no_divergence_is_none():
    clf = ChanlunBspClassifier()
    clf.classify(_sig(0, Side.BUY, 1, 5.0, 100.0, birth_price=100.0))
    # 创新低但幅度增大（力度未减）→ 中继，非买点
    t = clf.classify(_sig(4, Side.BUY, 1, 9.0, 90.0, birth_price=90.0))
    assert t == ChanBspType.NONE


def test_classifier_ignores_sell_side():
    clf = ChanlunBspClassifier()
    t = clf.classify(_sig(0, Side.SELL, 1, 10.0, 100.0, birth_price=100.0))
    assert t == ChanBspType.NONE
