# 工位 #25 [goal-usability] 结果包

行动类（018 四分法），简化版三要素。

## 结论

修复 `ceremony_scan.py` 输出双层嵌套——`/goal` 端到端可用。

- 根因：`ceremony_scan.py:1488-1489` 把 `reduce_goal` 完整 dict（内层键 `current_goal` 与 ceremony 输出键 `current_goal` **同名**）整体塞进 `result["current_goal"]`，真正 goal_id 落在 `result["current_goal"]["current_goal"]["goal_id"]`，解析少读一层即误报「goal 未设定」。
- 修复：拍平赋值——`result["current_goal"]` 直接 = reducer 内层 goal projection（顶层含 goal_id/description/acceptance/base_head/base_head_stale）；其余 reducer 键提到 result 顶层加 `goal_` 前缀（`goal_ready_workstations` / `goal_ready_details` / `goal_blocked` / `goal_terminated` / `goal_skipped_event_lines`）。goal_result 为 None 时 `result["current_goal"]=None`（seed bootloader 不阻塞冷启动）。

## 端到端验证

```
$ python scripts/ceremony_scan.py | jq -r .current_goal.goal_id
g-mutex-eat-every-element

$ ... | jq .goal_ready_workstations
["mutex-derive"]
```

goal_id 一层直出，ready_workstations 顶层可读，无双层嵌套。

## 消费者审计（grep 全部，一起改）

- `goal_driven_workstations(goal_result)`（同文件）：参数是**完整 reducer dict**（`goal_result`），不是 `result["current_goal"]`，独立于输出层 → 不受拍平影响，未改。
- `interrupt_materializer.py`：走独立路径（自调 `reduce_goal`，不读 `result["current_goal"]`）→ 不受影响，未改。
- 无其他读 `result["current_goal"]` 的下游（grep `result\["current_goal"\]` 仅 ceremony_scan.py 一处赋值）。

## 边界条件

- 若未来有消费者读 `result["current_goal"]["current_goal"]`（旧双层路径），拍平后会拿到 None——但 grep 确认当前无此消费者。新双层路径不存在 = 不可能有旧消费者依赖它（双层是 bug，非契约）。
- `test_materialize_interrupt_flag_writes_projection` 原硬编码旧 goal_id `g-l2-nautilus-production`，#22 改真实 events.jsonl 为 `g-mutex-eat-every-element` 后该测试脆断（测试隔离 bug：断言耦合真实数据快照）。一并修：改为从 reducer 取当前 goal_id 对照，不硬编码。属测试错误（修复不改任何定义），非定义冲突。

## 影响声明

改动文件：
1. `scripts/ceremony_scan.py`（main 输出层拍平，+16 行）
2. `scripts/tests/test_ceremony_scan_goal_driven.py`（materialize 测试去硬编码 goal_id，+8 行）

未碰：`goal_reducer.py`、`goal_events.py`（#22 在改，无文件冲突——只重叠测试套件，`test_goal_events.py::test_goal_amend_idempotent_rebind_noop` 失败属 #22 进行中改动，非本工位）、`interrupt_materializer.py`、`goal_driven_workstations`。

测试：ceremony_scan + goal_reducer + interrupt_materializer 套件全绿（6+其余 passed）。
