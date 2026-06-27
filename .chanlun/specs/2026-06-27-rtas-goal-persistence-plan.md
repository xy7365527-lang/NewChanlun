# RTAS-Goal 持久化架构改造（D′）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐 task 执行。步骤用 `- [ ]` 跟踪。
> **上游 spec**: `.chanlun/specs/2026-06-27-rtas-goal-persistence-design.md`（已批准）。回退点: git tag `pre-rtas-goal-arch`。

**Goal:** 把 goal 从命令语义提升为 RTAS 蜂群的动态根——goal 事件源契约 + 确定性 reducer，state 降为 projection，Lead 降为 bootstrap/IO actuator。

**Architecture:** 事件源（`.chanlun/goals/events.jsonl`）→ 纯函数 reducer（`goal_reducer.py`）→ ready workstations。声明放 `dispatch-dag.yaml`，执行放 `goal_reducer.py`，ceremony_scan 降 seed bootloader。波1 不碰 meta-lead 基因组；波2 改 meta-lead（020 已批）。

**Tech Stack:** Python 3（reducer + schema 校验，纯函数 + pytest）、YAML（契约/声明）、Markdown（命令/角色定义）。

---

## File Structure（决策锁定）

| 文件 | 责任 | 波 |
|------|------|----|
| `.chanlun/goals/SCHEMA.md` | goal 契约 + 事件 schema 文档（唯一真相源） | 1 |
| `.chanlun/goals/current.yaml` | 当前运行 goal 指针（reducer 输出的 projection） | 1 |
| `.chanlun/goals/events.jsonl` | append-only 事件源 | 1 |
| `.chanlun/goals/archive/.gitkeep` | 闭合 goal 归档目录 | 1 |
| `scripts/goal_reducer.py` | 纯函数 reducer（events+facts→ready workstations） | 1 |
| `scripts/tests/test_goal_reducer.py` | reducer 单测（确定性保证） | 1 |
| `.chanlun/dispatch-dag.yaml` | 新增 `goal_reducer:` 声明段（what） | 1 |
| `scripts/ceremony_scan.py` | 改：读 current goal（roadmap 之前）；降 seed bootloader | 1 |
| `.claude/commands/goal.md` | 改：写 events + 调 reducer | 1 |
| `.claude/agents/meta-lead.md` | 改：Lead 降 bootstrap/IO actuator + 修裁定矛盾 | 2 |
| `.claude/commands/ceremony.md` | 改：恢复先读 goal | 2 |

---

# 波 1：不碰 meta-lead 基因组（降 020 风险）

## Task 1：goal 契约 + 事件 schema

**Files:**
- Create: `.chanlun/goals/SCHEMA.md`
- Create: `.chanlun/goals/archive/.gitkeep`

- [ ] **Step 1: 写 SCHEMA.md（事件源 + 契约定义）**

```markdown
# Goal 契约与事件 Schema（D′ 唯一真相源）

## events.jsonl（append-only 事件源）
每行一个 JSON 事件。事件类型：

| event | 必填字段 | 语义 |
|-------|---------|------|
| GOAL_SET | goal_id, description, acceptance[], base_head, ts | 设定 goal（acceptance 每项必须 falsifiable=true） |
| DECOMPOSE | goal_id, sub_goals[]（{id, desc, blocked_by[]}）, ts | 分解为 sub-goal 树（containment 树 + blocked_by 执行 DAG） |
| EVIDENCE | sub_goal_id, artifact, ts | 工位产出证据 |
| CHECK_PASS | sub_goal_id, check, ts | 某验收项通过 |
| BLOCKED | sub_goal_id, blocker, ts | 阻塞（定义冲突/缺数据/需编排者裁决） |
| SUPERSEDE | old_goal_id, new_goal_id, ts | goal 被替代 |
| CLOSED | goal_id, ts | goal 闭合（所有 acceptance CHECK_PASS） |

## current.yaml（reducer 输出的 projection，非手写真相源）
```yaml
goal_id: <id>
description: <一句话>
acceptance:
  - check: <可证伪验收项>
    falsifiable: true
    passed: <bool>
