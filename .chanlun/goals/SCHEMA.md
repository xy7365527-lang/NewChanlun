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

## 分层防污染（575号）
goal events ≠ genealogy。只有 goal 定义/验收/分解原则**语义变更**才升格 genealogy（用 goal_event_id 反向引用）。
