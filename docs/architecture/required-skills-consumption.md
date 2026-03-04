# required_skills 消费机制设计

## 一、问题陈述

`ceremony_scan.py` 的 `get_required_skills()` 从 dispatch-dag.yaml 的 `event_skill_map` 中提取 `skill_type=structural` 的 skill 列表，输出到 JSON 的 `required_skills` 字段。但 `ceremony.md`（Lead 指令）不消费此字段。

结果：结构工位（genealogist、quality-guard、meta-observer、code-verifier、topology-mutator）虽在 dispatch-dag 中声明为 structural，但 ceremony 序列不 spawn 它们。

**异质审计补充（Part 2 发现）**：
- required_skills 不是唯一的死数据字段——`tensions_count`、`definitions`、`proposed_transitions` 同样无消费者
- 问题的根因是模式 B（生产者-消费者演化不同步），不是单个字段的遗漏

## 二、平台约束分析

### Claude Code Hook 系统的事件粒度

| 支持的事件类型 | 粒度 |
|--------------|------|
| SessionStart | session 级 |
| PreCompact | session 级 |
| PreToolUse | 工具调用级（matcher: 工具名） |
| PostToolUse | 工具调用级（matcher: 工具名） |
| Stop | session 级 |

Claude Code **不支持**语义级事件（task_complete、genealogy_settlement 等）。因此 dispatch-dag.yaml 中声明的事件→skill 映射无法通过 hook 系统自动触发。

### D策略（082号）的意图

D策略是对平台约束的适配：
- Hook 在工具调用时提供**提示**（advisory message）
- Lead 收到提示后**认领**对应 skill（手动 spawn agent）

但当前实现的问题：Lead 指令（ceremony.md）中没有"检查 hook 提示并认领 skill"的步骤。

## 三、设计方案

### 方案核心：ceremony 序列消费 required_skills

在 ceremony.md 的步骤 5（spawn workstations）之后，增加一步 required_skills 消费：

```
5b. JSON.required_skills[] 中 skill_type=structural 的 skill：
    根据当前蜂群状态决定是否 spawn 对应 agent。
```

### 三类 required_skills 的消费策略

| Skill | 每轮 spawn？ | 条件 spawn？ | 条件 |
|-------|------------|------------|------|
| genealogist | 否 | 是 | workstations 中有概念层任务（非纯工程任务）时 spawn |
| quality-guard | 否 | 是 | workstations 中有代码修改任务时 spawn |
| meta-observer | 否 | 是 | 仅在 ceremony 终止阶段（步骤10 不动点）spawn 一次 |
| code-verifier | 否 | 是 | workstations 中有代码修改任务时 spawn |
| topology-mutator | 否 | 是 | ceremony_scan 输出中 detect_pending_topo_effects() 返回非空时 spawn |

### 条件 spawn 的判断逻辑

**genealogist**：
- 触发条件：任意 workstation 的 source 不是 "topo_mapper"/"genealogy_anomaly_detection" 的纯结构任务
- 职责：蜂群循环结束后执行回溯扫描、张力检查
- spawn 时机：步骤 7（所有工位完成后、commit 前）

**quality-guard**：
- 触发条件：workstations 中存在 source="roadmap"/"session"/"research_lines" 的业务工位
- 职责：检查产出的结果包合规
- spawn 时机：业务工位完成后、commit 前

**meta-observer**：
- 触发条件：ceremony 到达终止阶段（步骤 10 不动点 或 步骤 2 干净终止）
- 职责：执行二阶观察
- spawn 时机：TeamDelete 前

**code-verifier**：
- 触发条件：workstations 中存在涉及 src/**/*.py 或 tests/**/*.py 修改的工位
- 职责：运行 pytest 验证
- spawn 时机：业务工位完成后、commit 前

**topology-mutator**：
- 触发条件：ceremony_scan JSON 中 detect_pending_topo_effects 返回非空列表
- 职责：执行待执行的拓扑操作
- spawn 时机：步骤 5 与业务工位并行

### 不需要在 ceremony 中 spawn 的 skill

以下 conditional skill 不由 ceremony 自动 spawn，保持 D策略（Lead 手动认领或命令触发）：