base_head: <git sha at GOAL_SET>
status: active | blocked | closed
ready_workstations: [<sub_goal_id>...]
```

## 分层防污染（575）
goal events ≠ genealogy。只有 goal 定义/验收/分解原则**语义变更**才升格 genealogy（用 goal_event_id 反向引用）。
```

- [ ] **Step 2: 验证 schema 文件存在**

Run: `ls .chanlun/goals/SCHEMA.md .chanlun/goals/archive/.gitkeep`
Expected: 两文件列出

- [ ] **Step 3: Commit**

```bash
git add .chanlun/goals/SCHEMA.md .chanlun/goals/archive/.gitkeep
git commit -m "feat(goal): D′ 波1 — goal 契约+事件 schema（SCHEMA.md 唯一真相源）"
```

---

## Task 2：goal_reducer.py 纯函数（TDD 核心）

**Files:**
- Create: `scripts/goal_reducer.py`
- Test: `scripts/tests/test_goal_reducer.py`

- [ ] **Step 1: 写失败测试（确定性 + 核心 reduce 行为）**

```python
# scripts/tests/test_goal_reducer.py
from scripts.goal_reducer import reduce_goal

def _facts(head="abc123"):
    return {"git_head": head, "genealogy_pending": [], "genealogy_settled": [], "roadmap": []}

def test_empty_events_no_goal():
    out = reduce_goal([], _facts())
    assert out["current_goal"] is None
    assert out["ready_workstations"] == []

def test_goal_set_rebuilds_current():
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "形式化",
               "acceptance": [{"check": "lake build 绿", "falsifiable": True}],
               "base_head": "abc123", "ts": "2026-06-27T00:00:00Z"}]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g1"
    assert out["current_goal"]["status"] == "active"

def test_decompose_ready_excludes_blocked():
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "DECOMPOSE", "goal_id": "g1", "ts": "t1",
         "sub_goals": [{"id": "g1.1", "desc": "a", "blocked_by": []},
                       {"id": "g1.2", "desc": "b", "blocked_by": ["g1.1"]}]},
    ]
    out = reduce_goal(events, _facts())
    assert "g1.1" in out["ready_workstations"]
    assert "g1.2" not in out["ready_workstations"]  # blocked by g1.1

def test_check_pass_unblocks_dependent():
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "DECOMPOSE", "goal_id": "g1", "ts": "t1",
         "sub_goals": [{"id": "g1.1", "desc": "a", "blocked_by": []},
                       {"id": "g1.2", "desc": "b", "blocked_by": ["g1.1"]}]},
        {"event": "CHECK_PASS", "sub_goal_id": "g1.1", "check": "done", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert "g1.2" in out["ready_workstations"]  # g1.1 passed → g1.2 unblocked

def test_all_acceptance_pass_closes_goal():
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "c1", "ts": "t1"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["status"] == "closed"

def test_supersede_invalidates_old():
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "SUPERSEDE", "old_goal_id": "g1", "new_goal_id": "g2", "ts": "t1"},
        {"event": "GOAL_SET", "goal_id": "g2", "description": "y",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g2"

def test_determinism_same_input_same_output():
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "x",
               "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"}]
    assert reduce_goal(events, _facts()) == reduce_goal(events, _facts())

def test_goal_set_rejects_non_falsifiable_acceptance():
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "x",
               "acceptance": [{"check": "差不多", "falsifiable": False}], "base_head": "abc123", "ts": "t0"}]
    import pytest
    with pytest.raises(ValueError, match="falsifiable"):
        reduce_goal(events, _facts())
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd /Users/silencehan/Projects/NewChanlun && python -m pytest scripts/tests/test_goal_reducer.py -v`
Expected: FAIL（ModuleNotFoundError: goal_reducer）

- [ ] **Step 3: 写 goal_reducer.py 实现**

