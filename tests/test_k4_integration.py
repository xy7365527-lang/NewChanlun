"""k4_integration 胶水层单元测试。

测试覆盖：
  - polarity_from_config 对 config_space.polarity_index 的忠实转发
  - regime_from_config：金/油相对方向 → ω regime（全 9 种金油组合）
  - build_pool_from_config：S 与 regime 单一数据源一致性
  - select_state_from_config：元信息打包正确
  - long_allowed：LOW 拦截 / MEDIUM·HIGH 放行 的门控边界

认识论标注：L1（验证管线拼装正确性，输入为构造的 Configuration，零信息增量）。
依赖的 regime_from_config 经验假设的有效域由 analysis/m2_e2e_backtest.py（L2）检验。
"""

from __future__ import annotations

import itertools

import pytest

from newchan.strategy.asset_classifier import DirectionPolicy
from newchan.strategy.k4_integration import (
    K4SelectionState,
    build_pool_from_config,
    long_allowed,
    polarity_from_config,
    regime_from_config,
    select_state_from_config,
)
from newchan.strategy.omega_regime import OmegaRegime
from newchan.strategy.selection_pool import (
    SelectionConfidence,
    SelectionEntry,
    SelectionPool,
    build_selection_pool,
)
from newchan.topology.config_space import (
    CENTER,
    FULL_RISK_OFF,
    FULL_RISK_ON,
    Configuration,
    WalkDirection,
    polarity_index,
)

UP = WalkDirection.UP
FLAT = WalkDirection.FLAT
DOWN = WalkDirection.DOWN

_ALL_CONFIGS = [
    Configuration(e, c, r)
    for e, c, r in itertools.product(WalkDirection, repeat=3)
]


# ── polarity_from_config：忠实转发 ────────────────────────────


@pytest.mark.unit
@pytest.mark.parametrize("config", _ALL_CONFIGS)
def test_polarity_forwards_polarity_index(config: Configuration) -> None:
    """polarity_from_config 必须与 config_space.polarity_index 逐配置相等。"""
    assert polarity_from_config(config) == polarity_index(config)


@pytest.mark.unit
def test_polarity_range() -> None:
    """所有配置的极性指数落在 [-3, +3]。"""
    vals = {polarity_from_config(c) for c in _ALL_CONFIGS}
    assert vals == {-3, -2, -1, 0, 1, 2, 3}


# ── regime_from_config：金/油相对方向 → ω regime ──────────────


@pytest.mark.unit
@pytest.mark.parametrize(
    "sigma_c,sigma_r,expected",
    [
        # 金强于油 → ω↑ → BULL_COMMODITY
        (UP, FLAT, OmegaRegime.BULL_COMMODITY),
        (UP, DOWN, OmegaRegime.BULL_COMMODITY),
        (FLAT, DOWN, OmegaRegime.BULL_COMMODITY),
        # 油强于金 → ω↓ → BULL_EQUITY
        (FLAT, UP, OmegaRegime.BULL_EQUITY),
        (DOWN, UP, OmegaRegime.BULL_EQUITY),
        (DOWN, FLAT, OmegaRegime.BULL_EQUITY),
        # 金油同向同速 → NEUTRAL
        (UP, UP, OmegaRegime.NEUTRAL),
        (FLAT, FLAT, OmegaRegime.NEUTRAL),
        (DOWN, DOWN, OmegaRegime.NEUTRAL),
    ],
)
def test_regime_from_gold_oil_relative(
    sigma_c: WalkDirection,
    sigma_r: WalkDirection,
    expected: OmegaRegime,
) -> None:
    """regime 只由 (金边 − 油边) 的符号决定，与股指边无关。"""
    # 股指边取所有可能值，结果都应相同（验证不掺入 sigma_p）
    for sigma_p in WalkDirection:
        cfg = Configuration(sigma_p=sigma_p, sigma_c=sigma_c, sigma_r=sigma_r)
        assert regime_from_config(cfg) is expected


@pytest.mark.unit
def test_regime_ignores_equity_edge() -> None:
    """股指边方向变化不改变 regime（regime 仅源自金/油）。"""
    base = {OmegaRegime.BULL_COMMODITY: None}
    regimes = {
        regime_from_config(Configuration(e, UP, DOWN))
        for e in WalkDirection
    }
    assert regimes == {OmegaRegime.BULL_COMMODITY}
    assert base  # 占位防止 flake8 未用


