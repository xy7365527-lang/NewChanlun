# 诊断报告：为什么新的并行研究线没有出现

**时间**: 2026-03-05
**模式**: diagnose（Codex 异质否定）
**诊断者**: codex-heterogeneous-reviewer
**Codex 持久化**: `.chanlun/review-results/codex-diagnose-20260305-0012.md`

---

## 结论

**Codex 判定：定义冲突，不是实现错误。**

我判定：**否定成立，诊断准确**。以下是完整分析。

---

## 1. ceremony_scan 能否生成新研究线？

**不能。**

`_scan_research_lines()` 的逻辑严格等价于：
```python
for gang in gangmu.yaml["gang"]:
    for mu in gang["mu"]:
        if mu.status != "active":
            continue
        for action in mu["next_actions"]:
            if action.is_unblocked and not action.is_completed:
                yield workstation(action)
```

整个 ceremony_scan 扫描链路（roadmap → session → pattern-buffer → review-results → topo-context → encounter → research-lines → anomalies → async-self-ref → meta-rule）**没有任何分支会向 gangmu.yaml 写入新 mu**。

ceremony_scan 是纯粹的消费器，不是生成器。

---

## 2. 当前 gangmu.yaml 状态（精确扫描）

### 活跃目：仅 1 个

| gang | mu | 未完成的 action |
|------|----|----------------|
| simatrix-quanyu | multi-target-scanner | scanner-l2-validation（completion_check: genealogy_settled: scanner-validation，当前未满足） |

注意：scanner-quotient-rank 已标注 completed_at: 2026-03-04，所以 scanner-l2-validation 的 blocked_by 已解除。该 action 在逻辑上是 unblocked 的，应该出现在 ceremony_scan 输出中。如果 v159 没有输出它，需要单独检查 `tests/test_quotient_rank.py` 是否存在。

### 阻塞目：1 个

| gang | mu | 阻塞原因 |
|------|----|---------|
| simatrix-quanyu | k4-regime-analysis | 等待新的全图同步压缩数据事件 |

### 其他：全部 closed

---

## 3. 总方针 vs gangmu 覆盖度差距

| 总方针阶段/内容 | gangmu 是否覆盖 |
|----------------|----------------|
| 阶段A（协议设计） | 无对应 mu |
| 阶段B（区块拓扑+回测） | 已关闭（block-topology-eng） |
| 阶段B-T（穿越基础设施） | 已关闭（traverse-infra）—— 但组件三/四是否完成需确认 |
| **阶段B-M（类型化离散Morse数学研究）** | **无对应 mu，即将触发（355号L2结果就是触发条件）** |
| 阶段C（实盘接入） | 无对应 mu |
| 阶段D（本地模型接入） | 无对应 mu |
| 阶段E-H（分布式/智能合约/资源/退出） | 无对应 mu |
| 阶段I-1至I-5（核心回路全域展开） | 无对应 mu |
| 353号推论1/2（spec-execution-gap + ceremony_scan 双向扫描） | **无对应 mu** |
| 355号边界条件（加权 Morse） | **无对应 mu** |
| topo-analyst 发现（schema漂移/supersedes=0/51张力） | **无对应 mu** |

---

## 4. 可立即开启的新研究线候选

按紧迫性排序（源自已结算谱系的下游推论，无需编排者额外判断）：

### 候选1（高优先级）：353号两条未执行推论 → 工程目

353号已结算（2026-03-04），其下游推论1/2在 v158 结束时明确标注"未执行"（356号观察5）：

- **目1**：spec-execution-gap-bidirectional — 更新 `.claude/skills/spec-execution-gap/SKILL.md`，增加消费断裂（产出→消费）检测方向
- **目2**：ceremony-scan-bidirectional — 修改 `ceremony_scan.py`，增加"产出端是否有消费者"扫描

这两个是定理类（从 353号结算态直接推导），应该直接做，不需要等编排者。

### 候选2（中优先级）：阶段B-M 触发 → 数学研究目

总方针§八.五第二段：
> "触发条件：在穿越实践中遭遇 BFS 退化版和直觉严重不一致的情况"

