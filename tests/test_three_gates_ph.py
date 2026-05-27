"""scripts/tencent_three_gates_ph 的纯逻辑正确性测试（L0 管线层）。

认识论：只验证闸门管线（zigzag 分段、段内中枢计数、真假底标注、综合判定）算法
正确——**不**验证"三闸门标记真假底"的 L2 结论（那由腾讯 700 实测产生，已得否定性
结果）。在合成数据上"验证"结论 = 合成确认偏差（231号禁止）。
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

_SCRIPT = Path(__file__).resolve().parent.parent / "scripts" / "tencent_three_gates_ph.py"
_spec = importlib.util.spec_from_file_location("tencent_three_gates_ph", _SCRIPT)
mod = importlib.util.module_from_spec(_spec)
sys.modules["tencent_three_gates_ph"] = mod
_spec.loader.exec_module(mod)  # type: ignore[union-attr]


@pytest.mark.unit
def test_down_legs_extracts_high_to_low():
    """down_legs 提取 H→L 段，段内 close[h] > close[lo]。"""
    closes = [100.0, 90, 100, 80, 95, 70, 90]
    legs = mod.down_legs(closes, 0.05)
    assert legs  # 至少一段
    for h, lo in legs:
        assert closes[h] > closes[lo]
        assert h < lo


@pytest.mark.unit
def test_count_zhongshu_in_segment_detects_synthetic():
    """合成中枢段（同层 3 摆动重叠）→ 段内中枢数 ≥ 1。"""
    # 与 test_ph_zhongshu 同款振荡 band
    closes = [20.0, 10, 18, 11, 17, 12, 19, 5, 8, 2]
    n = mod.count_zhongshu_in_segment(closes, 0, len(closes) - 1, tau=0.0)
    assert n >= 1


@pytest.mark.unit
def test_count_zhongshu_monotone_zero():
    """纯单调下跌段 → 0 中枢。"""
    assert mod.count_zhongshu_in_segment([20.0, 16, 12, 8, 4, 2], 0, 5, tau=0.0) == 0


@pytest.mark.unit
def test_ground_truth_fake_when_new_low():
    """后续创新低 → 假底。"""
    closes = [100.0, 80, 90, 70]  # idx1=80 后续有 70 更低
    assert mod.ground_truth(closes, 1, 80.0) == "假底"


@pytest.mark.unit
def test_ground_truth_real_when_rebounds():
    """无更低 + 后续反弹 ≥10% → 真底。"""
    closes = [100.0, 80, 85, 92]  # idx1=80 无更低，反弹到 92 = +15%
    assert mod.ground_truth(closes, 1, 80.0) == "真底"


@pytest.mark.unit
def test_ground_truth_unconfirmed():
    """无更低但反弹不足 10% → 未确认。"""
    closes = [100.0, 80, 82, 83]  # 反弹仅 +3.75%
    assert mod.ground_truth(closes, 1, 80.0) == "未确认"


@pytest.mark.unit
def test_evaluate_gates_returns_structured_result():
    """evaluate_gates 产出结构化结果，predict_real = 三闸门合取。"""
    closes = [20.0, 10, 18, 11, 17, 12, 19, 5, 8, 2]
    fine = mod.down_legs(closes, 0.05)
    coarse = mod.down_legs(closes, 0.12)
    if not fine:
        pytest.skip("合成序列无 fine 下跌段")
    g = mod.evaluate_gates(closes, 0.0, fine, coarse, fine[-1][1])
    assert g.predict_real == (g.gate1_trend and g.gate2_divergent and g.gate3_synced)