| Skill | 触发方式 | 理由 |
|-------|---------|------|
| gemini-challenger | /challenge 命令 或 Lead 手动 | 消耗外部 API，按需使用 |
| claude-challenger | Lead 手动 | 二次质询，仅在异质产出存疑时 |
| codex-challenger | /code-review 命令 或 Lead 手动 | 消耗外部 API，按需使用 |
| topology-analyst | Lead 手动 | 冷读分析，仅在 block-topology 有足够数据时 |
| source-auditor | Lead 手动（需先注册 hook） | 溯源审计按需 |
| topology-manager | Lead 手动 | 拓扑收缩评估按需 |
| skill-crystallizer | ceremony_scan 工位 + Lead 认领 | 已通过 workstations 机制覆盖 |

## 四、实现路径

### 步骤 1：修改 ceremony_scan.py——增加 spawn_condition

在 `get_required_skills()` 的返回中增加 `spawn_condition` 字段，标注每个 skill 的 spawn 条件。

`spawn_condition` 的判断必须在 workstations 列表构建完成后执行（依赖 workstations 内容），因此在 `main()` 中 workstations 构建完成后调用：

```python
def _evaluate_spawn_conditions(skills, workstations, scan_result):
    """评估每个 structural skill 是否应 spawn。"""
    for skill in skills:
        sid = skill["id"]
        if sid == "genealogist":
            skill["spawn_condition"] = any(
                w.get("source") not in ("topo_mapper", "genealogy_anomaly_detection")
                for w in workstations
            )
        elif sid in ("quality-guard", "code-verifier"):
            skill["spawn_condition"] = any(
                w.get("source") in ("roadmap", "session", "research_lines")
                for w in workstations
            )
        elif sid == "meta-observer":
            skill["spawn_condition"] = False  # 终止阶段 Lead 控制
        elif sid == "topology-mutator":
            # detect_pending_topo_effects 已在 scan 中执行
            skill["spawn_condition"] = bool(
                scan_result.get("pending_topo_effects")
            )
        else:
            skill["spawn_condition"] = False
    return skills
```

在 `main()` 的 workstations 构建完成后（约第 1028 行 `result["workstations"] = workstations` 之后）插入：

```python
# 评估 structural skills 的 spawn 条件
pending_topo = detect_pending_topo_effects(root)
if pending_topo:
    result["pending_topo_effects"] = pending_topo
result["required_skills"] = _evaluate_spawn_conditions(
    result["required_skills"], workstations, result
)
```

### 步骤 2：修改 ceremony.md——消费 required_skills

在步骤 5 之后增加步骤 5b：

```
5b. JSON.required_skills[] 中 spawn_condition=true 的 skill：
    并行 spawn 为 agent（与业务工位并行）。
    spawn 模板同步骤5，但使用 skill 的 agent 文件作为指令源。
    prompt 前缀："你是结构工位 {skill.id}，读取 {skill.agent} 执行职责。"
```

在步骤 10（终止阶段）增加步骤 10a：

```
10a. rescan.workstations 为空（不动点到达）→ spawn meta-observer agent 执行二阶观察。
     等待完成 → TeamDelete → 停止。
```

### 步骤 3：注册缺失的 hooks

将 `source-auditor-prompt.sh` 注册到 settings.json 的 PostToolUse 段：

```json
{
  "matcher": "Write",
  "hooks": [{
    "type": "command",
    "command": ".claude/hooks/source-auditor-prompt.sh"
  }]
},
{
  "matcher": "Edit",
  "hooks": [{
    "type": "command",
    "command": ".claude/hooks/source-auditor-prompt.sh"
  }]
}
```

**注意**：source-auditor-prompt.sh 已实现对 Write/Edit 的 PostToolUse 处理逻辑，注册后立即生效。

### 步骤 4：test_pass 语义修复

**问题**（Part 2 缺口15）：gangmu.yaml 使用 `type: test_pass`，但 `_check_completion()` 实际只检查文件存在不执行测试。名称 `test_pass` 是声明膨胀。

**修复方案**：

