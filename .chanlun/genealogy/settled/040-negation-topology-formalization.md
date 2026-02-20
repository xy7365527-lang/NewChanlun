# 040 — 否定拓扑形式维度显式化

**类型**: 语法记录
**状态**: 已结算
**结算日期**: 2026-02-20
**结算方式**: 概念（negation_source/negation_form 双字段 schema）已在后续谱系中广泛使用，形式化完成
**日期**: 2026-02-20
**negation_source**: homogeneous
**negation_form**: separation（形式维度从来源维度中分离出来——同一个"否定类型"概念内部暴露出两个不兼容的分类轴）
**前置**: 029-move-c-segment-coverage, 030a-gemini-position
**关联**: 005-object-negation-principle, 030-baseline-verification-runtime, 031-move-range-semantics, 032-divine-madness-lead-self-restriction

## 矛盾描述

030a 引入 `negation_type: homogeneous/heterogeneous` 区分否定来源。但 030 的实际标注 `negation_type: expansion` 已经溢出了这个二分法——expansion 描述的不是"谁产出的否定"（来源），而是"否定的结构形式"（规定者违反自身规定）。

一个字段承载了两个正交的分类轴：
- **来源维度**：谁产出的否定（homogeneous / heterogeneous）
- **形式维度**：否定的结构是什么（waiting / expansion / separation / ...）

这不是编码错误，是概念层面的未分化——030a 在命名时尚未辨认出形式维度的独立性。

## 推导链

1. 029：敞开性原则——分类框架必须为"未知类型"留位置
2. 030a：引入 negation_type，但 030 的 expansion 标注已溢出 homogeneous/heterogeneous 二分
3. 2026-02-19 对话：编排者辨认出来源与形式是两个独立维度
4. ∴ 概念分离：negation_type → negation_source（来源）+ negation_form（形式）

## 三种基本否定形式（工作假说，非封闭框架）

- **waiting**：当前不可结算，需后续回溯规定（apres-coup）
- **expansion**：规定者在执行中违反自身规定（膨胀型）
- **separation**：统一范畴内部暴露不兼容的异质性（分离型）
- **unclassified**：无法归入已知类型——可能是新亏格信号，上浮编排者

## 产出

1. genealogy-template.md：重命名 negation_type → negation_source，新增 negation_form 字段
2. SKILL.md：新增"否定形式类型"元编排规则段
3. dispatch-spec.yaml：ceremony 序列增加 definition-base-check 步骤
4. 全部已有谱系中 negation_type → negation_source 同步重命名
5. 本条谱系

## 被否定的方案

- **"negation_type 单字段足够"**：030 的 expansion 标注已经证明不够——它描述的是形式而非来源，但被塞进了来源字段。
- **"形式维度可以后补"**：029 的敞开性原则要求显式化，不是隐式容忍。未显式化的维度会在后续谱系写入中被遗忘。

## 影响范围

- **genealogy-template.md**：字段结构变更
- **SKILL.md**：新增否定形式类型规则段
- **dispatch-spec.yaml**：ceremony 序列增加步骤
- **已有谱系**（030, 030a, 031, 032）：字段重命名
- **methodology-v3.3.md**：表格中 negation_type 标注更新
- **gemini-challenger.md**：谱系写入字段更新

## 谱系关联

- **parent**: 029-move-c-segment-coverage（敞开性原则）, 030a-gemini-position（negation_type 首次引入）
- **children**: （待后续谱系填充）
