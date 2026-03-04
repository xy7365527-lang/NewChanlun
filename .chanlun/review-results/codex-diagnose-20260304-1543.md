# Codex diagnose — 2026-03-04 15:43:36 UTC

## 元数据

- **mode**: diagnose
- **subject**: ceremony 多线并行持续推进机制缺失：Lead 在 gangmu.yaml next_actions 全部 completion_check 通过后进入不动点终止，而非继续追加新推进动作
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-04 15:43:36 UTC
- **context-file**: tmp/codex-multiline-persist-ctx.md

## Prompt

## 诊断目标

ceremony 多线并行持续推进机制缺失：Lead 在 gangmu.yaml next_actions 全部 completion_check 通过后进入不动点终止，而非继续追加新推进动作

## 上下文

# Codex Diagnose 上下文：多线并行持续推进 + 持久化断裂

## 诊断目标

**失败现象**：Lead 每轮 ceremony 完成 scan 输出的工位后就进入收尾模式，session 持久化不连续。

编排者原话：
> "我觉得这个不持存的原因和纲目以及之前讨论过的并行线的问题有关，这一轮可能达到目的就停下来不持存了，但是我们实际上是要多线并行的。"

---

## 当前架构：ceremony_scan.py（核心文件）

### ceremony_scan.py 的工作逻辑

**位置**：`scripts/ceremony_scan.py`

扫描优先级（代码第880-1027行）：
1. `roadmap.yaml`（最高优先级，结构化业务目标）
2. session 遗留项 + pending 谱系
3. pattern-buffer candidates
4. 审查结果（review-results/）
5. 区块拓扑映射缺口
6. 偶遇检测（encounter detection）
7. **研究线扫描（`_scan_research_lines`）——gangmu.yaml 的 active 目的 unblocked next_actions**
8. 谱系编号异常
9. 异步自指审计
10. no_work_fallback（只有全部为空时触发）

关键函数 `_scan_research_lines(root)`（第633-777行）：
- 读取 `gangmu.yaml`，遍历所有 gang 的 mu（目）
- 对 `status == "active"` 的目：提取 `next_actions` 中 `blocked_by == null` 的项
- 执行 `completion_check`（file_exists / genealogy_settled / script_exists / test_pass）
- 未完成的 unblocked action 生成工位，格式：`"纲[{gang_name}]目[{mu_id}]：{target}"`

**终止条件**（第1051-1056行）：
```python
result["clean_terminate"] = (
    len(roadmap_tasks) == 0
    and len(workstations) == 0  # <- 这里包含了 research_lines 生成的工位
    and pending_count == 0
    and len(review_results) == 0
)
```

### gangmu.yaml 当前状态

active 状态的目（mu）：
1. `operational-methodology`（纲一 实盘闭环）
   - G1-position-management：file_exists → `scripts/position_manager.py`（✅ fc8c8c0 已创建）
   - G2-fugue-state-machine：blocked_by 赋格状态机定义未结算
   - I1-xiaozhuan-da-orchestration：test_pass → `tests/test_xiaozhuan_da_integration.py`（✅ fc8c8c0 已创建）

2. `k4-engineering`（纲二 四矩阵全域）
   - k4-monitor-vps-deploy：script_exists → `scripts/k4_monitor_deploy.sh`（❓ 待确认）
   - k4-config-encoder：file_exists → `src/newchan/k4_config.py`（✅ fc8c8c0 已创建）

3. `block-topology-eng`（纲三 区块拓扑）
   - next_actions: []（空列表——无工位生成）

4. `traverse-infra`（纲五 知识谱系）
   - encounter-record-mechanism：genealogy_settled → keyword "encounter-record"（❓ 待确认）

---

## 问题核心

### 编排者的诊断

Lead 的行为模式是"单轮单目标"——完成当前 ceremony scan 输出的工位后就进入收尾模式。

但纲目 gangmu.yaml 中有 5 条纲、多条 active 目，很多 next_actions 处于 active 状态。

具体表现：
- ceremony_scan.py 输出 N 个工位 → Lead spawn N 个 → N 个完成 → Lead 关闭 ceremony → 不动点终止
- 但 gangmu.yaml 中还有其他活跃线的 next_actions 未推进
- session 写入自然也不持存——因为 Lead 已经在"关闭"模式中

### 上一轮 Codex 诊断结论（第一轮）

判定为实现错误：持久化语义绑定在"轮次结束"而非"事件提交"。修复方案：每条 completion 处理后立即 write_session()。

### 编排者追问

持存断裂不只是"提交点位置错误"，而是更深层的问题：Lead 在 ceremony 不动点终止后就不再写 session，因为它认为"这轮任务已全部完成"。

---

## 需要诊断的三个问题

