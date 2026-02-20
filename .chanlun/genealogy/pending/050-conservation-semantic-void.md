# 050 — 守恒约束语义空洞：方向守恒 ≠ 资本守恒

**类型**: 定理
**状态**: 生成态
**日期**: 2026-02-20
**negation_source**: heterogeneous（Gemini 数学验证 + 异质质询）
**negation_form**: negation（"守恒约束"的语义被否定：它不表达资本守恒）
**前置**: 026-cash-vertex-dual-role, 047-cash-vertex-redefinition
**关联**: 025-flow-relation-matrix

## 现象

Gemini 对 spec/theorems/012-conservation-constraint.md 的形式化验证通过（数学证明成立），但异质质询发现三个语义缺陷：

1. 定理证明的是方向指标（{-1,0,+1}）的代数和为零，不是资本量的守恒
2. 反对称性使得 Σnet(v)=0 恒成立——"守恒破缺"在模型内不可能发生
3. check_conservation 函数永远返回 True，是语义空洞的

## 推导链

1. 完全图 K₄ 上反对称流的 Σnet(v)=0 是图论基本性质（定理，非选择）
2. 但 flow ∈ {-1,0,+1} 丢弃了 magnitude → 无法表达"资本量守恒"
3. 反对称性是定义内置的 → Σnet(v)=0 是同义反复 → check_conservation 永远为 True
4. "守恒破缺"需要重新定义：检测 magnitude 加权后的不平衡，或引入外部节点

## 已结算原则

**方向守恒（Σnet(v)=0）是图论恒等式，不是物理守恒律。**

- check_conservation 的当前实现是语义空洞的（永远返回 True）
- 要检测真正的"资本守恒破缺"，需要 magnitude 加权的守恒检查
- 026号消歧函数不受影响（基于方差，不依赖守恒约束）

## 待结算条件

1. [ ] 编排者确认此定理
2. [ ] 审查 check_conservation 函数的实际调用方，评估影响范围
3. [ ] 决定是否重写 check_conservation 为 magnitude 加权版本

## 溯源

[新缠论]（Gemini 数学验证揭示的语义缺陷）
