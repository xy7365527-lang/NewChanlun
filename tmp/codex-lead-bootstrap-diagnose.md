# Codex Diagnose 结果：Lead 最小自举形式

模式：diagnose
日期：2026-02-26
Codex 模型：gpt-5.3-codex
持久化：`.chanlun/review-results/codex-diagnose-20260226-0556.md`

---

## 总判定

Codex 否定成立。当前 `ceremony.md` **未达到"形式即行为"**。
Lead 仍在做业务分流判断 + 执行性工作，是记忆依赖，不是结晶。

我的质询结论：Codex 的四项否定全部有效，无误判。

---

## 1. 逐行标注结果（调度 vs 执行）

Codex 使用四类标记：
- **[D]** 调度逻辑（Lead 必保留）
- **[E]** 执行逻辑（应 spawn 为工位）
- **[S]** 决策应脚本化（移入 `ceremony_scan.py`）
- **[N]** 叙述噪声（可删）
- **[X]** 定义冲突

关键标注：

| ceremony.md 内容 | 标记 | 说明 |
|----------------|------|------|
| 设计原则6条 | [N] + [X] | 叙述噪声；"结构能力=skill非teammate"与后文冲突 |
| `ceremony_scan.py` 调用 | [D] | Lead 核心不可委托 |
| `required_skills` 字段 | [X] | skill vs teammate 边界未统一 |
| TeamCreate | [D] | 不可委托 |
| 并行 spawn workstations | [D] | 核心调度行为 |
| "待Gemini/长期"分支4条 | [S] | 决策树，不能留给 Lead 解释 |
| 循环4：拓扑分析家/meta-observer | [E] | 应 spawn 为工位 |
| 循环7：`gangju_analysis.py` | [E] | 可工位化或并入 scan |
| `audit_needed → spawn 审计` | [S] | 决策应脚本化 |
| git add/commit/push | [D] | 持久化责任链不可委托 |
| 步骤2摘要输出、步骤5行动声明 | [N] | 叙述噪声 |

---

## 2. 根因诊断（四项失败）

### F1：Lead 仍在做业务分流判断（实现错误）
- 失败现象：步骤4分支（Gemini/长期/可执行）由自然语言定义，Lead 需要推理
- 根本原因：`ceremony_scan.py` 输出是"状态摘要"，不是"动作清单"
- 修复：扩展 scan 输出为 `actions[]`（已决策的 spawn/route/defer），Lead 只执行，不解释

### F2：Lead 仍在执行工作（实现错误）
- 失败现象：循环4要求 Lead 自行做拓扑分析、meta 观察
- 根本原因：调度器与执行器边界未被代码化约束
- 修复：拓扑/meta/gangju 全部建模为 workstation，由 ceremony_scan.py 产出，Lead 统一 spawn

### F3：结构能力是 skill 还是 teammate？（定义冲突）
- 失败现象：前文"结构能力=skill，不是teammate"，后文又要求 spawn 拓扑/meta/审计
- 根本原因：架构定义未定稿（skill自动触发 vs workstation显式调度）
- 冲突方：
  - A：结构能力只能 skill 事件驱动，不 spawn teammate
  - B：结构能力可作为工位并行 spawn
- 不可弥合：二者在调度可见性、生命周期、资源归属上互斥
- **→ 需要矛盾上浮**

### F4：文档含大量非行为文本（实现错误）
- 失败现象：Lead 需要理解摘要、行动声明、格式B、编号注释等人类叙事
- 修复：运行文档只保留状态机动作，理论原则迁移到独立设计文档

---

## 3. Bash 白名单分析

| 项 | 可 spawn？ | 判定 |
|---|---|---|
| `python scripts/ceremony_scan.py` | 否 | Lead 核心调度输入，不可委托 |
| `python scripts/gangju_analysis.py` | 是 | 可工位化（gangju-analyst）或并入 scan |
| `git add/commit/push` | 否 | 持久化责任链必须在 Lead |
| `git fetch/rebase` | 否 | push 失败恢复，与提交责任同属 Lead |
| session 文件写入 | 否 | 与提交一致性强绑定 |

白名单从 5 项缩减为 3 类（gangju 下放后）：ceremony_scan.py + git 操作 + session 写入。

---

## 4. Lead 最小形式伪代码（Codex 版，~25行）

