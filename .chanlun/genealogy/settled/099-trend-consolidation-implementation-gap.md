---
id: "099"
title: "趋势/盘整实体化缺口记录"
status: "已结算"
type: "语法记录"
date: "2026-02-22"
depends_on: []
related: ["007", "009", "010", "029", "031"]
negated_by: []
negates: []
---

# 099 — 趋势/盘整实体化缺口记录

**类型**: 语法记录（记录性质，非解决性质）
**状态**: 已结算
**日期**: 2026-02-22
**域**: 走势类型（Move）、趋势（Qushi）、盘整（Panzheng）
**溯源**: [旧缠论:选择] — 007/009 号编排者决断

---

## 缺口描述

007 号谱系决断"趋势建模为 Trend(Move) 子类型"，009 号谱系对称地确认"盘整建模为 Consolidation(Move) 子类型"。两条谱系均已结算。

但当前代码和定义文件中，趋势/盘整仍以 `Move(kind="trend")` / `Move(kind="consolidation")` 字符串标签形式存在，未实现子类型特化。

### 声明-实现落差详细

| 维度 | 007/009 号声明 | 当前实现 |
|------|---------------|---------|
| 数据模型 | `Trend(Move)` / `Consolidation(Move)` 子类型 | `Move(kind="trend"/"consolidation")` 字符串标签 |
| 独有属性 | 趋势: 方向、a+A+b+c 结构、趋势力度 | 无（Move 基类无此字段） |
| 独有行为 | 趋势背驰 vs 盘整背驰语义分离 | 未分离（均在 Move 上操作） |
| 定义文件 | `qushi.md` 待创建 | **不存在** |
| zoushi.md | 需引用 qushi.md | 未引用（分散描述） |

### 后续谱系的事实立场

029 号（Move C段覆盖修复）和 031 号（Move 价格范围语义）均在 `Move(kind=...)` 基础上工作，未质疑字符串标签模式。这构成了对 007 号决断的**事实上的静默否定**。

## 影响评估

### 当前 kind 字符串足以支撑的操作

- 盘整/趋势基本分类（`len(centers)==1` vs `>=2`）
- Move 生命周期管理（settled 标记）
- 递归构造（RecursiveLevelEngine 不依赖 kind）
- 中枢组件匹配（zhongshu_from_components 不依赖 kind）

### 若不实现子类型会缺失的能力

- 趋势独有属性的类型安全访问（方向、a+A+b+c 结构）
- 趋势背驰 vs 盘整背驰的编译期分离
- 趋势力度度量的独立定义
- 对 Move 子类型的穷尽模式匹配（exhaustive pattern matching）

## 边界条件

- 如果 `Move(kind=...)` 字符串标签足以满足所有当前和可预见的分类操作，此缺口可作为**设计选择**保留（不是所有谱系决断都必须立即实现）
- 如果后续需要实现严格的走势类型系统（趋势力度、趋势背驰语义分离等），则需将 kind 从字符串标签重构为 `ZoushiType` 枚举或子类型层级
- qushi.md 的缺失是独立于代码的定义缺口——即使代码暂不重构，定义文件也应创建以提供可引用的概念锚点

## 建议行动（非阻塞）

1. **短期**：创建 `qushi.md` 定义文件，从 zoushi.md 提取趋势/盘整独立定义
2. **中期**：评估 `Trend(Move)` / `Consolidation(Move)` 子类型重构的收益/成本
3. **长期**：如果走势类型系统需要趋势力度等高级功能，执行子类型重构

## 谱系链接

- 来源: 007-trend-is-entity（趋势独立实体决断）
- 来源: 009-consolidation-identity（盘整独立实体决断）
- 相关: 010-construction-vs-classification（构造层/分类层二层架构——子类型属于分类层）
- 相关: 029-move-c-segment-coverage（在 Move(kind=...) 上工作）
- 相关: 031-move-range-semantics（在 Move(kind=...) 上工作）
