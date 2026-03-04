# Context Limit 根因严格诊断

## 结论

Context Limit 反复触发的根因是 **Lead 的 context 消耗率远超 session 容量**，具体由三个独立因子叠加：

1. **谱系 DAG 读取**（55KB/次）：dag_add_node.py 只解决了"添加"，但 Lead 在验证、查询边关系、确认节点存在性时仍需读取完整 dag.yaml
2. **Gemini 讨论循环**（40-80KB/轮）：ctx 文件 + 产出报告 + Lead 读取报告，两轮收敛 ≈ 80KB
3. **pattern-buffer.yaml 膨胀**（169KB）：即使清理到 324 条仍达 169KB，任何扫描操作都吃 context

加上 CLAUDE.md 本身 + skills + hooks system-reminder 注入的固定开销（估算 30-50KB），单个 session 的可用 context 在 1-2 个 ceremony 循环后即耗尽。

---

## 问题 A：Lead 为什么反复读取大文件？

### 判定：架构缺口——DAG 缺少查询接口

**现状**：
- `dag_add_node.py` 解决了"写入"——Lead 不需要读取 dag.yaml 来添加节点
- 但 Lead 在以下场景仍需读取完整 dag.yaml：
  - 验证边是否已存在（避免重复添加）
  - 查询某节点的所有依赖/被依赖关系（谱系结算时需确认上下文）
  - 检查节点是否存在（添加边前的前置检查）
  - `validate_dag.py` 完整读取（验证操作）

**根因**：dag.yaml 只有"写入工具"（dag_add_node.py），没有"查询工具"。Lead 只要需要**任何**关于 DAG 结构的信息，就必须读取完整 55KB。

**修复方案（当前可做）**：

创建 `scripts/dag_query.py`，提供以下子命令：
```bash
# 查询节点是否存在
python scripts/dag_query.py --exists 143
# 输出: true/false

# 查询节点的所有边
python scripts/dag_query.py --edges 143
# 输出: depends_on: [140, 141], related: [069], tensions_with: []

# 查询所有边类型中是否有特定边
python scripts/dag_query.py --edge-exists depends_on 143 140
# 输出: true/false

# 获取节点基本信息（不含完整 DAG）
python scripts/dag_query.py --node 143
# 输出: {id: 143, title: "...", status: "...", type: "..."}

# 获取统计摘要
python scripts/dag_query.py --stats
# 输出: nodes: 143, edges: 420, edge_types: [depends_on, related, ...]
```

**效果**：Lead 的 DAG 操作变为 Bash 工具调用（返回几十字节），而非 Read 工具调用（返回 55KB）。context 节省 ≈ 55KB × 读取次数/session。

---

## 问题 B：Gemini 讨论的 context 成本

### 判定：设计意图与实现之间的效率缺口

**现状分析**：

Gemini 讨论的 context 成本来自三个环节：

| 环节 | 大小 | 是否可压缩 |
|------|------|-----------|
| ctx 文件写入（Lead → Gemini） | 3-12KB | 可控——Lead 控制 ctx 内容 |
| Gemini 产出（报告） | 10-88KB（极端 340KB） | **此处是主要变量** |
| Lead 读取报告 | = Gemini 产出 | 取决于上一步 |

**根因**：Gemini 产出的大小无约束。`gemini-rtas-audit.md`（340KB）和 `gemini-post089-audit.md`（143KB）是极端案例——这两个文件单独就超过了一个 session 的有效 context。

**修复方案（当前可做）**：

1. **Gemini ctx 模板增加产出约束**：在 ctx 文件中显式声明产出格式和大小上限

```markdown
## 输出约束
- 产出总长度 ≤ 8KB（约 4000 中文字）
- 超出部分写入单独的附件文件，主报告只引用
- 格式：判定（≤500字）+ 论证（≤2000字）+ 修复方案（≤1500字）
```

2. **Gemini 产出拆分**：大报告拆为 `summary.md`（≤8KB，Lead 读取）+ `detail.md`（完整内容，按需读取）

3. **Lead 读取策略**：Lead 只读取 summary 部分。如果需要详情，使用 `head` 或行范围读取

**效果**：Gemini 讨论 per-round 成本从 40-80KB 降至 ≈ 15-25KB（ctx 12KB + summary 8KB + Lead 处理 5KB）。

---

## 问题 C：compact 是否应该更早触发？

### 判定：当前不可行（平台限制），但可以用 hook 近似

**平台现状**：
- Claude Code **不支持**自动 compact。compact 由用户手动触发（/compact）或在 context limit 时强制触发
- Claude Code **不提供** context 使用率 API——Lead 无法查询"当前 context 使用了多少"
- precompact-save.sh 是 `Compact` 事件的 hook，在 compact **已经触发后**执行，不是预防性的

**可行近似方案（当前可做）**：

Lead 无法获取精确 context 使用率，但可以通过**启发式计数**估算：

创建 `scripts/context_budget.py`：
```python
"""估算当前 session 的 context 消耗。

原理：统计 Lead 在本 session 中读取的文件总大小 + Gemini 交互次数。
不是精确值，但足够在"即将撞墙"时发出预警。
"""

# 输入：session 起始时间（从 session 文件名提取）
# 遍历 session 期间的操作痕迹：
#   - tmp/ 中新增/修改的文件（ctx + 报告）
#   - git log 中的 commit 数量（每个 commit ≈ Lead 处理了一轮）
# 输出：estimated_usage_pct（粗略百分比）+ 建议（继续/compact）
```

但更务实的方案是：**不试图预测 compact 时机，而是减少每个操作的 context 成本**（问题 A 和 B 的修复已覆盖）。

compact 后的恢复成本已经被 precompact-save.sh + session-start-ceremony.sh 覆盖。问题不是 compact 本身，而是在 compact 之间能完成的工作量太少。

