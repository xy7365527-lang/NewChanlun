---
status: settled
created: 2026-06-22
settled: 2026-06-23
crystallizer_session: skill-crystallizer/2026-06-23
pattern_source: scan-state-unobservable
gemini_decide_result: "A（独立新建），CONFIDENCE: HIGH — 531号先例：gemini-challenger decide 模式（配额恢复后重试成功）"
settled_action: "已写入 .claude/commands/ceremony-scan-completeness.md + 注册 manifest.yaml"
---

# 结晶草案：ceremony-scan-completeness

以下为 skill 草案全文（等待人类批准后写入 `.claude/commands/ceremony-scan-completeness.md`）：

---

```markdown
---
description: >
  ceremony_scan 完备性守卫。检测蜂群中"全局状态 X 既无业务工位消费者、又无 ceremony_scan
  消费者"的结构性盲区。结晶自 429/430/432/433 号 meta-rule 谱系，429号候选1首次识别（5实例）。
genealogy_source: "429-候选1"
---

# /ceremony-scan-completeness — scan 状态不可观测模式检测

## 核心原则

ceremony_scan 基于**静态文件扫描**（YAML frontmatter + 文本匹配），不消费**运行时产出**
（block topology blocks、daemon 状态、genealogist 评估结论）。

这导致两类盲区：

| 盲区类型 | 现象 | 根因 |
|---------|------|------|
| 已解决但 scan 不知 | 已执行的工位在下次 scan 中重复出现 | scan 消费的是静态声明，不消费执行结果 |
| 已消费但 scan 不知 | 已消费的推论在 rescan 中再次被检出 | coverage_status 基于文本匹配，不基于 genealogist 评估结论 |

**094号谱系 Gap 在工程基础设施层的复现**：094号识别的"审计层断裂"在 ceremony_scan
这一元编排基础设施层有系统性复现。这不是单个代码 bug，而是 scan 架构层面的设计边界。

## 与 spec-execution-gap 的区分

| skill | 处理的问题 | 层级 | 检测对象 |
|-------|-----------|------|---------|
| spec-execution-gap | 单个声明/产出端无消费者 | 代码层 | 函数返回值、字段、模块导出 |
| ceremony-scan-completeness | scan 架构整体覆盖盲区 | 元编排基础设施层 | frontmatter 字段、运行时变量 |

两者均涉及"消费者缺失"，但定义域和修复路径不同。

## 检测清单

每次向 frontmatter 新增字段或引入运行时变量时，执行以下检查：

1. **业务工位消费**：是否有任何业务工位在执行中读取/消费此字段？
2. **ceremony_scan 消费**：是否在 `ceremony_scan.py` 的检测逻辑中被消费？
3. **运行时状态**：此字段的值是否依赖运行时产出（daemon 状态、block topology、agent 评估结论）？

如果第3条为是，且第1/2条均为否 → 此字段是 scan 盲点。

**处置方式**：
- 将消费逻辑添加到 ceremony_scan 的消费器
- 或写入 pending 谱系（同 pending-001-scan-state-unobservable 格式）

## 已知应消费但未消费列表（截至 433号，2026-03-12）

以下字段/状态在 ceremony_scan 中为盲点：

| 字段/状态 | 来源 | 盲区描述 | 初次记录 |
|----------|------|---------|---------|
| `topo_effect`（frontmatter） | 谱系 YAML | 已执行的 effect 在每次 scan 中重复出现 | 429号观察3 |
| 推论 `coverage_status` | genealogist 评估结论 | 已消费推论在 rescan 中再次被检出 | 429号观察3 |
| ARTICULATE 实装状态 | traversal.py 运行时 | scan 无法区分"代码已写入"和"已产生ARTICULATED边" | 429号观察3 |
| 已消费推论（ceremony 轮次） | genealogist 历次评估 | v231 rescan 重复检出已消费推论 | 432号观察3 |
| async_self_ref stagnation delta=0 | daemon 运行时 | 快速连续 session 下 stagnation 误报 | 433号观察1 |

## 形式化模式

当全局状态 X 满足：
- (a) X 不被任何业务工位消费（无局部消费者）
- (b) X 不被 ceremony_scan 消费
- (c) X 是 frontmatter 字段或运行时变量

则 X 处于 **scan 状态不可观测** 状态，形成 Lead 盲点。

消除方式：将 X 加入 ceremony_scan 的消费器，或将运行时状态转化为 scan 可读的静态标记。

## 修复模式

### 静态化（推荐）

将运行时状态写入可被 scan 静态读取的文件：

```yaml
# 示例：genealogist 评估结论写入谱系 frontmatter
evaluation_status: consumed  # 而非依赖文本匹配
```

### 消费器注入

在 ceremony_scan.py 中增加对运行时状态的消费逻辑：

```python
# 示例：读取 daemon 状态文件而非检查代码存在性
articulate_status = read_daemon_state("articulate_edge_count")
if articulate_status > 0:
    mark_topo_effect_as_executed()
```

### pending 归档

若无法立即修复，写入 pending 谱系（格式同 pending-001），让 genealogist 追踪：

```yaml
# .chanlun/genealogy/pending/pending-XXX-scan-blind-spot.md
type: scan-blind-spot
field: <字段名>
source: <frontmatter 所在文件>
discovered: <谱系编号>
status: pending
```

## 边界条件

本 skill 在以下条件下应被重新审视：

- ceremony_scan.py 重构为消费运行时状态（架构变化使本 skill 大部分失效）
- 蜂群引入语义事件总线（016号 runtime enforcement layer 实装）
- scan 盲点条目超过 10 个（复杂度可能超过收益，考虑重构 scan 架构）
- 已知盲点列表被清空（本 skill 进入 auto-verified 状态）

## 谱系来源

- 094号：审计层断裂 Gap（根源）
- 429号候选1：scan 状态不可观测首次识别（3实例）
- 430号候选1：候选继承
- 432号观察3/候选1：第4实例（已消费推论重复检出）
- 433号候选1：第5实例，"已达阈值"明确声明
- 051号：运行时连接设计，规定 Pull 模型下 ceremony_scan 的消费职责
```