355号结论："Morse 跳跃幅度不是'关键否定'的预测器"——这就是 BFS 退化版与编排者直觉不一致的 L2 证据。

**触发条件已满足，但没有人把它转化为 gangmu 新 mu。**

候选目：
- **morse-topology 纲下新增 mu**：`typed-morse-theory`
  - action1：构造最优离散 Morse 函数（不依赖 BFS 生成树选择）
  - action2：按关系类型加权（negates > supersedes > references > depends_on）的加权 Morse
  - action3：分岔点（临界集质变的谱系编号 t）——离散 Cerf 理论片段

### 候选3（中优先级）：topo-analyst 的结构性发现 → 工程目

topo-analyst 发现的问题没有对应的 gangmu action：
- schema 三格式并存（需统一）
- supersedes/reopens/modifies 关系数 = 0（拓扑层关系缺失）
- 51条张力悬置

候选目：
- **block-topology 纲下新增 mu**：`topo-schema-cleanup`
  - 修复 schema 漂移、补全缺失关系类型、消化悬置张力

### 候选4（低优先级/需编排者决策）：阶段C 实盘接入

需要真实市场数据、编排者提供初始资金，不是蜂群自主能推进的。属于"选择"类，需要编排者价值判断。

---

## 5. 定义冲突详细说明（Codex 原话）

> **冲突方**：
> - A（现有定义）：`ceremony_scan` 只消费 `gangmu.yaml` 的 active 条目与既有队列，不负责生成新研究线。
> - B（期望定义）：当活跃纲目关闭/耗尽时，系统应从已结算谱系或下游推论中自动生成新研究线。
>
> **不可弥合理由**：这需要新增"推论→纲目"的规则、优先级、审批与写入机制，已超出现有 `ceremony_scan` 定义。

---

## 6. 四种方案评估

| 方案 | 描述 | 可行性 | 决断类型 |
|------|------|--------|---------|
| A | 编排者手动添加新研究线到 gangmu.yaml | 立即可行 | 编排者操作（选择） |
| B | ceremony_scan 从下游推论自动提取新工位 | 可实现，但需要定义"推论→gangmu"的规则 | 语法记录（新机制） |
| C | ceremony_scan 从总方针扫描未开启阶段 | 可实现，但总方针的阶段边界是定性描述 | 语法记录（新机制） |
| D | gangmu_update.py 扩展 + 谱系 frontmatter 新增 `downstream_actions` 字段 | 最严格，从谱系结构出发 | 架构决定（选择） |

---

## 7. 否定性判定

Codex 否定是否成立？**成立**。理由：

1. Codex 正确识别了 ceremony_scan 的角色边界（消费者 vs 生成者）
2. Codex 正确诊断为定义冲突而非实现错误
3. "从谱系下游推论自动生成 gangmu 条目"这个能力当前确实不存在

Codex 的否定没有基于误读——它的诊断与代码完全对应。

---

## 8. 影响声明

- **ceremony_scan.py**：当前逻辑正确（就现有定义而言），不需要修改才能"正常工作"
- **gangmu.yaml**：需要由编排者或新机制向其中注入新 active mu，蜂群才能继续生长
- **353号推论1/2**：这两个是定理类推论，可立即作为工程目添加到 gangmu.yaml 的 swarm-infra 纲下
- **总方针阶段B-M**：355号L2结果已满足触发条件，可作为 morse-topology 纲下新 mu 开启

---

## 建议汇报给 Lead

1. **立即**：将 353号推论1/2 作为两个 next_action 添加到 gangmu.yaml（swarm-infra 纲，新增 `spec-execution-gap-upgrade` 目）
2. **立即**：将 topo-analyst 发现的三类问题添加到 gangmu.yaml（block-topology 纲，新增 `topo-schema-cleanup` 目）
3. **建议编排者决断**：阶段B-M（类型化 Morse 数学研究）是否现在开启（355号触发条件已满足）
4. **可选**：实现 ceremony_scan 从谱系 frontmatter 的 `downstream_actions` 字段自动生成 gangmu 条目的机制（方案D，解决根本问题）
