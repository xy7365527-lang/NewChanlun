# 自主推进机制设计（Autonomous Progression）

## 一、问题陈述

当前 ceremony 到达不动点的条件是"无新工位产出"。但不动点有两种性质截然不同的情况：

1. **真不动点**：所有研究线的目标已完成，无新工作——系统结算态
2. **伪不动点**：某个 active 目的 `next_actions` 耗尽（全部完成或为空列表），但谱系 DAG 中存在未消费的下游推论——这不是结算，是推进断裂

当前 ceremony_scan.py 的 `_scan_research_lines` 只能消费已经写入 gangmu.yaml 的 `next_actions`，不能从谱系 DAG 推导新的 actions。伪不动点需要编排者手动注入新 actions 才能打破。

**具体例证**：
- `block-topology-eng`：status=active，但 `next_actions: []`——谱系中有多条未消费的下游推论指向区块拓扑方向
- `operational-methodology`：三个 next_actions 中两个 unblocked——但完成后不会自动从谱系 DAG 的下游推论推导下一步

## 二、设计目标

1. ceremony 能自主检测伪不动点并从谱系 DAG 的下游推论推导新 actions
2. 推导出的 actions 带依赖关系（从谱系 `depends_on` 图自然涌现，非线性 DAG）
3. 推导结果区分"可自主执行"和"需确认"两类
4. 兼容现有 ceremony 序列（不修改 ceremony.md 的步骤定义）

## 三、核心架构变更：推导来源从总方针改为谱系 DAG

### 3.0 设计原理

**编排者洞察**："总方针有一些是未来的战略方向，但是知识来源几乎全部来自谱系。"

这意味着两者的存在论位置根本不同：
- **总方针** = 方向指引（未来性，战略层）——包含尚未发生的战略目标
- **谱系** = 概念基础（已知性，知识层）——记录已结算的概念发现和它们之间的关系

旧设计让 `_derive_next_actions()` 读取总方针章节来推导新 actions。这是错误的——从未来的战略方向推导具体行动，等于用尚未成立的知识指导当前工作。正确的推导只能从已结算的知识出发：
- 谱系 DAG 记录了已结算的概念和它们之间的 depends_on 关系
- 每条谱系的"下游推论"章节是已结算概念对后续工作的**建议性延伸**（079号语法规则）
- `downstream_audit.py` 已经实现了对下游推论执行状态的扫描（140号闭合数据通路）
- `ceremony_scan.py` 已经将 unresolved 下游推论转化为工位

**正确的推导链**：谱系节点的 unresolved 下游推论 → proposed_actions 的来源。总方针降级为方向校验——推导出的 actions 与总方针方向一致性检查，不一致则标记为张力点。

### 3.1 新阶段：research_line_derivation

在 ceremony_scan.py 的 `_scan_research_lines` 返回后增加一个推导阶段。不修改 `_scan_research_lines` 本身——它保持消费已有 actions 的职责，推导逻辑独立。

```
现有流程:
  _scan_research_lines(root)
    → 扫描 gangmu.yaml
    → 输出 workstations（已有 unblocked actions）
    → 输出 proposed_transitions（closed/blocked 状态建议）

现有流程（downstream_audit 集成，140号）:
  downstream_audit(root)
    → 扫描谱系 settled/*.md 的下游推论
    → 输出 unresolved 下游推论 → 生成工位

新增流程:
  _derive_next_actions(root, rl_context, downstream_audit_result)
    → 检测"可推进但无 actions"的 active 目
    → 遍历谱系 DAG，匹配该目相关的 unresolved 下游推论
    → 将匹配的下游推论结构化为 proposed_actions
    → 写入 gangmu.yaml 的 proposed_actions 字段
    → 输出 derivation_workstations
```

### 3.2 触发条件

推导阶段仅在以下条件同时满足时触发：

1. 存在 active 目且 `unblocked_actions == 0`（所有 next_actions 已完成或全部被阻塞或列表为空）
2. 该目没有 `proposed_actions`（避免重复推导）
3. 谱系 DAG 中存在与该目相关的 unresolved 下游推论

