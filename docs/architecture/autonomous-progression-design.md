# 自主推进机制设计（Autonomous Progression）

## 一、问题陈述

当前 ceremony 到达不动点的条件是"无新工位产出"。但不动点有两种性质截然不同的情况：

1. **真不动点**：所有研究线的目标已完成，无新工作——系统结算态
2. **伪不动点**：某个 active 目的 `next_actions` 耗尽（全部完成或为空列表），但该目对应的总方针章节仍有未展开的工作——这不是结算，是推进断裂

当前 ceremony_scan.py 的 `_scan_research_lines` 只能消费已经写入 gangmu.yaml 的 `next_actions`，不能推导新的 actions。伪不动点需要编排者手动注入新 actions 才能打破。

**具体例证**：
- `block-topology-eng`：status=active，但 `next_actions: []`——对应总方针§八有大量未展开工作
- `operational-methodology`：三个 next_actions 中两个 unblocked——但完成后不会自动从§三+§八.七阶段C推导下一步

## 二、设计目标

1. ceremony 能自主检测伪不动点并从总方针推导新 actions
2. 推导出的 actions 带依赖关系（非线性 DAG，不是串行队列）
3. 推导结果区分"可自主执行"和"需确认"两类
4. 兼容现有 ceremony 序列（不修改 ceremony.md 的步骤定义）

## 三、架构设计

### 3.1 新阶段：research_line_derivation

在 ceremony_scan.py 的 `_scan_research_lines` 返回后增加一个推导阶段。不修改 `_scan_research_lines` 本身——它保持消费已有 actions 的职责，推导逻辑独立。

```
现有流程:
  _scan_research_lines(root)
    → 扫描 gangmu.yaml
    → 输出 workstations（已有 unblocked actions）
    → 输出 proposed_transitions（closed/blocked 状态建议）

新增流程:
  _derive_next_actions(root, rl_context)
    → 检测"可推进但无 actions"的 active 目
    → 读取对应总方针章节
    → 推导 proposed_actions
    → 写入 gangmu.yaml 的 proposed_actions 字段
    → 输出 derivation_workstations
```

### 3.2 触发条件

推导阶段仅在以下条件同时满足时触发：

1. 存在 active 目且 `unblocked_actions == 0`（所有 next_actions 已完成或全部被阻塞或列表为空）
2. 该目没有 `proposed_actions`（避免重复推导）
3. 该目的 `zongfangzhen_section` 非空（有总方针锚点可查）

不触发的情况：
- 目 status == closed/blocked：已终止或已阻塞，不需推导
- 目有 unblocked_actions > 0：还有可执行工作，不需推导
- 目已有 proposed_actions：上一轮推导结果尚未处理

### 3.3 推导数据流

```
gangmu.yaml                 总方针-v8.md
    │                            │
    ▼                            ▼
 mu.zongfangzhen_section → 定位章节内容
    │                            │
    ▼                            ▼
 mu.next_actions（已完成）  章节中的未覆盖目标
    │                            │
    └─────────┬──────────────────┘
              ▼
     差集 = 未展开的工作
              │
              ▼
     proposed_actions[]
     （写入 gangmu.yaml）
              │
              ▼
     derivation_workstations[]
     （追加到 scan 输出）
```

### 3.4 依赖关系模型（非线性 DAG）

每个 action（无论 next_actions 还是 proposed_actions）可以声明依赖：

```yaml
- type: engineering
  target: traverse-query-interface
  description: 穿越查询接口——traverse.py 的 MCP 暴露
  depends_on:
    - target: traverse-bfs-marking  # 同目内依赖
    - target: encounter-record-mechanism  # 同目内依赖
    - mu: block-topology-eng  # 跨目依赖
      target: block-relations-api
  confidence: high
  source_section: "§八.五组件一"
  completion_check:
    type: file_exists
    path: scripts/traverse.py
```

**依赖解析规则**（275号局部依赖原则）：
- 每个 action 只声明自己的直接依赖（`depends_on`）
- 不需要声明间接依赖——DAG 由局部声明自然涌现
- 无 `depends_on` 的 actions 全部并行 spawn
- 有 `depends_on` 的 actions，ceremony_scan 检查前置 target 是否已完成（completion_check 通过）：
  - 全部前置已完成 → unblocked，生成工位
  - 存在未完成前置 → blocked，`blocked_by` 自动填写为未完成前置的 target 列表

**跨目依赖**：`depends_on` 条目加 `mu` 字段指定依赖的目 id。ceremony_scan 在全局扫描时解析跨目依赖——先收集所有目的 action completion 状态，再对每个 action 解析 depends_on。

