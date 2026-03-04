# required_skills 消费机制设计

## 一、问题陈述

`ceremony_scan.py` 的 `get_required_skills()` 从 dispatch-dag.yaml 的 `event_skill_map` 中提取 `skill_type=structural` 的 skill 列表，输出到 JSON 的 `required_skills` 字段。但 `ceremony.md`（Lead 指令）不消费此字段。

结果：结构工位（genealogist、quality-guard、meta-observer、code-verifier、topology-mutator）虽在 dispatch-dag 中声明为 structural，但 ceremony 序列不 spawn 它们。

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

### 步骤 1：修改 ceremony_scan.py

在 `get_required_skills()` 的返回中增加 `spawn_condition` 字段，标注每个 skill 的 spawn 条件：

```python
def get_required_skills(root):
    # ... 现有逻辑 ...
    for skill in structural_skills:
        skill["spawn_condition"] = _get_spawn_condition(skill["id"], result)
    return skills

def _get_spawn_condition(skill_id, scan_result):
    """根据 scan 结果判断 structural skill 是否应 spawn。"""
    if skill_id == "genealogist":
        # 有非纯结构的业务工位时 spawn
        return any(w.get("source") not in ("topo_mapper", "genealogy_anomaly_detection")
                   for w in scan_result.get("workstations", []))
    if skill_id in ("quality-guard", "code-verifier"):
        return any(w.get("source") in ("roadmap", "session", "research_lines")
                   for w in scan_result.get("workstations", []))
    if skill_id == "meta-observer":
        return False  # 仅在终止阶段 spawn，由 Lead 控制
    if skill_id == "topology-mutator":
        return bool(scan_result.get("pending_topo_effects"))
    return False
```

### 步骤 2：修改 ceremony.md

在步骤 5 之后增加：

```
5b. JSON.required_skills[] 中 spawn_condition=true 的 skill：
    并行 spawn 为 agent（与业务工位并行）。
    spawn 模板同步骤5，但使用 skill 的 agent 文件作为指令源。
```

在步骤 10（终止阶段）中增加：

```
10a. TeamDelete 前，spawn meta-observer agent 执行二阶观察。
     等待完成后再 TeamDelete。
```

### 步骤 3：注册缺失的 hooks

将 `source-auditor-prompt.sh` 注册到 settings.json：

```json
{
  "matcher": "Write",
  "hooks": [{
    "type": "command",
    "command": ".claude/hooks/source-auditor-prompt.sh"
  }]
}
```

### 步骤 4：test_pass 语义修复

将 gangmu.yaml 中使用 `test_pass` 的条目改为 `test_file_exists`，并在 `_check_completion` 中增加此类型作为 `test_pass` 的别名，保持向后兼容：

```python
if check_type in ("test_pass", "test_file_exists"):
    pattern = check.get("pattern", "")
    return os.path.isfile(os.path.join(root, pattern))
```

## 五、不在本方案范围内的问题

1. **语义级事件系统**：构建 task_complete、genealogy_settlement 等语义事件的产生和分发机制——这需要更深层的架构变更（可能需要自定义事件总线），不在 required_skills 消费方案的范围内。

2. **D策略的系统化**：让 Lead 能够系统地识别和响应 hook 提示——这需要修改 Lead 的 RTAS 循环逻辑，属于更大范围的架构改进。

## 六、优先级排序

| 优先级 | 动作 | 理由 |
|--------|------|------|
| P0 | ceremony.md 增加 required_skills 消费步骤 | 解除最核心的消费链断裂 |
| P1 | ceremony_scan.py 增加 spawn_condition 逻辑 | 为 ceremony 消费提供判断依据 |
| P1 | 注册 source-auditor-prompt.sh 到 settings.json | 修复死代码 hook |
| P2 | test_pass → test_file_exists 重命名 | 消除声明膨胀 |
| P3 | ceremony.md 终止阶段 spawn meta-observer | 确保每轮 ceremony 有二阶观察 |