不触发的情况：
- 目 status == closed/blocked：已终止或已阻塞，不需推导
- 目有 unblocked_actions > 0：还有可执行工作，不需推导
- 目已有 proposed_actions：上一轮推导结果尚未处理

### 3.3 推导数据流

```
谱系 DAG (dag.yaml)              downstream_audit 结果
    │                                    │
    ▼                                    ▼
 节点 depends_on 关系           unresolved 下游推论列表
    │                                    │
    └────────────┬───────────────────────┘
                 ▼
  按目 id / 谱系主题 匹配 active 目
                 │
                 ▼
  匹配的 unresolved 下游推论
                 │
                 ▼
  proposed_actions[]                总方针 (方向校验)
  （写入 gangmu.yaml）                    │
                 │                        ▼
                 ├── 方向一致 → 保留
                 └── 方向不一致 → 标记 direction_mismatch
                 │
                 ▼
  derivation_workstations[]
  （追加到 scan 输出）
```

**数据来源优先级**（替代旧设计的总方针驱动）：
1. **谱系下游推论**（一级来源）：downstream_audit 扫描的 unresolved 项，携带具体推导链和谱系 ID
2. **谱系 DAG 关系**（关系上下文）：depends_on 边提供概念间的依赖拓扑，自然涌现为 action 间的依赖
3. **总方针**（方向校验）：推导结果与总方针方向对比，不一致时标记但不阻塞

### 3.4 谱系→目 的匹配逻辑

每条 unresolved 下游推论需要匹配到对应的 active 目。匹配方式：

1. **显式标注**：下游推论文本中含有 `目[xxx]` 或 `research_line: xxx` → 直接匹配
2. **谱系 ID 关联**：谱系节点 ID 在 gangmu.yaml 的某个目的 `related_genealogy` 字段中 → 匹配该目
3. **主题关键词**：谱系标题/类型与目的 name/description 有关键词重叠 → 候选匹配（confidence: medium）
4. **DAG 邻居传播**：谱系节点的 depends_on 上游/下游节点中，有已匹配到某目的节点 → 传播匹配（confidence: low）

匹配失败的 unresolved 下游推论不生成 proposed_actions——它们继续作为 downstream_audit 的 unresolved 项存在，等待被工位主动认领（079号语义）。

### 3.5 依赖关系模型（从谱系 DAG 自然涌现）

每个 proposed_action 来自谱系节点的下游推论。其依赖关系**不需要额外声明**——直接从谱系 DAG 的 `depends_on` 边推导：

- 如果下游推论 A 来自谱系节点 X，下游推论 B 来自谱系节点 Y，且 DAG 中 Y depends_on X → B depends_on A
- 同一谱系节点的多条下游推论默认无依赖（并行）
- 跨目依赖从 DAG 的跨节点 depends_on 自然涌现

```yaml
# 示例：从谱系推导的 proposed_action
- type: engineering
  target: traverse-bfs-marking
  description: BFS生成树 + tree/critical边标记
  source_genealogy: "140"           # 来自140号谱系的下游推论
  source_downstream_index: 1         # 下游推论第1条
  depends_on:                        # 从 DAG depends_on 推导
    - target: block-relations-api    # 140号 depends_on 134号 → 134号的下游推论已生成此 action
  confidence: high                   # 下游推论文本明确（非推测性匹配）
  direction_check: consistent        # 与总方针§八方向一致
```

**依赖解析规则**（275号局部依赖原则）：
- 每个 action 只声明自己的直接依赖（从 DAG 的直接 depends_on 推导）
- 不需要声明间接依赖——DAG 由局部声明自然涌现
- 无 `depends_on` 的 actions 全部并行 spawn
- 有 `depends_on` 的 actions，ceremony_scan 检查前置 target 是否已完成（completion_check 通过）：
  - 全部前置已完成 → unblocked，生成工位
  - 存在未完成前置 → blocked，`blocked_by` 自动填写为未完成前置的 target 列表

### 3.6 proposed_actions 与 next_actions 的区分

| 字段 | 来源 | 信任等级 | ceremony_scan 行为 |
|------|------|---------|-------------------|
| `next_actions` | 编排者/工位写入 | 已确认 | 直接生成工位 |
| `proposed_actions` | 谱系下游推论推导 | 待确认 | 按四分法分类后处理 |

