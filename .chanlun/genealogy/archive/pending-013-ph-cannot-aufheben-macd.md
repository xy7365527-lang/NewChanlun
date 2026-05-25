---
id: pending-013-ph-cannot-aufheben-macd
timestamp: 2026-05-24
status: 已结算
settlement: 强化
settled_by: 自结算（定理类）
settled_date: 2026-05-24
settled_classification: 定理
settlement_scope: "PH（含时间修正 pers×√span）不能扬弃（aufheben）MACD。MACD 的『0轴』= 跨段相对均衡基准（EMA12=EMA26），∉ PH 段内 information set（H0 birth/death = 段内绝对价位，无移动基准）；笔内路径几何亦不可约。aufheben 假设被 L2 否证。"
type: domain
negation_source: empirical+textual
negation_form: refutation
topo_effect: "无新分离——强化 pending-012 的 split:persistent_homology:operational_layer（PH 与 MACD 不可相互归约，互补分层）"
depends_on:
  - pending-012  # PH=缠论旁独立结构监视层，非力度附庸（239号结算）
  - '239'        # H0 sublevel persistence ≈ 振幅 ∈ ker(D)，时间盲
  - '231'        # 形式化有效域规则（L0-L3，否定性结果优先）
related:
  - pending-011-topo-divergence-vs-yichan-alignment-gap
source_authority:
  - "docs/chanlun/text/blog/024-第24课.md（MACD 对背弛的辅助判断，一级权威）"
---

## 问题（2026-05-24 编排者）

pending-012 已结算「PH 不作 MACD 的力度附庸」。本号追问一个**更强**的主张：
PH **加时间维度修正**后（用户假设 persistence×√span），能否在功能上**完全平替
并超越**（aufheben = 否定+保留+提升）MACD？若 MACD 面积是 (persistence, span)
的确定函数（高 R²），则 MACD = PH 的降维投影，可被扬弃。

## 决定性实验（scripts/tencent_ema_reset_decisive.py）

MACD 用 ewm(adjust=False) = IIR 滤波器，每笔起点 EMA 初值由笔前历史指数加权
决定；PH（H0）只看笔内序列。隔离方法：对笔内序列独立调 compute_macd（EMA 从
段首初始化），严格消除跨笔记忆，使 MACD information set 与 PH 对齐。

| 时段 | 全局MACD R² | 段内MACD R²（重置EMA） | ΔR² | 段内时间指数 b |
|------|------------|----------------------|-----|--------------|
| 日线 (n=27) | 0.399 | **0.954** | +0.555 | 0.57 (≈0.5，√span 成立) |
| 30分 (n=22) | 0.654 | 0.771 | +0.117 | **−0.10** (负，速度敏感) |
| 合并 (n=49) | 0.586 | 0.860 | +0.274 | 0.31 |

**分级别否定性结果**：
- 日线：H_mem 成立——残差 55% 是跨笔记忆。段内 MACD ≈ PH(pers,√span)（R²=0.954）。
- 30分：H_mem 被否证——即使段内，PH 仍差 23%（笔内路径几何），且 b<0 与 √span 方向相反。

## 原文锚定（第24课，一级权威——决定 aufheben 成败的关键）

> 「这个中枢一般会把 **MACD 的黄白线（DIFF 和 DEA）回拉到 0 轴附近**。而 C 段...
> 对应的 **MACD 柱子面积比 A 段对应的面积要小**，这时候就构成标准的背弛。」

缠论 MACD 背驰判据**本质依赖全局连续 EMA**：
1. **「黄白线回拉0轴」是背驰前提条件**（B 段中枢的作用）。0轴 = DIFF=0 = EMA12=EMA26
   = 价格相对于自身**移动历史均衡**的基准。段内重置 EMA 后每段从 DIFF=0 起步，
   「回拉0轴」概念**根本不存在**。
2. 红绿柱面积 A vs C 比较 = 全局连续 hist 的积分。

**实验含义被翻转**：日线段内 R²=0.954 不是「PH 能平替 MACD」的证据——它恰恰证明
**PH 等价的是被抽掉记忆后的退化 MACD**。而被抽掉的「0轴跨段相对基准」正是缠论背驰
判据赖以成立的核心。**PH 等价的恰恰是缠论 MACD 超越掉的那个退化版本。**

## 判定（定理类自结算）

四分法分类：**定理**（无自由价值参数）。
- 备选「PH 扬弃 MACD」被排除：MACD 的 0轴 = 跨段移动均衡基准，PH 的 H0 birth/death
  = 段内绝对价位（每段独立，finite_cap=max(段)，无移动基准）——是 PH 框架**定义上**
  排除的维度（范畴差异，非参数选择）。
- 用户让查原文 → 第24课锁定「0轴」为背驰前提 → 选项确定为「全局连续 MACD」，
  无价值判断余地 → 自结算（no-unnecessary-escalation：该走不走禁止）。

**PH（含时间修正）不能扬弃 MACD。** MACD 携带两个 PH 定义上不可约的维度，按级别分工：
- 跨笔递归记忆 / 0轴跨段相对基准（日线主导，残差 55%）；
- 笔内路径几何 / 速度敏感（30分钟主导，残差 23%）。
反之 PH 独有 H1 中枢 loop 几何（相空间往返），MACD 完全无。**二者互补，互不归约。**

## 结果包六要素

1. **结论**：PH（即使加 persistence×√span 时间修正）不能在功能上扬弃 MACD；二者互补不可相互归约。MACD 不可被 aufheben。
2. **定义依据**：第24课 MACD 背驰判据（黄白线回0轴 + 红绿柱面积 A/C 比较，均依赖全局连续 EMA）；239号（H0 sublevel persistence ≈ 振幅 ∈ ker(D)）；a_persistence_barcode.py H0 birth/death = 段内绝对价位、无移动基准。
3. **边界条件**：结论翻转条件——若缠论背驰**放弃 0轴判据、改用段内重置 MACD**，则日线上 PH(pers,√span) R²=0.954 可平替（但 30 分钟仍差 23%）。但段内 MACD 不是缠师定义的 MACD（无「回拉0轴」），此翻转以放弃缠论原文判据为代价。
4. **下游推论**：pending-012「PH=结构监视层」获 aufheben 维度的实证支撑。PH 不进入背驰/力度决策层替换 a_macd/a_divergence；PH 的独立贡献走 H1 中枢几何 + 区间套监视通道。a_divergence 的 MACD 力度判据保留。
5. **谱系引用**：强化 pending-012（239号定理类结算，PH 非力度附庸）；depends_on 239/231。本号是 pending-012 的 L2+原文锚定延伸，给出比「H0≈振幅时间盲」更精确的不可约机制（0轴=跨段移动基准）。
6. **影响声明**：新增 scripts/tencent_ema_reset_decisive.py（决定性实验）；新增本谱系；不改 src/。锚定 PH 模块架构定位（不替换 a_macd / a_divergence）。

**认识论等级**：L2（真实腾讯 700 日线 + 30分钟，可否证，**已产生否定性结果**——「PH 扬弃 MACD」假设被否证；日线/30分钟跨时段 = 弱 L3）。L0 部分：度量/回归算法。第24课语义 = 一级权威文本依据（非数据等级）。
