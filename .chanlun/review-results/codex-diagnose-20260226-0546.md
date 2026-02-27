# Codex diagnose — 2026-02-26 05:46:21 UTC

## 元数据

- **mode**: diagnose
- **subject**: Lead 最小形式——ceremony.md 从胖节点到递归拓扑最小自举形式的代码级诊断
- **model**: gpt-5.2-codex
- **timestamp**: 2026-02-26 05:46:21 UTC
- **context-file**: tmp/codex-lead-minimal-form-ctx.md

## Prompt

## 诊断目标

Lead 最小形式——ceremony.md 从胖节点到递归拓扑最小自举形式的代码级诊断

## 上下文

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

以下为**严格诊断报告**，按“失败现象 → 直接原因 → 根本原因 → 分类判定 → 修复方案/定义冲突”输出，并逐项覆盖 5 个诊断问题。

---

# 诊断总览（Lead 最小形式）

**失败现象**：`ceremony.md` 仍是“胖节点”，步骤内混合调度逻辑与执行逻辑，违反 069/090/218 的“Lead 必须与拓扑同构、非串行瓶颈、严格形式化”的要求。  
**直接原因**：步骤 1–6 与 RTAS 循环内包含脚本运行、会话写入、分析执行等执行逻辑，由 Lead 直接承担。  
**根本原因**：ceremony.md 没有把“调度逻辑”与“执行逻辑”在代码层划界，导致 Lead 持续吸纳执行责任。  
**分类判定**：实现错误（不改变定义即可修复）。  
**修复方案**：将执行逻辑全部迁移为工位任务，Lead 仅保留调度、串行化边界与持久化不变量。

---

# 问题1：ceremony.md 逐步骤标注（调度逻辑 vs 执行逻辑）

## 诊断链
- **失败现象**：步骤 1–6 与循环内操作混合执行与调度。  
- **直接原因**：`python scripts/*.py`、session 写入、结果处理与测试运行被描述为 Lead 行为。  
- **根本原因**：ceremony.md 未显式规定“可委托执行”与“必须保留调度”的边界。  
- **分类判定**：实现错误。  
- **修复方案**：按以下表格划界，执行逻辑必须 spawn 为工位（或在规则允许时委托）。

## 步骤划界表（必须保留 vs 应下放）

| 步骤/子步骤 | 调度逻辑（Lead 保留） | 执行逻辑（应 spawn 为工位） |
|---|---|---|
| **步骤1：运行扫描脚本** | 触发扫描、获取输出、阻塞等待结果 | 运行 `python scripts/ceremony_scan.py` |
| **步骤2：输出摘要** | 输出摘要格式与调度记录 | 无 |
| **步骤3：TeamCreate** | 创建 team、维护唯一标识 | 无 |
| **步骤4：并行 spawn 工位** | 遍历 JSON、判断空/待 Gemini/长期、决定 spawn/route、TeamName 绑定、流程分支 | 实际任务执行（每个工位任务本体）|
| **步骤4：无工位分支** | 决定终止、宣布无新区分 | session 写入 / commit / push（执行逻辑，但因持久化不变量可保留） |
| **步骤5：输出行动声明** | 输出“监控 N 个工位” | 无 |
| **步骤6：RTAS 循环** | 调度 TaskList、集中并行汇报、触发 shutdown_request、spawn 辅助分析/测试/审计 | 结果分析、测试执行、拓扑分析、meta-observer 等执行工作 |
| **循环内：增量持久化** | 决定何时持久化 | session 写入（执行逻辑）|
| **循环内：git add/commit/push** | 决定提交时机 | git 操作（执行逻辑；但由于串行化不变量必须保留在 Lead） |
| **循环内：重扫描** | 决定是否重扫、是否继续 | 运行 `gangju_analysis.py` / `ceremony_scan.py` |

> 注：session 写入与 git 操作虽是“执行逻辑”，但因不变量必须串行化，通常保留在 Lead。

---

# 问题2：`ceremony_scan.py` 作为工位的可行性

## 诊断链
- **失败现象**：扫描脚本由 Lead 直接调用，导致 Lead 承担执行逻辑。  
- **直接原因**：步骤 1 中 `python scripts/ceremony_scan.py` 以内联 Bash 运行。  
- **根本原因**：未定义“扫描输出如何跨工位回传”的机制。  
- **分类判定**：实现错误。  
- **修复方案**：将 scan 作为独立工位执行并返回 JSON 给 Lead。

## 结论（回答三问）

### 1) 能否作为独立工位执行？
**可以**。脚本是确定性扫描，不依赖 LLM 状态；执行逻辑天然可下放。

### 2) Lead 如何消费输出？
**必须通过工位消息回传 JSON（stdout → 工位结果）**。  
原因：绝对禁止额外 Read/Glob，不允许 Lead 读文件。  
可执行方案：  
- 工位执行 `python scripts/ceremony_scan.py`  
- 将原始 JSON 原封不动写入任务输出  
- Lead 从 TaskList/TaskResult 消费 JSON

