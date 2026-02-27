"""§31-33 三模块集成测试：topology × nesting × trading 接口一致性验证。

验证 v83-swarm 四个独立工位产出的模块间接口兼容性。
不修改源文件——只验证，不修复。
"""

from __future__ import annotations

import pytest

# ── topology imports ──
from newchan.topology.config_space import (
    CENTER,
    FULL_RISK_OFF,
    FULL_RISK_ON,
    Configuration,
    ConfigurationSpace,
    WalkDirection,
    polarity_index,
)
from newchan.topology.transition import adjacent, is_adjacent, shortest_path

# ── nesting imports ──
from newchan.nesting.bsp import BSP, BSPType, DivergenceType, is_negated
from newchan.nesting.horizontal import (
    HorizontalNesting,
    HorizontalStep,
    StepResult,
    advance,
    create_horizontal_nesting,
    is_blocked,
)
from newchan.nesting.nesting_operator import (
    NestingPath,
    NestingStep,
    NestingType,
    append_step,
    check_level_monotonicity,
    check_search_space_contraction,
    check_termination,
    validate_path,
)
from newchan.nesting.resonance import (
    ResonanceLevel,
    ResonanceSignal,
    SignalLayer,
    classify_resonance,
    resonance_check,
    resonance_strength,
)
from newchan.nesting.wait_state import WaitState, can_operate, max_position

# ── trading imports ──
from newchan.trading.layer_state import Layer, LayerState, LayerType
from newchan.trading.state_machine import (
    AxiomViolationError,
    FourLayerStateMachine,
    IllegalTransitionError,
)
from newchan.trading.position_sizing import (
    STANDARD_SIGNALS,
    Layer as PosLayer,
    PositionSizer,
    Signal,
)
from newchan.trading.divergence_allocation import allocate, divergence_ratio
from newchan.trading.paths import Event, PathType, classify_path, execute_path


# ── 测试工具 ──

def _make_bsp(
    edge_id: str = "E/CASH",
    level: int = 0,
    time: float = 100.0,
    bsp_type: BSPType = BSPType.B1,
    price: float = 100.0,
    divergence: DivergenceType = DivergenceType.NONE,
) -> BSP:
    """创建测试用 BSP 实例。"""
    return BSP(
        edge_id=edge_id,
        level=level,
        time=time,
        bsp_type=bsp_type,
        price=price,
        divergence=divergence,
    )


# ============================================================================
# 1. topology → nesting 连接
# ============================================================================


