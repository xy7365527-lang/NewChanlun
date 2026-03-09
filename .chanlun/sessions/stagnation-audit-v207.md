# 停滞审计报告 v207

**日期**: 2026-03-09
**触发**: ceremony_scan 检测到 P1 stagnation（settled 计数不变 + depends_on 未引用）
**审计者**: v207-swarm/stagnation-audit

---

## 1. 停滞诊断

### 1.1 表面现象

ceremony_scan 的 async_self_reference 模块报告两个 stagnation finding：

1. **settled 计数不变**：t-1(0445) 和 t(0453) 两个 session 的 settled 计数均为 404
2. **depends_on 未引用**：t-1 产出的谱系 ['404'] 未被后续谱系 depends_on 引用

### 1.2 根因分析

**v206-swarm 的产出性质**：

v206-swarm 完成了 5 个工位（git log: `f40028b`），产出全部是基础设施/代码层面的：

| 工位 | 产出类型 | 谱系影响 |
|------|----------|----------|
| peer-instances | 前端对等架构 | 无新谱系 |
| kfull-migration | k_full.jsonl→block topology 迁移 | 无新谱系 |
| snet-expand | S_net 辞典全量补充 | 无新谱系 |
| articulation-feedback | 匹配率 66.7%→90.7% | 无新谱系 |
| genealogy-fix | frontmatter 修复 + 重复编号解决 | 产出 403/404/405（从旧编号 382/383/384 重编号）|

关键事实：
- 404 号和 405 号是 genealogy-fix 工位将重复编号的旧谱系（原 383/384）重编号的结果，不是新的概念发现
- v206-swarm 的 5 个工位全部是基础设施工作（代码迁移、辞典扩容、前端架构、谱系元数据修复）
- 基础设施工作不产生新的谱系条目是正常的

### 1.3 诊断结论

**这是 false positive stagnation**。原因：

1. `async_self_reference.py` 的 stagnation 检测逻辑仅比较 settled 计数的 delta。delta=0 时触发 stagnation，但不区分"无进展"和"非谱系型进展"
2. v206-swarm 的进展全部在代码/基础设施层面，这种进展不在谱系 settled 计数的观测面上
3. depends_on 未引用的 404 号本身是重编号产物（原 383 号），不是新发现的概念——后续谱系自然不会引用它

### 1.4 ceremony_scan 改进建议

`async_self_reference.py` 的 stagnation 检测应增加一个维度：**git commit 活动**。

当 settled 计数不变但 git 有新 commit 时，stagnation 应降级为 `info`（非谱系型进展）而非 `stagnation`（P1）。具体建议：

```
if delta_settled == 0:
    if has_new_commits_since_last_session():
        classify_finding("info", "settled计数未变但有新commit——非谱系型进展")
    else:
        classify_finding("stagnation", "settled计数未变且无新commit——可能停滞")
```

这不是"绕过矛盾"——分类更精确不等于降低严格性。当前的 stagnation 检测对非谱系型进展产生 false positive，这是检测器的有效域（仅覆盖谱系型进展）小于其定义域（所有类型进展）的实例。

---

## 2. 15 条推论处理

### 2.1 四分法分类

逐条扫描 15 条 proposed_new_mu，按其在谱系原文中的审计标记分类：

#### A. consumed（已消费，标记为 resolved）— 6 条

| # | 来源 | 推论 | 消费去向 | 分类 |
|---|------|------|----------|------|
| a | 395号 | "否定发生在冗余处" | v198 审计铭写，经验发现已记录于 395号下游推论 2 | resolved |
| b | 393号 | 392号P1修正 | v198 审计铭写，393号已修正 392号P1（测量错误） | resolved |
| c | 393号 | f=0是fold强充分条件 | v198 审计铭写，方法论记录 | resolved |
| d | 393号 | Jaccard是fold最强单指标 | v198 审计铭写，方法论记录 | resolved |
| f | 393号 | 测量时机原则 | v198 审计铭写，方法论记录 | resolved |
| 390-1 | 390号 | 389号四阶段→三阶段修正 | v198 审计铭写，390号数据已回答 | resolved（隐含于 lead 分类列表中的 consumed 标记）|

说明：这 6 条在谱系原文中已标注 `[audited: v198, consumed]`，意味着 v198-swarm 已经审计并确认这些推论被后续谱系消费。它们不需要新的谱系条目或 gangmu 工位——ceremony_scan 将其标记为 not_covered 是因为 gangmu.yaml 中没有对应的 mu，但 consumed 推论不需要 gangmu 条目（它们已完成生命周期）。

#### B. deferred（需要实验数据）— 9 条

