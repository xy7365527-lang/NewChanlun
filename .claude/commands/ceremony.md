# /ceremony — Lead 最小自举序列

Lead 的全部行为是执行此序列。序列之外的行为不合法。
scan 输出什么就 spawn 什么。Lead 不做实质认知工作。

## 序列（不可委托，不可重排）

1. `python scripts/ceremony_state.py write 1 initial` → `python scripts/ceremony_scan.py --phase initial` → JSON
2. JSON.workstations 为空 → `python scripts/ceremony_state.py clear` → `[020号反转] 干净终止` → 写 session → commit → push → 停止
3. 输出摘要：`[ceremony] {mode} | 谱系 {settled}s/{pending}p | 工位 {len(workstations)}`
4. `TeamCreate(team_name="v{N}-swarm")`
5. JSON.workstations[] 全部并行 spawn：
   ```
   Task(name="{简写}", subagent_type="general-purpose", team_name="{蜂群名}",
        mode="bypassPermissions", run_in_background=true, prompt="...")
   ```
   无 depends_on 的工位并行，有 depends_on 的按序。

   **递归判断注入（强制，不可省略）**：每个工位的 prompt 开头必须包含以下递归判断块。
   Lead 在构建 prompt 时将模板变量替换为实际值后注入。

   ```
   ## 递归判断（必须执行，不可跳过）

   在开始执行任务前，评估当前任务：
   1. 是否可分解为 ≥2 个独立子任务？
   2. 如果是 → 读取 .claude/skills/sub-swarm-ceremony/SKILL.md，执行子蜂群创建
   3. 如果否 → 在当前层直接执行（扁平退化特例，需在产出中记录理由："任务不可分解因为 [具体原因]"）

   topo_address: {team_name}/{agent_name}
   depth_budget: {N}  # Lead 从自身 depth_budget 减1 传递；depth_budget=0 时禁止递归，直接执行
   parent_callback: team-lead
   ```

   depth_budget 初始值由 Lead 在 ceremony 入口设定，默认 3。每层递归减 1 传递给子工位。
6. RTAS 循环（consume）：
   - `TaskList` 查看状态
   - 完成的工位：汇报 + `shutdown_request`（批量并行，不逐个串行）
   - 每条 completion 到达时增量写 session（不等 consume_all）
   - 空闲工位无新任务：`shutdown_request`
   - 仍有 in_progress：`SendMessage` 询问 + `TaskList` 轮询
7. 全部完成 → 写 session → `bash scripts/ceremony_push_and_rescan.sh "commit message"` （原子链：commit→push→rescan，消除 LLM 决策间隙）
8. 解析 rescan JSON 输出
9. rescan.workstations[] 非空且与上轮不同 → 回到步骤 5 spawn 新工位
10. rescan.workstations[] 为空或与上轮相同（不动点） → `python scripts/ceremony_state.py clear` → `TeamDelete` → 停止
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

## 角色边界（226号类型C缓解）

Lead 遇到需要修改文件的任务 → 唯一合法行为是 spawn 工位。
Lead 不直接执行 Write/Edit（白名单路径除外）。违反此条 = 僭越。

## 绝对禁止

- 白名单外的 Bash 调用
- 白名单外的 Write/Edit（Lead 不做实质认知工作——spawn 工位）
- 额外的 Read/Glob（信息已在 JSON 中）
- 确认请求（"是否正确"、"待确认"）
- 等待信号
- Explore agent 替代 Task

## 谱系引用

058号（Swarm₀）、057号（LLM非状态机）、075号（skill事件驱动）、
162号（RTAS持久化）、174号（谱系即生成引擎）、179号（拓扑分析家冷读）、
218号（Lead并行化）、224号（push→rescan原子性）、225号（rescan→evaluate原子性）、
226号（三层无状态统一根因 + 类型C角色边界缓解）