在 `_check_completion` 注释中明确语义，不改变行为（避免引入 ceremony 扫描阻塞）：

```python
if check_type in ("test_pass", "test_file_exists"):
    # 语义：检查测试文件是否存在（不执行测试，避免扫描阻塞）。
    # type 名称 test_pass 是历史遗留，实际语义等同于 test_file_exists。
    pattern = check.get("pattern", "")
    return os.path.isfile(os.path.join(root, pattern))
```

同时在 gangmu.yaml 中将 `test_pass` 逐步迁移为 `test_file_exists`，保持 `_check_completion` 向后兼容。

### 步骤 5：消费 tensions_count（Part 2 缺口9 修复）

在 ceremony_scan.py 中，将 `tensions_count > 0` 转化为 workstation 条目：

```python
if tensions_found:
    result["tensions_count"] = len(tensions_found)
    workstations.append({
        "priority": "P3",
        "name": f"谱系张力：{len(tensions_found)}条 tensions_with 边",
        "status": "tensions_detected",
        "source": "tension_scan",
    })
```

### 步骤 6：消费 proposed_transitions（Part 2 缺口11 修复）

在 ceremony_scan.py 的 `_scan_research_lines` 返回后，将 proposed_transitions 转化为工位：

```python
if rl_context and rl_context.get("proposed_transitions"):
    for pt in rl_context["proposed_transitions"]:
        workstations.append({
            "priority": "P2",
            "name": f"纲目状态转换：{pt['line']} {pt['from']}→{pt['to']}",
            "status": f"proposed:{pt['reason'][:80]}",
            "source": "gangmu_transition",
        })
```

## 五、不在本方案范围内的问题

1. **语义级事件系统**：构建 task_complete、genealogy_settlement 等语义事件的产生和分发机制——需要更深层架构变更（自定义事件总线），不在 required_skills 消费方案范围内。

2. **D策略系统化**：让 Lead 能系统地识别和响应 hook 提示——需要修改 Lead 的 RTAS 循环逻辑。

3. **orchestrator-proxy decide 实现**（Part 2 缺口4）：gemini_challenger CLI 增加 decide 子命令——这是独立的工程任务，不属于消费机制设计。

4. **plan-review 自动触发**（Part 2 缺口7）：需要在 /plan 命令执行流程中硬编码 plan-review skill 加载——属于 /plan 命令的增强。

## 六、优先级排序

| 优先级 | 动作 | 理由 | 来源 |
|--------|------|------|------|
| P0 | ceremony.md 增加 required_skills 消费步骤（步骤2/5b/10a） | 解除最核心的消费链断裂 | Part 1 |
| P1 | ceremony_scan.py 增加 spawn_condition 评估逻辑 | 为 ceremony 消费提供判断依据 | Part 1 |
| P1 | 注册 source-auditor-prompt.sh 到 settings.json | 修复死代码 hook | Part 1 |
| P1 | ceremony_scan.py 将 tensions_count 转化为 workstation | 修复死数据 | Part 2 缺口9 |
| P2 | ceremony_scan.py 将 proposed_transitions 转化为 workstation | 修复死数据 | Part 2 缺口11 |
| P2 | test_pass → test_file_exists 语义修复 | 消除声明膨胀 | Part 2 缺口15 |
| P3 | ceremony.md 终止阶段 spawn meta-observer | 确保每轮 ceremony 有二阶观察 | Part 1 |
| P3 | topology-mutator-prompt.sh 标注废弃（功能已内联 dispatcher） | 消除重复代码歧义 | Part 2 缺口13 |
| P3 | meta-observer-guard.sh 注释与代码默认值对齐 | 消除声明-能力不一致 | Part 2 缺口14 |

## 七、依赖关系

```
步骤1(scan spawn_condition) → 步骤2(ceremony.md 消费)
步骤3(注册 hook)           → 独立
步骤4(test_pass 语义)      → 独立
步骤5(tensions workstation) → 独立
步骤6(transitions workstation) → 独立
```

步骤 1→2 有严格数据依赖（ceremony 消费 spawn_condition 字段）。步骤 3-6 互相独立，可与步骤 1-2 并行执行。