### 问题一：根因定位

是 ceremony_scan.py 的输出不够全面（漏了活跃线的工位），还是 Lead 的消费逻辑不够（scan 输出足够但 Lead 只处理一部分就停了）？

**当前代码行为**：`_scan_research_lines` 在每次 scan 时遍历所有 active 目，对每个未完成的 unblocked action 生成工位。如果 `completion_check` 通过（文件存在），工位不生成。

**可能的缺口**：
1. `completion_check` 通过了（文件已创建），Lead 认为"这条研究线已完成"，但研究线实际上还有后续推进
2. `gangmu.yaml` 中的 `next_actions` 列表没有被更新到反映新的推进需求
3. Lead 在不动点终止时，gangmu.yaml 中的 active 目实际上已没有 unblocked + 未完成的 next_actions

### 问题二："多线持续推进"的机制设计

当前 rescan 循环（ceremony 步骤 8-10）的逻辑：
- rescan → 扫描工位 → 若工位非空且与上轮不同 → spawn → rescan 循环
- 若工位为空或与上轮相同 → 不动点终止

"多线并行持续推进"要求什么不同的机制？选项：
A. ceremony_scan.py 每轮输出所有活跃线的下一步（当前行为——已经这样做了）
B. rescan 循环不以"工位为空"作为终止条件，而是以"所有活跃线都推进了一步"
C. 需要一个新的"纲目驱动循环"：在 scan→spawn→rescan 之外，额外追踪每条研究线的推进状态
D. gangmu.yaml 的 next_actions 需要在工位完成后动态更新，而非静态列表

### 问题三：持存在"多线持续推进"模式下的自然涌现

如果 Lead 持续推进多条线：
- session 需要在每条线推进后增量写入（不是在轮次结束时批量写入）
- 但"持续推进"本身依赖的是 gangmu.yaml 中 next_actions 的动态状态

---

## 关键文件摘要

### ceremony_scan.py：_scan_research_lines（第633-777行）

核心逻辑：
```python
for action in next_actions:
    action_blocked_by = action.get("blocked_by")
    is_blocked = action_blocked_by is not None and action_blocked_by != ""

    if is_blocked:
        all_completed = False
        continue

    # unblocked action——检查 completion_check
    all_blocked = False
    completion_check = action.get("completion_check")
    completed = _check_completion(root, completion_check)

    if completed:
        continue  # <- 通过检查的 action 跳过，不生成工位

    # 未完成的 unblocked action → 生成工位
    new_workstations.append({
        "priority": "P2",
        "name": f"纲[{gang_name}]目[{mu_id}]：{target}",
        ...
    })
```

**关键观察**：当所有 unblocked action 的 completion_check 都通过时，该目不生成任何工位。
但目的 status 仍然是 "active"——ceremony_scan 不会自动关闭它。
仅生成 "proposed_transitions"（提议 active→closed），不实际修改 gangmu.yaml。

### gangmu.yaml：active 目的状态

```yaml
- id: operational-methodology
  status: active
  next_actions:
    - target: "G1-position-management"
      completion_check: {type: file_exists, path: "scripts/position_manager.py"}
    - target: "G2-fugue-state-machine"
      blocked_by: "谱系定义未结算：赋格状态机..."
    - target: "I1-xiaozhuan-da-orchestration"
      completion_check: {type: test_pass, pattern: "tests/test_xiaozhuan_da_integration.py"}
```

**推断**：G1 和 I1 的文件 fc8c8c0 已创建 → completion_check 通过 → 工位不生成。
G2 blocked → 工位不生成。
结果：`operational-methodology` 这个 active 目本轮不产生任何工位。

---

## 诊断假说（供 Codex 验证/否定）

**假说**：持存断裂是"ceremony 提前终止"的症状，而"ceremony 提前终止"的根因是 gangmu.yaml 中的 next_actions 列表静态化——工位完成后 completion_check 通过但 next_actions 没有追加新的推进动作，导致 scan 输出工位为空，Lead 进入不动点终止。

**推论**：
1. "多线持续推进"需要的不是改变 ceremony 循环的终止逻辑，而是让 gangmu.yaml 的 next_actions 保持动态更新
2. 实现机制：工位完成时（completion_check 通过），通知纲目驱动层（gangmu.yaml 更新下一步 next_actions）
3. 持存在这个机制下自然涌现：每次 gangmu.yaml 更新 → 下轮 scan 产出新工位 → 持续推进不断裂

**反假说（待验证）**：Lead 的消费逻辑本身有问题——即使 scan 输出了工位，Lead 也会在第一批工位完成后停止，不等待 gangmu 状态更新。

---

## 被审查代码位置

