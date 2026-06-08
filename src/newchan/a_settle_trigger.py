"""A 系统 — alive→settled 触发阈值：从 online merge tree 直接读出"止跌确认价"。

存在论位置（§17.1 形态学层 + §17.3 规则3 的量化）
--------------------------------------------------
`persistence_theory.md` §17.1 把"高级别走势完整性"形式化为 online merge tree 的
**alive vs settled**；§17.3 规则3 给出定性结论："高级别 component 仍 alive → 次级别
区间套收敛大概率假收敛"。本模块把规则3 的**定性前置条件量化为具体价格**——读出每个
alive 下跌分量的 **settle 屏障价**（反弹升到该价并维持，该分量的死亡因果确定）。

这不是新判据，是 §10 因果 settle 判据（`a_online_persistence` 模块顶部已证 L0 定理）
在操盘侧的读出。settle 屏障 = 缠论"顶/底分型需后续 K 线确认"的拓扑形式。

关键诚实点（090号 / formalization-validity-domain，必读）
--------------------------------------------------------
**全局最低 alive 分量（当前最深的下跌低点）不能由反弹 settle**——它是 merge tree
所有合并的 elder（valley 最低者永远存活），只在 finalize 封顶时因序列终止而确定。
所以本模块对它诚实报告 ``can_settle_by_rebound=False``、``settle_price=None``，并给出
**反向否定幅度**（reversal_amplitude = 当前下跌分量的 persistence）——要否定这个下跌
假设，需要一个反向结构的 persistence 超过它（"对象否定对象"，见 `stop_signal`）。

把全局低点伪装成"反弹到 X 即 settle"= 声明膨胀（090号）。两种读出严格区分：
- **nearest rebound settle**（最近可 settle 的较浅分量）= 近端"止跌"信号（最近一段
  下跌腿被反弹吸收）；
- **dominant 全局下跌分量** = 不可由反弹 settle，对应 §17.3 规则3 的"高级别未完成"。

认识论等级
----------
- settle 屏障算法（dry-run 级联）：**L0**（merge tree 确定性属性，与 _merge_top 同构）。
- "settle 屏障 = 缠论分型确认"同构：**L0/候选**。
- MOS 单标的实测翻译为具体价格：**L2**（可否证；未来创新低会改写阈值——见边界条件）。

设计约束（coding-style）
------------------------
- SettleTrigger：frozen + slots，immutable。
- 只读 OnlineMergeTree 的公开 API（alive_settle_thresholds / current_barcode /
  last_price / running_max），不触碰其内部状态。

概念溯源标签
-----------
- alive→settled 止跌阈值 [新缠论:候选——§17.3 规则3 的量化]
- 全局低点不可反弹 settle [新缠论:候选——§7.5 栈底永远 alive]
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Sequence

from newchan.a_online_persistence import OnlineMergeTree

__all__ = [
    "SettleTrigger",
    "settle_triggers",
    "settle_triggers_from_prices",
    "nearest_rebound_settle",
    "dominant_trigger",
]


@dataclass(frozen=True, slots=True)
class SettleTrigger:
    """一个 alive 分量的 settle 触发快照（immutable）。

    Attributes
    ----------
    birth_idx : int
        分量身份标识（诞生 valley 的时间索引）。
    birth_price : float
        分量诞生价（valley 低点）。
    settle_price : float | None
        反弹升到此价（含）该分量作为 younger 被合并、死亡因果确定（settle）。
        全局最低分量为 None（反弹不能使其 settle）。
    settle_persistence : float | None
        settle_price − birth_price = 反弹相对 valley 需达到的幅度。None 同上。
    current_cap : float
        当前运行最高价（alive 分量 death 的当前估计上界）。
    current_persistence : float
        current_cap − birth_price = `current_barcode()` 约定的 alive persistence。
        注意：可能 ≠ settle_persistence（cap 是历史高点估计，settle_price 是因果屏障）。
    last_price : float
        最近一根 close。
    gap_to_settle : float | None
        settle_price − last_price = 从当前价到 settle 还需的反弹空间。
        ≤0 表示当前价已达/超过该屏障（该分量本轮即将/已 settle）。None=全局分量。
    is_dominant : bool
        是否为 persistence（=cap−val）最大的 alive 分量（最深下跌腿）。
    can_settle_by_rebound : bool
        False ⟺ 全局最低分量（只能反向否定，不能反弹 settle）。
    reversal_amplitude : float | None
        仅全局分量非 None：要否定该下跌假设，反向结构 persistence 需超过的幅度
        （= current_persistence）。非全局分量为 None。
    """

    birth_idx: int
    birth_price: float
    settle_price: float | None
    settle_persistence: float | None
    current_cap: float
    current_persistence: float
    last_price: float
    gap_to_settle: float | None
    is_dominant: bool
    can_settle_by_rebound: bool
    reversal_amplitude: float | None


def settle_triggers(tree: OnlineMergeTree) -> tuple[SettleTrigger, ...]:
    """从一棵活的 OnlineMergeTree 读出全部 alive 分量的 settle 触发快照。

    注意：必须在 **finalize 之前** 调用（finalize 后无 alive 分量）。

    认识论等级：L0（确定性读出）。
    """
    last = tree.last_price
    cap = tree.running_max
    if last is None or cap is None:
        return ()
    snap = tree.current_barcode()
    if not snap.alive_bars:
        return ()
    # birth_idx → 当前 persistence（cap − val），用于判定 dominant
    cur_pers = {b.birth_idx: b.persistence for b in snap.alive_bars}
    dominant_idx = max(cur_pers, key=lambda k: cur_pers[k])

    triggers: list[SettleTrigger] = []
    for birth_idx, birth_price, settle_price in tree.alive_settle_thresholds():
        cur_p = cap - birth_price
        can_settle = settle_price is not None
        if can_settle:
            settle_pers: float | None = settle_price - birth_price
            gap: float | None = settle_price - last
            reversal: float | None = None
        else:
            settle_pers = None
            gap = None
            reversal = cur_p  # 全局分量：反向结构需超过的幅度
        triggers.append(
            SettleTrigger(
                birth_idx=birth_idx,
                birth_price=birth_price,
                settle_price=settle_price,
                settle_persistence=settle_pers,
                current_cap=cap,
                current_persistence=cur_p,
                last_price=last,
                gap_to_settle=gap,
                is_dominant=(birth_idx == dominant_idx),
                can_settle_by_rebound=can_settle,
                reversal_amplitude=reversal,
            )
        )
    return tuple(triggers)


def settle_triggers_from_prices(prices: Sequence[float]) -> tuple[SettleTrigger, ...]:
    """逐根 update 一段价格后，读出活树的 settle 触发快照（不 finalize）。

    认识论等级：L0（算法）；翻译为价格的操盘断言 L2（调用方标注）。
    """
    tree = OnlineMergeTree()
    for p in prices:
        tree.update(p)
    return settle_triggers(tree)


def nearest_rebound_settle(
    triggers: Sequence[SettleTrigger],
) -> SettleTrigger | None:
    """最近端可反弹 settle 的分量 = settle_price 最小者（最先被反弹触及）。

    操盘意义：这是近端"止跌确认"信号——价格升到它的 settle_price，最近一段下跌腿
    被反弹吸收（最近的次级别下跌完成）。注意它**不等于**深层主导下跌腿完成
    （后者见 dominant_trigger，§17.3 规则3）。无可 settle 分量时返回 None。
    """
    settleable = [t for t in triggers if t.can_settle_by_rebound and t.settle_price is not None]
    if not settleable:
        return None
    return min(settleable, key=lambda t: t.settle_price)  # type: ignore[arg-type,return-value]


def dominant_trigger(
    triggers: Sequence[SettleTrigger],
) -> SettleTrigger | None:
    """主导（最深）下跌分量的 settle 快照——通常 can_settle_by_rebound=False。

    对应 §17.3 规则3 的"高级别下跌分量"：若它不可反弹 settle，则任何次级别反弹的
    区间套收敛大概率是假收敛。无 alive 分量时返回 None。
    """
    for t in triggers:
        if t.is_dominant:
            return t
    return None
