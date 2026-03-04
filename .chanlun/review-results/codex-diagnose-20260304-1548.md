# Codex diagnose — 2026-03-04 15:48:21 UTC

## 元数据

- **mode**: diagnose
- **subject**: 纲目递归与 RTAS 无限递归蜂群的映射：ceremony 递归深度被限制在1层，是持存断裂+多线并行断裂的统一根因——评估三方案的可行性与 RTAS 语义一致性
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-04 15:48:21 UTC
- **context-file**: tmp/codex-gangmu-recursion-ctx.md

## Prompt

## 诊断目标

纲目递归与 RTAS 无限递归蜂群的映射：ceremony 递归深度被限制在1层，是持存断裂+多线并行断裂的统一根因——评估三方案的可行性与 RTAS 语义一致性

## 上下文

# Codex Diagnose Round 3 上下文：纲目递归 = RTAS 递归蜂群能力

## 编排者新洞察（追加到 Round 2 基础上）

> "这个多线本身也要是递归的，纲举目张就是一种递归，这也正好适配着我们的 RTAS 无限递归蜂群的能力。"

**含义分解**：
1. 纲目结构本身是递归的：纲→目→next_action→（完成后产生新目/action）
2. RTAS 的无限递归蜂群能力应该驱动这个递归：每条目的 action 可以 spawn 子蜂群，子蜂群 rescan 发现新 action，如此递归直到不动点
3. 当前问题：ceremony 只处理 scan 输出的扁平工位列表，没有利用纲目的递归结构
4. 持存断裂是递归深度=1的症状：Lead 只展开一层（scan→spawn→完成→停），自然只需要一次 session 写入

**核心命题**：ceremony 的递归深度被人为限制在了一层（深度=1），这把"持存断裂"和"多线并行"和"递归蜂群"统一到一个根因上。

---

## Round 2 诊断结论（已确认，供参考）

### 根因链
```
失败现象（持续推进断裂 + 持存不连续）
→ 直接原因 A：_scan_research_lines 把"通过 completion_check 的 action"静默跳过
→ 直接原因 B：clean_terminate 只看"工位为空"，不检查"是否还有 active 但无工位的目"
→ 根本原因：没有"completed 后推进下一步"的状态机机制
→ 结果：active 目可进入"静默 active"——既不产工位、也不关闭——全局终止把它吞掉
```

Round 2 修复方案（三步）：
1. active 目加"不静默"约束（强制产治理工位）
2. clean_terminate 收紧（加 stagnant_active_mu_count == 0 条件）
3. 事件级持久化

---

## 现有架构：RTAS 递归蜂群

### 蜂群架构原则（来自 swarm-architecture SKILL.md）

关键条文（原则15）：
- "teammate 通过 TeamCreate 创建子 team → 子 team 内部自治 → 结果向上回传"
- "子蜂群同样是递归拓扑异步自指蜂群——子 team 不是简化版 agent pool，它复制父蜂群的完整结构"
- "真递归是默认架构模式（不需要理由），扁平 Lead→Worker 是退化特例"
- 递归深度工程上限 ≈ 3-4 层

创世 Gap 递归化（原则15）：
- "L(n-1) Teammate 执行 sub-swarm-ceremony skill 时充当 L(n) Swarm₀"
- 每层递归都有自己的创世 Gap

### ceremony 的当前架构（深度=1）

```
ceremony_scan.py → 扁平工位列表 → Lead spawn 扁平 teammate → teammate 完成 → rescan
```

扁平工位列表的来源：
- roadmap.yaml
- session 遗留
- gangmu.yaml._scan_research_lines（只展开一层：active 目的 next_actions）
- 其他扫描源

**关键约束**：当前 _scan_research_lines 展开深度固定为 1：
- 纲（gang）→ 目（mu）→ next_actions（展开到这层就停）
- next_actions 完成后，不会自动展开"目完成后的纲级推进"

---

## 递归展开的三种实现方案（需要 Codex 评估）

### 方案 A：ceremony_scan.py 多层递归展开（静态展开）

在 `_scan_research_lines` 中，纲目递归结构静态展开为多层工位：
- 层1：gang（纲）→ 对每个 active 目检查是否需要推进
- 层2：mu（目）→ 对每个 unblocked action 生成工位
- 层3：action 完成后 → 检查目是否可以 close / 纲是否需要下一步

但这仍然是扁平展开——ceremony_scan.py 把所有层级的工位都扁平化到同一个工位列表，Lead 仍然是一层蜂群。

**局限**：纲级别的推进（"整个目完成后，纲需要下一步"）没有独立的治理层。

