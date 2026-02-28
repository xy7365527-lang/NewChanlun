"""纤维丛配置空间 — 从直积到纤维丛的拓扑重构。

231号谱系：ConfigurationSpace 纤维丛重构。

230号否定性结果：
  - E-C 偏相关 = -0.029 → 控制 $ 后独立 → 直积因子
  - E-R 偏相关 = -0.336 → 结构性耦合 → 不可消除
  - C-R 偏相关 =  0.153 → 弱残余 → 二阶效应

直积结构 {-1,0,+1}^3 是一级近似。严格结构是纤维丛：
  - 底空间 B = {-1,0,+1}^2 (E × C，直积，因为 E-C 独立)
  - 纤维 F = {-1,0,+1} (R 的取值空间)
  - 联络 ω: B → Δ(F)，由 E-R 偏相关 (-0.336) 和 C-R 偏相关 (0.153) 参数化
  - 全空间 = B ×_ω F，27 种配置的代数枚举不变，但概率结构改变
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Mapping

from newchan.topology.config_space import (
    Configuration,
    WalkDirection,
    polarity_index,
)

# ── 联络参数（从 230号数据校准）──────────────────────────────

# 偏相关值（控制 $ 后）
PARTIAL_CORR_ER: float = -0.3355
PARTIAL_CORR_CR: float = 0.1531

# WalkDirection 值域
_FIBER_VALUES: tuple[int, ...] = (-1, 0, 1)
_BASE_VALUES: tuple[int, ...] = (-1, 0, 1)


@dataclass(frozen=True, slots=True)
class BasePoint:
    """底空间 B 中的一个点：(sigma_e, sigma_c)。

    E 和 C 构成直积因子（230号：E-C 偏相关 ≈ 0）。
    """

    sigma_e: int
    sigma_c: int

    def __post_init__(self) -> None:
        if self.sigma_e not in _BASE_VALUES:
            raise ValueError(f"sigma_e 必须在 {{-1,0,+1}} 内，收到：{self.sigma_e}")
        if self.sigma_c not in _BASE_VALUES:
            raise ValueError(f"sigma_c 必须在 {{-1,0,+1}} 内，收到：{self.sigma_c}")

    @property
    def as_tuple(self) -> tuple[int, int]:
        return (self.sigma_e, self.sigma_c)


@dataclass(frozen=True, slots=True)
class Connection:
    """纤维丛联络：给定底空间点，决定纤维上的条件概率分布。

    模型：P(sigma_r | sigma_e, sigma_c) ∝ exp(beta_er * sigma_e * sigma_r
                                                + beta_cr * sigma_c * sigma_r)

    beta_er < 0 编码 E-R 反向耦合（risk-on/off 跷跷板）。
    beta_cr > 0 编码 C-R 正向耦合（避险同向性）。

    Attributes
    ----------
    beta_er : float
        E-R 耦合强度。负值 = 反向耦合。
    beta_cr : float
        C-R 耦合强度。正值 = 正向耦合。
    """

    beta_er: float
    beta_cr: float

    def fiber_logits(self, base: BasePoint) -> dict[int, float]:
        """计算给定底空间点上纤维各状态的 logit。

        Parameters
        ----------
        base : BasePoint
            底空间坐标 (sigma_e, sigma_c)。

        Returns
        -------
        dict[int, float]
            {sigma_r: logit} 映射。
        """
        return {
            sigma_r: (
                self.beta_er * base.sigma_e * sigma_r
                + self.beta_cr * base.sigma_c * sigma_r
            )
            for sigma_r in _FIBER_VALUES
        }

    def fiber_distribution(self, base: BasePoint) -> dict[int, float]:
        """计算给定底空间点上纤维的条件概率分布。

        P(sigma_r | sigma_e, sigma_c) = softmax(logits)

        Parameters
        ----------
        base : BasePoint
            底空间坐标 (sigma_e, sigma_c)。

        Returns
        -------
        dict[int, float]
            {sigma_r: probability} 映射，概率和为 1。
        """
        logits = self.fiber_logits(base)
        max_logit = max(logits.values())
        exp_vals = {k: math.exp(v - max_logit) for k, v in logits.items()}
        total = sum(exp_vals.values())
        return {k: v / total for k, v in exp_vals.items()}

    def is_product(self, tol: float = 1e-6) -> bool:
        """联络是否退化为直积（平坦联络）。

        当 beta_er ≈ 0 且 beta_cr ≈ 0 时，纤维丛退化为直积。
        """
        return abs(self.beta_er) < tol and abs(self.beta_cr) < tol


def calibrate_connection(
    config_counts: Mapping[tuple[int, int, int], int],
) -> Connection:
    """从经验配置频率校准联络参数（条件 MLE）。

    最大化 Π P(sigma_r | sigma_e, sigma_c; beta)，不假设底空间先验。
    """
    total = sum(config_counts.values())
    if total == 0:
        return Connection(beta_er=0.0, beta_cr=0.0)

    # 按底空间点分组
    base_groups: dict[tuple[int, int], dict[int, int]] = {}
    for (e, c, r), count in config_counts.items():
        base_groups.setdefault((e, c), {})[r] = count

    beta_er = 0.0
    beta_cr = 0.0
    lr = 0.5
    for _ in range(2000):
        grad_er = 0.0
        grad_cr = 0.0
        conn = Connection(beta_er, beta_cr)

        for (e, c), r_counts in base_groups.items():
            n_base = sum(r_counts.values())
            if n_base == 0:
                continue
            dist = conn.fiber_distribution(BasePoint(e, c))
            # 指数族梯度：n * (empirical_stat - model_stat)
            for r, cnt in r_counts.items():
                grad_er += (cnt - n_base * dist.get(r, 0.0)) * e * r
                grad_cr += (cnt - n_base * dist.get(r, 0.0)) * c * r

        grad_er /= total
        grad_cr /= total
        beta_er += lr * grad_er
        beta_cr += lr * grad_cr

        if abs(grad_er) < 1e-8 and abs(grad_cr) < 1e-8:
            break

    return Connection(beta_er=beta_er, beta_cr=beta_cr)


# ── 纤维丛配置空间 ──────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class FiberBundlePoint:
    """纤维丛全空间中的一个点。

    包含底空间坐标、纤维坐标，以及由联络决定的条件概率。

    Attributes
    ----------
    base : BasePoint
        底空间坐标 (sigma_e, sigma_c)。
    sigma_r : int
        纤维坐标。
    fiber_prob : float
        P(sigma_r | sigma_e, sigma_c)，由联络决定。
    """

    base: BasePoint
    sigma_r: int
    fiber_prob: float

    @property
    def config(self) -> Configuration:
        """转换为 Configuration（向后兼容）。"""
        return Configuration(
            sigma_e=WalkDirection(self.base.sigma_e),
            sigma_c=WalkDirection(self.base.sigma_c),
            sigma_r=WalkDirection(self.sigma_r),
        )

    @property
    def as_tuple(self) -> tuple[int, int, int]:
        return (self.base.sigma_e, self.base.sigma_c, self.sigma_r)

    @property
    def polarity(self) -> int:
        return polarity_index(self.config)

    @property
    def is_parallel(self) -> bool:
        """是否为平行截面上的点。

        平行截面 = 沿联络的自然选择。
        当 fiber_prob 是纤维分布的 mode（最大概率态）时为 True。
        """
        # 由 FiberBundleConfigSpace 在构建时设置
        # 这里通过概率阈值判断：> 1/3 即高于均匀分布
        return self.fiber_prob > 1.0 / 3.0 + 1e-9


class FiberBundleConfigSpace:
    """纤维丛配置空间：27 种配置 + 联络结构。

    代数枚举不变（27 种配置全部保留）。
    拓扑结构改变：配置概率不再均匀 1/27，而是由纤维丛结构决定。

    不可变。创建后只读。
    """

    __slots__ = (
        "_connection",
        "_points",
        "_by_tuple",
        "_base_space",
    )

    def __init__(self, connection: Connection) -> None:
        self._connection = connection

        points: list[FiberBundlePoint] = []
        by_tuple: dict[tuple[int, int, int], FiberBundlePoint] = {}
        base_space: dict[tuple[int, int], BasePoint] = {}

        for e in _BASE_VALUES:
            for c in _BASE_VALUES:
                base = BasePoint(e, c)
                base_space[base.as_tuple] = base
                dist = connection.fiber_distribution(base)
                for r in _FIBER_VALUES:
                    pt = FiberBundlePoint(
                        base=base,
                        sigma_r=r,
                        fiber_prob=dist[r],
                    )
                    points.append(pt)
                    by_tuple[pt.as_tuple] = pt

        self._points: tuple[FiberBundlePoint, ...] = tuple(points)
        self._by_tuple: dict[tuple[int, int, int], FiberBundlePoint] = by_tuple
        self._base_space: dict[tuple[int, int], BasePoint] = base_space

    @property
    def connection(self) -> Connection:
        """纤维丛联络。"""
        return self._connection

    @property
    def size(self) -> int:
        """配置总数（恒为 27）。"""
        return len(self._points)

    def __len__(self) -> int:
        return self.size

    def __iter__(self):
        return iter(self._points)

    def get(self, e: int, c: int, r: int) -> FiberBundlePoint:
        """按数值获取纤维丛点。"""
        key = (e, c, r)
        if key not in self._by_tuple:
            raise KeyError(f"非法配置：{key}，每个分量必须在 {{-1, 0, +1}} 内")
        return self._by_tuple[key]

    def fiber_over(self, base: BasePoint) -> tuple[FiberBundlePoint, ...]:
        """返回底空间某点上方的整条纤维（3 个点，sigma_r = -1, 0, +1）。"""
        return tuple(
            self._by_tuple[(base.sigma_e, base.sigma_c, r)]
            for r in _FIBER_VALUES
        )

    def base_points(self) -> tuple[BasePoint, ...]:
        """返回底空间的 9 个点。"""
        return tuple(self._base_space.values())

    def joint_probability(self, e: int, c: int, r: int) -> float:
        """配置的联合概率。P(e,c,r) = (1/9) * P(r|e,c)。"""
        pt = self.get(e, c, r)
        return pt.fiber_prob / 9.0

    def parallel_section(self) -> tuple[FiberBundlePoint, ...]:
        """平行截面：底空间每个点上纤维概率最高的状态（9 个点）。"""
        section: list[FiberBundlePoint] = []
        for base in self._base_space.values():
            fiber = self.fiber_over(base)
            mode = max(fiber, key=lambda pt: pt.fiber_prob)
            section.append(mode)
        return tuple(section)

    def non_parallel_points(self) -> tuple[FiberBundlePoint, ...]:
        """纤维概率低于均匀分布的配置（联络不偏好的"非自然"配置）。"""
        return tuple(pt for pt in self._points if not pt.is_parallel)

    def effective_dimension(self) -> float:
        """有效自由度 D_eff = H(joint) / log(3)。直积→3.0，完全耦合→2.0。"""
        entropy = 0.0
        for pt in self._points:
            p = self.joint_probability(*pt.as_tuple)
            if p > 0:
                entropy -= p * math.log(p)
        return entropy / math.log(3)

    def kl_divergence_from_product(self) -> float:
        """与直积（均匀 1/27）的 KL 散度。= 0 iff 平坦联络。"""
        uniform_p = 1.0 / 27.0
        kl = 0.0
        for pt in self._points:
            p = self.joint_probability(*pt.as_tuple)
            if p > 0:
                kl += p * math.log(p / uniform_p)
        return kl


# ── 230号数据的默认联络 ──────────────────────────────────────

# 从 230号实测配置频率校准
_230_CONFIG_COUNTS: dict[tuple[int, int, int], int] = {
    (-1, -1, -1): 229, (-1, -1, 0): 97, (-1, -1, 1): 40,
    (-1, 0, -1): 121, (-1, 0, 0): 132, (-1, 0, 1): 105,
    (-1, 1, -1): 42, (-1, 1, 0): 45, (-1, 1, 1): 155,
    (0, -1, -1): 350, (0, -1, 0): 240, (0, -1, 1): 40,
    (0, 0, -1): 266, (0, 0, 0): 579, (0, 0, 1): 209,
    (0, 1, -1): 49, (0, 1, 0): 275, (0, 1, 1): 243,
    (1, -1, -1): 65, (1, -1, 0): 40, (1, -1, 1): 14,
    (1, 0, -1): 147, (1, 0, 0): 318, (1, 0, 1): 129,
    (1, 1, -1): 69, (1, 1, 0): 395, (1, 1, 1): 385,
}


def default_connection() -> Connection:
    """从 230号数据校准的默认联络。"""
    return calibrate_connection(_230_CONFIG_COUNTS)


def default_fiber_bundle() -> FiberBundleConfigSpace:
    """使用 230号数据联络的默认纤维丛配置空间。"""
    return FiberBundleConfigSpace(default_connection())
