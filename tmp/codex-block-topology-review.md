# 代码层严格质询报告：谱系区块拓扑化方案

> 质询者：代码层（代替 Codex 角色）
> 日期：2026-02-24
> 对象：`tmp/block-topology-proposal.md`

---

## 维度1：与现有代码的集成成本

### 1.1 ceremony_scan.py 的扫描目标变化

**现状**：`ceremony_scan.py`（698行）扫描以下数据源：
- `.chanlun/genealogy/settled/*.md`——frontmatter 解析（topo_effect、tensions_with、id 一致性）
- `.chanlun/genealogy/dag.yaml`——frozen 节点、DAG 完整性、承重点
- `.chanlun/genealogy/pending/*.md`——pending 谱系
- `.chanlun/sessions/*-session.md`——遗留工位
- `.chanlun/roadmap.yaml`——业务目标
- `.chanlun/pattern-buffer/`——结晶候选

**区块拓扑引入后**：扫描目标需增加 `.chanlun/block-topology/blocks/` 和 `relations.jsonl`。关键问题：

1. **谱系异常检测**（`detect_genealogy_anomalies`）目前验证 settled 文件与 dag.yaml 节点的一致性。区块拓扑引入后，需要同时验证区块 ↔ settled 文件 ↔ dag.yaml 三者的一致性——复杂度从 O(n) 升到 O(n²)。
2. **topo_effect 扫描**（`detect_pending_topo_effects`、`get_topo_effects_from_genealogy`）目前从 settled 文件的 frontmatter 提取。区块拓扑下，topo_effect 也会以 event 区块形式存在——扫描需覆盖两套数据源，且需防止重复计数。
3. **delta_genealogy 检测**（`compute_delta_genealogy`）按 settled 文件数统计。区块拓扑引入后，一个谱系事件对应多个区块（event + consensus + residue + tension），文件数与谱系数不再 1:1。

**评价**：**问题——迁移路径不可渐进**。ceremony_scan 的几乎每个函数都硬编码了 `settled/*.md` + `dag.yaml` 的数据模型。渐进迁移意味着长期维护两套扫描逻辑，违反 no-patch-mentality 规则（"保留旧逻辑加新分支"是禁止模式）。一次性迁移则需要同时重写 ceremony_scan + topology_operator + dag_add_node.py。

**工程建议**：如果决定引入区块拓扑，应先定义数据访问抽象层（DAG reader interface），ceremony_scan 通过接口读取，底层实现可以从 dag.yaml 切到 block-topology。但这本身就是一次中等规模的重构。

### 1.2 topology_operator.py 的写入目标变化

**现状**：`topology_operator.py`（644行）的 freeze/split/sever 操作直接修改 `dag.yaml`：
- `apply_freeze`：修改节点 `frozen` 字段 + 边的 `frozen_by` 字段
- `apply_split`：向 `nodes` 列表追加分裂节点
- `apply_sever`：修改边的 `severed_by` 字段
- `save_dag`：整体覆写 dag.yaml

**区块拓扑下**：每个拓扑操作需要：
1. 写入一个 event 区块（type=rewrite）到 `blocks/`
2. 追加关系到 `relations.jsonl`
3. 同时写入一个 reflexive 区块（因为关系层被追加了）
4. 更新 `index.json`

**评价**：**问题——写入路径从1个文件变为4个文件**。当前 `save_dag(dag, dag_path)` 是单文件原子写入（覆写）。区块拓扑下每个操作涉及4个写入点，原子性问题显著（见维度5）。

---

## 维度2：SHA256 内容寻址的工程问题

### 2.1 timestamp 参与 hash 的问题

**方案定义的区块结构**包含 `timestamp` 字段。如果 timestamp 参与 SHA256 计算：
- 同一逻辑事件在不同时刻写入 → 不同 hash → 不同区块 ID → 无法去重
- 重试/幂等性丧失：如果写入中途崩溃重试，产生新 hash

如果 timestamp 不参与 hash：
- hash 仅覆盖 `type` + `source` + `content` + `refs` → 可去重
- 但 `id` 字段声明为 "sha256_of_this_content"，而 `this_content` 包含 timestamp → 自指矛盾

**评价**：**问题——方案定义存在自指矛盾**。`id = sha256(content)` 而 `content` 包含 `id` 字段本身。这是鸡生蛋问题。需要明确定义 hash 的输入范围（canonical form），排除 `id` 和 `timestamp`。

**工程建议**：定义 canonical content = `{type, source, content, refs}` 的确定性序列化（sorted keys + 无空白）。`id = sha256(canonical_content)`。`timestamp` 存储但不参与 hash。

### 2.2 查询性能

当前 177 个 settled 谱系文件。假设每个谱系事件平均产生 3 个区块（event + 若干 consensus/residue/tension），加上 rewrite 区块，半年后（假设增长到 500 条谱系）：
- blocks 目录：~2000-3000 个 JSON 文件
- 每个文件名是 64 字符 hex string

