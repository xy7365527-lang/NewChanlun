"""A 系统 — 中枢基线偏离积分（Zhongshu Baseline Deviation Integral）：无参数力度度量。

命名与定位（与 521号/§5 的关系，强制阅读——避免声明膨胀 090号）
----------------------------------------------------------------
521号定理（L0）：纯拓扑因果动量不变量不存在——动量（背驰）= 红绿柱面积的时间积分
（第24课），需要**时间度量**。本模块**不**声称是纯拓扑动量；它显式携带时间度量
（对段内每根 K 线求和 = 时间外延积分），故不与 521号冲突。

与 §5 两个原因的对应（关键设计动机）
-----------------------------------
§5 证明 PH（H0 persistence）不能平替 MACD，给出两个 H0 缺失而 MACD 携带的维度：
  原因一（239号）：H0 ∈ ker(D)，时间盲——只看端点（birth/death），不看快慢。
  原因二（第24课）：背驰的 0 轴判据依赖**全局连续 EMA**（移动均衡基准）。

本模块针对这两个原因各做一个**替换尝试**（不是绕过，是换基底重新表达）：
  对原因一：用 **Σ|close(t) − M|（段内逐根求和）** 替代 H0 的端点 persistence
            → 时间外延，对"急跌 vs 缓跌"（相同幅度不同耗时）敏感（§5 决定性 case）。
  对原因二：用 **中枢中间价 M = (ZD+ZG)/2** 替代 EMA 的 0 轴
            → M 是**结构性 0 轴**（中枢由三段重叠定义），不依赖任何移动平均参数。

这是否真能区分 §5 的 急跌#33 vs 缓跌#37、是否与 MACD 面积高相关——是**经验问题**，
本模块只提供确定性算法（L0），验证在 `scripts/tencent_zhongshu_force.py`（L2）。

无参数性（核心卖点，对应 522号参数鲁棒性）
------------------------------------------
本度量**无任何自由参数**：
  - 基线 M 由中枢区间 [ZD, ZG] 决定，中枢由结构（≥3 段重叠）定义，非参数。
  - 偏离 |close − M| 与求和 Σ 都不含参数。
故对 MACD 的 (fast, slow, signal) 参数扰动**天然 100% 不变**——它根本不读 MACD 参数。
这与 MACD 面积（依赖 3 个 EMA 参数）形成对照：参数鲁棒性是 L0 的（同义反复，无需实测），
但"无参数度量是否给出与 MACD 一致的背驰判断"是 L2 的（可否证）。

合法 vs 非法用途
----------------
- ✅ 合法：§6 监视层的力度描述量；研究"结构性 0 轴偏离积分 vs EMA 0 轴面积"的差异
          （L2 可否证）；生成态证伪探针（231号否定性结果价值）。
- ❌ 非法：声称已**平替** MACD 背驰判据而无 L2 一致性证据（§5 警示：H0 段内 R²=0.954
          也曾被误读为"可平替"，实则等价的是被抽掉记忆的退化 MACD）。本度量换的是
          0 轴的**来源**（结构 vs EMA），是否捕获到第24课要求的"跨段相对基准"待 L2 检验。

认识论等级（formalization-validity-domain 规则）
-----------------------------------------------
- midprice / 偏离积分 / signed / normalized / 背驰判据：**L0**（纯算法，确定性，零信息增量）。
- 对 MACD 参数 100% 鲁棒：**L0**（不读参数 → 同义反复，无信息增量）。
- "与 MACD 面积高相关 / 背驰判断一致 / 能区分急跌缓跌"：腾讯 700 **L2**（可否证，优先否定性结果）。

概念溯源标签
-----------
- 中枢中间价 M=(ZD+ZG)/2 [缠论:第17-19课中枢定义 / 本仓库 a_ph_zhongshu.Zhongshu.zd/zg]
- 偏离积分作力度 [新缠论:候选——时间外延的结构基线偏离，对应 §5 两原因的替换尝试]
- 背驰 = C段力度 < A段力度 [缠论:第24课]
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Literal

__all__ = [
    "ForceField",
    "ZhongshuForce",
    "BaselineDivergence",
    "zhongshu_midprice",
    "baseline_deviation_force",
    "compare_force",
]

# 背驰判据可比较的力度字段（详见 compare_force 的设计说明）
ForceField = Literal["force", "force_normalized", "force_signed"]


# ====================================================================
# 数据类型（frozen + slots，immutable）
# ====================================================================


@dataclass(frozen=True, slots=True)
class ZhongshuForce:
    """一个推动段相对其中枢基线 M 的偏离积分（无参数力度）。

    Attributes
    ----------
    force : float
        偏离积分 = Σ|close(t) − M|（t 遍历段内每根 K 线）。
        **时间外延**：段越长、偏离越深，force 越大——MACD 面积的结构基线类比。
    force_signed : float
        带符号积分 = Σ(close(t) − M)。> 0 = 段整体在 M 之上（多头偏离），
        < 0 = 段整体在 M 之下（空头偏离）。区分推动方向。
    force_normalized : float
        归一化力度 = force / span = 每根 K 线平均偏离 = **动量强度**（523号默认背驰判据）。
        **时间内涵**：剔除段长影响，急跌（深而短）> 缓跌（浅而长）。523号编排者裁定：
        第24课"面积"本质是动量强度，故此字段（非 force）对应第24课的力度语义。
    span : int
        段覆盖的 K 线根数（求和的项数）。span=0 时 force=0、normalized=0。
    zhongshu_midprice : float
        基线 M = (ZD + ZG) / 2 = 中枢中间价（结构性 0 轴）。
    """

    force: float
    force_signed: float
    force_normalized: float
    span: int
    zhongshu_midprice: float

    def value(self, field: ForceField = "force") -> float:
        """按指定字段取力度标量（供 compare_force 的判据切换）。"""
        if field == "force":
            return self.force
        if field == "force_normalized":
            return self.force_normalized
        return self.force_signed


@dataclass(frozen=True, slots=True)
class BaselineDivergence:
    """A 段 vs C 段的中枢基线偏离积分背驰判定（第24课结构基线版）。

    Attributes
    ----------
    force_a, force_c : ZhongshuForce
        A 段（在前）、C 段（在后）的偏离积分力度。
    field : str
        背驰判据所比较的力度字段（"force" / "force_normalized" / "force_signed"）。
    value_a, value_c : float
        按 field 取出的 A、C 段力度标量。
    ratio : float
        value_c / value_a（< 1 = C 段力度更弱 = 背驰方向）。value_a=0 时为 inf。
    is_divergent : bool
        是否背驰 = (|value_a| > noise_floor) ∧ (|value_c| < |value_a|)。
        用绝对值比较以兼容 force_signed（空头偏离 value 为负，比较其量级）。
    noise_floor : float
        A 段力度量级下限（低于此视为无实质动量，背驰判定退化为 False）。
    """

    force_a: ZhongshuForce
    force_c: ZhongshuForce
    field: str
    value_a: float
    value_c: float
    ratio: float
    is_divergent: bool
    noise_floor: float


# ====================================================================
# 中枢中间价（结构性 0 轴，替代 EMA 0 轴 —— §5 原因二的替换尝试）
# ====================================================================


def zhongshu_midprice(zd: float, zg: float) -> float:
    """中枢中间价 M = (ZD + ZG) / 2（缠论中枢的几何中心）。

    Parameters
    ----------
    zd : float
        中枢下沿（Zhongshu.zd = max(member births)）。
    zg : float
        中枢上沿（Zhongshu.zg = min(member deaths)）。

    Returns
    -------
    float
        M = (zd + zg) / 2。要求 zd ≤ zg（缠论中枢重叠区非空）；
        zd > zg 时仍按公式返回（调用方应已用 Zhongshu 保证 zd < zg）。

    认识论等级：L0（纯定义）。
    """
    return (float(zd) + float(zg)) / 2.0


# ====================================================================
# 偏离积分（时间外延 —— §5 原因一的替换尝试）
# ====================================================================


def baseline_deviation_force(
    prices: Sequence[float],
    midprice: float,
    *,
    i0: int = 0,
    i1: int | None = None,
) -> ZhongshuForce:
    """段 [i0, i1] 相对基线 M 的偏离积分力度（无参数）。

    force            = Σ_{t∈[i0,i1]} |close(t) − M|
    force_signed     = Σ_{t∈[i0,i1]} (close(t) − M)
    force_normalized = force / span

    Parameters
    ----------
    prices : Sequence[float]
        价格序列（通常用 close）。
    midprice : float
        基线 M = 中枢中间价（由 `zhongshu_midprice` 算出）。
    i0, i1 : int
        段的闭区间索引（i1=None → 序列末端）。i1 < i0 → 空段（force=0）。

    Returns
    -------
    ZhongshuForce

    认识论等级：L0（纯算法，确定性）。
    """
    n = len(prices)
    if n == 0:
        return ZhongshuForce(0.0, 0.0, 0.0, 0, float(midprice))
    if i1 is None:
        i1 = n - 1
    i0 = max(0, min(i0, n - 1))
    i1 = max(0, min(i1, n - 1))
    if i1 < i0:
        return ZhongshuForce(0.0, 0.0, 0.0, 0, float(midprice))

    m = float(midprice)
    abs_sum = 0.0
    signed_sum = 0.0
    for t in range(i0, i1 + 1):
        dev = float(prices[t]) - m
        abs_sum += abs(dev)
        signed_sum += dev
    span = i1 - i0 + 1
    return ZhongshuForce(
        force=abs_sum,
        force_signed=signed_sum,
        force_normalized=abs_sum / span,
        span=span,
        zhongshu_midprice=m,
    )


# ====================================================================
# 背驰判定（A 段 vs C 段，第24课：C 段力度 < A 段 = 背驰）
# ====================================================================


def compare_force(
    force_a: ZhongshuForce,
    force_c: ZhongshuForce,
    *,
    field: ForceField = "force_normalized",
    noise_floor: float = 0.0,
) -> BaselineDivergence:
    """A 段（在前）vs C 段（在后）的偏离积分背驰判定。

    缠论第24课：趋势中 C 段（离开中枢的推动）力度 < A 段（进入中枢的推动）→ 背驰。
    本函数把"力度"实现为中枢基线偏离积分，背驰 = |C 力度| < |A 力度|（且 A 力度量级
    > noise_floor，排除对噪声的算术比较，§3 干净前提）。

    判据字段（`field`）的选择——这是一个缠论建模决策，不是实现细节（523号已结算）
    ----------------------------------------------------------------------------
    背驰比较哪个力度字段，决定本度量行为更像速率还是更像积分：

      - "force_normalized"（默认，每 bar 平均偏离 = 动量强度）= Σ|close−M| / span。
        编排者裁定（523号）：**第24课的"面积"本质是动量强度，不是时间外延积分**。
        红绿柱"面积"在缠论语境中表达推进的**强度/速率**（每单位推进的力量），不是
        "偏离持续了多久"的时间累加。故默认取此字段。腾讯 700 L2 也旁证：force_normalized
        的背驰一致率（68%）高于 force（58%）。

      - "force"（时间外延积分）= Σ|close−M|。段越长积分越大——把"缓跌（多 bar）"
        误判为力度更强。523号裁定此字段**不**对应第24课的面积语义，仅作对照/研究保留。

      - "force_signed"（带符号）= 区分多空方向的偏离，比较其量级。

    无论取哪个字段，本度量都**不能平替 MACD**（523号 L2 否定性结果）：静态结构基线 M
    无法复刻 MACD 移动 EMA 0 轴的跨段路径记忆（§5 原因二）。字段选择只决定本度量
    内部最忠实第24课语义的读法，不改变"不可平替 MACD"的结论。字段仍作显式参数暴露。

    Parameters
    ----------
    force_a, force_c : ZhongshuForce
        A 段、C 段的偏离积分（通常 A、C 相对同一中枢 M，对应第24课"回拉 0 轴"）。
    field : ForceField
        背驰比较的力度字段（见上）。
    noise_floor : float
        A 段力度量级下限。

    Returns
    -------
    BaselineDivergence

    认识论等级：算法 L0；"该判据≡MACD 面积背驰"经验断言 L2（见验证脚本）。
    """
    raw_a = force_a.value(field)
    raw_c = force_c.value(field)
    mag_a = abs(raw_a)
    mag_c = abs(raw_c)
    ratio = mag_c / mag_a if mag_a > 0 else float("inf")
    is_div = mag_a > noise_floor and mag_c < mag_a
    return BaselineDivergence(
        force_a=force_a,
        force_c=force_c,
        field=field,
        value_a=raw_a,
        value_c=raw_c,
        ratio=ratio,
        is_divergent=is_div,
        noise_floor=noise_floor,
    )
