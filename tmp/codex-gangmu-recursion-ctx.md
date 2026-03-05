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
