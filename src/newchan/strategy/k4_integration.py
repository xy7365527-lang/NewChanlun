"""M2 K4 集成胶水层 — config_space 极性 → selection_pool 方向表。

填补 M2 管线缺口：

    topology/config_space.py
        Configuration Γ = (σ_p, σ_c, σ_r)  →  polarity_index(Γ) = S ∈ [-3, +3]
                            │
                            │  ← 本模块（胶水）
                            ▼
    strategy/selection_pool.py
        build_selection_pool(symbols, regime, k4_polarity) → 品种池+方向表

类型缺口本身很小（S 已是 int，与 k4_polarity: int 同型）。真正缺的是
build_selection_pool 还需要一个 ``regime`` 参数，而 Configuration 里没有现成的
regime。本模块的核心是 ``regime_from_config``——从 K4 的金/油边推导 ω regime。

认识论标注
----------
- ``polarity_from_config``：L0（config_space 既有定义，零信息增量）。
- ``regime_from_config``：L0 映射，但承载**经验假设**（ω=金/油的方向→宏观 regime）。
  这条假设曾被真实数据反向证伪（见谱系引用），故它是【待证伪假设】，
  不是已确认结论。是否产生 alpha 属 L2，由 analysis/m2_e2e_backtest.py 检验。
- ``build_pool_from_config`` / ``long_allowed``：L1（管线拼装 + 门控读数）。

边界绑定说明
------------
config_space 的三个 σ 槽位（sigma_p/sigma_c/sigma_r）是**槽位**，其语义由喂入的
数据决定。本项目 M2 e2e 按用户指定绑定为 (Equity=ES, Gold=GC, Oil=CL)，即

    sigma_p := σ(P/M)   (股指, ES)    ← 正典：P=生产资本金融化（330号）
    sigma_c := σ(Au/$)  (黄金, GC)
    sigma_r := σ(Oil/$) (原油, CL)

⚠ 消费者层绑定偏离（528号待统一）：本模块的 M2 绑定与正典折叠通道映射
（topology/data_mapping.py 单一真相源）不一致——
    正典：σ_C := σ(C/M)=广义商品(DBC)，σ_R := σ(R/M)=不动产(TLT)；金/油是折叠通道观测量。
    M2 本模块：σ_c=黄金、σ_r=原油（把折叠通道观测量塞进配置主轴）。
这是 528号识别的"四套冲突映射"之一。重写 M2 绑定为读取 data_mapping 单一真相源属
528号下游"复核回测↔实时可比性"任务，不在本次结构重构范围内，此处诚实标注不静默。

谱系引用
--------
- 527号：config σ 已结算为走势方向态（σ 可 −1→+1 跳变）。本模块读 σ 为走势方向态，一致。
- project_omega_regime_falsified：ω→美股方向曾被真实数据反向证伪（p=0.069 方向相反）。
  → regime_from_config 是待证伪假设，不得包装为确认。
- project_config_sigma_ontology（527号）：σ 是走势方向态，可跳变。
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.strategy.asset_classifier import AssetType
from newchan.strategy.omega_regime import OmegaRegime
from newchan.strategy.selection_pool import (
    SelectionConfidence,
    SelectionEntry,
    SelectionPool,
    build_selection_pool,
)
from newchan.topology.config_space import Configuration, polarity_index


# ═══════════════════════════════════════════════════════════════
# σ → 极性指数 S（纯 L0 转发）
# ═══════════════════════════════════════════════════════════════


def polarity_from_config(config: Configuration) -> int:
    """K4 配置 Γ → 极性指数 S。

    转发 config_space.polarity_index，使本胶水层成为 M2 管线的单一入口。

    Parameters
    ----------
    config : Configuration
        K4 配置三元组 (σ_e, σ_c, σ_r)。

    Returns
    -------
    int
        极性指数 S ∈ {-3, -2, -1, 0, +1, +2, +3}。
    """
    return polarity_index(config)


# ═══════════════════════════════════════════════════════════════
# Γ → ω regime（承载经验假设的唯一函数）
# ═══════════════════════════════════════════════════════════════


def regime_from_config(config: Configuration) -> OmegaRegime:
    """从 K4 配置的金/油边推导 ω regime。

    ω = gold / oil（omega_regime.py 定义）。在 K4 走势方向态语境下，ω 的方向由
    金边方向与油边方向的**相对关系**给出：

        relative = σ(Au/$) − σ(Oil/$)
          relative > 0  → 金强于油 → ω 上升 → BULL_COMMODITY（信用收缩，大宗做多）
          relative < 0  → 油强于金 → ω 下降 → BULL_EQUITY（信用扩张，美股做多）
          relative == 0 → 金油同向同速 → NEUTRAL（ω 无明确方向）

    仅使用金边（sigma_c）与油边（sigma_r）——与 ω=金/油 的定义严格一致，
    不掺入股指边（sigma_p）。

    ⚠ 经验假设警告：此映射方向（ω→regime）曾被真实数据反向证伪
    （project_omega_regime_falsified，p=0.069 方向相反）。本函数保留 omega_regime.py
    的既有方向约定以维持定义一致性，但其有效域属【待证伪】，需 L2 回测检验。

    Parameters
    ----------
    config : Configuration
        K4 配置。sigma_c=σ(Au/$)，sigma_r=σ(Oil/$)（见模块文档边界绑定）。

    Returns
    -------
    OmegaRegime
    """
    relative = config.sigma_c.value - config.sigma_r.value
    if relative > 0:
        return OmegaRegime.BULL_COMMODITY
    if relative < 0:
        return OmegaRegime.BULL_EQUITY
    return OmegaRegime.NEUTRAL


# ═══════════════════════════════════════════════════════════════
# 完整胶水：Γ → 品种池+方向表
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class K4SelectionState:
    """K4 配置驱动的选股状态快照——管线一次贯通的完整产物。

    Attributes
    ----------
    config : Configuration
        输入 K4 配置 Γ。
    polarity : int
        极性指数 S = polarity_index(Γ)。
    regime : OmegaRegime
        regime_from_config(Γ) 推导的 ω regime。
    pool : SelectionPool
        build_selection_pool(symbols, regime, S) 输出的品种池+方向表。
    """

    config: Configuration
    polarity: int
    regime: OmegaRegime
    pool: SelectionPool


def build_pool_from_config(
    symbols: tuple[str, ...],
    config: Configuration,
) -> SelectionPool:
    """胶水主函数：K4 配置 → 品种池+方向表。

    管线：
      S      = polarity_from_config(config)
      regime = regime_from_config(config)
      pool   = build_selection_pool(symbols, regime, S)

    Parameters
    ----------
    symbols : tuple[str, ...]
        候选标的代码列表。
    config : Configuration
        当前 K4 配置 Γ。

    Returns
    -------
    SelectionPool
        完整品种池+方向表（regime 与 k4_polarity 均源自同一 Γ，单一数据源）。
    """
    s = polarity_from_config(config)
    regime = regime_from_config(config)
    return build_selection_pool(symbols, regime, s)


def select_state_from_config(
    symbols: tuple[str, ...],
    config: Configuration,
) -> K4SelectionState:
    """build_pool_from_config 的带元信息版本——附带 polarity/regime 以便审计。

    Parameters
    ----------
    symbols : tuple[str, ...]
        候选标的代码列表。
    config : Configuration
        当前 K4 配置 Γ。

    Returns
    -------
    K4SelectionState
    """
    s = polarity_from_config(config)
    regime = regime_from_config(config)
    pool = build_selection_pool(symbols, regime, s)
    return K4SelectionState(
        config=config,
        polarity=s,
        regime=regime,
        pool=pool,
    )


# ═══════════════════════════════════════════════════════════════
# 门控：方向表 → 是否放行做多（供 M1 回测消费）
# ═══════════════════════════════════════════════════════════════


def long_allowed(entry: SelectionEntry) -> bool:
    """K4 选股门控：在当前 regime/极性下是否放行该标的的**做多入场**。

    M1 的 E 版本（背驰定位器）是纯多头引擎——无做空逻辑。故 K4 选股对它的
    "方向驱动"退化为**多头入场门控**：

      LOW 置信  → 不放行做多（宏观 regime 不利该资产类型）
      MEDIUM/HIGH → 放行做多

    置信度由 selection_pool.apply_regime_adjustment 既有逻辑给出（regime + 极性
    共同决定），本函数不引入任何新的自由参数——门控完全由既有品种池逻辑决定。

    边界条件：本门控只作用于**入场**，不强制平仓。已持仓由 E 引擎自身的趋势顶
    背驰逻辑离场（保守门控：risk-off 阻止开新仓，不打断既有持仓的自然离场）。

    Parameters
    ----------
    entry : SelectionEntry
        品种池中该标的的方向表条目。

    Returns
    -------
    bool
        True = 放行做多入场；False = 阻止做多入场。
    """
    return entry.confidence is not SelectionConfidence.LOW
