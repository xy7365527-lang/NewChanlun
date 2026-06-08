"""纤维丛 pipeline 适配器 — 直积配置到纤维丛修正的桥梁。

236号谱系：纤维丛集成——直积近似 vs 纤维丛精确解的策略偏差量化。
240号谱系：纤维丛上下文集成——FiberTradingContext 接入策略层。

编排者指令：不替换直积（直积作为一级近似保留），而是并行运行，量化策略信号的偏差。

适配器模式：
  - 接收现有 pipeline 的 Configuration 输出
  - 用纤维丛联络计算修正后的 R 条件概率
  - 返回修正信号（FiberBundlePoint + 偏差度量）

240号扩展：
  - FiberTradingContext：包装 TradingContext + 纤维丛修正
  - create_fiber_context：从 TradingContext 创建纤维丛增强版
  - FiberSignalFilter：polarity 分歧检测 + 修正报告
"""

from __future__ import annotations

import math
from dataclasses import dataclass

from newchan.topology.config_space import Configuration, polarity_index
from newchan.topology.fiber_bundle import (
    BasePoint,
    Connection,
    FiberBundleConfigSpace,
    FiberBundlePoint,
    default_connection,
    default_fiber_bundle,
)


@dataclass(frozen=True, slots=True)
class FiberCorrection:
    """纤维丛对直积配置的修正结果。

    Attributes
    ----------
    original_config : Configuration
        原始直积配置。
    fiber_point : FiberBundlePoint
        纤维丛中对应的点（含条件概率）。
    r_prob_product : dict[int, float]
        直积假设下 R 的概率（均匀 1/3）。
    r_prob_fiber : dict[int, float]
        纤维丛联络下 R 的条件概率 P(R|E,C)。
    kl_divergence : float
        直积分布与纤维丛分布的 KL 散度。
    polarity_flipped : bool
        polarity_index 是否因纤维丛修正而改变。
    correction_magnitude : float
        修正幅度 = max|P_fiber(r) - P_product(r)| over r。
    """

    original_config: Configuration
    fiber_point: FiberBundlePoint
    r_prob_product: dict[int, float]
    r_prob_fiber: dict[int, float]
    kl_divergence: float
    polarity_flipped: bool
    correction_magnitude: float


def _kl_divergence(p: dict[int, float], q: dict[int, float]) -> float:
    """计算 KL(p || q)。

    Parameters
    ----------
    p : dict[int, float]
        真实分布（纤维丛）。
    q : dict[int, float]
        参考分布（直积）。

    Returns
    -------
    float
        KL 散度，非负。
    """
    kl = 0.0
    for k in p:
        pk = p[k]
        qk = q.get(k, 0.0)
        if pk > 0 and qk > 0:
            kl += pk * math.log(pk / qk)
    return kl


def _product_polarity(config: Configuration) -> int:
    """直积假设下的 polarity_index（与原始一致）。"""
    return polarity_index(config)


def _fiber_polarity(
    base: BasePoint,
    fiber_dist: dict[int, float],
    observed_r: int,
) -> int:
    """纤维丛联络下的期望 polarity_index。

    用纤维条件概率加权 R 的贡献，取最可能的 R 值构建配置后计算 polarity。
    当多个 R 值概率相同时（如底空间原点的均匀分布），使用实际观测的 R 值。

    Parameters
    ----------
    base : BasePoint
        底空间坐标。
    fiber_dist : dict[int, float]
        P(sigma_r | sigma_p, sigma_c)。
    observed_r : int
        实际观测的 sigma_r 值，用于打破概率平局。

    Returns
    -------
    int
        纤维丛下的 polarity_index。
    """
    max_prob = max(fiber_dist.values())
    modes = [r for r, p in fiber_dist.items() if abs(p - max_prob) < 1e-10]
    if len(modes) == 1:
        mode_r = modes[0]
    elif observed_r in modes:
        # 概率平局时使用实际观测值（保守原则：无联络偏好时不修改）
        mode_r = observed_r
    else:
        mode_r = modes[0]
    return base.sigma_p + base.sigma_c + mode_r