```python
# scripts/goal_reducer.py
"""D′ goal reducer：纯函数 (events + facts) → ready workstations projection。
确定性：同输入同输出（无 IO、无时间、无随机）。这是 RTAS 蜂群递归展开的确定性规则。"""

def reduce_goal(events, facts):
    """events: list[dict]（events.jsonl 解析）；facts: {git_head, genealogy_pending, genealogy_settled, roadmap}。
    返回 {current_goal: dict|None, ready_workstations: list[str], blocked: list[dict], terminated: bool}。"""
    # 1. 重建 current goal：最后一个未被 SUPERSEDE/CLOSED 的 GOAL_SET
    superseded = {e["old_goal_id"] for e in events if e["event"] == "SUPERSEDE"}
    closed = {e["goal_id"] for e in events if e["event"] == "CLOSED"}
    goal = None
    for e in events:
        if e["event"] == "GOAL_SET" and e["goal_id"] not in superseded:
            for acc in e.get("acceptance", []):
                if not acc.get("falsifiable", False):
                    raise ValueError(f"acceptance 项必须 falsifiable: {acc.get('check')}")
            goal = {"goal_id": e["goal_id"], "description": e["description"],
                    "acceptance": [dict(a, passed=False) for a in e["acceptance"]],
                    "base_head": e["base_head"], "status": "active"}
    if goal is None:
        return {"current_goal": None, "ready_workstations": [], "blocked": [], "terminated": False}

    gid = goal["goal_id"]
    # 2. 重建 sub-goal 树（DECOMPOSE）
    subs = {}
    for e in events:
        if e["event"] == "DECOMPOSE" and e["goal_id"] == gid:
            for sg in e["sub_goals"]:
                subs[sg["id"]] = {"id": sg["id"], "desc": sg["desc"],
                                  "blocked_by": list(sg.get("blocked_by", [])), "passed": False, "blocker": None}
    # 3. 应用 CHECK_PASS / BLOCKED
    passed_checks = set()
    for e in events:
        if e["event"] == "CHECK_PASS":
            sid = e["sub_goal_id"]
            passed_checks.add((sid, e.get("check")))
            if sid in subs:
                subs[sid]["passed"] = True
        elif e["event"] == "BLOCKED" and e["sub_goal_id"] in subs:
            subs[e["sub_goal_id"]]["blocker"] = e["blocker"]
    # 4. ready = 未 passed + 未 blocker + 所有 blocked_by 已 passed
    ready = [s["id"] for s in subs.values()
             if not s["passed"] and s["blocker"] is None
             and all(subs.get(b, {}).get("passed", False) for b in s["blocked_by"])]
    blocked = [{"id": s["id"], "blocker": s["blocker"]} for s in subs.values() if s["blocker"]]
    # 5. goal 验收：所有 acceptance CHECK_PASS → closed
    for acc in goal["acceptance"]:
        if (gid, acc["check"]) in passed_checks or any(c == acc["check"] for _, c in passed_checks):
            acc["passed"] = True
    terminated = bool(goal["acceptance"]) and all(a["passed"] for a in goal["acceptance"])
    if terminated or gid in closed:
        goal["status"] = "closed"
    elif blocked and not ready:
        goal["status"] = "blocked"
    return {"current_goal": goal, "ready_workstations": sorted(ready), "blocked": blocked, "terminated": terminated}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cd /Users/silencehan/Projects/NewChanlun && python -m pytest scripts/tests/test_goal_reducer.py -v`
Expected: PASS（8 passed）

- [ ] **Step 5: Commit**

```bash
git add scripts/goal_reducer.py scripts/tests/test_goal_reducer.py
git commit -m "feat(goal): D′ 波1 — goal_reducer.py 纯函数 reducer（8 测试绿，确定性保证）"
```

---

## Task 3：dispatch-dag.yaml goal_reducer 声明段

**Files:**
- Modify: `.chanlun/dispatch-dag.yaml`（在元编排根节点段后追加）