```python
plan = run("python scripts/ceremony_scan.py --json")   # 确定性计划

if plan.noop:
    persist_session(plan.session_delta)
    git_add_commit_push()
    exit()

team = TeamCreate(plan.team_name, plan.description)

spawn_parallel(plan.spawn_tasks)        # 业务 + 结构工位统一
route_parallel(plan.route_tasks)        # Gemini 路由
record_deferred(plan.deferred_tasks)    # 长期积压

while True:
    states = TaskList(team)

    done = collect_done(states)
    shutdown_parallel(done)
    consume_parallel(done)
    persist_session(done.session_delta)

    if has_in_progress(states):
        continue

    persist_session(full_snapshot=True)
    git_add_commit_push_with_recovery()

    plan = run("python scripts/ceremony_scan.py --json")   # 重扫描

    if plan.terminate:
        TeamDelete(team)
        break

    spawn_parallel(plan.spawn_tasks)
    route_parallel(plan.route_tasks)
    record_deferred(plan.deferred_tasks)
```

---

## 5. 与 Gemini 方案对比

### 一致（Codex 与 Gemini 收敛）
1. 最小链路：`scan → TeamCreate → spawn_all → consume → persist → re-scan → TeamDelete`
2. 决策逻辑移入 `ceremony_scan.py`
3. Lead 不跑 pytest，只消费验证工位结果
4. 并行优先，去串行残余
5. gangju_analysis.py 可工位化或并入 scan

### 分歧（Codex 发现 Gemini 未明确标注的冲突）
- **结构能力表示法**：Gemini 倾向"结构能力也可作为工位输出"；当前文档前部写"结构能力=skill非teammate"
- Codex 将此标注为 **[X] 定义冲突**，Gemini 方案中未显式处理这个矛盾
- 分歧根因：skill 自动触发体系 vs workstation 显式调度体系，边界定义未统一

---

## 6. "形式即行为"缩减：应删除的行

从运行文档删除/迁移：
- 整个"设计原则"段（迁移到设计文档）
- 步骤2（摘要输出）
- 步骤5（行动声明）
- 步骤4中"Gemini/长期"自然语言分支（改为 scan 输出动作）
- 步骤6.4中 Lead 自行执行拓扑/meta
- 步骤7中 `gangju_analysis.py` 作为 Lead Bash 调用（改工位或并入 scan）
- 所有"格式B/编号注释"展示性要求

保留：scan、TeamCreate、并行 spawn、TaskList 消费、session 持久化、git 顺序、TeamDelete、串行白名单约束。

---

## 7. 风险分析

1. **协调归属风险**：工位协调落在 `ceremony_scan.py` 上；若输出不完整，会漏 spawn 或重复 spawn
2. **状态一致性风险**：需要稳定任务 ID 与去重键（idempotency key），否则重扫会重复派发
3. **并发收敛风险**：批量完成时必须原子写 session delta，再提交
4. **可行性结论**：代码层可行，前提是 `ceremony_scan.py` 从"状态扫描器"升级为"确定性调度计划生成器"

---

## 我的判定

### 否定成立（F1/F2/F4）→ 实现错误，可修复
- F1：ceremony_scan.py 扩展为动作清单（spawn_tasks/route_tasks/deferred_tasks/terminate）
- F2：拓扑/meta/gangju 全部移入 workstations 列表
- F4：删除所有叙述性段落，只保留状态机动作

### 否定成立（F3）→ 定义冲突，需矛盾上浮
- "结构能力=skill事件驱动" vs "结构能力可作为工位显式spawn"
- 这不是实现错误，是架构定义层的矛盾
- 在此矛盾解决前，ceremony.md 无法完成最终重写

---

## 边界条件

- 如果 F3 定义冲突解决为"结构能力可作为工位"：ceremony_scan.py 扩展方案完全可行
- 如果 F3 解决为"结构能力只能 skill"：则 meta-observer/topology-analyst 不能出现在 workstations 列表，Lead 的"不空转"逻辑需要完全删除（而不是移入 scan）

## 影响声明

- 涉及模块：`ceremony.md`、`scripts/ceremony_scan.py`、`scripts/gangju_analysis.py`
- 涉及谱系：075号（结构能力定义）、069号（递归拓扑）、174号（谱系即生成引擎）
