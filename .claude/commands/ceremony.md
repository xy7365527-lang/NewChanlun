# /ceremony — Lead 最小自举序列

Lead 的全部行为是执行此序列。序列之外的行为不合法。
scan 输出什么就 spawn 什么。Lead 不做实质认知工作。

## 序列（不可委托，不可重排）

1. `python scripts/ceremony_scan.py --phase initial` → JSON
2. JSON.workstations 为空 → `[020号反转] 干净终止` → 写 session → commit → push → 停止
3. 输出摘要：`[ceremony] {mode} | 谱系 {settled}s/{pending}p | 工位 {len(workstations)}`
4. `TeamCreate(team_name="v{N}-swarm")`
5. JSON.workstations[] 全部并行 spawn：
   ```
   Task(name="{简写}", subagent_type="general-purpose", team_name="{蜂群名}",
        mode="bypassPermissions", run_in_background=true, prompt="...")
   ```
   无 depends_on 的工位并行，有 depends_on 的按序。
6. RTAS 循环（consume）：
   - `TaskList` 查看状态
   - 完成的工位：汇报 + `shutdown_request`（批量并行，不逐个串行）
   - 每条 completion 到达时增量写 session（不等 consume_all）
   - 空闲工位无新任务：`shutdown_request`
   - 仍有 in_progress：`SendMessage` 询问 + `TaskList` 轮询
7. 全部完成 → 写 session → commit → push
8. `python scripts/ceremony_scan.py --phase rescan` → JSON
9. rescan.workstations[] 非空且与上轮不同 → 回到步骤 5 spawn 新工位
10. rescan.workstations[] 为空或与上轮相同（不动点） → `TeamDelete` → 停止
11. 安全阀：rescan 循环 ≤ 3 次（max_rescan_depth=3），超过则强制终止

## 白名单（Lead 只执行这三类操作）

| 类 | 操作 |
|----|------|
| 调度 | `ceremony_scan.py`、`TeamCreate`、`TeamDelete` |
| 路由 | `Task` spawn、`SendMessage` 转发、`shutdown_request` |
| 持久化 | `git add/commit/push`、session 写入 |

## 不变量

- **确定性**：相同文件系统状态 → 相同 scan 输出 → 相同 workstations
- **并行默认**（218号）：无依赖的工位全部并行 spawn，Lead 的独立操作并行执行
- **增量持久化**：每条 completion 到达时写 session，不等 consume_all
- **不动点终止**：rescan 输出与上轮相同 → 循环终止
- **只读扫描**：scan 不写文件、不运行测试（VDW 除外）
- **持久化不变量**：每条退出路径都以 session + commit + push 结束，无例外

## 中断处理

- Stop hook → Session 退出流程（写 session 快照 → 停止）
- 概念分离 → `/escalate`（不在序列中插入判断步骤）
- push 失败 → `git fetch && git rebase` → 重推

## 绝对禁止

- 白名单外的 Bash 调用
- 额外的 Read/Glob（信息已在 JSON 中）
- 确认请求（"是否正确"、"待确认"）
- 等待信号
- Explore agent 替代 Task

## 谱系引用

058号（Swarm₀）、057号（LLM非状态机）、075号（skill事件驱动）、
162号（RTAS持久化）、174号（谱系即生成引擎）、179号（拓扑分析家冷读）、
218号（Lead并行化）
