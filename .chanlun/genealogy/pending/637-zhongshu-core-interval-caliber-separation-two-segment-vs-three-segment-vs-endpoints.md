---
id: "637"
title: "中枢核心区间 ZG/ZD 的三口径分离"
status: "生成态"
type: "概念分离（reference 裁决触发）"
date: "2026-06-27"
depends_on: ["003", "098"]
related: ["003", "098", "231", "248"]
negated_by: []
negates: []
---

# 概念分离 637：中枢核心区间 ZG/ZD 的三口径

**状态**: 生成态（reference 已裁 v0 错；前两段 vs 全三段的有效域关系待编排者/L2 确认）
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
- **权威链**: chan99 编纂版 §6.4「由前两段确定的中枢区间」（知识库行 267）。

### 口径 B：全三段（reference v1 / chan99 §6.4 原文，知识库行 253）

- **定义**: `ZG = min3(g₁,g₂,g₃)`、`ZD = max3(d₁,d₂,d₃)`（前三段共同重叠的上沿/下沿）。
- **实装**: `Origin.CenterConstruct.refZhongshusFromComponents`/`refV1Interval`、
  `rust theta_v0::classifier::ref_v1::ref_zhongshus_from_components`/`ref_v1_interval`。
- **fixture 值**: `zd=max(10,12,11)=12, zg=min(20,15,18)=15 → [12,15]`。
- **权威链**: engine-audit `reference_chanlun.py` frozen v1 rule（README §1，行 119-120）+
  chan99 §6.4 原文「走势中枢由前三个连续次级别走势类型的**重叠部分**确定」（知识库行 253）。

### 口径 C：首尾段（legacy v0，已裁决错）

- **定义**: `ZG = min(g₁, g₃)`、`ZD = max(d₁, d₃)`（首段与第三段）。
- **实装**: `Origin.CenterConstruct.legacyV0Interval`、`rust ...::legacy_v0_interval`（仅作反例保留）。
- **fixture 值**: `zd=max(10,11)=11, zg=min(20,18)=18 → [11,18]`。
- **权威链**: 无——`legacy_v0_not_correct_for_v1_reference` 裁决其不是 v1 正确实现。

## 裁决状态

| 口径 | 裁决 | 依据 |
|------|------|------|
| C（首尾段） | **已否定** | `NewChanlunEngineAudit.lean` 机器反例 + 任务明文「实装 v1 正确语义」 |
| B（全三段） | **engine canonical** | reference frozen v1 + chan99 §6.4 原文 |
| A（前两段） | **生成态** | chan99 §6.4 再定义口径；与 B 在第三段贯穿核心时重合，第三段更窄时分叉 |

## A 与 B 的关系（待澄清，生成态根因）

口径 A（前两段）与 B（全三段）**不总是相等**：
- 当第三段贯穿核心（`d₃ ≤ ZG_A ∧ ZD_A ≤ g₃`）且第三段不比前两段更窄时：A = B。
- 当第三段比前两段更窄（`g₃ < min(g₁,g₂)` 或 `d₃ > max(d₁,d₂)`）时：B 的核心严格窄于 A。
  - 例：`s0=[10,30], s1=[5,20], s2=[8,15]` → A=[10,20]，B=[10,15]（B 更窄）。

chan99 §6.4 自身的两处表述（行 253「前三个重叠部分」vs 行 267「由前两段确定」）构成**编纂版内部张力**。
reference v1 取行 253 原文口径（全三段）。本仓库现有 Origin/rust 主塔取行 267 口径（前两段）。

**生成态原因**：是否将主塔从 A 迁移到 B，涉及改动 `CenterConstruction.centersOf` + 全部下游契约锚
（CenterComplete 完整判据、Pipeline、已结算 parity）——超出本工位边界（不碰 Can 塔/组A owner）。
本工位的产出是**并行实装 B 口径 + 反例裁决 C**，不替换 A 塔。A↔B 迁移决断留编排者。

## 影响声明

- 新增口径 B 实装（Lean `Origin.CenterConstruct` § 5.5 + rust `ref_v1.rs`），与口径 A 主塔**并行**，
  有效域分离（B = engine reference 语义层；A = chan99 §6.4 派生主塔）。
- 口径 C 仅作反例保留（`legacyV0Interval`/`legacy_v0_interval`），不进任何生产路径。
- parity 耦合 `theta_v0_center_parity.rs`（Lean #eval → fixture → rust bit-exact）。

## 谱系依据

- 003：线段两口径分离（v0/v1）——本记录是中枢域的同构分离（「概念分离需要谱系化」）。
- 098：口径结算结晶范式——v0 降级为参考、v1 为正式口径的决断模式。
- 231：形式化有效域规则——口径 A/B 有效域分离（B 是 engine reference 域，非全定义域替换 A）。
- 248：线段引擎诊断——口径分歧的引擎层诊断先例。