**评价**：**可行但有条件**。2000-3000 个文件在现代文件系统上查询不是瓶颈（ext4/NTFS 均可处理）。但：
- 按语义查询（"找到所有 type=consensus 的区块"）需要全扫描 blocks 目录或依赖 index.json
- index.json 如果是全量索引，每次写入需要更新全量索引 → 并发冲突

**工程建议**：按类型分子目录（`blocks/event/`、`blocks/consensus/` 等）或在 index.json 中维护类型索引。

---

## 维度3：relations.jsonl 的查询效率

**现状**：dag.yaml 3284 行，包含 177 个节点 + 大量 depends_on/negates/related/tensions_with 边。整个文件 YAML 解析一次即可。

**区块拓扑下**：relations.jsonl 是追加式，每行一条关系。查询模式：
1. "找到区块 X 的所有入边/出边" → 全文扫描（每行解析 JSON 后比较 from/to）
2. "找到所有 type=negates 的关系" → 全文扫描
3. "找到某区域被反复改写的模式（奇点检测）" → 全文扫描 + 聚合

**评价**：**问题——当前规模可行，但架构上是退步**。dag.yaml 加载一次后在内存中是完整的图结构（Python dict），O(1) 节点查找，O(边数) 遍历。JSONL 每次查询都是 O(行数) 全扫描。

**具体数字**：假设关系数按谱系数的 5-10 倍增长（每条谱系平均 5-10 条关系——depends_on + negates + related + rewrite），500 条谱系时 relations.jsonl 约 2500-5000 行。Python 逐行解析 5000 行 JSON 大约 10-50ms——不是性能瓶颈但也没有优势。

当关系数达到数万时（长期积累的 rewrite 关系），每次查询成本线性增长。

**工程建议**：
- 短期：可以接受 JSONL 全扫描（<10000 行时性能可控）
- 中期：维护内存索引（启动时加载 JSONL 到 dict），类似当前 dag.yaml 的使用方式
- 长期：如果需要复杂拓扑查询（路径、环、连通分量），应考虑 SQLite 或 networkx 持久化

---

## 维度4：共识仪式的实现

### 4.1 plan-review 当前输出格式

`plan-review` SKILL.md 定义的输出：
- 每轮结果写入 `.chanlun/review-results/plan-review-{timestamp}-round{N}.md`
- 最终方案标记 `[plan-reviewed]`
- 终止条件：Codex 显式确认满意

**实际文件格式**（从 `codex-diagnose-20260223-2122.md` 观察）：
- Markdown 格式，含 `## 元数据` 节（mode、subject、model、timestamp）和 `## Response` 节

**评价**：**问题——无法自动提取 consensus/residue/tension 三元组**。当前 plan-review 输出是自由格式 Markdown。方案要求"每次共识仪式强制生成三个原子区块"，但：
1. plan-review 没有结构化的"让步清单"输出——Codex 的质疑和 Opus 的回应是自然语言对话
2. "为达成共识而放弃的部分"（residue）在对话中是隐含的——需要 LLM 事后提取，不是确定性可提取的
3. "未决张力"（tension）同样是隐含的

**工程建议**：
- 方案A：修改 plan-review 协议，强制每轮输出结构化 JSON（consensus + residue + tension 三字段）。成本：修改 SKILL.md + codex-challenger agent + 所有调用方
- 方案B：在共识仪式结束后，用 LLM 从对话记录中提取三元组。成本低但不确定性高（提取质量依赖 LLM）
- 方案C：接受 residue 和 tension 的提取不完整，将其作为人工标注的输入（编排者 review 时补充）。务实但违反自动化目标

### 4.2 Gemini/Codex 质询记录格式

`.chanlun/review-results/` 下目前只有 1 个文件。格式是 Markdown，无结构化 schema。

**评价**：**问题——缺少统一 schema**。区块拓扑要求每个事件有确定性的 JSON 结构。当前质询记录是自由 Markdown，迁移需要定义 schema 并追溯转换已有记录。

---

## 维度5：原子性保证

### 5.1 三个区块原子写入

方案要求"每次共识仪式强制生成三个原子区块"（consensus + residue + tension）。在文件系统上：

1. 写入 block-A.json
2. 写入 block-B.json
3. 写入 block-C.json
4. 追加 3 行到 relations.jsonl
5. 写入 3 个 rewrite 区块（reflexive layer）
6. 更新 index.json

如果在步骤 3 后崩溃：
- blocks/ 中有 3 个区块但 relations.jsonl 中没有对应关系 → 孤立区块
- 后续扫描需要检测和修复不一致状态

**评价**：**问题——文件系统无事务**。当前 dag.yaml 是单文件覆写，原子性由 OS 的 write+rename 保证（rename 是 POSIX 原子操作）。区块拓扑下 6+ 个写入点，无法用 rename 技巧。