def compute_fiber_correction(
    config: Configuration,
    fb: FiberBundleConfigSpace | None = None,
) -> FiberCorrection:
    """计算纤维丛对直积配置的修正。

    Parameters
    ----------
    config : Configuration
        原始直积配置。
    fb : FiberBundleConfigSpace | None
        纤维丛配置空间。None 时使用 230号数据的默认纤维丛。

    Returns
    -------
    FiberCorrection
        修正结果。
    """
    if fb is None:
        fb = default_fiber_bundle()

    base = BasePoint(config.sigma_p.value, config.sigma_c.value)
    fiber_point = fb.get(
        config.sigma_p.value,
        config.sigma_c.value,
        config.sigma_r.value,
    )

    # 直积假设：R 均匀分布 1/3
    r_prob_product: dict[int, float] = {-1: 1.0 / 3.0, 0: 1.0 / 3.0, 1: 1.0 / 3.0}

    # 纤维丛联络：P(R|E,C)
    fiber_dist = fb.connection.fiber_distribution(base)
    r_prob_fiber: dict[int, float] = dict(fiber_dist)

    # KL 散度
    kl = _kl_divergence(r_prob_fiber, r_prob_product)

    # polarity 比较
    product_pol = _product_polarity(config)
    fiber_pol = _fiber_polarity(base, fiber_dist, config.sigma_r.value)
    polarity_flipped = product_pol != fiber_pol

    # 修正幅度
    correction_magnitude = max(
        abs(r_prob_fiber[r] - r_prob_product[r]) for r in (-1, 0, 1)
    )

    return FiberCorrection(
        original_config=config,
        fiber_point=fiber_point,
        r_prob_product=r_prob_product,
        r_prob_fiber=r_prob_fiber,
        kl_divergence=kl,
        polarity_flipped=polarity_flipped,
        correction_magnitude=correction_magnitude,
    )


class FiberPipelineAdapter:
    """纤维丛 pipeline 适配器。

    接收现有 pipeline 的 Configuration 输出，
    用纤维丛联络计算修正后的策略信号。

    不修改原始 pipeline——纯附加层。

    Attributes
    ----------
    fb : FiberBundleConfigSpace
        纤维丛配置空间。
    """

    __slots__ = ("_fb",)

    def __init__(self, fb: FiberBundleConfigSpace | None = None) -> None:
        self._fb = fb if fb is not None else default_fiber_bundle()

    @property
    def fb(self) -> FiberBundleConfigSpace:
        """纤维丛配置空间。"""
        return self._fb

    def correct(self, config: Configuration) -> FiberCorrection:
        """计算纤维丛修正。

        Parameters
        ----------
        config : Configuration
            原始直积配置。

        Returns
        -------
        FiberCorrection
            修正结果。
        """
        return compute_fiber_correction(config, self._fb)

    def batch_correct(
        self, configs: list[Configuration],
    ) -> list[FiberCorrection]:
        """批量计算纤维丛修正。

        Parameters
        ----------
        configs : list[Configuration]
            配置序列。

        Returns
        -------
        list[FiberCorrection]
            修正结果序列。
        """
        return [self.correct(c) for c in configs]

    def effective_dimension(self) -> float:
        """纤维丛的有效自由度 D_eff。"""
        return self._fb.effective_dimension()

    def global_kl_divergence(self) -> float:
        """纤维丛与直积的全局 KL 散度。"""
        return self._fb.kl_divergence_from_product()


# ── 240号：FiberTradingContext 集成 ─────────────────────────────


@dataclass(frozen=True, slots=True)
class FiberTradingContext:
    """纤维丛增强的交易上下文。

    包装（而非继承）TradingContext，附加纤维丛修正信息。
    不修改原始 TradingContext 的任何字段——纯附加层。

    Attributes
    ----------
    ctx : TradingContext
        原始交易上下文（完整保留）。
    fiber_correction : FiberCorrection
        纤维丛对当前配置的修正结果。
    fiber_polarity : int
        纤维丛修正后的 polarity_index。
    polarity_divergence : bool
        直积 polarity 与纤维丛 polarity 是否不同。
    correction_confidence : float
        修正的置信度（= KL 散度，越大表示纤维丛分布偏离直积越远）。
    """

    ctx: object  # TradingContext — 避免循环导入，运行时类型检查
    fiber_correction: FiberCorrection
    fiber_polarity: int
    polarity_divergence: bool
    correction_confidence: float

    @property
    def product_polarity(self) -> int:
        """直积假设下的 polarity（即原始 ctx.polarity）。"""
        return self.ctx.polarity  # type: ignore[union-attr]

    @property
    def config(self) -> Configuration:
        """当前配置（委托给原始 ctx）。"""
        return self.ctx.config  # type: ignore[union-attr]

    @property
    def timestamp(self) -> float:
        """当前时刻（委托给原始 ctx）。"""
        return self.ctx.timestamp  # type: ignore[union-attr]