| # | 来源 | 推论 | 依赖项 | 分类 |
|---|------|------|--------|------|
| e | 393号 | f值替代Morse critical | 未实施替代规则 | deferred |
| g | 392号 | fold内禀判据重寻 | 393号已部分回答（Jaccard），但替代量计算未实施 | deferred |
| h | 392号 | g作为negate判据 | 性能评估未做（BFS独立路径枚举复杂度） | deferred |
| i | 392号 | Φ势函数修正 | 依赖推论 g 结果 + 393号 f 方向修正，Φ形式未更新 | deferred |
| j | 391号 | Φ实验验证 | λ内禀定义未实现，Φ未计算 | deferred |
| k | 391号 | λ内禀计算 | 特征环长度计算未实现 | deferred |
| l | 390号 | 多图并行优于单图 | 无多图数据，需 L3 多图实验 | deferred |
| m | 390号 | 动态平衡拓扑语义 | 稳态样本不足（仅 700 步），需 5000+ 步运行 | deferred |
| n | 389号 | "错误是燃料"可证伪条件 | FP 来源未分离（synthetic vs structural） | deferred |
| o | 387号 | fold密度阈值 | 无中间规模图数据（50-500 顶点梯度） | deferred |

### 2.2 依赖清单汇总

deferred 推论的依赖可归纳为 4 类实验需求：

| 依赖类 | 涉及推论 | 具体需求 |
|--------|----------|----------|
| **拓扑量计算实现** | e, g, h, i, j, k | 在 traversal.py 中实现 f 值替代规则 / g 的 BFS 性能基准 / λ（特征环长度倒数）/ Φ 势函数计算 |
| **多图 L3 实验** | l, o | 接入多个不同规模/结构的图，运行短穿越（500-1000 步）|
| **长运行实验** | m | 单图 5000+ 步运行，观测稳态样本 |
| **FP 来源分离** | n | 控制实验：分离 synthetic vs structural FP，禁用其中一类观测 β₁ 变化 |

当前无任何一类实验在 roadmap 或 gangmu 中被安排。这些是拓扑计算引擎的**研究线前沿**，不是当前 sprint 的任务。

### 2.3 处理结论

- **consumed 的 6 条**：ceremony_scan 的 not_covered 标记是 false positive。这些推论已完成生命周期（被后续谱系消费），不需要 gangmu 覆盖
- **deferred 的 9 条**：正确维持 deferred 状态。它们的推进依赖特定实验条件（代码实现 + 多图数据 + 长运行），这些条件当前不具备
- **需要新谱系的**：0 条。没有任何推论处于"可独立结算"状态——consumed 的已完成生命周期，deferred 的缺少实验前提

---

## 3. ceremony_scan 的 not_covered 检测精度问题

### 3.1 问题

ceremony_scan 的 `_scan_genealogy_proposals` 函数将所有在 gangmu.yaml 中无对应 mu 的下游推论标记为 `not_covered`。但下游推论有三种合法的非 gangmu 覆盖状态：

1. **consumed**：已被后续谱系消费（标注 `[audited: ..., consumed]`），生命周期已结束
2. **deferred**：等待实验条件（标注 `[audited: ..., deferred]`），不可执行
3. **resolved**：已在谱系原文中直接回答（标注 `→ resolved:`），不需要独立工位

当前检测不解析推论文本中的审计标记，导致 consumed/deferred/resolved 推论被误报为 not_covered。

### 3.2 改进建议

`_scan_genealogy_proposals` 应在提取下游推论后，检查推论文本中的审计标记：

- 匹配 `[audited: ..., consumed]` → coverage_status = "consumed"
- 匹配 `[audited: ..., deferred` → coverage_status = "deferred"
- 匹配 `→ resolved:` 或 `→ \`resolved:` → coverage_status = "resolved"
- 其他 → coverage_status = "not_covered"（真正需要处理的）

这将把 15 条 false positive 的 not_covered 减少到 0 条 true positive 的 not_covered。

---

## 4. 总结

| 维度 | 发现 | 处理 |
|------|------|------|
| settled 计数停滞 | false positive——v206 是基础设施 swarm | 记录诊断，建议改进检测 |
| depends_on 未引用 | false positive——404 号是重编号产物 | 记录诊断 |
| 6 条 consumed 推论 | 已完成生命周期，not_covered 是误报 | 标记为 resolved |
| 9 条 deferred 推论 | 正确维持 deferred，依赖实验条件 | 汇总依赖清单 |
| ceremony_scan 精度 | 两处改进点（stagnation 检测 + not_covered 解析） | 记录建议 |

---

## 谱系引用

- 404号：重编号产物（原 383号），非新概念发现
- 405号：重编号产物（原 384号），非新概念发现
- 387-395号：15 条下游推论的来源谱系
- 090号：严格性语法规则——false positive 的识别是严格性的体现，不是回避问题

## 影响声明

- 不产生新谱系条目（无新概念发现）
- 建议修改 `scripts/async_self_reference.py`（增加 git commit 活动检测）
- 建议修改 `scripts/ceremony_scan.py` 的 `_scan_genealogy_proposals`（解析审计标记）
- 15 条推论中 0 条需要当前行动，9 条维持 deferred 等待实验条件