1. `scripts/ceremony_scan.py`：第633-777行（`_scan_research_lines`）+ 第1022-1056行（终止逻辑）
2. `.chanlun/gangmu.yaml`：active 目的 next_actions 结构


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

结论先行：**这是实现错误，不是定义冲突**。  
失败链条是明确的：

> 失败现象（提前终止）  
→ 直接原因（scan 结果为空即 clean_terminate）  
→ 根本原因（缺失“active 目持续推进”的状态机机制，completed action 被静默吞掉且不触发 next_actions 追加）  
→ 修复（补“续推动作生成 + 终止条件收紧 + 事件级持久化”）

---

## 1) 失败现象

你描述的现象成立且和代码一致：

- Lead 完成本轮 scan 输出的工位后，下一次 rescan 很快变空；
- `clean_terminate=True`，ceremony 进入收尾；
- session 后续不再持续写入（因为流程已终止，事件不再产生）。

---

## 2) 直接原因（代码级）

### 直接原因 A：`_scan_research_lines` 会把“已完成 action”直接跳过
在你给的逻辑中：

- blocked action：`continue`（不出工位）
- completion_check 通过：`continue`（不出工位）
- 仅“未完成且 unblocked”才出工位

所以当一个 active 目下：
- unblocked action 都通过了 completion_check；
- 其余 action blocked 或 next_actions 为空；

则该 active 目**不产生任何工位**。

---

### 直接原因 B：终止条件把“无工位”当作“可清洁终止”
`clean_terminate` 关键条件之一是：

```python
len(workstations) == 0
```

且没有把“仍存在 active 但未闭环的目”作为否决条件。  
于是出现：**active 还在，但工位为 0，也会终止**。

---

## 3) 根本原因（设计/实现层）

根本不是 Lead“只消费一部分工位”。  
按你给的扫描逻辑，scan 每轮遍历全部 active 目，消费侧不是主因。

**根本原因是状态机缺口**：系统只有“发现未完成 action”的检测器，没有“completed 后推进下一步”的推进器。

更具体：

1. `next_actions` 被当作静态清单；
2. action 完成后只是不再出现，不会触发“追加新 action”；
3. active 目可进入“静默 active”（既不出工位也不关闭）；
4. 全局终止又只看工位是否为空，导致局部不动点被误判为全局终态。

---

## 4) 分类判定

**判定：实现错误（机制缺失 + 终止判定过宽）**

理由：不需要改“active/next_actions/completion_check”的语义定义，就可以修复。  
通过补状态推进与终止守卫即可闭合。

---

## 5) 修复方案（精确步骤）

### 步骤 1：给 active 目加“不静默”约束
在 `_scan_research_lines` 内，对每个 active 目统计结果后增加分支：

- 若未生成任何执行工位，则必须生成一个“治理工位”：
  - `blocked-only`：生成“解阻工位”（根据 `blocked_by`）
  - `all-completed`：生成“续推/闭合决策工位”（要求更新 gangmu：追加 next_actions 或显式 close）
  - `next_actions=[]`：生成“规划缺失工位”

这样 active 目不会无声消失。

---

### 步骤 2：收紧 `clean_terminate`
在终止条件加入：
- `unresolved_active_mu_count == 0`
- `stagnant_active_mu_count == 0`（active 但无可执行 action 且未闭合）

即：**不是“没工位就终止”，而是“没有未闭环 active 目才终止”**。

---

### 步骤 3：completion 事件后更新 gangmu（追加下一步）
在工位完成处理路径中加入 hook：

- 识别对应 mu/action 完成；
- 写回 `.chanlun/gangmu.yaml`（至少标记 done，必要时 append 新 next_actions）；
- 触发 rescan。

否则你永远依赖静态 next_actions，无法持续推进。

---

### 步骤 4：事件级持久化（你上一轮结论的落地）
在以下事件后立即写 session：

1. 工位 completion
2. gangmu 更新（done / append / unblock）
3. 终止判定状态变更

这能消除“终止后才批量写”的断裂。

---

## 对你三个问题的直接回答

1. **是 scan 输出不全面还是 Lead 消费不够？**  
   - 结论：**不是消费不够**；scan 已遍历全部 active。  
   - 问题在 scan/状态机语义：completed/blocked 会导致 active 目不出工位，进而被全局终止吞掉。

2. **多线持续推进需要改哪里？**  
   - 核心不是改“循环次数”，而是补 **C + D**：  
     - C：纲目驱动状态追踪（active 目不能静默）  
     - D：next_actions 动态更新（completion 后追加后续动作）

3. **持存如何自然涌现？**  
   - 先有持续推进事件流（工位完成→gangmu 更新→新工位），再做事件级 write_session。  
   - 仅改写入时机但不补推进机制，仍会在“空工位终止”处断流。
