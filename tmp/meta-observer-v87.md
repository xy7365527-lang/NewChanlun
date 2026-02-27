# v87-swarm 二阶观察

date: 2026-02-27
type: meta-observation
swarm: v87-swarm
observer: meta-observer (Lead 内联执行)

---

## 观察 1（收敛信号）：编排者战略决策闭环速度

### 现象

编排者提供三项战略决策 → v87-swarm 一轮全部实施完毕：
1. ceremony_scan roadmap_awareness 层 → roadmap-awareness 工位完成
2. K4 独立边独立性形式化 → k4-independence 工位完成
3. ceremony-step-guard hook 注册删除 → Lead 直接执行 + hook-cleanup 工位完成

从决策到实施：一轮闭环。

### 判定

正面信号。与 v86 meta-observer 指出的"v85 下游推论一轮闭环"同构——manual_dispatch 模式下 Lead 直接消费编排者决策，闭环速度快。

### 与 v86 观察2 的关系

v86 观察2 指出 manual_dispatch 模式下闭环速度快。v87 进一步证实：编排者决策 → manual_dispatch → 一轮闭环。这是 roadmap_awareness 层设计的实证支持——manual_dispatch 确实比 auto-dispatch 更适合战略推进。

---

## 观察 2（设计质量）：roadmap_awareness 层的拓扑位置

### 现象

`compute_advancement_candidates` 函数在 `main()` 中的位置：在所有 workstations 追加完成后、`clean_terminate` 计算之前。

输出字段 `advancement_candidates` 不进入 `workstations` 列表，不影响 `clean_terminate` 判定。

当前检出 2 个 advancement_candidates（replay_engine_upgrade 多TF路径、t6t7 谱系写入）。

### 判定

设计正确。编排者原话的核心约束得到了精确实现：
- "scan 负责呈现材料，编排者负责解释/行动" → advancement_candidates 是纯信息
- "把战略推进降格为'可扫描的债务'等于把主体位置从编排者那里抽走了" → 不进入 workstations，主体位置在编排者

### 边界条件

`advancement_candidates` 的信息来源有限（roadmap completion_note 中的关键词、最近10个谱系、最近一个 meta-observer 产出）。如果编排者需要的推进方向不在这三个来源中，advancement_candidates 不会呈现——但这不是缺陷，因为编排者可以直接 manual_dispatch 任何方向，不依赖 scan 呈现。

---

## 观察 3（收敛信号）：ceremony-step-guard 生命周期完成

### 现象

ceremony-step-guard.sh 的完整生命周期：
1. 创建（P0 修复）：解决 ceremony 协议的 LLM 无状态问题
2. 发现误伤（228-1，v82-swarm）：block 逻辑对非 Lead 工位产生误阻断
3. block 逻辑删除（v86-swarm）：所有步骤静默退出
4. hook 注册删除（v87-swarm）：从 settings.json 移除 9 个 matcher

从"必要防护"到"冗余噪声"到"空壳"到"删除注册"——完整的组件生命周期。

### 判定

收敛。228-1 从 v82 发现到 v87 完全清理，跨 5 轮完成。hook 文件保留作为谱系文档。

---

## 自环检查

| 历史观察 | v87 状态 | 判定 |
|---------|---------|------|
| 228-1 步骤7-10 误伤 + resolved 状态不一致 | **完全清理**——hook 注册删除，228-4 resolved | 收敛（v82→v87，跨5轮） |
| ceremony_scan 战略推进检测缺口 | **已解决**——roadmap_awareness 层实现 | 收敛（v83-v86 连续指出→v87 修复） |
| meta-observer 产出需手动消费 | **未变**——manual_dispatch 升格但仍需编排者主动消费 | 维持（设计如此） |
| 连续修复轮 | v87 是战略决策实施轮，非修复轮 | 正面——打破修复循环 |

## 核心发现

1. **v87 没有新的发散信号**——所有产出是已知信号的收敛或编排者决策的实施。
2. **ceremony_scan 战略推进检测缺口连续六轮被 meta-observer 指出（v83-v86），v87 正式解决**——roadmap_awareness 层。
3. **ceremony-step-guard 完整生命周期闭合**——从创建到删除注册，跨多轮完成。

## 下游推论

无。v87 是纯实施轮，无新概念发现。