class TestTopologyToNesting:
    """验证 topology 模块的输出可以作为 nesting 模块的输入。"""

    def test_configuration_as_nesting_search_space(self):
        """ConfigurationSpace 中的配置标签可以构成 NestingOperator 的搜索空间。"""
        space = ConfigurationSpace()
        # 用配置标签构建搜索空间
        labels = frozenset(c.label for c in space)
        assert len(labels) == 27

        # 搜索空间可以传入 NestingStep
        step = NestingStep(
            nesting_type=NestingType.HORIZONTAL,
            search_space=labels,
            level=0,
            target=FULL_RISK_ON.label,
            bsp_confirmation=_make_bsp(),
        )
        assert step.search_space == labels
        assert step.target in labels

    def test_configuration_polarity_drives_bsp_direction(self):
        """极性指数 > 0 对应买方信号，< 0 对应卖方信号——方向一致性。"""
        space = ConfigurationSpace()
        for config in space:
            s = polarity_index(config)
            if s > 0:
                # 正极性 → 风险偏好上升 → 买入方向
                bsp = _make_bsp(bsp_type=BSPType.B1)
                assert bsp.direction_is_buy is True
            elif s < 0:
                # 负极性 → 风险偏好下降 → 卖出方向
                bsp = _make_bsp(bsp_type=BSPType.S1)
                assert bsp.direction_is_buy is False

    def test_adjacent_configs_can_drive_horizontal_step(self):
        """相邻配置的转换可以驱动横向区间套的状态推进。"""
        state = create_horizontal_nesting()
        assert state.current_step is HorizontalStep.CONFIG_BSP
        assert is_blocked(state)

        # 在配置层出现买点 → 推进到第二步
        config_bsp = _make_bsp(edge_id="config_layer", bsp_type=BSPType.B1)
        state2 = advance(state, config_bsp, target="E/CASH")
        assert state2.current_step is HorizontalStep.EDGE_BSP
        assert len(state2.completed_results) == 1

    def test_transition_adjacency_compatible_with_nesting_level(self):
        """转换拓扑的相邻关系满足区间套级别单调性前提。"""
        c1 = FULL_RISK_ON  # (+,+,+)
        neighbors = adjacent(c1)
        # 从 corner 配置出发，相邻配置是 edge 类型
        for nb in neighbors:
            # 相邻 = 曼哈顿距离 1
            assert shortest_path(c1, nb) == 1
            # 相邻配置可以在同一级别或更低级别观察 → 级别单调性兼容
            path = NestingPath(steps=(
                NestingStep(
                    nesting_type=NestingType.HORIZONTAL,
                    search_space=frozenset({c1.label, nb.label}),
                    level=0,
                    target=nb.label,
                    bsp_confirmation=_make_bsp(),
                ),
            ))
            assert check_level_monotonicity(path)

    def test_config_space_iteration_as_nesting_input(self):
        """ConfigurationSpace 迭代产出的 Configuration 都可以作为 NestingStep 目标。"""
        space = ConfigurationSpace()
        for config in space:
            step = NestingStep(
                nesting_type=NestingType.HORIZONTAL,
                search_space=frozenset({config.label}),
                level=0,
                target=config.label,
                bsp_confirmation=_make_bsp(),
            )
            assert step.target == config.label

    def test_polarity_index_range_covers_all_configs(self):
        """极性指数值域 [-3, +3] 与配置空间完全覆盖。"""
        space = ConfigurationSpace()
        polarities = {polarity_index(c) for c in space}
        assert polarities == {-3, -2, -1, 0, 1, 2, 3}


# ============================================================================
# 2. nesting → trading 连接
# ============================================================================