def create_fiber_context(
    ctx: object,
    fb: FiberBundleConfigSpace | None = None,
) -> FiberTradingContext:
    """从现有 TradingContext 创建纤维丛增强版。

    Parameters
    ----------
    ctx : TradingContext
        原始交易上下文。
    fb : FiberBundleConfigSpace | None
        纤维丛配置空间。None 时使用 230号数据的默认纤维丛。

    Returns
    -------
    FiberTradingContext
        纤维丛增强的交易上下文。
    """
    config = ctx.config  # type: ignore[union-attr]
    correction = compute_fiber_correction(config, fb)

    base = BasePoint(config.sigma_p.value, config.sigma_c.value)
    fiber_dist = (fb or default_fiber_bundle()).connection.fiber_distribution(base)
    fiber_pol = _fiber_polarity(base, fiber_dist, config.sigma_r.value)

    product_pol = ctx.polarity  # type: ignore[union-attr]
    divergence = product_pol != fiber_pol

    return FiberTradingContext(
        ctx=ctx,
        fiber_correction=correction,
        fiber_polarity=fiber_pol,
        polarity_divergence=divergence,
        correction_confidence=correction.kl_divergence,
    )


class FiberSignalFilter:
    """纤维丛信号过滤器。

    当直积 polarity 与纤维丛 polarity 不同时（polarity_divergence=True），
    标记该交易日为"纤维丛修正区域"。

    策略含义：在修正区域内，直积给出的方向信号可能是错误的，
    纤维丛修正提供了更精确的方向判断。

    Attributes
    ----------
    adapter : FiberPipelineAdapter
        底层纤维丛适配器。
    kl_threshold : float
        KL 散度阈值——低于此值时认为修正不显著，不覆盖 polarity。
    """

    __slots__ = ("_adapter", "_kl_threshold")

    def __init__(
        self,
        adapter: FiberPipelineAdapter | None = None,
        kl_threshold: float = 0.0,
    ) -> None:
        self._adapter = adapter if adapter is not None else FiberPipelineAdapter()
        self._kl_threshold = kl_threshold

    @property
    def kl_threshold(self) -> float:
        """KL 散度阈值。"""
        return self._kl_threshold

    def should_override_polarity(self, fiber_ctx: FiberTradingContext) -> bool:
        """判断是否应覆盖直积 polarity。

        条件：
        1. polarity_divergence = True（两种模型给出不同方向）
        2. correction_confidence > kl_threshold（修正显著）

        Parameters
        ----------
        fiber_ctx : FiberTradingContext
            纤维丛增强的交易上下文。

        Returns
        -------
        bool
            True 表示应使用纤维丛 polarity 替代直积 polarity。
        """
        return (
            fiber_ctx.polarity_divergence
            and fiber_ctx.correction_confidence > self._kl_threshold
        )

    def correction_report(self, fiber_ctx: FiberTradingContext) -> dict:
        """生成修正报告。

        Parameters
        ----------
        fiber_ctx : FiberTradingContext
            纤维丛增强的交易上下文。

        Returns
        -------
        dict
            修正报告，包含 polarity 对比、KL 散度、R 概率分布等。
        """
        correction = fiber_ctx.fiber_correction
        return {
            "product_polarity": fiber_ctx.product_polarity,
            "fiber_polarity": fiber_ctx.fiber_polarity,
            "polarity_divergence": fiber_ctx.polarity_divergence,
            "should_override": self.should_override_polarity(fiber_ctx),
            "kl_divergence": correction.kl_divergence,
            "correction_magnitude": correction.correction_magnitude,
            "r_prob_product": dict(correction.r_prob_product),
            "r_prob_fiber": dict(correction.r_prob_fiber),
            "config": correction.original_config.as_tuple,
            "timestamp": fiber_ctx.timestamp,
        }
