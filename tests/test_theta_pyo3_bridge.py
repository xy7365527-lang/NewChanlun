"""#951 D1 出口 PyO3 行为不变式测试（设计稿 §4.2 验收 B1/C1/C2/C3 的测试锁面）。

seam：`ThetaStream.push_bar(o, h, l, c, p_t, nav) -> f64`（返回目标净敞口 p_star）。
本测试只打这条 seam + D1 两样出口（目标净敞口 / per-leg 账本）——不 mock 内部协作者、
不测内部实现；期望值用字面量/工作例，不源自实现同源推导。

合成锯齿 bar（确定性）——不依赖真实行情文件，CI 的 `pytest -m "not slow"` 可跑。
"""

from __future__ import annotations

import pytest

newchan_rust = pytest.importorskip("newchan_rust")


def synthetic_ohlc(n: int = 80) -> list[tuple[float, float, float, float]]:
    """确定性锯齿行情（摆动高低点足够形成分型/笔；不依赖是否形成买卖点）。"""
    bars: list[tuple[float, float, float, float]] = []
    close = 100.0
    for swing in range(n // 5):
        direction = 1.0 if swing % 2 == 0 else -1.0
        for _ in range(5):
            open_ = close
            close += direction * 0.05
            high = max(open_, close) + 0.01
            low = min(open_, close) - 0.01
            bars.append((open_, high, low, close))
    return bars


def test_push_bar_returns_float_and_deterministic():
    bars = synthetic_ohlc()
    a = newchan_rust.ThetaStream()
    b = newchan_rust.ThetaStream()
    p_t = 0.0
    for i, (o, h, l, c) in enumerate(bars):
        pa = a.push_bar(o, h, l, c, p_t, 1.0e6)
        pb = b.push_bar(o, h, l, c, p_t, 1.0e6)
        assert isinstance(pa, float), f"bar {i}: 返回值非 float"
        assert pa == pb, f"bar {i}: 同输入确定性破（{pa} vs {pb}）"
        p_t = pa


def test_last_order_action_in_seven_constructors():
    bars = synthetic_ohlc()
    s = newchan_rust.ThetaStream()
    p_t = 0.0
    for o, h, l, c in bars:
        p_t = s.push_bar(o, h, l, c, p_t, 1.0e6)
    action, qty, exec_index = s.last_order()
    assert action in {"buy", "sell", "add", "reduce", "hold", "close", "wait"}
    assert isinstance(qty, int)
    assert isinstance(exec_index, int)


def test_p_star_within_nominal_cap():
    # 名义上限：cap = γ̄·U_ℓ（default risk.gamma × nav/px）。只断言有符号手数界内不爆表
    # （默认 gamma 小 ⟹ |p_star| 被 U_ℓ 尺度约束），并断言 p_star 有限。
    bars = synthetic_ohlc()
    s = newchan_rust.ThetaStream()
    p_t = 0.0
    for o, h, l, c in bars:
        p_star = s.push_bar(o, h, l, c, p_t, 1.0e6)
        assert abs(p_star) < 1.0e6, f"p_star={p_star} 超名义上限"
        p_t = p_star


def test_finish_full_reconcile_and_lee_net_witness():
    bars = synthetic_ohlc()
    s = newchan_rust.ThetaStream()
    p_t = 0.0
    for o, h, l, c in bars:
        p_t = s.push_bar(o, h, l, c, p_t, 1.0e6)
    d = s.finish_full()
    assert isinstance(d, dict)
    # C1：reconcile_residual < 1e-6（PDF §11 对账恒等）。
    assert d["reconcile_residual"] < 1e-6, f"reconcile_residual={d['reconcile_residual']}"
    # C2：lee_net_witness 的恒等残差恒 0（整数手数求和，非 eps 容差）。
    n_obs, max_resid, max_net, witnessed = d["lee_net_witness"]
    assert max_resid == 0, f"LEE-Net 残差 {max_resid} != 0"
    # 锯齿合成若无非零净敞口 ⟹ witnessed 为 False 是合法退化；不强行断言 True。
    assert isinstance(witnessed, bool)
    # active_voices / closed_voices 形状（ClosedVoice 无 q，10 元组；VoiceBook 有 q，9 元组）。
    for row in d["active_voices"]:
        assert len(row) == 9, f"active_voices 行形状 {len(row)}"
    for row in d["closed_voices"]:
        assert len(row) == 10, f"closed_voices 行形状 {len(row)}"


def test_sep_legs_shape_and_snapshot():
    bars = synthetic_ohlc()
    s = newchan_rust.ThetaStream()
    p_t = 0.0
    for o, h, l, c in bars:
        p_t = s.push_bar(o, h, l, c, p_t, 1.0e6)
    legs = s.sep_legs()
    assert isinstance(legs, list)
    for row in legs:
        assert len(row) == 6, f"sep_legs 行形状 {len(row)}"
        level, ordinal, side, q_units, role_v, parent = row
        assert isinstance(level, int) and isinstance(ordinal, int)
        assert side in {"long", "short"}
        assert role_v in {"ambient", "follow_parent", "reverse_open"}
        assert parent is None or (isinstance(parent, tuple) and len(parent) == 2)
    bar_count, p_star, n_active, n_levels = s.snapshot()
    assert isinstance(bar_count, int) and bar_count == len(bars)
    assert isinstance(p_star, float)