### 3.5 proposed_actions 与 next_actions 的区分

| 字段 | 来源 | 信任等级 | ceremony_scan 行为 |
|------|------|---------|-------------------|
| `next_actions` | 编排者/工位写入 | 已确认 | 直接生成工位 |
| `proposed_actions` | 推导阶段自动生成 | 待确认 | 按四分法分类后处理 |

四分法应用于 proposed_actions：
- **定理类**（confidence: high + 总方针可直接推导）：自动提升为 next_actions，生成工位
- **行动类**（纯操作，无概念选择）：自动提升为 next_actions，生成工位
- **选择类**（confidence: medium/low + 多种合理方案）：保留为 proposed_actions，生成"推导审查"工位
- **语法记录类**：保留为 proposed_actions，生成"推导审查"工位

### 3.6 推导工位的结构

推导阶段产出的工位有两种：

**类型一：直接执行工位**（定理类/行动类 proposed_action 提升后）
```json
{
  "priority": "P2",
  "name": "纲[区块拓扑]目[block-topology-eng]：traverse-bfs-marking",
  "status": "research_line:engineering",
  "source": "research_line_derivation",
  "description": "BFS生成树 + tree/critical边标记",
  "research_line": "block-topology-eng",
  "derived_from": "§八.五组件三",
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
  "description": "审查从§八推导的 proposed_actions，确认或修正",
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
  zongfangzhen_section: "§八实现架构"  # 已有字段，用于推导锚点
  next_actions: []                     # 已有字段，不变
  proposed_actions:                    # 新增字段
  - type: engineering
    target: block-relations-api
    description: 关系层 API——relations.jsonl 的结构化读写接口
    depends_on: []
    confidence: high
    source_section: "§八.二"
    derived_at: "2026-03-04T..."       # 推导时间戳
    completion_check:
      type: file_exists
      path: src/newchan/block_topology/relations.py
  - type: engineering
    target: traverse-bfs-marking
    description: BFS生成树 + tree/critical边标记（穿越基础设施组件三）
    depends_on:
      - target: block-relations-api
    confidence: high
    source_section: "§八.五组件三"
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
| `confidence` | enum(high/medium/low) | proposed_actions 必填 | 推导置信度 |
| `source_section` | string | proposed_actions 必填 | 总方针来源章节引用 |
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

### 5.1 新增函数：`_derive_next_actions(root, rl_context)`

位置：在 `_scan_research_lines` 之后，独立函数。

伪代码：

```python
def _derive_next_actions(root, rl_context):
    """检测伪不动点，从总方针推导 proposed_actions。

    触发条件：active 目的 unblocked_actions == 0 且无已有 proposed_actions。
    推导逻辑由 LLM 工位执行（本函数只做检测和结构化输出）。

    返回 (derivation_context, derivation_workstations)。
    """
    if rl_context is None:
        return None, []

    gm_path = os.path.join(root, ".chanlun/gangmu.yaml")
    zf_path = os.path.join(root, "docs/architecture/总方针-v8.md")

    if not os.path.isfile(gm_path) or not os.path.isfile(zf_path):
        return None, []

    with open(gm_path, encoding="utf-8") as f:
        gangmu = yaml.safe_load(f)

    stalled_lines = []  # 伪不动点目列表

    for gang in gangmu.get("gang", []):
        for mu in gang.get("mu", []):
            if mu.get("status") != "active":
                continue
            mu_id = mu.get("id", "")

            # 检查是否已有 proposed_actions
            if mu.get("proposed_actions"):
                continue

            # 检查 unblocked_actions
            # 从 rl_context 中查找该目的 unblocked_actions
            active_info = next(
                (a for a in rl_context.get("active", []) if a["id"] == mu_id),
                None
            )
            if active_info is None:
                continue

            # 伪不动点条件：unblocked_actions == 0
            if active_info.get("unblocked_actions", 0) == 0:
                stalled_lines.append({
                    "mu_id": mu_id,
                    "mu_name": mu.get("name", mu_id),
                    "gang_id": gang.get("id", ""),
                    "gang_name": gang.get("name", ""),
                    "zongfangzhen_section": mu.get("zongfangzhen_section", ""),
                    "completed_targets": [
                        a.get("target", "")
                        for a in mu.get("next_actions", [])
                        if isinstance(a, dict)
                    ],
                })

    if not stalled_lines:
        return {"stalled_count": 0}, []

    # 为每个 stalled 目生成"推导工位"
    derivation_workstations = []
    for line in stalled_lines:
        derivation_workstations.append({
            "priority": "P2",
            "name": f"推导：{line['gang_name']}/{line['mu_name']}",
            "status": "derivation:pending",
            "source": "research_line_derivation",
            "description": (
                f"目[{line['mu_id']}]的 next_actions 已耗尽，"
                f"从总方针{line['zongfangzhen_section']}推导下一阶段 actions。"
                f"已完成 targets: {', '.join(line['completed_targets']) or '(无)'}。"
                f"推导结果写入 gangmu.yaml 的 proposed_actions 字段。"
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

在现有的 `# 2e. 研究线扫描` 之后添加：

```python
    # 2f-new. 推导阶段：检测伪不动点，生成推导工位
    try:
        derivation_context, derivation_workstations = _derive_next_actions(root, rl_context)
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

1. 读取目标目的 `zongfangzhen_section` 定位总方针章节
2. 读取总方针对应内容
3. 对比该目已完成的 targets，识别总方针中未覆盖的目标
4. 为每个识别出的目标生成 proposed_action：
   - `target`：简短 id（kebab-case）
   - `description`：从总方针提取的目标描述
   - `depends_on`：从总方针中的依赖描述推导
   - `confidence`：基于推导清晰度评估
   - `source_section`：精确引用（如"§八.五组件一"）
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

每个 proposed_action 必须携带 `source_section`，精确到总方针的子节点。
不接受"从总方针推导"这样的笼统声明。

### 7.2 置信度分级

| confidence | 含义 | ceremony_scan 行为 |
|-----------|------|-------------------|
| high | 总方针明确描述了这个步骤 | 自动提升为 next_actions |
| medium | 总方针隐含但未明确描述 | 保留为 proposed_actions |
| low | 推导链较长，可能有多种解读 | 保留为 proposed_actions |

### 7.3 防止推导膨胀

- 每个目每轮最多推导 5 个 proposed_actions
- 推导出的 actions 只覆盖"下一步"，不覆盖整个总方针章节的全部目标
- 推导工位本身带有 `derived_at` 时间戳，可追溯

## 八、与现有机制的兼容性

### 8.1 ceremony 序列不变

ceremony.md 定义的步骤不修改。推导机制完全在 ceremony_scan.py 内部实现：
- scan 输出中新增 `research_line_derivation` 字段
- 推导工位和普通工位一样被 spawn
- 推导工位完成后 gangmu.yaml 更新，下一轮 rescan 自然发现新 actions

### 8.2 gangmu_update.py 兼容

gangmu_update.py 已经处理 `next_actions` 的 completion 标记。新增的 `proposed_actions` 和 `depends_on` 字段对它透明——它只操作已知字段。

### 8.3 不动点判断变化

当前不动点 = workstations 为空。推导机制加入后：
- 如果存在 stalled lines → 生成推导工位 → workstations 非空 → 不会到达不动点
- 真不动点 = 所有目 closed/blocked 且无 stalled lines → 无推导工位 → workstations 为空

这意味着伪不动点被自动消除——只有真不动点才会终止 ceremony。

## 九、实现计划

### 步骤 1：gangmu.yaml schema 扩展
- 文件：`.chanlun/gangmu.yaml`
- 改动：为现有 active 目添加 `proposed_actions: []` 字段（初始为空列表）
- 不修改已有 `next_actions` 的格式

### 步骤 2：`_resolve_depends_on` 函数
- 文件：`scripts/ceremony_scan.py`
- 改动：新增函数，实现 depends_on 解析
- 在 `_scan_research_lines` 中集成

### 步骤 3：`_derive_next_actions` 函数
- 文件：`scripts/ceremony_scan.py`
- 改动：新增函数，实现伪不动点检测和推导工位生成
- 在 `main()` 的 2e 之后集成

### 步骤 4：`_scan_research_lines` 的 depends_on 兼容
- 文件：`scripts/ceremony_scan.py`
- 改动：在 action blocked 判断中增加 depends_on 路径
- 保持旧 `blocked_by` 字符串作为 fallback

### 步骤 5：推导工位的 prompt 模板
- 文件：`scripts/ceremony_scan.py` 或独立模板文件
- 改动：定义推导工位被 spawn 时的标准 prompt
- 包含：读取总方针章节→对比已完成 targets→生成 proposed_actions→四分法分类→写入

### 步骤 6：测试
- 文件：`tests/test_ceremony_scan.py`（或新建）
- 测试项：
  - 伪不动点检测：active 目 + next_actions 为空 → 生成推导工位
  - 真不动点：所有目 closed → 不生成推导工位
  - depends_on 解析：同目依赖 + 跨目依赖 + 循环依赖检测
  - proposed_actions 不触发重复推导
  - 向后兼容：旧格式 gangmu.yaml（无 depends_on/proposed_actions）正常工作

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