- [ ] **Step 1: 追加 goal_reducer 声明段**

```yaml
# ───────── goal_reducer（D′ RTAS-goal 动态根，619/033 声明式 dispatch 延伸）─────────
goal_reducer:
  description: >
    goal 是蜂群运行时动态根（CLAUDE.md genome 是创世静态根）。
    蜂群递归展开 = goal reducer 的确定性运行（非 Lead 认知）。
  spec_what:  # 声明：什么是 ready
    ready_condition: "sub_goal 未 passed ∧ 无 blocker ∧ 所有 blocked_by 已 passed"
    terminate_condition: "所有 acceptance 项 CHECK_PASS（acceptance 必须 falsifiable）"
    block_condition: "定义冲突 / 缺外部数据 / 需编排者裁决 → BLOCKED 事件 + escalate"
  impl_how: "scripts/goal_reducer.py::reduce_goal（纯函数，确定性）"
  event_source: ".chanlun/goals/events.jsonl"
  projection: ".chanlun/goals/current.yaml"
  # state 降 projection：session/interrupt 仅 base_head 匹配时作 hint（非真相源）
  truth_sources: ["goal events", "git HEAD", "genealogy"]
```

- [ ] **Step 2: 校验 YAML 合法**

Run: `cd /Users/silencehan/Projects/NewChanlun && python -c "import yaml; yaml.safe_load(open('.chanlun/dispatch-dag.yaml'))" && echo OK`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add .chanlun/dispatch-dag.yaml
git commit -m "feat(goal): D′ 波1 — dispatch-dag goal_reducer 声明段（what/how 分离）"
```

---

## Task 4：ceremony_scan.py 读 goal（降 seed bootloader）

**Files:**
- Modify: `scripts/ceremony_scan.py`（在 roadmap 读取之前插入 goal 读取）

- [ ] **Step 1: 写测试（ceremony_scan 输出含 current_goal）**

```python
# scripts/tests/test_ceremony_scan_goal.py
import json, subprocess, os
def test_scan_reads_current_goal(tmp_path):
    # 若 .chanlun/goals/events.jsonl 有 GOAL_SET，scan 输出应含 current_goal
    root = "/Users/silencehan/Projects/NewChanlun"
    out = subprocess.run(["python", "scripts/ceremony_scan.py"], cwd=root, capture_output=True, text=True)
    data = json.loads(out.stdout)
    assert "current_goal" in data  # 新增字段（无 goal 时为 None）
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd /Users/silencehan/Projects/NewChanlun && python -m pytest scripts/tests/test_ceremony_scan_goal.py -v`
Expected: FAIL（KeyError: current_goal）

- [ ] **Step 3: 在 ceremony_scan.py 顶部 import + 主输出加 current_goal**

在 ceremony_scan.py 的结果 dict 构建处（roadmap_tasks 之前）插入：

```python
# D′：读 goal events → reduce → current_goal（roadmap 之前，goal 是当前交易性承诺，roadmap 是 backlog）
def _load_current_goal():
    import json, os
    from goal_reducer import reduce_goal
    ev_path = ".chanlun/goals/events.jsonl"
    if not os.path.exists(ev_path):
        return None
    events = [json.loads(l) for l in open(ev_path) if l.strip()]
    head = os.popen("git rev-parse HEAD").read().strip()
    facts = {"git_head": head, "genealogy_pending": [], "genealogy_settled": [], "roadmap": []}
    return reduce_goal(events, facts)

