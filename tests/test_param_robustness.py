"""scripts/tencent_param_robustness 的纯统计 helper 正确性测试。

注意（认识论）：本测试只验证**统计算法本身正确**（CV、bottleneck、zigzag、
相邻下跌段提取）——L0 管线层。它**不**验证"PH 比面积更鲁棒"这个 L2 结论
（那只能由腾讯 700 实测产生，且否定性结果同样有效）。在合成数据上"验证"鲁棒性
结论 = 合成确认偏差（231号禁止）。故这里仅测管线，不测结论。
"""

from __future__ import annotations

import importlib.util
import math
import sys
from pathlib import Path

import numpy as np
import pytest

_SCRIPT = Path(__file__).resolve().parent.parent / "scripts" / "tencent_param_robustness.py"
_spec = importlib.util.spec_from_file_location("tencent_param_robustness", _SCRIPT)
mod = importlib.util.module_from_spec(_spec)
sys.modules["tencent_param_robustness"] = mod  # dataclass 需模块先注册
_spec.loader.exec_module(mod)  # type: ignore[union-attr]


@pytest.mark.unit
def test_cv_known_values():
    """CV = std/|mean|（总体 std，与 numpy 默认一致）。"""
    assert mod.cv([5.0, 5.0, 5.0]) == pytest.approx(0.0)
    vals = [2.0, 4.0, 6.0]
    expected = float(np.std(vals)) / abs(float(np.mean(vals)))
    assert mod.cv(vals) == pytest.approx(expected)


@pytest.mark.unit
def test_cv_zero_mean_is_inf():
    assert math.isinf(mod.cv([-1.0, 1.0]))


@pytest.mark.unit
def test_cv_single_value():
    assert mod.cv([3.0]) == 0.0


@pytest.mark.unit
def test_pairwise_mean():
    # 三点两两差: |1-2|+|1-4|+|2-4| = 1+3+2 = 6, /3 = 2
    assert mod.pairwise_mean([1.0, 2.0, 4.0]) == pytest.approx(2.0)


@pytest.mark.unit
def test_pairwise_bottleneck_identical_zero():
    """相同 diagram 的 bottleneck = 0。"""
    dgm = np.array([[0.0, 1.0], [0.0, 2.0]])
    out = mod.pairwise_bottleneck([dgm, dgm])
    assert out == pytest.approx([0.0])


@pytest.mark.unit
def test_zigzag_alternates_high_low():
    """zigzag 输出 H/L 交替。"""
    closes = [10, 12, 9, 14, 8, 15, 7]
    pv = mod.zigzag([float(c) for c in closes], 0.1)
    kinds = [p.kind for p in pv]
    for a, b in zip(kinds, kinds[1:]):
        assert a != b  # 严格交替


@pytest.mark.unit
def test_adjacent_down_pairs_c_makes_new_low():
    """相邻下跌段对：C 段低点严格低于 A 段低点（创新低，可比背驰）。"""
    # 两段下跌，第二段更低
    closes = [100, 90, 100, 80, 95, 70, 90]
    pairs = mod.adjacent_down_pairs([float(c) for c in closes], 0.05)
    for a, c in pairs:
        assert closes[c[1]] < closes[a[1]]
