"""M2 选股层——品种池+方向表。

将 K4 极性指数、ω regime、资产分类三个信号合并，
产出一张完整的品种池+方向表，供 M1 回测脚本消费。

管线：
  classify(symbol) → AssetProfile（基础方向）
  detect_regime(ω_series) → OmegaState（regime）
  polarity_index(config) → int（K4极性）
  ────────────────────────────────────────
  apply_regime_adjustment() → 最终方向表（regime修正后）

认识论标注：L0（从M1发现+ω理论推导，尚需L2验证）。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.strategy.asset_classifier import (
    AssetProfile,
    AssetType,
    DirectionPolicy,
    classify,
)
from newchan.strategy.omega_regime import OmegaRegime


class SelectionConfidence(Enum):
    """选股置信度。regime与K4极性一致时置信度更高。"""

    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"


@dataclass(frozen=True, slots=True)
class SelectionEntry:
    """品种池中的单条记录。

    Attributes
    ----------
    symbol : str
        标的代码。
    asset_type : AssetType
        资产类型（index/stock/commodity）。
    direction : DirectionPolicy
        最终方向策略（经 regime 修正后）。
    confidence : SelectionConfidence
        选入品种池的置信度。
    reason : str
        选入/方向的综合理由。
    """

    symbol: str
    asset_type: AssetType
    direction: DirectionPolicy
    confidence: SelectionConfidence
    reason: str


@dataclass(frozen=True, slots=True)
class SelectionPool:
    """完整的品种池+方向表。

    Attributes
    ----------
    entries : tuple[SelectionEntry, ...]
        品种池条目（按置信度降序）。
    regime : OmegaRegime
        当前 ω regime。
    k4_polarity : int
        当前 K4 极性指数（-3 到 +3）。
    """

    entries: tuple[SelectionEntry, ...]
    regime: OmegaRegime
    k4_polarity: int

    def symbols(self) -> tuple[str, ...]:
        """返回所有标的代码。"""
        return tuple(e.symbol for e in self.entries)

    def long_only_symbols(self) -> tuple[str, ...]:
        """返回仅做多的标的。"""
        return tuple(
            e.symbol for e in self.entries
            if e.direction is DirectionPolicy.LONG_ONLY
        )

    def both_symbols(self) -> tuple[str, ...]:
        """返回可多可空的标的。"""
        return tuple(
            e.symbol for e in self.entries
            if e.direction is DirectionPolicy.BOTH
        )

    def to_dict(self) -> dict[str, dict[str, str]]:
        """输出为 M1 回测脚本可消费的字典格式。"""
        return {
            e.symbol: {
                "type": e.asset_type.value,
                "direction": e.direction.value,
                "confidence": e.confidence.value,
                "reason": e.reason,
            }
            for e in self.entries
        }


# ═══════════════════════════════════════════════════════════════
# 核心：regime 调整逻辑
# ═══════════════════════════════════════════════════════════════


def apply_regime_adjustment(
    profile: AssetProfile,
    regime: OmegaRegime,
    k4_polarity: int,
) -> SelectionEntry:
    """将 regime 信号叠加到基础方向策略上，产出最终方向。

    这是 M2 的核心决策函数——regime 如何修正方向表。

    规则（待用户确认）：
    1. 指数 ETF 的 long_only 是硬约束，regime 不改变方向
    2. regime 影响置信度：
       - equity_bull + 股票 → HIGH
       - commodity_bull + 大宗 → HIGH
       - 逆 regime（equity_bull + 大宗）→ LOW
       - 中性 → MEDIUM
    3. K4 极性与 regime 共振 → 置信度提升

    Parameters
    ----------
    profile : AssetProfile
        标的基础画像。
    regime : OmegaRegime
        当前 ω regime。
    k4_polarity : int
        K4 极性指数。

    Returns
    -------
    SelectionEntry
        经 regime 修正后的品种池条目。
    """
    direction = profile.base_direction
    confidence = SelectionConfidence.MEDIUM
    reasons: list[str] = [profile.reason]

    if regime is OmegaRegime.BULL_EQUITY:
        if profile.asset_type is AssetType.STOCK:
            confidence = SelectionConfidence.HIGH
            reasons.append("ω信用扩张利好股票")
        elif profile.asset_type is AssetType.INDEX:
            confidence = SelectionConfidence.HIGH
            reasons.append("ω信用扩张利好指数")
        elif profile.asset_type is AssetType.COMMODITY:
            confidence = SelectionConfidence.LOW
            reasons.append("ω信用扩张对大宗不利")

    elif regime is OmegaRegime.BULL_COMMODITY:
        if profile.asset_type is AssetType.COMMODITY:
            confidence = SelectionConfidence.HIGH
            reasons.append("ω信用收缩利好大宗")
        elif profile.asset_type in (AssetType.STOCK, AssetType.INDEX):
            confidence = SelectionConfidence.LOW
            reasons.append("ω信用收缩对股票不利")

    if k4_polarity > 0 and confidence is SelectionConfidence.HIGH:
        reasons.append("K4极性risk-on共振")
    elif k4_polarity < 0 and confidence is SelectionConfidence.HIGH:
        if profile.asset_type is not AssetType.COMMODITY:
            confidence = SelectionConfidence.MEDIUM
            reasons.append("K4极性risk-off削弱股票置信")

    return SelectionEntry(
        symbol=profile.symbol,
        asset_type=profile.asset_type,
        direction=direction,
        confidence=confidence,
        reason="；".join(reasons),
    )


# ═══════════════════════════════════════════════════════════════
# 品种池构建
# ═══════════════════════════════════════════════════════════════

_CONFIDENCE_ORDER = {
    SelectionConfidence.HIGH: 0,
    SelectionConfidence.MEDIUM: 1,
    SelectionConfidence.LOW: 2,
}


def build_selection_pool(
    symbols: tuple[str, ...],
    regime: OmegaRegime,
    k4_polarity: int,
) -> SelectionPool:
    """从标的列表 + regime + K4极性构建品种池。

    Parameters
    ----------
    symbols : tuple[str, ...]
        候选标的代码列表。
    regime : OmegaRegime
        当前 ω regime。
    k4_polarity : int
        K4 极性指数（-3 到 +3）。

    Returns
    -------
    SelectionPool
        完整的品种池+方向表。
    """
    entries = []
    for symbol in symbols:
        profile = classify(symbol)
        entry = apply_regime_adjustment(profile, regime, k4_polarity)
        entries.append(entry)

    sorted_entries = tuple(sorted(
        entries,
        key=lambda e: _CONFIDENCE_ORDER[e.confidence],
    ))

    return SelectionPool(
        entries=sorted_entries,
        regime=regime,
        k4_polarity=k4_polarity,
    )


# ═══════════════════════════════════════════════════════════════
# M1 回测接口
# ═══════════════════════════════════════════════════════════════


def pool_to_backtest_config(pool: SelectionPool) -> dict[str, bool]:
    """将品种池转换为 M1 回测参数。

    Returns
    -------
    dict[str, bool]
        {symbol: short_enabled} 映射。
        long_only 标的 short_enabled=False。
        both 标的 short_enabled=True。
    """
    return {
        e.symbol: e.direction is DirectionPolicy.BOTH
        for e in pool.entries
    }