### 3) 边界条件：何时必须保留在 Lead 内？
必须保留在 Lead 内的条件只有一种：  
> **如果无法保证 JSON 原样回传（LLM 参与导致内容变形，或无可用输出通道）**  
此时扫描结果不可信，调度逻辑无法正确执行。

---

# 问题3：Bash 白名单分析（可 spawn vs 不可委托）

## 诊断链
- **失败现象**：白名单内操作未明确“可委托/不可委托”。  
- **直接原因**：ceremony.md 未区分“执行逻辑可下放”与“持久化必须串行化”。  
- **根本原因**：未将持久化不变量与 Bash 白名单绑定。  
- **分类判定**：实现错误。  
- **修复方案**：如下表强制划界。

## 白名单划界表

| 白名单项 | 可 spawn？ | 原因 |
|---|---|---|
| `python scripts/ceremony_scan.py` | ✅ 可 | 执行逻辑，且可通过任务输出回传 JSON |
| `python scripts/gangju_analysis.py` | ✅ 可 | 执行逻辑，输出 JSON/结论用于调度 |
| `git add / git commit / git push` | ❌ 不可 | 串行化不变量；必须由 Lead 统一控制 |
| `git fetch / git rebase` | ❌ 不可 | 仅用于 push 失败的恢复，涉及 repo 状态，需单点控制 |
| `session 文件写入` | ⚠️ 建议不委托 | 执行逻辑，但与 commit/push 强绑定，需串行一致性 |

---

# 问题4：Lead 最小形式代码级方案（~30 行伪代码）

## 诊断链
- **失败现象**：ceremony.md 过长，Lead 逻辑过重。  
- **直接原因**：步骤描述细节化，将执行逻辑内嵌。  
- **根本原因**：未进行调度-执行剥离与最小循环抽象。  
- **分类判定**：实现错误。  
- **修复方案**：如下 30 行伪代码 + 删除/合并说明。

## 保留 / 删除 / 合并
- **保留**：扫描 → 生成工位 → TeamCreate → spawn → TaskList 监控 → 持久化 → 重扫描 → TeamDelete  
- **删除**：冗长解释、Gemini/长期工位细节（移入工位或 scan 规则）  
- **合并**：步骤 2 与步骤 5；步骤 6 与循环内操作

## 最小伪代码（≈30 行）

```pseudo
scan = run_scan()                    # 可 spawn 工位
summary(scan)
if scan.workstations.empty:
    persist_session(); git_commit_push(); exit

team = TeamCreate("v{N}-swarm")
spawn_all(scan.workstations, team)

while true:
    status = TaskList(team)
    done = collect_done(status)
    if done:
        update_session(done)
    if any_in_progress(status):
        spawn_aux_analysis()         # 拓扑/observer/测试等
        continue

    persist_session(); git_commit_push()

    gangju = run_gangju()            # 可 spawn 工位
    scan = run_scan()                # 可 spawn 工位
    if gangju.audit_needed or scan.workstations:
        spawn_all(scan.workstations, team)
        continue

    TeamDelete(team)
    break
```

---

# 问题5：风险分析（Lead 变薄后的四个关键风险）

## 风险1：工位间协调丢失
- **失败现象**：工位 A 输出不能驱动工位 B。  
- **直接原因**：Lead 不再执行中间逻辑，且依赖未显式编码。  
- **根本原因**：`ceremony_scan`/session 未提供明确依赖图。  
- **分类判定**：实现错误。  
- **修复方案**：在 scan 输出中加入依赖字段，Lead 只基于依赖调度 spawn。

## 风险2：持久化不变量被破坏
- **失败现象**：退出路径未触发 session + commit + push。  
- **直接原因**：执行逻辑被委托，Lead 未统一把控退出路径。  
- **根本原因**：持久化不变量未硬编码为 Lead 责任。  
- **分类判定**：实现错误。  
- **修复方案**：session 写入 + git 操作由 Lead 单点执行，工位只返回增量变更。

## 风险3：scan 作为工位导致同步失效
- **失败现象**：Lead 在 scan 输出到达前就 spawn 业务工位。  
- **直接原因**：scan 工位异步执行，无阻塞等待。  
- **根本原因**：缺失“scan 完成 → spawn”的同步门槛。  
- **分类判定**：实现错误。  
- **修复方案**：明确 TaskList 等待 scan 完成并解析 JSON 后再继续。

## 风险4：是否引入新的串行依赖？
- **结论**：不会引入新的依赖。  
- **原因**：`ceremony_scan 输出 → spawn` 已被明确列为严格串行操作。  
- **判定**：非定义冲突，不改变现有定义即可满足。

---

# 结论

所有问题均指向同一根因：**调度与执行未分层**。  
这是实现错误，可通过 **“工位化执行 + Lead 保留调度与持久化”** 修复。  
不存在必须更改定义才能解决的冲突项。