四分法应用于 proposed_actions：
- **定理类**（confidence: high + 下游推论文本明确无歧义）：自动提升为 next_actions，生成工位
- **行动类**（纯操作，无概念选择）：自动提升为 next_actions，生成工位
- **选择类**（confidence: medium/low + 多种合理方案）：保留为 proposed_actions，生成"推导审查"工位
- **语法记录类**：保留为 proposed_actions，生成"推导审查"工位

### 3.7 总方针的角色：方向校验（非推导来源）

总方针从旧设计中的"推导来源"降级为"方向校验器"：

1. 推导出的 proposed_actions 与总方针对应章节进行方向一致性检查
2. 一致 → `direction_check: consistent`
3. 不一致 → `direction_check: mismatch`，附带说明
4. 无对应章节 → `direction_check: uncovered`

方向不一致**不阻塞** proposed_action 的处理——它只是一个标记，供推导审查工位参考。因为谱系是概念发现的实际产出，总方针可能滞后于谱系的发展。

### 3.8 总方针与谱系的张力（编排者洞察记录）

编排者在审查初版设计时指出了核心张力：

> "总方针有一些是未来的战略方向，但是知识来源几乎全部来自谱系。"

这一洞察揭示了旧设计的根本错误——将总方针作为推导来源，混淆了两种不同性质的信息：

| 维度 | 总方针 | 谱系 |
|------|--------|------|
| 时间性 | 面向未来（战略目标，部分尚未发生） | 面向过去（已结算的概念发现） |
| 知识性质 | 方向指引（可能变化） | 概念基础（已固化为 DAG 节点） |
| 推导合法性 | 不能从中推导具体行动（未来的目标不是已知的知识） | 可以从中推导下一步（下游推论是已结算知识的逻辑延伸） |
| 在本机制中的角色 | 方向校验器：推导结果与战略方向是否一致 | 推导来源：下游推论直接涌现为 proposed_actions |

**张力的处理方式**：

1. 当谱系下游推论推导的 action 与总方针方向一致 → 正常流转（`direction_check: consistent`）
2. 当谱系下游推论推导的 action 与总方针方向不一致 → 标记为张力点（`direction_check: mismatch`），但**不阻塞**——因为谱系记录的是已发生的概念发现，总方针可能尚未更新以反映新发现
3. 当推导出的 action 在总方针中无对应章节 → `direction_check: uncovered`——可能是总方针的空白区域，也可能是谱系发展超出了总方针的预设范围

**不一致时的修正方向**：direction_mismatch 不意味着推导结果错误。它意味着总方针和谱系之间存在尚未调和的张力——这本身是有价值的信号，应当在推导审查工位中被明确记录，并可能触发总方针的更新提案。

### 3.9 推导工位的结构

推导阶段产出的工位有两种：

**类型一：直接执行工位**（定理类/行动类 proposed_action 提升后）
```json
{
  "priority": "P2",
  "name": "纲[区块拓扑]目[block-topology-eng]：traverse-bfs-marking",
  "status": "research_line:engineering",
  "source": "research_line_derivation",
  "description": "BFS生成树 + tree/critical边标记（来自140号下游推论#1）",
  "research_line": "block-topology-eng",
  "source_genealogy": "140",
  "source_downstream_index": 1,
  "confidence": "high"
}
```

**类型二：推导审查工位**（选择类/语法记录类）
```json
{
  "priority": "P3",
  "name": "推导审查：block-topology-eng 的3个 proposed_actions",
  "status": "derivation_review",
  "source": "research_line_derivation",
  "description": "审查从谱系下游推论推导的 proposed_actions，确认或修正",
  "review_targets": ["traverse-query-interface", "morse-terrain-nav", "concept-registry"]
}
```

## 四、gangmu.yaml Schema 变更

### 4.1 mu 级别新增字段