### 方案 B：纲目驱动的子蜂群（递归展开，与 RTAS 同构）

纲对应一层蜂群，目对应子蜂群，next_action 对应子蜂群的工位：

```
Lead（根 ceremony）
├── 纲一蜂群（实盘闭环）
│   ├── 目[operational-methodology] 工位
│   │   ├── G1-position-management（action → 工位）
│   │   └── I1-xiaozhuan-da（action → 工位）
│   └── 目[k4-engineering] 工位
│       ├── k4-monitor-vps-deploy（action → 工位）
│       └── k4-config-encoder（action → 工位）
└── 纲五蜂群（知识谱系）
    └── 目[traverse-infra] 工位
        └── encounter-record-mechanism（action → 工位）
```

每个纲级蜂群有自己的 rescan 循环，发现目级别的推进需求，spawn 目级别的工位，目级别工位完成后触发纲级 rescan，如此递归。

**优势**：与 RTAS 无限递归蜂群完全同构——纲是 Lead，目是 Worker，action 是子任务。
**问题**：
- 创建"纲级蜂群"的 API 机制是否存在？（TeamCreate + sub-swarm-ceremony）
- 纲级蜂群的 ceremony_scan 是否等同于根 ceremony_scan？还是需要纲专用的 scan 逻辑？
- 纲级蜂群完成后如何向根 ceremony 汇报？

### 方案 C：gangmu.yaml 驱动 + rescan 深度参数（渐进式）

不创建子蜂群，但 ceremony_scan.py 支持"递归深度"参数：
- `--depth 1`（当前行为）：只展开 action 层
- `--depth 2`：展开 action + 目关闭判断
- `--depth 3`：展开 action + 目关闭 + 纲推进建议

每次 rescan 自动提升深度，直到不动点。

**局限**：仍然是扁平展开，不是真递归蜂群。但工程实现简单，是从当前状态到方案 B 的过渡路径。

---

## 需要 Codex 评估的核心问题

### 问题一：编排者的"纲目递归 = RTAS 递归"命题是否技术上可行？

具体要评估：
- gangmu.yaml 的纲目结构（2层：纲→目→action）是否自然映射到 RTAS 递归蜂群（Lead→Worker→subWorker）？
- 还是两者之间存在阻抗——纲目是"研究线管理结构"，RTAS 是"工程执行结构"？

### 问题二：三方案中，哪个最符合 RTAS 语义 + 当前工程能力？

评估维度：
1. 与 RTAS 无限递归蜂群的语义一致性
2. gangmu.yaml 现有结构是否需要修改
3. ceremony_scan.py 修改量
4. 持存在该方案下是否自然涌现

### 问题三：持存断裂在递归展开后是否自动解决？

Round 2 结论：持存需要"事件级 write_session"。
追问：如果纲目展开为递归子蜂群，session 的写入语义是什么？
- 根 ceremony 的 session vs 纲级蜂群的 session vs 目级工位的 checkpoint
- 三层 session 的合并策略

---

## 边界条件（已知约束）

1. **gangmu.yaml 是编排者定义的**，ceremony 不越权自动修改纲目结构
2. **ceremony_scan.py 工程约束**：scan 不能耗时过长（pytest fallback 默认关闭就是这个原因）
3. **递归深度工程上限**：RTAS 限定约 3-4 层（API 成本、并发约束）

---

## 被审查文件

1. `scripts/ceremony_scan.py`：`_scan_research_lines`（第633-777行）+ `clean_terminate`（第1051行）
2. `.chanlun/gangmu.yaml`：纲目结构（纲→目→next_actions 两层嵌套）
3. 蜂群架构原则（swarm-architecture SKILL.md）：真递归默认架构 + 子蜂群创建机制


## 诊断要求

请执行严格根因诊断：
1. 失败现象：观测到什么？
2. 直接原因：哪行代码导致了失败？
3. 根本原因：为什么会写出那行代码？是设计错误还是实现疏忽？
4. 分类判定：实现错误 or 定义冲突？
5. 修复方案（仅限实现错误）：精确的修复步骤

如果是定义冲突，输出：
- **冲突方**：A 说什么 vs B 说什么
- **不可弥合理由**：为什么不能同时满足两边


## Response

结论先行：**“ceremony 递归深度被限制为 1 层”是成立的统一根因判断**。  
它同时解释了你说的三件事：持存断裂、多线并行断裂、RTAS 递归蜂群能力未被利用。

---

## 1) 严格诊断链（按你要求）

