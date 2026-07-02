---
id: 638
status: 生成态   # 待#37 codex 裁定
title: 买卖点候选→走势元素附着判准（本级右端点命中，非区间包含）
date: 2026-06-28
type: 语法记录
authority_chain: [缠师博文(一级), codex异质审查, spec/Lean源头审计]
related: [547, 574, 608, 006, coverage-engine-needs-tower-export-bridge, b2s2-still-missing-tower]
---

# 638 买卖点候选 Γ(x) → LeveledMove 走势元素附着判准

## 触发

塔导出桥 (ii) 喂入段需把每个买卖点候选 BspPoint 附着到其所属走势元素
LeveledMove，再由该走势在嵌套塔里的真 RMove::Compose 父取父级方向 σ_{p(g)}，
据此算操作角色 V（Ambient/FollowParent/ShortDiff）。`BspPoint`
(rust classifier/bsp.rs:101) 只带 source_index/bits/pivot，**无所属走势引用**——
附着判准未在 spec/Lean 显式定义。

## 矛盾的精确形式（为何是真定义层问题而非平凡）

机器事实：`BspPoint.source_index` 由构造**恒等于宿主走势/线段的 end_index**
（signal.rs:99 `source_index: s.end_index`；一/三类 signal.rs:257、二类回拉走势 m2
recursive_tower.rs:240）。即**每个买卖点都恰坐在某走势端点上**。端点是相邻两走势
的共享交界（前一条结束=后一条开始）。因此"区间包含 [start,end]∋source_index"在
**每个** bsp 处都落边界 → 恒二义（同 608 号点-区间问题的结构，但域不同）。

## 三路证据交叉裁定

| 维度 | 缠师原文(一级) | codex异质审查 | 收敛 |
|------|--------------|--------------|------|
| 附着判准 | 本级端点命中（第21课:66升跌完备性定理：走势由线段构成，线段端点是某级别三类买卖点之一） | 选项1：同级+规范塔+产出该点的右端点 host（end_index==source_index） | ✅ 端点命中 |
| 定理 vs 选择 | 称"领域定理钉死" | **纠正：定义层选择(语法记录)非缠论定理** | codex 更严格 |
| 父方向 | 真 Compose 父 | 真 Compose 父(push_element_tree sub_moves 真包含 coverage.rs:124) | ✅ |
| 608 关系 | 同类新实例 | 只能类比不能直接套(域不同:source_index vs span ≠ price vs [ZD,ZG]) | codex 更精确 |

**codex 对原文工位的纠正（关键，防声明膨胀）**：缠师原文钉死的是"买卖点有级别
(第16课:526/第17课:70)+ 买卖点是端点(第21课:66)"，但**没给 raw point → host move
的全函数 hostOf**。spec §13 只设 p(g)∈C_{ℓ_g+1}（父级别）+ V 依赖 σ_{p(g)}，
未定 hostOf。故 hostOf 是**工程语法定义（已运作的生产约定显式化）**，非可从缠论文本
推出的唯一定理。把它说成"原文证明的定理"是声明膨胀的反面错误。

## 裁定（canonical 定义式）

```
hostOf(g) = 规范递归塔中、候选所在级别 ℓ_g、产出该 BspPoint 的走势元素 LeveledMove
            （其 end_index == g.source_index）
parent(g) = hostOf(g) 在塔里的真 RMove::Compose 父
σ_{p(g)}  = parent(g).rmove_side（父走势方向）
```

- **选项1 > 选项2 >>> 选项3**（codex 排序）。选项2（区间包含）在端点恒二义，加"右端归产出段"边界约定后退化为选项1。选项3（级别差伪造父）= 547 直接否定的旧 bug，删除。
- 父步骤复用既有真塔机制（push_element_tree parent 只来自 sub_moves 真包含），真 Fugue 安全。

## 真 Fugue 隐藏漏洞（codex 标，实装必防）

1. BspPoint 不带 level/host id 只带 source_index → 附着函数必须携带 LevelState/候选 ℓ_g 上下文。
2. 必须严格用"右端点 end_index==source_index"，**禁** [start,end] 包含或任一端点命中（否则相邻走势二义）。
3. `index_of_in` (recursive_tower.rs:246) 按 RMove 结构相等找首个 submove，文件自承结构全等时坐标不可区分 → 严格版须传稳定路径/ordinal 身份，不能只靠结构相等。

## 为何 pending（生成态，未 settle）

- 含 3 个待实装坐实的 Fugue 漏洞（结构相等歧义须改 ordinal 身份）。
- (B) 操作角色"主力级别"绝对锚仍未裁（原文第48课:84 留给操作者，547 暂定"最高活跃走势级别"仍生成态）——这是**独立的**待编排者裁的自由度，不属本条 hostOf 附着，但下游 V→FollowParent/ShortDiff 翻转依赖它。
- settle 条件：塔导出桥 (ii) 实装 + 测试坐实 end_index 唯一命中 + ordinal 身份消歧 + (B) 主力锚口径编排者裁定。

## 边界条件（结论翻转）

- 若 source_index 不再恒等于 end_index（构造改变）→ 端点命中失效，需重裁。
- 若找到缠师原文把买卖点定义为"区间内部点"而非端点 → (C) 翻转（目前第21课:66 直接反对）。
- 若 (B) 主力锚被裁为客观定理而非操作者选择 → 操作角色绝对锚升为领域定理。

## 影响声明

定义 hostOf 附着判准（塔导出桥 (ii) 喂入段的前置）。影响：买卖点→操作角色 V 的
形式化自由度划分——附着=语法记录(本条)、方向逻辑=原文钉死、主力锚=待裁(独立)。
不改既有代码（本条是定义登记）。