```yaml
mu:
- id: block-topology-eng
  name: 区块拓扑
  status: active
  opened_at: '301'
  zongfangzhen_section: "§八实现架构"  # 保留，用于方向校验
  related_genealogy: ["140", "134"]    # 新增：关联谱系节点 ID 列表
  next_actions: []                     # 已有字段，不变
  proposed_actions:                    # 新增字段
  - type: engineering
    target: block-relations-api
    description: 关系层 API——relations.jsonl 的结构化读写接口
    source_genealogy: "134"            # 来自哪条谱系
    source_downstream_index: 2         # 下游推论第几条
    depends_on: []                     # 从 DAG 推导的依赖
    confidence: high
    direction_check: consistent        # 与总方针方向校验结果
    derived_at: "2026-03-04T..."       # 推导时间戳
    completion_check:
      type: file_exists
      path: src/newchan/block_topology/relations.py
  - type: engineering
    target: traverse-bfs-marking
    description: BFS生成树 + tree/critical边标记
    source_genealogy: "140"
    source_downstream_index: 1
    depends_on:
      - target: block-relations-api    # 从 DAG 关系推导：140 depends_on 134
    confidence: high
    direction_check: consistent
    derived_at: "2026-03-04T..."
    completion_check:
      type: file_exists
      path: scripts/traverse_bfs.py
```

### 4.2 action 级别新增字段

在现有 action schema（next_actions 和 proposed_actions 共用）上新增：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `depends_on` | list | 否 | 直接依赖列表，每项含 `target` 和可选 `mu` |
| `source_genealogy` | string | proposed_actions 必填 | 来源谱系节点 ID |
| `source_downstream_index` | int | proposed_actions 必填 | 来源下游推论编号 |
| `confidence` | enum(high/medium/low) | proposed_actions 必填 | 推导置信度 |
| `direction_check` | enum(consistent/mismatch/uncovered) | proposed_actions 必填 | 与总方针方向校验结果 |
| `derived_at` | ISO timestamp | proposed_actions 必填 | 推导时间戳 |
| `completed_at` | ISO timestamp | 否 | 完成时间（gangmu_update.py 标记） |

### 4.3 depends_on 条目格式

```yaml
depends_on:
  - target: some-action-target         # 同目内依赖（默认当前目）
  - mu: other-mu-id                    # 跨目依赖
    target: other-action-target
```

## 五、ceremony_scan.py 修改方案

### 5.1 新增函数：`_derive_next_actions(root, rl_context, da_result)`

位置：在 `_scan_research_lines` 和 `downstream_audit` 之后，独立函数。

与旧设计的关键区别：**不读取总方针推导，而是消费 downstream_audit 的 unresolved 结果**。

伪代码：