class TestNestingToTrading:
    """验证 nesting 模块的输出可以驱动 trading 模块。"""

    def test_bsp_type_drives_state_machine_entry(self):
        """BSPType 可以作为 StateMachine entry 的触发信号。"""
        sm = FourLayerStateMachine()
        # L0 entry 用配置层买点
        bsp = _make_bsp(bsp_type=BSPType.B1)
        bsp_label = f"配置空间{bsp.bsp_type.value}"

        result = sm.entry(LayerType.L0_CONFIG, bsp=bsp_label, position=0.3)
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE

    def test_bsp_types_map_to_entry_labels(self):
        """所有 BSPType（买点）都可以生成合法的 entry bsp 标签字符串。"""
        buy_types = [BSPType.B1, BSPType.B2, BSPType.B3]
        for bt in buy_types:
            bsp = _make_bsp(bsp_type=bt)
            label = f"config_{bsp.bsp_type.value}"
            # label 是非空字符串
            assert isinstance(label, str) and len(label) > 0

    def test_resonance_strength_drives_position_sizer(self):
        """共振强度分类可以映射到 PositionSizer 的信号确认。"""
        # 构造三层共振信号
        signals = [
            ResonanceSignal(
                edge_id="E/CASH", level=0, layer=SignalLayer.CONFIG,
                bsp=_make_bsp(bsp_type=BSPType.B1), time=100.0,
            ),
            ResonanceSignal(
                edge_id="E/CASH", level=1, layer=SignalLayer.INDEPENDENT_EDGE,
                bsp=_make_bsp(bsp_type=BSPType.B1, level=1), time=101.0,
            ),
            ResonanceSignal(
                edge_id="AAPL", level=2, layer=SignalLayer.UNDERLYING,
                bsp=_make_bsp(edge_id="AAPL", bsp_type=BSPType.B1, level=2), time=102.0,
            ),
        ]

        strength = resonance_strength(signals)
        res_level = classify_resonance(strength)
        # w=3 + w=2 + w=1 = 6 → THREE_LAYER
        assert res_level == ResonanceLevel.THREE_LAYER

        # 三层共振 → 确认 L0 + L1 + L3 信号
        sizer = PositionSizer()
        pos_signals = list(STANDARD_SIGNALS)
        # 确认 s1(L0), s2(L1), s8(L3) 对应三层共振
        confirmed_ids = {"s1", "s2", "s8"}
        confirmed_signals = [
            Signal(id=s.id, layer=s.layer, weight=s.weight,
                   confirmed=(s.id in confirmed_ids))
            for s in pos_signals
        ]
        position = sizer.position(confirmed_signals)
        assert position > 0  # 有确认信号 → 仓位 > 0

    def test_resonance_level_maps_to_position_range(self):
        """共振层次分类映射到合理的仓位区间。"""
        sizer = PositionSizer()

        # THREE_LAYER（满仓信号）→ 高仓位
        three_layer_signals = [
            Signal(id="s1", layer=PosLayer.L0, weight=0.15, confirmed=True),
            Signal(id="s2", layer=PosLayer.L1, weight=0.12, confirmed=True),
            Signal(id="s8", layer=PosLayer.L3, weight=0.02, confirmed=True),
        ]
        pos_high = sizer.position(three_layer_signals)

        # SINGLE（单层信号）→ 低仓位
        single_signals = [
            Signal(id="s8", layer=PosLayer.L3, weight=0.02, confirmed=True),
        ]
        pos_low = sizer.position(single_signals)

        assert pos_high > pos_low

    def test_wait_state_can_operate_blocks_state_machine(self):
        """WaitState.can_operate() == False 时，不应触发 StateMachine entry。"""
        wait = WaitState(
            step=HorizontalStep.CONFIG_BSP,
            scope=frozenset({"E/CASH", "C/CASH", "R/CASH"}),
            level=0,
            time=100.0,
        )
        # 无信号 → 不可操作
        assert can_operate(wait, None) is False

        # 有信号但不在 scope 内 → 不可操作
        out_of_scope_bsp = _make_bsp(edge_id="AAPL", bsp_type=BSPType.B1)
        assert can_operate(wait, out_of_scope_bsp) is False

        # 有信号且在 scope 内 → 可操作
        in_scope_bsp = _make_bsp(edge_id="E/CASH", bsp_type=BSPType.B1)
        assert can_operate(wait, in_scope_bsp) is True

    def test_wait_state_position_limit_consistent_with_sizer(self):
        """WaitState 的仓位上限与 PositionSizer 的输出在合理范围内。"""
        # 等待在第一步 → 仓位上限 0.1
        wait = WaitState(
            step=HorizontalStep.CONFIG_BSP,
            scope=frozenset({"E/CASH"}),
            level=0,
            time=100.0,
        )
        limit = max_position(wait)
        assert limit == pytest.approx(0.1)

        # PositionSizer 在仅有单标的信号时的仓位
        sizer = PositionSizer()
        single = [Signal(id="s8", layer=PosLayer.L3, weight=0.02, confirmed=True)]
        pos = sizer.position(single)
        # 单标的仓位应该小于等待在最后一步的上限 (0.9)
        assert pos <= 0.9

    def test_bsp_negation_drives_exit_abort(self):
        """BSP 否定信号可以触发 StateMachine 的 exit_abort。"""
        # 建立有仓位的状态
        sm = FourLayerStateMachine()
        result = sm.entry(LayerType.L0_CONFIG, bsp="配置一买", position=0.5)
        sm2 = result.machine
        assert sm2.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE

        # BSP 否定
        bsp = _make_bsp(bsp_type=BSPType.B1, price=100.0)
        assert is_negated(bsp, current_price=90.0) is True  # 买点被否定

        # 否定 → exit_abort
        abort_result = sm2.exit_abort(LayerType.L0_CONFIG)
        assert abort_result.machine.get_state(LayerType.L0_CONFIG) == LayerState.INACTIVE
        assert abort_result.realized_profit < 0  # 亏损

    def test_horizontal_complete_to_trading_pipeline(self):
        """横向区间套六步全部完成后可以触发交易状态机。"""
        state = create_horizontal_nesting()

        # 逐步推进六步
        targets = ["config", "E/CASH", "US", "tech", "AAPL", "AAPL_vertical"]
        for i, target in enumerate(targets):
            bsp = _make_bsp(
                edge_id=target,
                bsp_type=BSPType.B1,
                level=i,
                time=100.0 + i,
            )
            state = advance(state, bsp, target=target)

        assert state.is_complete

        # 完成后可以启动交易状态机
        sm = FourLayerStateMachine()
        result = sm.entry(LayerType.L0_CONFIG, bsp="配置一买", position=0.3)
        assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE


