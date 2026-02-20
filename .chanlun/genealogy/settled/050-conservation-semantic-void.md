---
id: "050"
title: "守恒约束语义空洞：方向守恒 ≠ 资本守恒"
status: "已结算"
type: "定理"
date: "2026-02-20"
depends_on: ["026", "047"]
related: ["025"]
negated_by: []
negates: []
---

# 050 — 守恒约束语义空洞：方向守恒 ≠ 资本守恒

**类型**: 定理
**状态**: 已结算
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

## 调用方审查（2026-02-20 ceremony 审计）

**生产代码调用方：0 个。** check_conservation 仅在测试中被调用。

**测试调用方（10 处 assert）：**
- tests/test_cash_disambiguation.py:250 — 1 处
- tests/test_flow_relation.py:52,71,82 — 3 处
- tests/test_flow_relation_e2e.py:209,316 — 2 处
- tests/test_flow_timeline.py:77,128,173 — 3 处
- tests/test_flow_timeline_e2e.py:213 — 1 处

所有调用形式均为 `assert check_conservation(states)`，永远为 True（同义反复）。

**影响评估：** 删除或重写 check_conservation 不影响任何生产逻辑。测试中的 10 处 assert 需同步修改。

## 待结算条件

1. [x] 编排者确认此定理 ← 四分法复核：定理级自动结算
2. [x] 审查 check_conservation 函数的实际调用方，评估影响范围 ← 已完成
3. [x] 决定处置方式 ← 四分法复核：方案A（删除）是逻辑唯一解
   - 方案C 被 no-workaround 消除（注释代替处置 = 软性绕过）
   - 方案B 被 testing-override 消除（magnitude 定义未结算，不可构建）
   - 方案A 是唯一剩余 → 定理，非选择

## 结算提案（definition-lawyer 工位，2026-02-20）

### 定理部分（自动结算，无需编排者确认）

**"方向守恒（Σnet(v)=0）是图论恒等式"** 已满足自动结算条件：
- 推导链唯一（反对称流的代数性质，无选择空间）
- 026 号谱系已结算，消歧函数不依赖守恒约束（026 号第 45 行明确标注）
- 四分法分类：定理

→ 此部分直接结算，写入已结算原则，无需编排者审查。

### 选择部分（需编排者决断）

`check_conservation` 的处置方式有三个方案，无法从已有定义中推导出唯一答案：

---

**方案 A：删除 check_conservation**

- 操作：删除 `flow_relation.py:169-175`，同步删除 10 处测试 assert
- 优点：
  - 消除语义空洞，代码库不再携带永远为 True 的断言
  - 减少认知负担：读者不会误以为存在真实的守恒检查
  - 符合"不绕过矛盾"原则——空洞函数本身是一种绕过
- 缺点：
  - 丢失了"此处曾有守恒约束"的历史痕迹（但谱系已记录）
  - 若未来引入 magnitude 加权，需重新添加函数

---

**方案 B：重写为 magnitude 加权守恒检查**

- 操作：将 `check_conservation` 改为检查 `Σ|net_flow * magnitude|` 的不平衡度
- 前提：需要 `VertexFlowState` 携带 magnitude 字段（当前不存在）
- 优点：
  - 赋予函数真实语义——检测资本量层面的守恒破缺
  - 为未来的资本流量分析提供基础
- 缺点：
  - 需要修改 `VertexFlowState` 数据结构（涉及 047 号谱系的现金角重定义）
  - magnitude 的来源和单位需要新的定义（当前 flow ∈ {-1,0,+1} 无量纲）
  - 工作量显著大于方案 A/C，且引入新的概念依赖
  - 在 magnitude 定义结算前，方案 B 处于生成态——不可立即执行

---

**方案 C：保留但标注为 tautology**

- 操作：在函数 docstring 中明确标注"此函数是同义反复，永远返回 True"，测试保留但改为文档性注释
- 优点：
  - 最小变更，零风险
  - 保留历史痕迹，提醒读者此处有概念债务
- 缺点：
  - 语义空洞依然存在于代码库中
  - "永远为 True 的断言"仍然占据测试文件，可能误导覆盖率统计
  - 是一种软性绕过（用注释代替真正的处置）

---

### 推荐路径（definition-lawyer 分析）

从"不绕过矛盾"原则出发：
- 方案 C 是软性绕过，不推荐
- 方案 B 在 magnitude 定义未结算前不可执行，且引入新依赖链
- **方案 A 是当前最干净的选择**：删除语义空洞，谱系已记录历史，未来需要时可重建

但方案 A vs 方案 B 涉及价值判断（"现在删除 vs 未来重建"的工程取舍），属于选择类，路由编排者决断。

### 待编排者决断的问题

> 选择 A（立即删除）还是 B（重写为 magnitude 加权，需先结算 magnitude 定义）？

方案 C 不推荐，已排除。

## 溯源

[新缠论]（Gemini 数学验证揭示的语义缺陷）