```python
def _derive_next_actions(root, rl_context, da_result):
    """检测伪不动点，从谱系下游推论推导 proposed_actions。

    触发条件：active 目的 unblocked_actions == 0 且无已有 proposed_actions。
    推导来源：downstream_audit 的 unresolved 项 + DAG depends_on 关系。
    总方针角色：方向校验，不参与推导。

    返回 (derivation_context, derivation_workstations)。
    """
    if rl_context is None or da_result is None:
        return None, []

    gm_path = os.path.join(root, ".chanlun/gangmu.yaml")
    dag_path = os.path.join(root, ".chanlun/genealogy/dag.yaml")

    if not os.path.isfile(gm_path) or not os.path.isfile(dag_path):
        return None, []

    with open(gm_path, encoding="utf-8") as f:
        gangmu = yaml.safe_load(f)
    with open(dag_path, encoding="utf-8") as f:
        dag = yaml.safe_load(f)

    # Phase 1: 识别 stalled lines（伪不动点）
    stalled_lines = []
    for gang in gangmu.get("gang", []):
        for mu in gang.get("mu", []):
            if mu.get("status") != "active":
                continue
            mu_id = mu.get("id", "")
            if mu.get("proposed_actions"):
                continue

            active_info = next(
                (a for a in rl_context.get("active", []) if a["id"] == mu_id),
                None
            )
            if active_info is None:
                continue
            if active_info.get("unblocked_actions", 0) == 0:
                stalled_lines.append({
                    "mu_id": mu_id,
                    "mu_name": mu.get("name", mu_id),
                    "gang_id": gang.get("id", ""),
                    "gang_name": gang.get("name", ""),
                    "related_genealogy": mu.get("related_genealogy", []),
                    "zongfangzhen_section": mu.get("zongfangzhen_section", ""),
                    "completed_targets": [
                        a.get("target", "")
                        for a in mu.get("next_actions", [])
                        if isinstance(a, dict)
                    ],
                })

    if not stalled_lines:
        return {"stalled_count": 0}, []

    # Phase 2: 收集 unresolved 下游推论
    unresolved_items = [
        item for item in da_result.get("items", [])
        if item.get("status") == "unresolved"
    ]

    # Phase 3: 构建谱系 ID → 相关目的映射（正向+反向）
    genealogy_to_mu = {}  # {genealogy_id: [mu_id, ...]}
    for line in stalled_lines:
        for gid in line.get("related_genealogy", []):
            genealogy_to_mu.setdefault(str(gid), []).append(line["mu_id"])

    # Phase 4: 匹配 unresolved 下游推论 → stalled 目
    # 并为每个 stalled 目生成推导工位
    derivation_workstations = []
    for line in stalled_lines:
        matched_items = [
            item for item in unresolved_items
            if str(item.get("genealogy_id", "")) in
               [str(g) for g in line.get("related_genealogy", [])]
        ]
        derivation_workstations.append({
            "priority": "P2",
            "name": f"推导：{line['gang_name']}/{line['mu_name']}",
            "status": "derivation:pending",
            "source": "research_line_derivation",
            "description": (
                f"目[{line['mu_id']}]的 next_actions 已耗尽。"
                f"从谱系 DAG 下游推论推导下一阶段 actions。"
                f"关联谱系: {', '.join(str(g) for g in line.get('related_genealogy', [])) or '(待匹配)'}。"
                f"匹配到 {len(matched_items)} 条 unresolved 下游推论。"
                f"已完成 targets: {', '.join(line['completed_targets']) or '(无)'}。"
                f"推导结果写入 gangmu.yaml 的 proposed_actions 字段。"
                f"总方针{line['zongfangzhen_section']}仅用于方向校验。"
            ),
            "research_line": line["mu_id"],
            "gang_id": line["gang_id"],
        })

    context = {
        "stalled_count": len(stalled_lines),
        "stalled_lines": stalled_lines,
    }

    return context, derivation_workstations
```

### 5.2 新增函数：`_resolve_depends_on(root, gangmu)`

位置：在 `_scan_research_lines` 内部调用，替换当前简单的 `blocked_by` 检查。

伪代码：

```python
def _resolve_depends_on(root, gangmu):
    """解析所有 action 的 depends_on，返回每个 action 的阻塞状态。

    遍历所有 gang.mu.next_actions + proposed_actions，
    收集每个 target 的完成状态（completion_check），
    然后对有 depends_on 的 action 检查前置是否全部完成。

    返回 dict: {(mu_id, target): {"blocked": bool, "blocked_by": [target_str]}}
    """
    # Phase 1: 收集所有 action 的完成状态
    completion_map = {}  # (mu_id, target) -> bool
    for gang in gangmu.get("gang", []):
        for mu in gang.get("mu", []):
            mu_id = mu.get("id", "")
            for action_list_key in ("next_actions", "proposed_actions"):
                for action in mu.get(action_list_key, []) or []:
                    if not isinstance(action, dict):
                        continue
                    target = action.get("target", "")
                    if action.get("completed_at"):
                        completion_map[(mu_id, target)] = True
                    else:
                        cc = action.get("completion_check")
                        completion_map[(mu_id, target)] = _check_completion(root, cc)

    # Phase 2: 解析每个 action 的 depends_on
    resolved = {}
    for gang in gangmu.get("gang", []):
        for mu in gang.get("mu", []):
            mu_id = mu.get("id", "")
            for action_list_key in ("next_actions", "proposed_actions"):
                for action in mu.get(action_list_key, []) or []:
                    if not isinstance(action, dict):
                        continue
                    target = action.get("target", "")
                    deps = action.get("depends_on", [])
                    if not deps:
                        resolved[(mu_id, target)] = {"blocked": False, "blocked_by": []}
                        continue

                    blocked_by = []
                    for dep in deps:
                        dep_mu = dep.get("mu", mu_id)  # 默认同目
                        dep_target = dep.get("target", "")
                        if not completion_map.get((dep_mu, dep_target), False):
                            blocked_by.append(f"{dep_mu}/{dep_target}")

                    resolved[(mu_id, target)] = {
                        "blocked": len(blocked_by) > 0,
                        "blocked_by": blocked_by,
                    }

    return resolved
```