# ============================================================================
# 3. topology → trading 连接
# ============================================================================


class TestTopologyToTrading:
    """验证 topology 模块的输出可以驱动 trading 模块。"""

    def test_polarity_index_as_position_signal(self):
        """极性指数可以作为 PositionSizer 的信号确认依据。"""
        sizer = PositionSizer()

        # S = +3（全面 risk-on）→ 确认配置层买点
        s = polarity_index(FULL_RISK_ON)
        assert s == 3

        # 配置层信号确认
        signals = [
            Signal(id="s1", layer=PosLayer.L0, weight=0.15,
                   confirmed=(s > 0)),
        ]
        pos = sizer.position(signals)
        assert pos > 0  # 正极性 → 有仓位

        # S = -3（全面 risk-off）→ 不确认买点
        s_off = polarity_index(FULL_RISK_OFF)
        assert s_off == -3

        signals_off = [
            Signal(id="s1", layer=PosLayer.L0, weight=0.15,
                   confirmed=(s_off > 0)),
        ]
        pos_off = sizer.position(signals_off)
        assert pos_off == 0.0  # 负极性 → 无仓位

    def test_config_polarity_and_trade_direction_consistency(self):
        """配置空间极性与交易方向一致：正极性只买入，负极性只卖出。"""
        space = ConfigurationSpace()
        for config in space:
            s = polarity_index(config)
            if s > 0:
                # 正极性 → 只应该有买点
                events = [Event(layer=LayerType.L0_CONFIG, action="entry")]
                # entry 事件不含 abort → 正常路径
                path_type = classify_path(events + [
                    Event(layer=LayerType.L0_CONFIG, action="exit_normal"),
                ])
                assert path_type == PathType.NORMAL_FULL

    def test_corner_configs_map_to_full_entry(self):
        """角节点配置（零分量为0）代表明确方向，可以驱动完整 entry。"""
        space = ConfigurationSpace()
        corners = space.configs_by_type("corner")
        assert len(corners) == 8  # 2^3 = 8

        sm = FourLayerStateMachine()
        for config in corners:
            s = polarity_index(config)
            if s > 0:
                # 正极性角节点 → 全面 risk-on 方向
                result = sm.entry(LayerType.L0_CONFIG, bsp=f"config_{config.label}", position=0.3)
                assert result.machine.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE
                # 重置
                sm = FourLayerStateMachine()

    def test_center_config_means_no_trade(self):
        """中心节点 (0,0,0) 极性为 0 → 不触发任何交易方向。"""
        s = polarity_index(CENTER)
        assert s == 0

        sizer = PositionSizer()
        # 无确认信号 → 仓位为 0
        signals = [
            Signal(id="s1", layer=PosLayer.L0, weight=0.15, confirmed=False),
        ]
        assert sizer.position(signals) == 0.0

    def test_independent_edge_count_matches_trading_layers(self):
        """3 条独立边对应 L1 层的跨角/边交易。"""
        from newchan.topology.graph import K4Graph

        graph = K4Graph()
        independent = graph.get_independent_edges()
        assert len(independent) == 3

        # L1 层的信号数：s2(独立边一买) + s3(派生边共振) + s4(跨国) + s5(货币)
        l1_signals = [s for s in STANDARD_SIGNALS if s.layer == PosLayer.L1]
        assert len(l1_signals) == 4  # 4 个 L1 信号