**结论**：不建议投入自动 compact 检测。将精力集中在降低单位操作的 context 成本上。

---

## 问题 D：ceremony 循环的 context 预算

### 判定：单个 ceremony 循环的 context 消耗可估算，且确实接近 session 上限的 50%+

**估算模型**：

| 阶段 | 估算 context 成本 |
|------|-----------------|
| ceremony 启动（CLAUDE.md + skills + hooks） | 30-50KB（固定） |
| ceremony_scan.py 输出 | 2-5KB |
| Lead 评估 + spawn 工位 | 5-10KB（Lead 思考 + TaskCreate） |
| 工位执行（Gemini 或其他） | 15-80KB（取决于任务复杂度） |
| 工位汇报 + Lead 读取 | 10-40KB |
| 谱系结算（dag.yaml 读取 + 写入） | 55-60KB（如果读取 dag）或 2-5KB（如果用 dag_query） |
| commit + push | 5-10KB |

**当前模式（无 dag_query，无 Gemini 约束）**：
- 最低：30 + 2 + 5 + 15 + 10 + 55 + 5 = **122KB**
- 典型：40 + 3 + 8 + 40 + 25 + 55 + 8 = **179KB**
- 极端（含 Gemini 大报告）：50 + 5 + 10 + 80 + 40 + 55 + 10 = **250KB**

Claude Code 的有效 context ≈ 200KB token（200K token × ~1 byte/token 的中文混合比）。一个典型循环消耗 179KB ≈ session 的 89%。这解释了"每个 session 最多完成一个循环"。

**修复后模式（dag_query + Gemini 约束）**：
- 最低：30 + 2 + 5 + 15 + 10 + 3 + 5 = **70KB**（35%）
- 典型：40 + 3 + 8 + 25 + 15 + 3 + 8 = **102KB**（51%）
- 极端：50 + 5 + 10 + 30 + 20 + 5 + 10 = **130KB**（65%）

修复后每个 session 可完成 1.5-2 个循环（典型情况）。

**是否设定硬预算？** 不建议。原因：
1. Lead 无法精确测量 context 使用率
2. 硬预算需要打断正在进行的操作，可能导致半成品
3. 更好的策略是降低每个操作的基线成本（已通过 A、B 的修复覆盖）

---

## 修复方案汇总

### 当前可做（按优先级排序）

| 优先级 | 修复 | 预期效果 | 工作量 |
|--------|------|---------|--------|
| **P0** | 创建 `dag_query.py` | 消除 55KB/次 的 DAG 读取 | 新建 1 个文件（~100行） |
| **P0** | Gemini ctx 模板增加产出约束（≤8KB） | 控制 Gemini 产出大小 | 修改 ctx 模板约定 |
| **P1** | Gemini 产出拆分（summary + detail） | Lead 只读 summary | 修改 Gemini 工位的输出协议 |
| **P1** | pattern-buffer.yaml 结构优化 | 减少 169KB 的 buffer 读取 | 待评估 buffer 使用模式 |
| **P2** | validate_dag.py 改为只输出错误摘要 | 避免 Lead 读完整验证报告 | 修改输出格式 |

### 需要平台支持（当前无法做）

| 项目 | 说明 | 平台需要提供的能力 |
|------|------|------------------|
| 自动 compact | context > 80% 时自动触发 | context usage API 或 auto-compact 配置 |
| context 使用率查询 | Lead 主动检查剩余 context | `claude.context.usage()` 类似的 API |
| 增量 context 注入 | CLAUDE.md + skills 按需加载而非全量注入 | 条件化的 system prompt 机制 |
| teammate context 隔离 | teammate 的 context 不消耗 Lead 的配额 | 已实现（teammate 有独立 context） |

---

## 定义依据

- 141号谱系：确定根因不是 hook system-reminder 注入，而是 Lead 读取大文件
- 089号谱系：dispatch-dag.yaml 的存在论位置（蜂群基因组层）
- 075号谱系：structural_nodes → required_skills 事件驱动架构
- dag_add_node.py 的设计意图：避免 Lead 读取完整 dag.yaml（但只覆盖了写入场景）

## 边界条件

- 如果 Claude Code 未来提供 context usage API，则问题 C 的判定需要更新
- 如果 Gemini 产出无法被压缩到 8KB（某些复杂审计确实需要大报告），则需要引入"分页读取"协议
- 如果 pattern-buffer.yaml 的使用模式要求 Lead 完整读取，则需要对 buffer 本身做拆分
- 如果 CLAUDE.md + skills 的固定注入成本继续增长（目前 30-50KB），需要考虑条件化加载

## 下游推论

1. dag_query.py 的引入意味着 Lead 对 DAG 的所有操作都应通过 CLI 工具，不再直接 Read dag.yaml
2. Gemini 产出约束意味着需要修改所有 Gemini ctx 模板的标准格式
3. context 预算估算（问题 D）表明：即使完全修复，每个 session 最多 2 个循环——这是 Claude Code 200K context 的硬约束，不是架构缺陷

## 谱系引用

- 141号：context limit 首次根因分析
- 089号：dispatch-dag.yaml 扬弃审计
- 075号：structural → skill 架构变迁
- 081号：ceremony_scan roadmap 优先级
- 057号：LLM 不是状态机（DAG 由 LLM 解释执行）

## 影响声明

- 本诊断不改动任何文件，只产出修复方案
- P0 修复（dag_query.py + Gemini 约束）如果实施，影响范围：
  - Lead 的 DAG 操作流程（从 Read → Bash）
  - Gemini 工位的产出协议（增加大小约束）
  - ceremony_scan.py 无需修改（它已经读的是 dispatch-dag.yaml，不是 genealogy/dag.yaml）