### 5.3 main() 中的集成点

在现有的 `# 2e. 研究线扫描` 和 downstream_audit 之后添加：

```python
    # 2f-new. 推导阶段：从谱系下游推论检测伪不动点，生成推导工位
    try:
        derivation_context, derivation_workstations = _derive_next_actions(
            root, rl_context, da_result
        )
        if derivation_context is not None:
            result["research_line_derivation"] = derivation_context
            workstations.extend(derivation_workstations)
    except Exception as exc:
        result["research_line_derivation_error"] = f"{type(exc).__name__}: {exc}"
```

### 5.4 `_scan_research_lines` 的 depends_on 集成

在现有的 `_scan_research_lines` 中，将当前的简单 `blocked_by` 字符串检查替换为 `depends_on` 感知逻辑。向后兼容：

```python
# 旧逻辑（保留为 fallback）
action_blocked_by = action.get("blocked_by")
is_blocked = action_blocked_by is not None and action_blocked_by != ""

# 新逻辑（depends_on 优先）
depends_on = action.get("depends_on", [])
if depends_on:
    # 使用 _resolve_depends_on 的结果
    resolution = resolved_map.get((mu_id, action.get("target", "")))
    if resolution and resolution["blocked"]:
        is_blocked = True
        action_blocked_by = "; ".join(resolution["blocked_by"])
```

## 六、推导工位的执行规范

推导工位被 spawn 后，执行以下流程：

1. 读取目标目的 `related_genealogy` 列表，定位关联谱系节点
2. 读取谱系 DAG (`dag.yaml`)，获取这些节点的 depends_on 关系和下游推论
3. 从 downstream_audit 结果中提取匹配的 unresolved 下游推论
4. 为每条 unresolved 下游推论生成 proposed_action：
   - `target`：从推论文本提炼的简短 id（kebab-case）
   - `description`：下游推论的原始文本
   - `source_genealogy`：来源谱系 ID
   - `source_downstream_index`：下游推论编号
   - `depends_on`：从 DAG depends_on 关系推导
   - `confidence`：基于推论文本清晰度评估
   - `direction_check`：与总方针方向校验结果
   - `completion_check`：根据 action 类型生成
5. 按四分法分类每个 proposed_action
6. 定理类/行动类：直接写入 `next_actions`
7. 选择类/语法记录类：写入 `proposed_actions`
8. commit gangmu.yaml 变更

推导工位的约束：
- 不执行推导出的 actions——只写入 gangmu.yaml
- 下一轮 rescan 时 ceremony_scan 会自然发现新的 next_actions 并生成工位
- 这保证了 ceremony 序列不变——推导是一个普通工位，rescan 是已有机制

## 七、认识论安全

### 7.1 来源标注（强制）

每个 proposed_action 必须携带 `source_genealogy` 和 `source_downstream_index`，精确到谱系节点和推论编号。
不接受"从谱系推导"这样的笼统声明。

对比旧设计：旧设计要求标注 `source_section`（总方针章节引用）。新设计的来源是谱系节点——可追溯性更强（谱系是已结算概念，总方针是高层方向）。

### 7.2 置信度分级

| confidence | 含义 | ceremony_scan 行为 |
|-----------|------|-------------------|
| high | 下游推论文本明确描述了具体步骤 | 自动提升为 next_actions |
| medium | 下游推论隐含但需推导 | 保留为 proposed_actions |
| low | 匹配链较长（关键词/邻居传播），可能有误匹配 | 保留为 proposed_actions |

### 7.3 防止推导膨胀

- 每个目每轮最多推导 5 个 proposed_actions
- 推导出的 actions 只覆盖当前 unresolved 下游推论的**最近** N 条（按谱系 ID 降序）
- 推导工位本身带有 `derived_at` 时间戳，可追溯
- 下游推论的 079 号语义保证：未被认领的推论超过 1 个 Ceremony 后降级为背景噪音——天然的膨胀刹车

### 7.4 与 downstream_audit 的协作（非竞争）