### 失败现象（测试否定对象）
失败测试实质否定了这个命题：  
> “只要 gang/mu 仍 active，ceremony 会持续推进直到不动点。”

实际观测是：
- action 完成后出现“静默 active mu”（仍 active 但不再产工位）
- 全局被 clean terminate 吞掉
- session 只在单轮边界写入，恢复后续推进断裂

### 直接原因（代码级）
1. `scripts/ceremony_scan.py::_scan_research_lines`（633-777 段）  
   - 对通过 `completion_check` 的 action 进行跳过（等价于 `continue`）  
   - **未生成“mu/gang 推进治理工位”**
   - 扫描展开固定在 `gang -> mu -> next_actions` 一层，不做后继状态推进展开

2. `clean_terminate`（约 1051）  
   - 终止条件只看“当前工位为空”
   - 不看“是否还有 active mu / pending transition / 子蜂群未 join”

### 根本原因（设计层）
当前 ceremony 的对象模型是“**扁平工位分发器**”，不是“**递归状态机**”：
- 把“是否还有工位”误当成“是否可终止”的充分条件
- 没有把 `action完成 -> mu推进 -> gang推进` 建模为显式状态迁移事件链
- 与 RTAS 原则15“真递归默认架构”发生实现漂移

### 分类判定
- 上述问题是：**实现错误（implementation error）**  
  因为不需要改 RTAS 定义即可修复（要改的是 ceremony 实现形态）。

---

## 2) “纲目递归 = RTAS 递归蜂群”可行性判断

**可行，且结构同构**：
- `gang` ↔ 上层 swarm scope
- `mu` ↔ 子目标/子循环（可做子蜂群或子状态机）
- `action` ↔ 可执行工位

不存在本体冲突（研究线管理结构 vs 工程执行结构）——前者可作为后者的调度语义层。

---

## 3) 三方案评估（RTAS一致性 + 工程可落地）

### 方案 A（scan 内静态多层展开）
- RTAS 语义一致性：低-中（仍是单层 Lead 扁平化）
- 工程改动：中（scan 逻辑复杂化，终止判定也要改）
- 持存：不会自然解决，仍需事件级写入
- 判定：**能止血，但不是目标态**

### 方案 B（gang/mu 子蜂群递归）
- RTAS 语义一致性：高（与原则15同构）
- 工程改动：中-高（TeamCreate、子 ceremony scope、汇报链）
- 持存：最自然（父子事件流 + join 语义）
- 判定：**目标最正确方案**

### 方案 C（depth 参数渐进展开）
- RTAS 语义一致性：中（仍扁平，不是真递归）
- 工程改动：低-中（最易上线）
- 持存：不会自动解决，仍需事件级写
- 判定：**过渡方案可行，不宜作为终态**

---

## 4) 持存断裂在递归后会自动解决吗？

**不会自动解决。**  
递归只增加层级，不会自动带来持久一致性。必须引入事件级写入语义。

建议最小语义（无论 B/C）：
- 每次状态迁移都写事件：`spawn / action_done / mu_transition / gang_transition / join / terminate`
- 事件带层级关联键：`team_id, parent_team_id, gang_id, mu_id, action_id`
- 恢复时按事件重放，而不是仅靠“最后一次 session 快照”

---

## 5) 定义冲突检查（你要求的冲突格式）

存在一个**潜在定义冲突**（需编排侧裁决）：

- **冲突方 A**：`gangmu.yaml` 由编排者定义，ceremony 不越权修改  
- **冲突方 B**：“action 完成后自动产生新 mu/action 并继续递归”

**不可弥合理由**：  
若“产生新 mu/action”被定义为“自动写回 gangmu.yaml”，则与 A 直接互斥，不能同时满足。

---

## 6) 修复方案（针对实现错误，给精确步骤）

1. 在 `_scan_research_lines` 中取消“completed action 静默跳过”  
   - 改为生成治理工位（mu_transition / gang_transition），保证 active 不静默。

2. `clean_terminate` 收紧为联合条件  
   - `no_workstations && no_active_mu && no_pending_transition && no_running_child_team` 才可终止。

3. 引入事件级持久化（最关键）  
   - 每个状态迁移后立即 append event；恢复走重放。

4. 架构路径建议  
   - 短期：先上 C（depth=2/3）+ 步骤1/2/3 止血  
   - 中期：迁移到 B（子蜂群递归）作为 RTAS 一致终态

---

**最终判定**：  
你的核心命题“深度=1 是统一根因”是**技术上成立**；  
三方案中 **B 最符合 RTAS 语义**，**C 是现实过渡**，A 仅局部补丁。