**工程建议**：
- 方案A：WAL（Write-Ahead Log）——先写意图日志，再执行写入，崩溃后从 WAL 恢复。实现复杂度高。
- 方案B：所有写入先到 staging 目录，完成后原子 rename 整个目录。但 NTFS 不支持目录原子 rename。
- 方案C：接受最终一致性——写入后运行一致性检查脚本修复孤立区块。ceremony_scan 已有异常检测逻辑（`detect_genealogy_anomalies`），可扩展。
- **推荐方案C**：与现有容错模式一致（ceremony_scan 已经在做异常检测+修复工位生成），新增区块一致性检查即可。

### 5.2 多 agent 并发写入

当前蜂群架构中多个 teammate 可能同时产生谱系事件。dag.yaml 的并发写入目前由 LLM 的串行化保证（同一时刻只有一个 agent 写 dag.yaml——没有显式锁但实际串行）。

区块拓扑下：
- blocks/ 写入天然无冲突（内容寻址 → 不同区块不同文件名）
- relations.jsonl 追加是文件锁问题——两个 agent 同时 append 可能导致行混合
- index.json 覆写有并发冲突

**评价**：**问题——relations.jsonl 追加不是并发安全的**。Python 的 `open('a')` 在 POSIX 上对小写入（< PIPE_BUF = 4096 bytes）是原子的，但在 Windows NTFS 上无此保证。

**工程建议**：每个 agent 写独立的 JSONL 分片（`relations-{agent_id}.jsonl`），查询时合并。或者用 `filelock` 库加文件锁。

---

## 维度6：与 git 的冗余

### 6.1 git 已是内容寻址

git objects 本身就是 SHA1（或 SHA256）内容寻址的。每次 commit 记录了文件变更的完整快照。

**方案与 git 的重叠**：
| 功能 | git | 区块拓扑 |
|------|-----|---------|
| 内容寻址 | SHA1/SHA256 object store | SHA256 block store |
| 不可变事件记录 | commit history | event blocks |
| 关系图 | commit graph (parent/child) | relations.jsonl |
| 追溯改写 | rebase/amend (改变 history) | rewrite blocks (记录改变) |

**关键差异**：git 的追溯改写（rebase）**替换**历史；区块拓扑的追溯改写**追加**新区块记录改变。这是本质区别——区块拓扑保留了改写行为本身的记录，git 不保留。

**评价**：**不冗余——解决的问题不同**。git 记录"文件发生了什么变化"，区块拓扑记录"概念拓扑发生了什么变化"。git 的 commit graph 是线性/分支结构，区块拓扑的关系图是任意有向图（含 negates、supersedes、reopens 等语义边）。

但是：git objects 可以被利用来实现部分功能（例如用 git notes 存储关系元数据），减少重复基础设施。

**工程建议**：不建议用 git objects 替代区块拓扑（语义层次不同）。但可以考虑：
- 区块写入后自动 git add + commit（区块拓扑的变更由 git 保护）
- 不需要在区块拓扑层面重新实现不可变性——git 的 commit history 已经保证了

---

## 综合评估

| 维度 | 评价 | 风险等级 |
|------|------|---------|
| 1. 集成成本 | 问题：ceremony_scan 全面重构 | HIGH |
| 2. SHA256 寻址 | 问题：自指矛盾需解决，性能可控 | MEDIUM |
| 3. JSONL 查询 | 问题：架构退步，短期可接受 | LOW-MEDIUM |
| 4. 共识仪式 | 问题：无法自动提取三元组 | HIGH |
| 5. 原子性 | 问题：多文件写入无事务 | MEDIUM |
| 6. git 冗余 | 不冗余，但可利用 git 减少基础设施 | LOW |

### 核心工程建议

1. **先解决维度4再动手**：如果无法从 plan-review 输出中确定性地提取 consensus/residue/tension，区块拓扑的核心价值（多主体共识的拓扑化记录）就无法实现。建议先修改 plan-review 协议输出结构化 JSON，验证提取可行性。

2. **定义数据访问抽象层**：ceremony_scan 和 topology_operator 应通过接口访问谱系数据，而非硬编码 dag.yaml。这是无论是否引入区块拓扑都值得做的重构——当前代码与数据格式强耦合。

3. **分阶段引入**：
   - Phase 0：定义 canonical block schema + hash 规则（解决维度2）
   - Phase 1：新产生的共识事件写入区块拓扑（增量），现有谱系保持 dag.yaml
   - Phase 2：ceremony_scan 增加区块拓扑扫描能力（双源）
   - Phase 3：迁移工具——将现有 177 条 settled 谱系转换为区块
   - Phase 4：切换主数据源到区块拓扑，dag.yaml 作为派生视图

4. **并发写入用分片 JSONL**：每个 agent 写独立分片，查询时合并。避免文件锁。

5. **原子性用最终一致性**：接受中间不一致，ceremony_scan 增加区块一致性检查。
