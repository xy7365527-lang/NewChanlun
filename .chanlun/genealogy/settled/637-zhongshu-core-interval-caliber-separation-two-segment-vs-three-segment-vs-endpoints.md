---
id: "637"
title: "中枢核心区间 ZG/ZD 的三口径分离"
status: "已结算"
settled_date: "2026-07-02"
settled_by: "genealogist via /ritual（编排者明令『并行全部推进』授权；裁决来源 codex 裁决①-⑤ + staging 归档）"
type: "概念分离（reference 裁决触发）"
date: "2026-06-27"
depends_on: ["003", "098"]
related: ["003", "098", "231", "248"]
negated_by: []
negates: []
---

> **[/ritual 结算段 · 2026-07-02]** 裁决来源：codex 裁决①-⑤（`.chanlun/review-results/codex-ritual-*.md`）+ 编排者明令「并行全部推进」授权。判决全文见 staging：CHOICES 637（实装#11 编排者已授权，主塔迁移 cefb29df69 既成）。判决摘要：口径 B（全三段交集 ZD=max(d1,d2,d3), ZG=min(g1,g2,g3)）；口径 A 保留为显式命名 legacy TwoSegmentCore。 **限定语（强制随行，脱落=声明膨胀090）：** 一级权威锚点：blog 018-第18课.md line 24 缠师严格公式=口径 B（最终权威，压倒 chan99 二级编纂版口径 A）；B canonical 非任意选择而是回归缠师原文


# 概念分离 637：中枢核心区间 ZG/ZD 的三口径

**状态**: 生成态（reference 已裁 v0 错；**A/B 有效域关系已由一级权威锚点澄清——见文末「一级权威补录」，B canonical，A=编纂版误差；最终结算待编排者 /ritual**）
**创建时间**: 2026-06-27
**类型**: 概念分离（engine-audit reference 反例触发）
**域**: 中枢核心区间（Zhongshu core interval ZD/ZG）
**来源**: 第Ⅱ类中枢 reference 语义补全工位（组B）——`~/Downloads/newchanlun-engine-formal-audit/`
  `NewChanlunEngineAudit.lean` 机器反例 + codex 裁决「中枢 reference 语义全部必须形式化+实装+parity」

---

## 分离描述

"中枢核心区间 `[ZD, ZG]`"在本仓库存在**三个独立口径**，曾被当作单一概念。本记录正式分离。

机器反例（`NewChanlunEngineAudit.lean`，`legacy_v0_not_correct_for_v1_reference`，native_decide）：
fixture `s0=[10,20], s1=[12,15], s2=[11,18]` 上三口径产出不同核心区间。

## 三个口径

### 口径 A：前两段（chan99 §6.4 再定义，知识库行 267-268）

- **定义**: `ZG = min(g₁, g₂)`、`ZD = max(d₁, d₂)`（仅前两段高/低点）。
- **实装**: `Origin.CenterConstruction.computeZG/computeZD`、`rust theta_v0::classifier::center::
  compute_zg/compute_zd`、`Origin.CenterConstruct.centersOfWithOuter`（经 centersOf）。
- **fixture 值**: `zd=max(10,12)=12, zg=min(20,15)=15 → [12,15]`。
- **权威链**: chan99 编纂版 §6.4「由前两段确定的中枢区间」（知识库行 267）。**⚠ 二级编纂版口径——经一级权威补录判定为编纂版再定义误差（见文末）。**

### 口径 B：全三段（reference v1 / chan99 §6.4 原文，知识库行 253）

- **定义**: `ZG = min3(g₁,g₂,g₃)`、`ZD = max3(d₁,d₂,d₃)`（前三段共同重叠的上沿/下沿）。
- **实装**: `Origin.CenterConstruct.refZhongshusFromComponents`/`refV1Interval`、
  `rust theta_v0::classifier::ref_v1::ref_zhongshus_from_components`/`ref_v1_interval`。
- **fixture 值**: `zd=max(10,12,11)=12, zg=min(20,15,18)=15 → [12,15]`。
- **权威链**: **★一级权威（最终权威，2026-07-02 补录）：blog `018-第18课.md` line 24 缠师「严格的公式」`（max(a2,b2,c2), min(a1,b1,c1)）` = 全三段口径**（详见文末补录）；engine-audit `reference_chanlun.py` frozen v1 rule（README §1，行 119-120）+ chan99 §6.4 原文「走势中枢由前三个连续次级别走势类型的**重叠部分**确定」（知识库行 253，二级同向印证）。

### 口径 C：首尾段（legacy v0，已裁决错）

- **定义**: `ZG = min(g₁, g₃)`、`ZD = max(d₁, d₃)`（首段与第三段）。
- **实装**: `Origin.CenterConstruct.legacyV0Interval`、`rust ...::legacy_v0_interval`（仅作反例保留）。
- **fixture 值**: `zd=max(10,11)=11, zg=min(20,18)=18 → [11,18]`。
- **权威链**: 无——`legacy_v0_not_correct_for_v1_reference` 裁决其不是 v1 正确实现。

## 裁决状态

