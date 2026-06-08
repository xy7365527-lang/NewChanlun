"""标的池生成器 -- 帕萨卡利亚条件设定链（层0→层1→层2）。**默认扫描入口。**

352号谱系 §2-§4 实现：
  阶段1（层1→层2）：K4配置 → 极性指数 → 目标矩阵（资产类别）
  阶段2（层2）：目标矩阵内标的的折叠等价类构造（二值筛选 + 商空间排序）
  阶段3：输出排序后的等价类 + 代表元

层0状态由上游维护（352号 §2.3）——本模块不计算层0，只读取缓存。

排序方式：商空间排序 rank([S]) = T([S]) * W(fold([S]))。
366号 L2 验证确认商空间排序与扁平排序产生显著不同的选股序列（归一化位移 1.4167），
且差异方向与 292号区间套顺序一致（外层折叠优先）。
商空间排序是本项目的**默认排序方式**（366号下游推论1）。

认识论标注：
  - 三阶段架构 + 折叠等价类 + 商空间排序：L0（从已结算概念推导）
  - 目标矩阵的具体映射规则：L2（需真实数据验证）
  - 商空间排序作为默认：L2（366号确认）

谱系引用：352号多标的扫描器设计、350号收敛紧度、292号折叠拓扑、265号帕萨卡利亚、366号L2验证。
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.topology.config_space import Configuration, WalkDirection, polarity_index
from newchan.trading.fold_equivalence import (
    EquivalenceClass,
    TargetAttributes,
    build_quotient_space,
)


# ═══════════════════════════════════════════════════════════════
# 阶段1：层1 → 层2 条件设定
# ═══════════════════════════════════════════════════════════════


def layer1_target_matrix(config: Configuration) -> frozenset[str]:
    """K4 配置 → 目标矩阵的资产类别标签集合。

    三值逻辑（从267号操作方法论推导，352号 §2.2）：
    - polarity > 0（risk-on） → {"equity"}
    - polarity < 0（risk-off）→ {"rate"}
    - polarity == 0（中性）   → {"equity", "rate"}
    - sigma_p UP 且 sigma_c UP → 额外加入 "au"

    Parameters
    ----------
    config : Configuration
        当前 K4 配置。

    Returns
    -------
    frozenset[str]
        资产类别标签集合。标签是小写字符串：
        "equity", "rate", "au"。
    """
    pol = polarity_index(config)

    if pol > 0:
        categories: set[str] = {"equity"}
    elif pol < 0:
        categories = {"rate"}
    else:
        categories = {"equity", "rate"}

    if config.sigma_p is WalkDirection.UP and config.sigma_c is WalkDirection.UP:
        categories.add("au")

    return frozenset(categories)


# ═══════════════════════════════════════════════════════════════
# 阶段2：层2 折叠等价类构造
# ═══════════════════════════════════════════════════════════════


def layer2_build_equivalence_classes(
    targets: tuple[TargetAttributes, ...],
) -> tuple[EquivalenceClass, ...]:
    """对通过二值筛选的标的构造折叠等价类（352号 §3）。

    步骤：
    1. 二值筛选：排除 T(S) = 0 的标的
    2. 调用 build_quotient_space 构造商空间

    Parameters
    ----------
    targets : tuple[TargetAttributes, ...]
        候选标的属性集合（可能包含 T=0 的标的）。

    Returns
    -------
    tuple[EquivalenceClass, ...]
        按 rank 降序排列的折叠等价类。
    """
    filtered = tuple(t for t in targets if t.tightness > 0.0)
    if not filtered:
        return ()
    return build_quotient_space(filtered)


# ═══════════════════════════════════════════════════════════════
# 阶段3：完整管线结果
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class ScannerPoolResult:
    """标的池生成结果（352号 §4.4 输出）。

    Attributes
    ----------
    config : Configuration
        输入的 K4 配置。
    polarity : int
        极性指数。
    target_matrix : frozenset[str]
        层1确定的目标矩阵资产类别集合。
    equivalence_classes : tuple[EquivalenceClass, ...]
        按 rank 降序排列的折叠等价类。
    top_representative : TargetAttributes | None
        排名第一的等价类的代表元。None 表示无收敛标的。
    """

    config: Configuration
    polarity: int
    target_matrix: frozenset[str]
    equivalence_classes: tuple[EquivalenceClass, ...]
    top_representative: TargetAttributes | None


def build_scanner_pool(
    config: Configuration,
    targets: tuple[TargetAttributes, ...],
) -> ScannerPoolResult:
    """完整三阶段管线：配置 + 标的属性 → 排序后的等价类。

    步骤：
    1. layer1_target_matrix(config) → 目标矩阵
    2. layer2_build_equivalence_classes(targets) → 排序后等价类
    3. 封装为 ScannerPoolResult

    Parameters
    ----------
    config : Configuration
        当前 K4 配置。
    targets : tuple[TargetAttributes, ...]
        候选标的属性集合。上游已根据 target_matrix 筛选出符合资产类别的标的。

    Returns
    -------
    ScannerPoolResult
        包含排序后的等价类和最优代表元。
    """
    pol = polarity_index(config)
    matrix = layer1_target_matrix(config)
    eqs = layer2_build_equivalence_classes(targets)
    top_rep = eqs[0].representative if eqs else None

    return ScannerPoolResult(
        config=config,
        polarity=pol,
        target_matrix=matrix,
        equivalence_classes=eqs,
        top_representative=top_rep,
    )
