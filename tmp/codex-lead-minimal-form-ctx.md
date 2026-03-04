# Codex Diagnose 上下文：Lead 最小形式

## 诊断目标

从代码层面诊断 Lead（ceremony.md）的最小形式。
编排者洞见："像 Claude Code agent 用 30 行 bash 把自己调用出来的方式，让 team lead 只变成一种形式。这样做才叫递归拓扑。"

## 约束背景

- 069号：递归拓扑异步自指——Lead 必须在拓扑内，与工位同构
- 090号：严格性语法规则——Lead 的胖节点形式不严格
- 057号：LLM 不是状态机——确定性逻辑由 Python 脚本执行
- 218号：Lead 并行化原则——Lead 是 RTAS 的一环，不是串行瓶颈

## 当前 ceremony.md 完整内容

```markdown
# /ceremony — Swarm₀：递归蜂群的第0层（谱系驱动）

蜂群启动。**谱系决定做什么，RTAS决定怎么做**（174号）。
Python 脚本从谱系状态做确定性推导，LLM 负责 spawn 谱系推导出的业务工位。
结构能力由 skill 提供（事件驱动），不再作为 teammate spawn。

## 设计原则
- ceremony 是蜂群的第0层递归（Swarm₀），不是前置阶段（058号）
- **谱系是生成引擎**——ceremony 的一切行动从谱系推导，产出凝固回谱系（174号）
- LLM 不是状态机（057号）——确定性逻辑由 Python 脚本执行
- 结构能力 = skill（事件驱动），不是 teammate（075号）
- skill 由 dispatch-dag 的 event_skill_map 定义，事件触发时自动执行
- **RTAS 是谱系的实现**——递归循环服务于谱系的生成性，不是反过来（174号）

## 步骤 1：运行扫描脚本
```bash
python scripts/ceremony_scan.py
```
脚本输出 JSON，包含：mode, definitions, pending/settled, session, workstations, required_skills

## 步骤 2：输出摘要（谱系状态概览）
根据 JSON 输出一行摘要——这是谱系当前状态的快照，决定蜂群此次运动的方向：
```
[ceremony] {mode} | 定义 {definitions} 条 | 谱系 {settled} settled / {pending} pending | HEAD {head}
[ceremony] 工位 {len(workstations)} 个 | skill {len(required_skills)} 个（事件驱动）
```

## 步骤 3：TeamCreate
```
TeamCreate(team_name="v{N}-swarm", description="...")
```
命名一致性约束：TeamCreate 的 team_name 参数就是蜂群的唯一标识。

## 步骤 4：并行 spawn 谱系推导出的业务工位
谱系决定做什么——遍历 JSON 中的 workstations，为每个工位发出一个 Task 调用（全部并行）：
```
Task(name="{workstation.name}", subagent_type="general-purpose", team_name="{蜂群名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="{workstation.name}: {具体任务描述}")
```

如果 workstations 为空：输出 [020号反转] 无新区分可产出——系统干净终止，不执行步骤 3-4。但仍必须更新 session + commit。

如果 workstations 非空但全部状态含"待 Gemini decide"或"长期"：
1. "待 Gemini decide" 的工位 → 直接路由 Gemini
2. "长期"工位 → 不 spawn，保留在 session 遗留项
3. 如果路由 Gemini 后仍有可执行工位 → spawn 蜂群执行
4. 如果只剩"长期"工位 → 输出 [阻塞] 仅剩长期工程项
5. 更新 session + commit

持久化不变量：ceremony 的每条退出路径都必须以 session 更新 + commit + push 结束。

## 步骤 5：输出行动声明
```
→ 接下来：监控 N 个工位运行
```

## 步骤 6：汇报循环（RTAS 递归——谱系的实现）

spawn 完成后，立即调用 TaskList 查看任务状态。然后进入 RTAS 循环。

### 循环内操作（并行优先）
1. 调用 TaskList 查看所有任务状态
2. 批量并行处理完成/空闲的工位：
   - 所有 shutdown_request 在同一个消息中并行发出
   - 所有完成工位的结果汇报在同一轮中完成
   - 禁止逐个串行处理工位
3. 增量持久化：工位完成时更新 session 文件
4. Lead 不空转：如果仍有 in_progress 工位，Lead 在等待期间并行执行：
   - 拓扑分析家 spawn（179号，上下文隔离冷读）
   - meta-observer 二阶观察
   - session 增量写入
5. 测试验证 spawn 为工位：Lead 不自己跑 pytest
6. 所有工位完成后：写入完整 session → commit → push
7. 纲举目张 ∥ 重扫描（并行）：
   ```bash
   python scripts/gangju_analysis.py
   python scripts/ceremony_scan.py
   ```
   - gangju audit_needed: true → spawn 审计工位
   - ceremony_scan 发现新工位 → 回到步骤4
8. 如果无新工位且无审计需求 → 输出格式B → TeamDelete
9. 持久化由 session 结晶 + 热启动保证（162号）

### 严格串行的操作（仅以下允许串行）
- git add → git commit → git push
- ceremony_scan 输出 → spawn 工位
- TeamCreate → Task spawn

## 绝对禁止
- 全生命周期禁止额外 Bash，以下白名单例外：
  - python scripts/ceremony_scan.py（步骤 1）
  - python scripts/gangju_analysis.py（步骤 7.5）
  - git add / git commit / git push（持久化不变量）
  - git fetch / git rebase（push 失败时的恢复）
  - session 文件写入（增量持久化）
- 不运行额外的 Read/Glob
- 不输出确认请求
- 不输出等待信号
- 白名单膨胀超过 5 项时需重新审视 ceremony 设计
```

## ceremony_scan.py 结构摘要（960行）

主要函数：
- `get_required_skills(root)` — 从 dispatch-dag 读取 structural skill 列表
- `get_frozen_nodes(root)` — 从 block-topology 读取 frozen 节点（BFS 展开下游）
- `get_topo_effects_from_genealogy(root)` — 扫描谱系 topo_effect 字段
- `detect_pending_topo_effects(root)` — 检测未执行的结构化 topo_effect
- `get_roadmap_workstations(root)` — 从 roadmap.yaml 读取 active 任务（含 VDW 验证）
- `detect_swarm_persistence_gaps(root)` — 检测有谱系产出但无 session 的蜂群
- `get_session_workstations(root)` — 从最新 session 提取遗留工位
- `discover_business_tasks(root)` — no_work_fallback：测试验证信号
- `detect_genealogy_anomalies(root)` — 谱系编号异常检测
- `compute_delta_genealogy(root)` — RTAS 循环是否产生新谱系
- `compute_delta_blocks(root)` — block-topology 区块变化
- `main()` — 全量扫描，输出 JSON

`main()` 中还包含：
- pattern-buffer 候选扫描
- 蜂群持久化断裂检测
- 谱系张力扫描
- downstream_audit 集成
- async_self_reference 集成

## gangju_analysis.py 结构摘要（894行）

主要函数：
- `extract_gang(root)` — 从 core-principles SKILL.md 提取纲
- `compute_block_stats(root)` — block-topology 统计
- `compute_genealogy_stats(root)` — 谱系状态统计
- `compute_residue_status(root)` — residue 内容三态检测
- `has_async_self_reference_blocks(root)` — 异步自指区块检测
- `_scan_weak_inquiry_signals(root)` — 弱质询信号词扫描
- `derive_mu(block_stats, genealogy_stats, root)` — 从纲推导 filled_mu/empty_mu/new_mu
- `generate_pending_skeleton(root, new_mu)` — 生成谱系 pending 骨架
- `main()` — 输出 JSON

## 诊断问题

请从代码层面诊断以下 5 个问题：

### 问题1：ceremony.md 逐步骤标注
对 ceremony.md 的每个步骤（步骤1-6及循环内操作），标注：
- **调度逻辑**（Lead 必须保留）：决定 spawn 什么、何时 spawn、如何协调
- **执行逻辑**（应 spawn 为工位）：实际执行某项工作的逻辑

### 问题2：ceremony_scan.py 作为工位的可行性
ceremony_scan.py 当前是 Lead 直接调用的 Bash 命令。
- 它能否作为独立工位执行（而非 Lead 内联调用）？
- 如果作为工位，Lead 如何消费它的输出（JSON）？
- 作为工位的边界条件：什么情况下必须保留在 Lead 内？

### 问题3：Bash 白名单分析
当前白名单（5项）：
1. `python scripts/ceremony_scan.py`
2. `python scripts/gangju_analysis.py`
3. `git add / git commit / git push`
4. `git fetch / git rebase`
5. session 文件写入

哪些可以 spawn 出去？哪些是 Lead 不可委托的？

### 问题4：Lead 最小形式的具体代码级方案
给出 ceremony.md 的重写方案：
- 目标：~30 行伪代码
- 保留什么？删除什么？合并什么？
- Lead 的 RTAS 循环最小形式是什么？

### 问题5：风险分析
Lead 变薄后：
- 谁来处理工位之间的协调？（工位 A 的输出是工位 B 的输入）
- 持久化不变量（session + commit + push）如何保证？
- 如果 ceremony_scan.py 作为工位，Lead 如何等待它的输出再 spawn 业务工位？
- 这是否引入了新的串行依赖？