@pytest.mark.unit
def test_regime_special_configs() -> None:
    """特殊配置：全 risk-on/off 金油同向 → NEUTRAL；中心 → NEUTRAL。"""
    assert regime_from_config(FULL_RISK_ON) is OmegaRegime.NEUTRAL
    assert regime_from_config(FULL_RISK_OFF) is OmegaRegime.NEUTRAL
    assert regime_from_config(CENTER) is OmegaRegime.NEUTRAL


# ── build_pool_from_config：单一数据源一致性 ──────────────────


@pytest.mark.unit
def test_build_pool_returns_selection_pool() -> None:
    pool = build_pool_from_config(("QQQ", "OKLO"), FULL_RISK_ON)
    assert isinstance(pool, SelectionPool)
    assert set(pool.symbols()) == {"QQQ", "OKLO"}


@pytest.mark.unit
@pytest.mark.parametrize("config", _ALL_CONFIGS)
def test_build_pool_single_source_consistency(config: Configuration) -> None:
    """品种池的 k4_polarity / regime 必须与从同一 config 独立推导的值一致。"""
    symbols = ("QQQ", "OKLO", "NYMEX:CL1!")
    pool = build_pool_from_config(symbols, config)
    assert pool.k4_polarity == polarity_from_config(config)
    assert pool.regime is regime_from_config(config)


@pytest.mark.unit
@pytest.mark.parametrize("config", _ALL_CONFIGS)
def test_build_pool_equivalent_to_manual_chain(config: Configuration) -> None:
    """胶水函数等价于手工链：build_selection_pool(symbols, regime, S)。"""
    symbols = ("QQQ", "OKLO")
    glued = build_pool_from_config(symbols, config)
    manual = build_selection_pool(
        symbols,
        regime_from_config(config),
        polarity_from_config(config),
    )
    assert glued == manual


# ── select_state_from_config：元信息打包 ──────────────────────


@pytest.mark.unit
@pytest.mark.parametrize("config", _ALL_CONFIGS)
def test_select_state_fields(config: Configuration) -> None:
    state = select_state_from_config(("QQQ", "OKLO"), config)
    assert isinstance(state, K4SelectionState)
    assert state.config is config
    assert state.polarity == polarity_from_config(config)
    assert state.regime is regime_from_config(config)
    assert state.pool.k4_polarity == state.polarity
    assert state.pool.regime is state.regime


@pytest.mark.unit
def test_select_state_frozen() -> None:
    """K4SelectionState 不可变。"""
    state = select_state_from_config(("QQQ",), FULL_RISK_ON)
    with pytest.raises(Exception):
        state.polarity = 0  # type: ignore[misc]


# ── long_allowed：门控边界 ────────────────────────────────────


def _entry(conf: SelectionConfidence) -> SelectionEntry:
    return SelectionEntry(
        symbol="X",
        asset_type=__import__(
            "newchan.strategy.asset_classifier", fromlist=["AssetType"]
        ).AssetType.STOCK,
        direction=DirectionPolicy.BOTH,
        confidence=conf,
        reason="test",
    )


@pytest.mark.unit
def test_long_allowed_blocks_only_low() -> None:
    """LOW 拦截做多；MEDIUM/HIGH 放行。"""
    assert long_allowed(_entry(SelectionConfidence.HIGH)) is True
    assert long_allowed(_entry(SelectionConfidence.MEDIUM)) is True
    assert long_allowed(_entry(SelectionConfidence.LOW)) is False


@pytest.mark.unit
def test_long_allowed_on_bull_commodity_blocks_equity() -> None:
    """端到端门控语义：BULL_COMMODITY（金>油）使 QQQ/OKLO 做多被拦截。"""
    # 金强于油 → BULL_COMMODITY → 股票/指数 LOW → 拦截
    cfg = Configuration(sigma_p=UP, sigma_c=UP, sigma_r=DOWN)
    pool = build_pool_from_config(("QQQ", "OKLO"), cfg)
    for e in pool.entries:
        assert e.confidence is SelectionConfidence.LOW
        assert long_allowed(e) is False