| 口径 | 裁决 | 依据 |
|------|------|------|
| C（首尾段） | **已否定** | `NewChanlunEngineAudit.lean` 机器反例 + 任务明文「实装 v1 正确语义」 |
| B（全三段） | **canonical（一级权威背书）** | **blog 第18课 line 24 一级权威严格公式（最终权威）** + reference frozen v1 + chan99 §6.4 原文行253（二级同向）+ codex 裁决① |
| A（前两段） | **降 legacy（编纂版误差）** | chan99 §6.4 行267 二级编纂版再定义；与一级权威原文（行253/blog第18课）出入 → 三级权威链裁更高层级胜 → A 降 legacy `TwoSegmentCore` |

## A 与 B 的关系（一级权威补录后已澄清）

口径 A（前两段）与 B（全三段）**不总是相等**：
- 当第三段贯穿核心（`d₃ ≤ ZG_A ∧ ZD_A ≤ g₃`）且第三段不比前两段更窄时：A = B。
- 当第三段比前两段更窄（`g₃ < min(g₁,g₂)` 或 `d₃ > max(d₁,d₂)`）时：B 的核心严格窄于 A。
  - 例：`s0=[10,30], s1=[5,20], s2=[8,15]` → A=[10,20]，B=[10,15]（B 更窄）。

chan99 §6.4 自身的两处表述（行 253「前三个重叠部分」vs 行 267「由前两段确定」）构成**编纂版内部张力**。
reference v1 取行 253 原文口径（全三段）。本仓库现有 Origin/rust 主塔曾取行 267 口径（前两段）。
**此编纂版内部张力已由一级权威消解（见文末补录）：blog 第18课 line 24 缠师严格公式 = 全三段（B），行267「前两段」是编纂版再定义误差。B canonical，A 降 legacy。**

## 影响声明

- 新增口径 B 实装（Lean `Origin.CenterConstruct` § 5.5 + rust `ref_v1.rs`），主塔 A→B 迁移**已于 `cefb29df69` 既成**（全库无中枢构造路径停留 A）。口径 A 降 legacy `TwoSegmentCore`（显式命名保留，不作主 tower 默认）。
- 口径 C 仅作反例保留（`legacyV0Interval`/`legacy_v0_interval`），不进任何生产路径。
- parity 耦合 `theta_v0_center_parity.rs`（Lean #eval → fixture → rust bit-exact）。

## 一级权威补录（2026-07-02 · 结算就绪锚点）

**触发**：637B 迁移设计稿（`.chanlun/review-results/637b-design-20260702.md` §3）逐字考古的决定性发现。本记录原权威链最高层级仅到 chan99 §6.4（二级编纂版）+ engine-audit frozen v1 + codex 裁决，**缺一级权威博文锚点**——设计稿 §5 明确 flag 建议 genealogist 补录。

**一级权威严格公式（最终权威，压倒二级编纂版）**：

> blog `018-第18课.md` line 24：「具体的计算以前三个连续次级别的重叠为准，严格的公式可以这样表示：次级别的连续三个走势类型 A、B、C，分别的高、低点是 a1\a2,b1\b2,c1\c2。则，中枢的区间就是（**max（a2,b2,c2），min（a1,b1,c1）**）」

解析：a2/b2/c2 = 各段低点、a1/b1/c1 = 各段高点 ⟹ `ZD = max(三段低点)`、`ZG = min(三段高点)` = **口径 B 全三段**。缠师明示「严格的公式」= 权威口径。

**三级权威链裁决**（CLAUDE.md「层级间有出入时以更高层级为准」）：

| 层级 | 来源 | 口径 |
|------|------|------|
| 一级（最终权威·博文） | blog 第18课 line 24 严格公式 | **B 全三段** |
| 二级（编纂版） | chan99 第八节 line 23 / §6.4 行267 | A 前两段 |
| 二级（编纂版·原文段） | chan99 §6.4 行253「前三个重叠部分」 | B 全三段（与一级同向） |

一级权威（B）**压倒** chan99 编纂版内部张力中的 A 支（行267 再定义误差）。∴ **B = canonical，A = 编纂版误差口径降 legacy，C = 机器反例已否定**。

**结算就绪判定**：原「A/B 有效域关系待编排者/L2 确认」的生成态根因已由一级权威原文消解——不再是「待 L2 实证的有效域分离」，而是「一级权威直接裁定 B 为缠师原文口径」。codex 裁决① B 与最终权威一致（非任意选择）。主塔迁移 `cefb29df69` 既成。**637 达「结算就绪」态**，最终结算待编排者 /ritual（staging: `RITUAL-STAGING-CODEX-CHOICES-20260702.md` §637 已同步）。

**认识论等级（231号，强制）**：一级权威裁决 = 源头权威判定（非 L0 代数/非 L2 实证，而是**权威链层级裁决**——原文语义直接给定 canonical 口径，不依赖数据可证伪性）。有效域 = 中枢核心区间定义域全域（缠师严格公式适用于所有三连续次级别走势中枢，非特定标的/时段）。

## 谱系依据

- 003：线段两口径分离（v0/v1）——本记录是中枢域的同构分离（「概念分离需要谱系化」）。
- 098：口径结算结晶范式——v0 降级为参考、v1 为正式口径的决断模式。
- 231：形式化有效域规则——口径 A/B 有效域分离（B 是 engine reference 域）；**一级权威补录后 B 升为全定义域 canonical（缠师原文口径），A 降 legacy**。
- 248：线段引擎诊断——口径分歧的引擎层诊断先例。
- CLAUDE.md 三级权威链：blog 一级 > chan99 二级——本记录一级权威补录的规则依据（见文末补录）。
