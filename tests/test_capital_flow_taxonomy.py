"""资本流转完全分类的单元测试（L0：守恒律 + 病态判据）。

认识论：本测试验证 27 配置枚举、守恒律、病态判据的代数正确性（L0）。
病态↔配置的**经验有效性**（实际市场是否如此流转）属 L2，需 10 年数据验证（§10），
不在本测试范围。
"""

from __future__ import annotations

import pytest

from newchan.topology.capital_flow_taxonomy import (
    CapitalFlow,
    PathologyMode,
    classify,
    enumerate_taxonomy,
    flow_to_walk,
    mode_histogram,
)


def test_conservation_law():
    """ΔM = −(ΔP+ΔC+ΔR)，四顶点和恒为 0。"""
    f = CapitalFlow(1, -1, 0)
    assert f.dm == 0
    assert f.dp + f.dc + f.dr + f.dm == 0
    f2 = CapitalFlow(1, 1, 1)
    assert f2.dm == -3
    assert f2.dp + f2.dc + f2.dr + f2.dm == 0


def test_invalid_component_raises():
    with pytest.raises(ValueError, match="流量分量"):
        CapitalFlow(2, 0, 0)


def test_enumerate_27():
    es = enumerate_taxonomy()
    assert len(es) == 27
    # 配置唯一
    labels = {e.flow.label for e in es}
    assert len(labels) == 27


def test_histogram_partition():
    """病态分布之和 = 27（完全分类，无遗漏无重叠）。"""
    hist = mode_histogram()
    assert sum(hist.values()) == 27
    # 已推导的分布（代码生成确认）。
    assert hist[PathologyMode.NEUTRAL] == 1
    assert hist[PathologyMode.HEALTHY] == 3
    assert hist[PathologyMode.SPECULATION] == 6
    assert hist[PathologyMode.DAM_BREAK] == 4
    assert hist[PathologyMode.SEDIMENTATION] == 4
    assert hist[PathologyMode.FLIGHT] == 4
    assert hist[PathologyMode.TRANSITIONAL] == 5


def test_classify_neutral():
    assert classify(CapitalFlow(0, 0, 0)) is PathologyMode.NEUTRAL


def test_classify_speculation():
    """空转：ΔP>0 ∧ ΔP>ΔC（金融吸资超实体）。"""
    assert classify(CapitalFlow(1, -1, -1)) is PathologyMode.SPECULATION  # 资本入P出C/R
    assert classify(CapitalFlow(1, 0, 0)) is PathologyMode.SPECULATION    # P独强


def test_classify_dam_break():
    """溃坝：ΔP<0 ∧ ΔP<ΔC（金融流出实体流入，空转反演）。"""
    assert classify(CapitalFlow(-1, 1, 0)) is PathologyMode.DAM_BREAK
    assert classify(CapitalFlow(-1, 0, 0)) is PathologyMode.DAM_BREAK


def test_classify_sedimentation():
    """沉没：ΔR>0 ∧ ΔP≤0 ∧ ΔC≤0（资本撤入不动产）。"""
    assert classify(CapitalFlow(-1, -1, 1)) is PathologyMode.SEDIMENTATION
    assert classify(CapitalFlow(0, 0, 1)) is PathologyMode.SEDIMENTATION
    assert classify(CapitalFlow(-1, 0, 1)) is PathologyMode.SEDIMENTATION


def test_classify_flight():
    """走资：ΔM≥+2（资本涌向货币/外汇）。"""
    assert classify(CapitalFlow(-1, -1, -1)) is PathologyMode.FLIGHT  # 全流出 ΔM=+3
    assert classify(CapitalFlow(-1, -1, 0)) is PathologyMode.FLIGHT   # ΔM=+2
    assert CapitalFlow(-1, -1, -1).dm == 3


def test_classify_healthy():
    """健康：ΔP=ΔC≠0（金融实体同步）。"""
    assert classify(CapitalFlow(1, 1, 1)) is PathologyMode.HEALTHY
    assert classify(CapitalFlow(1, 1, -1)) is PathologyMode.HEALTHY


def test_flight_priority_over_sedimentation():
    """走资优先于沉没：ΔM≥+2 时即使 ΔR... 但 ΔR>0 使 ΔM<+2，不冲突。验证优先级链。"""
    # (-,-,0): ΔM=+2 → FLIGHT（非 sediment，因 ΔR=0 不满足 sediment）
    assert classify(CapitalFlow(-1, -1, 0)) is PathologyMode.FLIGHT


def test_time_reversal_asymmetry():
    """时间反演非对称：健康(+,+,+) 镜像是走资(-,-,-)，非健康（吸收态破缺对称）。"""
    healthy = CapitalFlow(1, 1, 1)
    assert classify(healthy) is PathologyMode.HEALTHY
    assert classify(healthy.reverse()) is PathologyMode.FLIGHT  # 非对称！


def test_flow_to_walk_bridge():
    """流量→走势桥接（经验假设：流量驱动价格）。"""
    assert flow_to_walk(CapitalFlow(1, 0, -1)) == (1, 0, -1)


def test_entry_economic_meaning():
    es = {e.flow.label: e for e in enumerate_taxonomy()}
    spec = es["(+,-,-)"]
    assert spec.mode is PathologyMode.SPECULATION
    assert "流入" in spec.economic_meaning
    assert "空转" in spec.economic_meaning