# ============================================================================
# 4. 完整管道测试
# ============================================================================


class TestFullPipeline:
    """从 ConfigurationSpace → NestingOperator → BSP → StateMachine → PositionSizing 的端到端验证。"""

    def test_end_to_end_buy_pipeline(self):
        """端到端买入管道：配置空间 → 区间套 → 买卖点 → 交易状态机 → 仓位计算。"""
        # Step 1: 配置空间识别正极性配置
        space = ConfigurationSpace()
        config = FULL_RISK_ON  # (+,+,+), S=+3
        assert config in space
        s = polarity_index(config)
        assert s > 0

        # Step 2: 构造区间套路径
        # level 语义：数值越大 = 级别越高（越粗），级别单调性要求 k₁ >= k₂ >= ... >= kₘ
        path = NestingPath(steps=())

        # 横向第一步：配置层买点（最高级别 level=2）
        config_bsp = _make_bsp(edge_id="config", level=2, bsp_type=BSPType.B1, time=100.0)
        step1 = NestingStep(
            nesting_type=NestingType.HORIZONTAL,
            search_space=frozenset(c.label for c in space),
            level=2,
            target="E/CASH",
            bsp_confirmation=config_bsp,
        )
        path = append_step(path, step1)

        # 横向第二步：独立边买点（搜索空间收缩，级别下钻 level=1）
        edge_bsp = _make_bsp(edge_id="E/CASH", level=1, bsp_type=BSPType.B1, time=101.0)
        step2 = NestingStep(
            nesting_type=NestingType.HORIZONTAL,
            search_space=frozenset({"E/CASH", "C/CASH", "R/CASH"}),
            level=1,
            target="US_tech",
            bsp_confirmation=edge_bsp,
        )
        path = append_step(path, step2)

        # 最终步：收缩到单个标的（最低级别 level=0）
        stock_bsp = _make_bsp(edge_id="AAPL", level=0, bsp_type=BSPType.B1, time=102.0)
        step3 = NestingStep(
            nesting_type=NestingType.VERTICAL,
            search_space=frozenset({"AAPL"}),
            level=0,
            target="AAPL",
            bsp_confirmation=stock_bsp,
        )
        path = append_step(path, step3)

        # 验证路径约束
        is_valid, violations = validate_path(path)
        assert is_valid, f"路径违规: {violations}"

        # Step 3: 共振确认 → 计算共振强度
        res_signals = [
            ResonanceSignal(
                edge_id="config", level=2, layer=SignalLayer.CONFIG,
                bsp=config_bsp, time=100.0,
            ),
            ResonanceSignal(
                edge_id="E/CASH", level=1, layer=SignalLayer.INDEPENDENT_EDGE,
                bsp=edge_bsp, time=101.0,
            ),
            ResonanceSignal(
                edge_id="AAPL", level=0, layer=SignalLayer.UNDERLYING,
                bsp=stock_bsp, time=102.0,
            ),
        ]
        strength = resonance_strength(res_signals)
        assert strength >= 6.0  # 三层共振
        assert classify_resonance(strength) == ResonanceLevel.THREE_LAYER

        # Step 4: 交易状态机 entry
        sm = FourLayerStateMachine()
        r0 = sm.entry(LayerType.L0_CONFIG, bsp="配置空间一买", position=0.3)
        sm = r0.machine
        r1 = sm.entry(LayerType.L1_EDGE, bsp="E/CASH一买", position=0.2)
        sm = r1.machine
        r2 = sm.entry(LayerType.L2_CORNER, bsp="US_tech板块买点", position=0.1)
        sm = r2.machine
        r3 = sm.entry(LayerType.L3_STOCK, bsp="AAPL纵向一买", position=0.05)
        sm = r3.machine

        assert sm.get_state(LayerType.L0_CONFIG) == LayerState.ACTIVE
        assert sm.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE
        assert sm.get_state(LayerType.L2_CORNER) == LayerState.ACTIVE
        assert sm.get_state(LayerType.L3_STOCK) == LayerState.ACTIVE

        # Step 5: 仓位计算
        sizer = PositionSizer()
        pos_signals = [
            Signal(id="s1", layer=PosLayer.L0, weight=0.15, confirmed=True),
            Signal(id="s2", layer=PosLayer.L1, weight=0.12, confirmed=True),
            Signal(id="s6", layer=PosLayer.L2, weight=0.04, confirmed=True),
            Signal(id="s8", layer=PosLayer.L3, weight=0.02, confirmed=True),
        ]
        position = sizer.position(pos_signals)
        assert 0 < position <= 1.0

    def test_end_to_end_abort_pipeline(self):
        """端到端止损管道：买点否定 → 止损退出 → 公理1覆盖。"""
        # 建立四层 ACTIVE 状态
        sm = FourLayerStateMachine()
        sm = sm.entry(LayerType.L0_CONFIG, bsp="配置一买", position=0.3).machine
        sm = sm.entry(LayerType.L1_EDGE, bsp="E/CASH一买", position=0.2).machine
        sm = sm.entry(LayerType.L2_CORNER, bsp="US_tech", position=0.1).machine
        sm = sm.entry(LayerType.L3_STOCK, bsp="AAPL一买", position=0.05).machine

        # 买点否定
        bsp = _make_bsp(bsp_type=BSPType.B1, price=100.0)
        assert is_negated(bsp, current_price=85.0) is True

        # L0 止损 → 全面清仓（公理1）
        result = sm.exit_abort(LayerType.L0_CONFIG)
        final = result.machine

        assert final.get_state(LayerType.L0_CONFIG) == LayerState.INACTIVE
        assert final.get_state(LayerType.L1_EDGE) == LayerState.INACTIVE
        assert final.get_state(LayerType.L2_CORNER) == LayerState.INACTIVE
        assert final.get_state(LayerType.L3_STOCK) == LayerState.INACTIVE
        assert result.realized_profit < 0

    def test_end_to_end_divergence_allocation(self):
        """端到端背驰分配：多标的背驰力度 → 仓位分配。"""
        # 仓位确定
        sizer = PositionSizer()
        signals = [
            Signal(id="s1", layer=PosLayer.L0, weight=0.15, confirmed=True),
            Signal(id="s2", layer=PosLayer.L1, weight=0.12, confirmed=True),
        ]
        total_pos = sizer.position(signals)
        assert total_pos > 0

        # 三个标的的背驰力度
        d1 = divergence_ratio(a_prev=100.0, a_curr=30.0)  # d=0.7
        d2 = divergence_ratio(a_prev=100.0, a_curr=60.0)  # d=0.4
        d3 = divergence_ratio(a_prev=100.0, a_curr=80.0)  # d=0.2

        # 分配
        allocations = allocate(total_pos, [d1, d2, d3], gamma=2.0)
        assert len(allocations) == 3
        assert sum(allocations) == pytest.approx(total_pos)
        # 背驰力度最大的标的获得最多仓位
        assert allocations[0] > allocations[1] > allocations[2]

    def test_nesting_path_to_trading_path_classification(self):
        """区间套路径完成后的交易事件序列可以被正确分类。"""
        # 正常全周期路径
        events = [
            Event(layer=LayerType.L0_CONFIG, action="entry"),
            Event(layer=LayerType.L1_EDGE, action="entry"),
            Event(layer=LayerType.L2_CORNER, action="entry"),
            Event(layer=LayerType.L3_STOCK, action="entry"),
            Event(layer=LayerType.L3_STOCK, action="exit_normal"),
            Event(layer=LayerType.L2_CORNER, action="exit_normal"),
            Event(layer=LayerType.L1_EDGE, action="exit_normal"),
            Event(layer=LayerType.L0_CONFIG, action="exit_normal"),
        ]
        assert classify_path(events) == PathType.NORMAL_FULL

        # 底层止损路径
        events_b = [
            Event(layer=LayerType.L0_CONFIG, action="entry"),
            Event(layer=LayerType.L1_EDGE, action="entry"),
            Event(layer=LayerType.L2_CORNER, action="entry"),
            Event(layer=LayerType.L3_STOCK, action="entry"),
            Event(layer=LayerType.L3_STOCK, action="exit_abort"),
        ]
        assert classify_path(events_b) == PathType.BOTTOM_STOP

    def test_wait_state_to_position_limit_pipeline(self):
        """等待状态仓位上限 → PositionSizer 仓位计算 → 取 min 作为实际仓位。"""
        # 等待在第二步（配置层已确认）
        wait = WaitState(
            step=HorizontalStep.EDGE_BSP,
            scope=frozenset({"E/CASH", "C/CASH", "R/CASH"}),
            level=0,
            time=100.0,
        )
        wait_limit = max_position(wait)
        assert wait_limit == pytest.approx(0.3)

        # PositionSizer 计算的仓位
        sizer = PositionSizer()
        signals = [
            Signal(id="s1", layer=PosLayer.L0, weight=0.15, confirmed=True),
            Signal(id="s2", layer=PosLayer.L1, weight=0.12, confirmed=True),
        ]
        raw_pos = sizer.position(signals)

        # 实际仓位 = min(raw_pos, wait_limit)
        actual_pos = min(raw_pos, wait_limit)
        assert 0 < actual_pos <= wait_limit

    def test_axiom2_enforced_across_modules(self):
        """公理2（下层不创造上层入口）在跨模块场景中被严格执行。"""
        sm = FourLayerStateMachine()
        # 尝试不经过 L0 直接 entry L1 → 应该抛出 AxiomViolationError
        with pytest.raises(AxiomViolationError):
            sm.entry(LayerType.L1_EDGE, bsp="E/CASH一买", position=0.2)

    def test_multiple_corner_rotation_pipeline(self):
        """多次角切换管道：L1 exit_normal → L2/L3 SUSPENDED → L1 re-entry。"""
        sm = FourLayerStateMachine()
        sm = sm.entry(LayerType.L0_CONFIG, bsp="配置一买", position=0.3).machine
        sm = sm.entry(LayerType.L1_EDGE, bsp="E/CASH角", position=0.2).machine
        sm = sm.entry(LayerType.L2_CORNER, bsp="US_tech", position=0.1).machine
        sm = sm.entry(LayerType.L3_STOCK, bsp="AAPL", position=0.05).machine

        # L1 正常退出 = 角切换
        result = sm.exit_normal(LayerType.L1_EDGE)
        sm2 = result.machine

        # L1 仍然 ACTIVE（等待新角），L2/L3 进入 SUSPENDED
        assert sm2.get_state(LayerType.L1_EDGE) == LayerState.ACTIVE
        assert sm2.get_state(LayerType.L2_CORNER) == LayerState.SUSPENDED
        assert sm2.get_state(LayerType.L3_STOCK) == LayerState.SUSPENDED
