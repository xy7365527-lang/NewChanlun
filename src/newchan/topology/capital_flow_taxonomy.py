"""资本流转守恒律的完全分类 — 27 配置 → 资本病理态。

任务（编排者）：从资本守恒 ΔM+ΔP+ΔC+ΔR=0 出发做完全分类，M2 选股层理论核心。
设计文档：docs/architecture/recursive_decomposition_tree.md §10。

## 出发点：资本守恒

四顶点 M/P/C/R 的资本变化 (ΔM,ΔP,ΔC,ΔR)，每个 ∈ {+1(流入), 0(中性), -1(流出)}。
守恒律 ΔM+ΔP+ΔC+ΔR=0。取 (ΔP,ΔC,ΔR) 为 3 自由度（3³=27 配置），
**ΔM = −(ΔP+ΔC+ΔR) 由守恒律决定**（M 是货币池/度量基准，026号双重身份——
资本流入 P/C/R 必从 M 流出）。ΔM ∈ {−3,…,+3}。

## 与 config_space 的关系（同构但不同语义）

本模块的流量配置 (ΔP,ΔC,ΔR) 与 config_space.Configuration 的走势方向
(σ_P,σ_C,σ_R) **结构同构**（都是 {−1,0,+1}³），但语义不同：
  - config_space：边走势方向（价格比值方向，527号 σ）。
  - 本模块：资本流量（守恒量）。
桥接：**流量驱动价格**（ΔX>0 → X 吸资 → X/M 价格涨 → σ_X=UP）——这是**经验假设**
（一阶近似，存量重估是偏差），非恒等。`flow_to_walk` 标注此假设。

## 病态判据（优先级，避免标签重叠）

依病理学映射（§8）与卢麒元资本三病态（auto-memory reference_lu_qiyuan）：
  1. NEUTRAL：(0,0,0) 全局中性。
  2. FLIGHT(走资)：ΔM≥+2（资本强烈涌向货币/外汇，避险外逃；254号跨国流出在单国 K4 的投影）。
  3. SEDIMENTATION(沉没)：ΔR>0 ∧ ΔP≤0 ∧ ΔC≤0（资本撤入不动产沉淀；482号"沉没"）。
  4. SPECULATION(空转)：ΔP>0 ∧ ΔP>ΔC（金融吸资超实体，MCM′ 膨胀；330号堰塞湖）。
  5. DAM_BREAK(溃坝)：ΔP<0 ∧ ΔP<ΔC（金融流出/实体流入，向心坍缩；329号，空转反演）。
  6. HEALTHY(健康)：ΔP=ΔC≠0（金融与实体同步流转，M→C→P→C′→M′ 运转）。
  7. TRANSITIONAL(过渡)：其余（单顶点独动等未定型态）。

> 关键发现：分类**不是时间反演对称**的——SPECULATION 的镜像（全符号翻转）本应是
> DAM_BREAK，但 SEDIMENTATION(ΔR 方向性) 与 FLIGHT(ΔM 方向性) 判据破坏对称。
> 这是实质发现：资本病理有方向偏好（沉没易、释放难；外逃骤、回流缓）。

认识论：配置枚举 + 判据 = **L0**（纯定义/代数）。病态↔配置映射的经验有效性 = **L2**
（需 10 年数据标注实际配置 + 转换路径验证，见 §10 验证）。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from itertools import product


class PathologyMode(Enum):
    """资本病理态（卢麒元三病态 + 健康/中性/过渡/溃坝）。"""

    NEUTRAL = "中性"           # (0,0,0) 全局均衡观望
    HEALTHY = "健康循环"        # ΔP=ΔC≠0 金融实体同步
    SPECULATION = "空转"        # ΔP>0∧ΔP>ΔC 金融吸资超实体（堰塞湖扩大）
    DAM_BREAK = "溃坝"          # ΔP<0∧ΔP<ΔC 金融流出实体流入（向心坍缩）
    SEDIMENTATION = "沉没"      # ΔR>0∧ΔP≤0∧ΔC≤0 资本撤入不动产
    FLIGHT = "走资"            # ΔM≥+2 资本涌向货币（避险外逃）
    TRANSITIONAL = "过渡"       # 其余未定型态


@dataclass(frozen=True, slots=True)
class CapitalFlow:
    """资本流量配置 (ΔP,ΔC,ΔR)，ΔM 由守恒律决定。"""

    dp: int  # ΔP 生产资本（股权）流量
    dc: int  # ΔC 商品资本流量
    dr: int  # ΔR 不动产流量

    def __post_init__(self) -> None:
        for v in (self.dp, self.dc, self.dr):
            if v not in (-1, 0, 1):
                raise ValueError(f"流量分量必须 ∈ {{-1,0,1}}，收到 {v}")

    @property
    def dm(self) -> int:
        """ΔM = −(ΔP+ΔC+ΔR)，守恒律决定。值域 {−3,…,+3}。"""
        return -(self.dp + self.dc + self.dr)

    @property
    def asset_net(self) -> int:
        """资产端净流量 S = ΔP+ΔC+ΔR = −ΔM。"""
        return self.dp + self.dc + self.dr

    @property
    def decoupling(self) -> int:
        """金融-实体脱钩度 D = ΔP−ΔC。>0 金融吸资超实体；<0 实体超金融。"""
        return self.dp - self.dc

    @property
    def label(self) -> str:
        """如 (+,-,0)。顺序 (ΔP,ΔC,ΔR)。"""
        s = {1: "+", 0: "0", -1: "-"}
        return f"({s[self.dp]},{s[self.dc]},{s[self.dr]})"

    @property
    def full_label(self) -> str:
        """含 ΔM 的完整标签 [M|P,C,R]。"""
        s = {1: "+", 0: "0", -1: "-", 2: "++", -2: "--", 3: "+++", -3: "---"}
        return f"[M{s[self.dm]}|{self.label}]"

    def reverse(self) -> "CapitalFlow":
        """时间反演（全符号翻转，牛↔熊镜像）。"""
        return CapitalFlow(-self.dp, -self.dc, -self.dr)


def classify(flow: CapitalFlow) -> PathologyMode:
    """配置 → 病理态（优先级判据，见模块 docstring）。L0。"""
    p, c, r, m = flow.dp, flow.dc, flow.dr, flow.dm
    if p == 0 and c == 0 and r == 0:
        return PathologyMode.NEUTRAL
    if m >= 2:
        return PathologyMode.FLIGHT
    if r > 0 and p <= 0 and c <= 0:
        return PathologyMode.SEDIMENTATION
    if p > 0 and p > c:
        return PathologyMode.SPECULATION
    if p < 0 and p < c:
        return PathologyMode.DAM_BREAK
    if p == c and p != 0:
        return PathologyMode.HEALTHY
    return PathologyMode.TRANSITIONAL


# ── 折叠通道状态 + 操作含义（按病态映射，§10）──────────────────

# ω = Au(C↔M 金油比) 倾向：空转抽实体流动性 → ω↑（482号）；溃坝实体回归 → ω↓。
_OMEGA_TENDENCY: dict[PathologyMode, str] = {
    PathologyMode.SPECULATION: "↑（金融压实体，剥削率升）",
    PathologyMode.DAM_BREAK: "↓（实体回归，剥削率降）",
    PathologyMode.FLIGHT: "↑（避险买金）",
    PathologyMode.SEDIMENTATION: "中性偏↑（金=沉没物之一）",
    PathologyMode.HEALTHY: "中性",
    PathologyMode.NEUTRAL: "中性",
    PathologyMode.TRANSITIONAL: "不定",
}

_OPERATION: dict[PathologyMode, str] = {
    PathologyMode.SPECULATION: "减金融化 P，加金/实物 C；等 P/C 顶背驰=空转崩溃入场点（做金油比下降）",
    PathologyMode.DAM_BREAK: "资本向实体回归：做多 C/大宗，减金融 P；溃坝=向心坍缩确认",
    PathologyMode.FLIGHT: "risk-off：持货币/避险资产；若跨国确认则做流入国 P 顶点",
    PathologyMode.SEDIMENTATION: "回避沉没标的（死水）；等 R 残差出背驰=沉没松动→接 R 释放的资本流",
    PathologyMode.HEALTHY: "正常选股：分解树 DRIVER 成分（金融实体同步，趋势可持有）",
    PathologyMode.NEUTRAL: "观望：宏观无方向，分解树空排序（§2.1）",
    PathologyMode.TRANSITIONAL: "等待定型：单顶点独动，方向未明，不重仓",
}


@dataclass(frozen=True, slots=True)
class TaxonomyEntry:
    """单配置的完全分类条目。"""

    flow: CapitalFlow
    mode: PathologyMode
    omega_tendency: str   # Au 折叠通道 ω 倾向
    operation: str        # 操作含义

    @property
    def economic_meaning(self) -> str:
        """卢麒元语言的经济含义。"""
        return _economic_meaning(self.flow, self.mode)


def _economic_meaning(flow: CapitalFlow, mode: PathologyMode) -> str:
    """生成配置的经济含义描述。"""
    p, c, r, m = flow.dp, flow.dc, flow.dr, flow.dm
    parts = []
    names = [("股权P", p), ("商品C", c), ("地产R", r), ("货币M", m)]
    inflow = [n for n, v in names if v > 0]
    outflow = [n for n, v in names if v < 0]
    if inflow:
        parts.append("资本流入" + "/".join(inflow))
    if outflow:
        parts.append("流出" + "/".join(outflow))
    if not inflow and not outflow:
        parts.append("全局静止")
    return "；".join(parts) + f"（{mode.value}）"


def entry_for(flow: CapitalFlow) -> TaxonomyEntry:
    """构造单配置的完整分类条目。"""
    mode = classify(flow)
    return TaxonomyEntry(
        flow=flow,
        mode=mode,
        omega_tendency=_OMEGA_TENDENCY[mode],
        operation=_OPERATION[mode],
    )


def enumerate_taxonomy() -> tuple[TaxonomyEntry, ...]:
    """枚举全部 27 配置的完全分类，按资产净流量 S 降序（+3→−3）。"""
    flows = [CapitalFlow(p, c, r) for p, c, r in product((1, 0, -1), repeat=3)]
    flows.sort(key=lambda f: (-f.asset_net, -f.dp, -f.dc, -f.dr))
    return tuple(entry_for(f) for f in flows)


def mode_histogram() -> dict[PathologyMode, int]:
    """各病态的配置计数（完全分类的分布）。"""
    hist: dict[PathologyMode, int] = {m: 0 for m in PathologyMode}
    for e in enumerate_taxonomy():
        hist[e.mode] += 1
    return hist


def flow_to_walk(flow: CapitalFlow) -> tuple[int, int, int]:
    """流量配置 → 走势方向 (σ_P,σ_C,σ_R)（**经验假设**：流量驱动价格，非恒等）。

    桥接 config_space.Configuration：ΔX>0 → X 吸资 → X/M 涨 → σ_X=+1。
    存量重估（价格变但无流量）是此映射的偏差源。L2 待验证（流量↔走势相关性）。
    """
    return (flow.dp, flow.dc, flow.dr)