# 结果 dict 加：
result["current_goal"] = _load_current_goal()
```
（注：`from goal_reducer import` 需 `sys.path.insert(0, os.path.dirname(__file__))` 已在脚本顶部）

- [ ] **Step 4: 跑测试 + 全量 ceremony_scan 确认无回归**

Run: `cd /Users/silencehan/Projects/NewChanlun && python -m pytest scripts/tests/test_ceremony_scan_goal.py -v && python scripts/ceremony_scan.py > /dev/null && echo SCAN_OK`
Expected: PASS + SCAN_OK

- [ ] **Step 5: Commit**

```bash
git add scripts/ceremony_scan.py scripts/tests/test_ceremony_scan_goal.py
git commit -m "feat(goal): D′ 波1 — ceremony_scan 读 current_goal（roadmap 之前，降 seed bootloader）"
```

---

## Task 5：/goal 命令改写契约

**Files:**
- Modify: `.claude/commands/goal.md`（增"写 events + 调 reducer"协议）

- [ ] **Step 1: 在 goal.md 的「运行协议」前插入「契约写入」段**

```markdown
## 契约写入（D′）

`/goal <目标>` 设定时：
1. 校验目标可收敛 + 验收可证伪（每个 acceptance 项 falsifiable=true，否则拒绝——lesson 0011 锋利问题②）
2. append GOAL_SET 事件到 `.chanlun/goals/events.jsonl`（含 goal_id, description, acceptance[], base_head=当前 git HEAD, ts）
3. 调 `python scripts/goal_reducer.py`（经 ceremony_scan）→ 输出 current.yaml projection + ready workstations
4. 进入运行协议循环（下方）

恢复时（无参 /goal 或 /ceremony）：reduce events → 若有 active goal 继续；无则从 roadmap/中断点推导候选 GOAL_SET。
**state 降 projection**：session/interrupt 仅 base_head 匹配时作 hint，不匹配则 reduce 重算（消除状态过时）。
```

- [ ] **Step 2: 验证 goal.md 含契约段**

Run: `grep -c "契约写入" .chanlun/commands/goal.md 2>/dev/null || grep -c "契约写入" .claude/commands/goal.md`
Expected: 1

- [ ] **Step 3: Commit**

```bash
git add .claude/commands/goal.md
git commit -m "feat(goal): D′ 波1 — /goal 改写契约（GOAL_SET 事件 + reduce，验收强制 falsifiable）"
```

---

## Task 6：session/interrupt 加 goal_id/base_head/derived

**Files:**
- Modify: `.chanlun/.interrupt-point.md`（头部加元数据块）

- [ ] **Step 1: 在 .interrupt-point.md 头部加元数据**

```markdown
<!-- D′ 元数据：恢复时校验 -->
goal_id: <当前 goal_id 或 none>
base_head: <写入时 git HEAD>
derived: true   # 此快照是 projection/cache，非真相源；base_head 不匹配即降级为参考
---
```

- [ ] **Step 2: 验证元数据存在**

Run: `head -5 .chanlun/.interrupt-point.md | grep -c "base_head"`
Expected: 1

- [ ] **Step 3: Commit**

```bash
git add .chanlun/.interrupt-point.md
git commit -m "feat(goal): D′ 波1 — interrupt-point 加 goal_id/base_head/derived（state 降 cache）"
```

---

# 波 2：meta-lead 基因组改（020 已批，回退点 pre-rtas-goal-arch）

## Task 7：meta-lead.md 降 bootstrap/IO actuator + 修裁定矛盾

**Files:**
- Modify: `.claude/agents/meta-lead.md`

- [ ] **Step 1: 改开头（line 10）**

旧：`你是蜂群的中断路由器，不是调度器。`
新：
```markdown
你是 RTAS 蜂群的 bootstrap/IO actuator，不是蜂群主体，不是 goal reducer。
你只是蜂群的一个特殊 bootstrap 功能 + 编排者对话渠道——这就是为什么你不做实质认知工作。
认知在蜂群递归展开（工位）+ 异质质询 + 编排者，不在你。
```

- [ ] **Step 2: 改「开端」段（line 102-116）为 goal 驱动**

```markdown
## 开端（D′：goal 驱动）
1. 读 `.chanlun/goals/current.yaml`（reducer projection）+ git HEAD + genealogy
2. 无 active goal → 从 roadmap/中断点推导候选，等编排者 GOAL_SET
3. 有 active goal → 调确定性 reducer（ceremony_scan）→ spawn ready workstations → 汇报编排者
4. state 快照（session/interrupt）仅 base_head 匹配时作 hint，不匹配 reduce 重算
562 bootstrap 结构工位强制保留。069 Swarm₀ 创世 Gap 仍在（ceremony 是 bootstrap 残余不消失）。
```

- [ ] **Step 3: 「你不做的事」补 3 条 + 修中断 #3 矛盾**

「你不做的事」末尾追加：
```markdown
- 不定义 goal（编排者 domain）
- 不分解 goal（reducer + 编排者 domain）
- 不评估目标价值（编排者 domain）
```
改中断 #3（line 60-65）"实现分歧 → 你裁定或建议方案" 为：
```markdown
  - 实现分歧 → 路由到审查/决策工位（code-reviewer/codex-challenger），不自己裁定
  - 概念分歧 → 转化为中断 #1（概念分离信号）