本机制与 downstream_audit 是互补的：
- downstream_audit 负责**扫描**所有谱系的下游推论执行状态（全量）
- 本机制负责**匹配**：将 unresolved 下游推论结构化为特定目的 proposed_actions
- downstream_audit 已经将 unresolved 项转化为通用工位（140号）——本机制进一步将匹配到目的项结构化为 gangmu.yaml 的 proposed_actions，使其进入研究线管理流

## 八、与现有机制的兼容性

### 8.1 ceremony 序列不变

ceremony.md 定义的步骤不修改。推导机制完全在 ceremony_scan.py 内部实现：
- scan 输出中新增 `research_line_derivation` 字段
- 推导工位和普通工位一样被 spawn
- 推导工位完成后 gangmu.yaml 更新，下一轮 rescan 自然发现新 actions

### 8.2 gangmu_update.py 兼容

gangmu_update.py 已经处理 `next_actions` 的 completion 标记。新增的 `proposed_actions`、`depends_on`、`source_genealogy` 等字段对它透明——它只操作已知字段。

### 8.3 downstream_audit 兼容

downstream_audit.py 不受影响。它继续扫描所有谱系的下游推论状态。本机制在它之后运行，消费它的结果。当 proposed_action 被执行并解决了对应的下游推论时，downstream_audit 在下一轮扫描中自然将其标记为 resolved。

### 8.4 不动点判断变化

当前不动点 = workstations 为空。推导机制加入后：
- 如果存在 stalled lines 且谱系 DAG 有匹配的 unresolved 下游推论 → 生成推导工位 → workstations 非空 → 不会到达不动点
- 真不动点 = 所有目 closed/blocked 或无匹配的 unresolved 下游推论 → 无推导工位 → workstations 为空

这意味着伪不动点被自动消除——只有真不动点才会终止 ceremony。

## 九、实现计划

### 步骤 1：gangmu.yaml schema 扩展
- 文件：`.chanlun/gangmu.yaml`
- 改动：为现有 active 目添加 `related_genealogy` 列表和 `proposed_actions: []` 字段
- 不修改已有 `next_actions` 的格式

### 步骤 2：`_resolve_depends_on` 函数
- 文件：`scripts/ceremony_scan.py`
- 改动：新增函数，实现 depends_on 解析
- 在 `_scan_research_lines` 中集成

### 步骤 3：`_derive_next_actions` 函数
- 文件：`scripts/ceremony_scan.py`
- 改动：新增函数，消费 downstream_audit 结果 + DAG 关系，推导 proposed_actions
- 在 `main()` 的 downstream_audit 之后集成

### 步骤 4：`_scan_research_lines` 的 depends_on 兼容
- 文件：`scripts/ceremony_scan.py`
- 改动：在 action blocked 判断中增加 depends_on 路径
- 保持旧 `blocked_by` 字符串作为 fallback

### 步骤 5：推导工位的 prompt 模板
- 文件：`scripts/ceremony_scan.py` 或独立模板文件
- 改动：定义推导工位被 spawn 时的标准 prompt
- 包含：读取关联谱系下游推论→匹配 unresolved 项→生成 proposed_actions→四分法分类→方向校验→写入

### 步骤 6：测试
- 文件：`tests/test_ceremony_scan.py`（或新建）
- 测试项：
  - 伪不动点检测：active 目 + next_actions 为空 + 谱系有 unresolved → 生成推导工位
  - 真不动点：所有目 closed 或无匹配 unresolved → 不生成推导工位
  - depends_on 解析：同目依赖 + 跨目依赖 + 循环依赖检测
  - proposed_actions 不触发重复推导
  - 谱系→目匹配逻辑的四种匹配方式
  - 方向校验标记正确性
  - 向后兼容：旧格式 gangmu.yaml（无 depends_on/proposed_actions/related_genealogy）正常工作

### 依赖关系
```
步骤1 ──→ 步骤2 ──→ 步骤4
              │
              └──→ 步骤3 ──→ 步骤5
                                │
步骤1 ─────────────────────→ 步骤6（全部完成后）
```

步骤2 和 步骤3 可并行（步骤2 只被步骤4 消费，步骤3 独立）。
步骤6 依赖步骤4 和步骤5 都完成。
