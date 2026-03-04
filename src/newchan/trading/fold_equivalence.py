"""折叠等价类构造器 — 从标的属性映射到292号折叠通道等价关系，输出商空间 F/~。

352号谱系 §3 定义：
  S1 ~ S2 当且仅当 S1 和 S2 处于同一个折叠通道的同一侧。

五类折叠通道（292号折叠盘点）：
  - Au: C <-> $  （全局共享，W=4）
  - Oil: C <-> $ （半共享，W=3）
  - Bond: $ <-> E （经济体特有，W=2）
  - RE: RE <-> E, RE <-> $ （地区特有，W=1）
  - Equity: 同板块且 D 算子三态一致 （板块特有，W=1）

等价类代表元选取规则（352号 §3.3）：
  1. T(S) 最大者
  2. 同 T 打破：流动性最好（日均成交量最大）

商空间排序函数（352号 §4.1）：
  rank([S]) = T([S]) * W(fold([S]))

认识论标注：
  - 等价关系定义 + 商空间构造：L0（从292号折叠盘点直接推导）
  - 折叠共享性权重 W 的具体数值：L2（需真实数据验证是否最优）
  - 代表元选取中流动性的度量：L2（日均成交量作为流动性代理需验证）

谱系引用：292号折叠拓扑本体论、352号多标的扫描器设计。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


# ═══════════════════════════════════════════════════════════════
# 折叠通道枚举
# ═══════════════════════════════════════════════════════════════


class FoldChannel(Enum):
    """292号折叠盘点的五类折叠通道。

    每个通道对应一个折叠关系（同一对象在不同域中的不同相位）。
    值 = 该通道的折叠共享性权重 W（352号 §4.2）。
    """

    AU = 4       # Au: C <-> $, 全局共享
    OIL = 3      # Oil: C <-> $, 半共享
    BOND = 2     # Bond: $ <-> E, 经济体特有
    RE = 1       # RE: RE <-> E, RE <-> $, 地区特有
    EQUITY = 1   # 同板块 Equity, 板块特有


class DTriState(Enum):
    """D 算子三态分类（235号谱系）。

    同板块 Equity 等价判据要求三态一致。
    """

    ABSORB = "absorb"    # 吸收（ker(D)）
    RETAIN = "retain"    # 保留
    AMPLIFY = "amplify"  # 放大


# ═══════════════════════════════════════════════════════════════
# 标的属性
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class TargetAttributes:
    """标的的折叠属性。

    从标的的市场属性映射而来。每个标的必须携带这些属性才能参与等价类构造。

    Attributes
    ----------
    symbol : str
        标的代码。
    fold_channel : FoldChannel
        所属折叠通道。
    sector : str
        板块/行业标签。仅 FoldChannel.EQUITY 时参与等价判定。
        非 EQUITY 通道的标的此字段为空字符串。
    d_tri_state : DTriState
        D 算子三态分类。仅 FoldChannel.EQUITY 时参与等价判定。
    tightness : float
        收敛紧度 T(S)（350号定义）。T(S) = 0 的标的不应进入此模块
        （二值筛选在上游已完成）。
    liquidity : float
        流动性度量（日均成交量或等价代理）。用于代表元同 T 打破。
    """

    symbol: str
    fold_channel: FoldChannel
    sector: str
    d_tri_state: DTriState
    tightness: float
    liquidity: float


# ═══════════════════════════════════════════════════════════════
# 等价关系
# ═══════════════════════════════════════════════════════════════


def equivalence_key(target: TargetAttributes) -> tuple[FoldChannel, str, DTriState | None]:
    """计算标的的等价类键。

    同一键的标的互相等价。键的定义（352号 §3.2）：
    - 非 EQUITY 通道：(fold_channel, "", None) — 同通道即等价
    - EQUITY 通道：(EQUITY, sector, d_tri_state) — 同板块 + 三态一致

    Returns
    -------
    tuple[FoldChannel, str, DTriState | None]
        等价类键。
    """
    if target.fold_channel is FoldChannel.EQUITY:
        return (FoldChannel.EQUITY, target.sector, target.d_tri_state)
    return (target.fold_channel, "", None)


def are_fold_equivalent(a: TargetAttributes, b: TargetAttributes) -> bool:
    """判定两个标的是否折叠等价。

    S1 ~ S2 当且仅当 equivalence_key(S1) == equivalence_key(S2)。
    """
    return equivalence_key(a) == equivalence_key(b)


# ═══════════════════════════════════════════════════════════════
# 等价类与商空间
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class EquivalenceClass:
    """折叠等价类 [S]。

    Attributes
    ----------
    representative : TargetAttributes
        代表元 rep([S])。
    members : tuple[TargetAttributes, ...]
        类内所有标的（含代表元），按 (tightness desc, liquidity desc) 排序。
    max_tightness : float
        类内最大 T 值：T([S]) = max{T(S') : S' in [S]}。
    fold_channel : FoldChannel
        折叠通道标记。
    """

    representative: TargetAttributes
    members: tuple[TargetAttributes, ...]
    max_tightness: float
    fold_channel: FoldChannel

    @property
    def size(self) -> int:
        """类内标的数量 |[S]|。"""
        return len(self.members)

    @property
    def weight(self) -> int:
        """折叠共享性权重 W(fold([S]))。"""
        return self.fold_channel.value

    @property
    def rank(self) -> float:
        """商空间排序键 rank([S]) = T([S]) * W(fold([S]))（352号 §4.1）。"""
        return self.max_tightness * self.weight


def _select_representative(members: tuple[TargetAttributes, ...]) -> TargetAttributes:
    """从等价类成员中选取代表元。

    规则（352号 §3.3）：
    1. T(S) 最大者
    2. 同 T 打破：流动性最好（日均成交量最大）
    """
    return members[0]  # members 已按 (tightness desc, liquidity desc) 排序


def build_quotient_space(
    targets: tuple[TargetAttributes, ...],
) -> tuple[EquivalenceClass, ...]:
    """构造商空间 F/~。

    步骤：
    1. 计算每个标的的 equivalence_key
    2. 按 key 分组
    3. 每组内按 (tightness desc, liquidity desc) 排序
    4. 选取代表元
    5. 构造 EquivalenceClass
    6. 按 rank 降序排序整个商空间

    Parameters
    ----------
    targets : tuple[TargetAttributes, ...]
        通过二值筛选的标的集合 F = {S : T(S) > 0}。

    Returns
    -------
    tuple[EquivalenceClass, ...]
        排序后的商空间 F/~。按 352号 §4.3 排序规则：
        rank desc → T desc → size desc。
    """
    if not targets:
        return ()

    # 分组
    groups: dict[tuple[FoldChannel, str, DTriState | None], list[TargetAttributes]] = {}
    for t in targets:
        key = equivalence_key(t)
        if key not in groups:
            groups[key] = []
        groups[key].append(t)

    # 构造等价类
    classes: list[EquivalenceClass] = []
    for key, group in groups.items():
        # 排序：tightness desc, liquidity desc
        sorted_members = tuple(
            sorted(group, key=lambda x: (-x.tightness, -x.liquidity))
        )
        representative = _select_representative(sorted_members)
        max_t = sorted_members[0].tightness  # 排序后第一个就是最大 T

        classes.append(
            EquivalenceClass(
                representative=representative,
                members=sorted_members,
                max_tightness=max_t,
                fold_channel=key[0],
            )
        )

    # 商空间排序（352号 §4.3）：rank desc → T desc → size desc
    return tuple(
        sorted(
            classes,
            key=lambda c: (-c.rank, -c.max_tightness, -c.size),
        )
    )


def rank_quotient_space(
    quotient: tuple[EquivalenceClass, ...],
) -> tuple[tuple[float, EquivalenceClass, TargetAttributes], ...]:
    """输出排序后的商空间 + 代表元（352号 §4.4 输出格式）。

    Returns
    -------
    tuple[tuple[float, EquivalenceClass, TargetAttributes], ...]
        [(rank, [S], rep([S])), ...] 按 rank 降序。
    """
    return tuple(
        (ec.rank, ec, ec.representative) for ec in quotient
    )