```

- [ ] **Step 4: 验证改动一致**

Run: `grep -c "bootstrap/IO actuator\|不定义 goal\|路由到审查/决策工位" .claude/agents/meta-lead.md`
Expected: 3（三处关键改动到位）

- [ ] **Step 5: Commit**

```bash
git add .claude/agents/meta-lead.md
git commit -m "feat(goal): D′ 波2 — meta-lead 降 bootstrap/IO actuator + 修中断#3裁定矛盾（基因组改,020已批）"
```

---

## Task 8：ceremony.md 恢复先读 goal

**Files:**
- Modify: `.claude/commands/ceremony.md`

- [ ] **Step 1: 热启动段加「先读 goal」**

在 ceremony.md 热启动流程顶部插入：
```markdown
0. **先读 current goal**（D′）：reduce `.chanlun/goals/events.jsonl` → 有 active goal 则恢复目标驱动循环；无 goal 才走下方 session 快照恢复。goal 是稳定恢复锚（session 快照易过时，仅 base_head 匹配作 hint）。
```

- [ ] **Step 2: 验证**

Run: `grep -c "先读 current goal\|先读 goal" .claude/commands/ceremony.md`
Expected: ≥1

- [ ] **Step 3: 全链路冒烟（goal 设定→恢复）**

Run: `cd /Users/silencehan/Projects/NewChanlun && python -m pytest scripts/tests/test_goal_reducer.py scripts/tests/test_ceremony_scan_goal.py -v`
Expected: PASS（全绿，reducer + scan 集成）

- [ ] **Step 4: Commit**

```bash
git add .claude/commands/ceremony.md
git commit -m "feat(goal): D′ 波2 — ceremony 恢复先读 goal（goal 是稳定锚，state 易过时）"
```

---

## 谱系结晶（执行后，编排者 /ritual）

D′ 实现完成后，备供编排者 /ritual：
- 新谱系：RTAS-goal 持久化（goal=动态根，state 降 projection，Lead=bootstrap/IO）
- 谱系依据：069/033/624/562/162/081/575
- 影响声明：见 spec §12

---

## 自审（writing-plans 要求）

**1. Spec 覆盖**：spec §3 组件（goal契约/events/reducer声明+实现/Lead/sub-goal树）→ T1(契约+events)/T2(reducer实现)/T3(reducer声明)/T7(Lead)/T2(sub-goal树在reducer)；§5 三机制统一 → T4/T5/T8；§6 最小基因组改 → T7/T8 + 波1不碰；§7 分层防污染 → T1 SCHEMA.md；§8 sub-goal树 → T2 reducer。✅ 全覆盖。
**2. Placeholder**：无 TBD/TODO，每步有具体代码/命令/期望输出。✅
**3. 类型一致**：reduce_goal 签名 (events, facts)→dict 贯穿 T2/T4；事件字段（goal_id/sub_goal_id/acceptance/falsifiable）SCHEMA(T1) 与 reducer(T2) 一致。✅
