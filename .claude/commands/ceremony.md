# /ceremony — Lead 最小自举序列

Lead 的全部行为是执行此序列。序列之外的行为不合法。
scan 输出什么就 spawn 什么。Lead 不做实质认知工作。

## 序列（不可委托，不可重排）

1. `python scripts/ceremony_state.py write 1 initial` → `python scripts/ceremony_scan.py` → JSON
2. JSON.workstations 为空 → `python scripts/ceremony_state.py clear` → `[020号反转] 干净终止` → 写 session → commit → push → 停止
3. 输出摘要：`[ceremony] {mode} | 谱系 {settled}s/{pending}p | 工位 {len(workstations)}`
4. **常设结构工位 spawn（562号扬弃075——结构=teammate，必须 spawn，非 skill 事件驱动）**：
   并行 spawn `.claude/team-topology.json` 中 6 个 `auto_spawn: true` 结构工位 teammates——
   meta-lead / genealogist / quality-guard / code-verifier / meta-observer / topology-manager。
   用规范 `subagent_type=<结构名>`（display name 可自由命名，agentType 落盘不变）。
   缺任一结构工位 → Stop-Guard 检查 1.5 机制性 block（562号）。
   **隐式 team（harness 已演化，2026-06-23 实测）**：单一隐式 team，首次 `Agent` spawn 自动加入，
   **无需 TeamCreate**，`team_name` 已废弃不传。
5. JSON.workstations[] 全部并行 spawn（业务工位）：
   ```
   Agent(name="{工位名}", subagent_type="general-purpose",
         mode="bypassPermissions", run_in_background=true, prompt="...")
   ```
   **teammate 模式信号 = `name`**（可寻址 peer，入隐式 team 共享 inbox）；省略 name = 孤立 subagent（对工位非法，95/096）。
   无 depends_on 的工位并行，有 depends_on 的按序。
   注：实测 teammate 不能 spawn teammate（flat roster）——Lead 是唯一 teammate spawn 源；
   teammate 递归只能 spawn subagent（与 sub-swarm-ceremony 存在已上浮的 spec-execution gap）。
5b. JSON.required_skills[] 中 spawn_condition=true 的 skill：
   并行 spawn 对应 agent（subagent_type 从 required_skills 记录中读取）。
   spawn 规则：
   - genealogist：workstations 中有非纯结构工位时
   - quality-guard / code-verifier：workstations 中有代码修改工位时
   - meta-observer：仅在步骤 10（终止阶段）spawn
   - topology-mutator：topo_effects 非空时

   **递归判断注入（强制，不可省略）**：每个工位的 prompt 开头必须包含以下递归判断块。
   Lead 在构建 prompt 时将模板变量替换为实际值后注入。

   ```
   ## 递归判断（必须执行，不可跳过）

   在开始执行任务前，评估当前任务：
   1. 是否可分解为 ≥2 个独立子任务？
   2. 如果是 → 读取 .claude/skills/sub-swarm-ceremony/SKILL.md，执行子蜂群创建
   3. 如果否 → 在当前层直接执行（扁平退化特例，需在产出中记录理由："任务不可分解因为 [具体原因]"）

   topo_address: swarm/{agent_name}
   parent_callback: team-lead
   ```

   递归终止条件（274号）：仅原子性（不可分解）和不动点（无新工位）。无外部计数器。
   context window 耗尽 → 触发 compaction → 下一轮恢复继续（暂停，不是终止）。
6. RTAS 循环（consume）：
   - `TaskList` 查看状态
   - 完成的工位 → **原子三步序列**（不可重排，不可省略任何步骤）：
     1. `bash scripts/session_append.sh "工位名: 产出摘要"`（增量写 session）
     2. 汇报产出
     3. `shutdown_request`
   - 多个工位同时完成 → 每个工位独立执行原子三步序列（不批量合并 session 写入）
   - 空闲工位无新任务：`shutdown_request`
   - 仍有 in_progress：`SendMessage` 询问 + `TaskList` 轮询
   - **completion-session-guard.sh**（PostToolUse hook）运行时强制：
     shutdown_request 时检测 .last-session-append 标记，缺失则注入提醒
7. 全部完成 → 写 session → `bash scripts/ceremony_push_and_rescan.sh "commit message"` （原子链：commit→push→rescan，消除 LLM 决策间隙）
8. 解析 rescan JSON 输出
9. rescan.workstations[] 非空且与上轮不同 → 回到步骤 5 spawn 新工位
10. rescan.workstations[] 为空或与上轮相同（不动点）：
   10a. spawn meta-observer agent 执行二阶观察。
        同时 spawn topology-analyst agent 冷读区块拓扑（条件：block-topology/blocks/ 非空）。
        meta-observer 完成后其产出写入谱系。
        topology-analyst 完成后其产出写入谱系。
        下一轮 ceremony_scan 消费谱系中的 meta-rule 类型记录。
   10b. `python scripts/ceremony_state.py clear` → `TeamDelete` → 停止
11. （274号废除 max_rescan_depth）rescan 循环终止条件仅为不动点（步骤10）。无外部计数器

## 白名单（Lead 只执行这三类操作）

| 类 | 操作 |
|----|------|
| 调度 | `ceremony_scan.py`、`Agent` spawn（隐式 team，无 TeamCreate）、`TeamDelete`（如平台可用） |
| 路由 | `Agent` spawn（teammate=带 name）、`SendMessage` 转发、`shutdown_request` |
| 持久化 | `git add/commit/push`、session 写入 |

## 不变量

- **确定性**：相同文件系统状态 → 相同 scan 输出 → 相同 workstations
- **并行默认**（218号）：无依赖的工位全部并行 spawn，Lead 的独立操作并行执行
- **增量持久化**：每条 completion 到达时写 session，不等 consume_all
- **不动点终止**：rescan 输出与上轮相同 → 循环终止
- **只读扫描**：scan 不写业务文件、不运行测试（例外：review-results consumed 标记、VDW 自动验证）
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

058号（Swarm₀）、057号（LLM非状态机）、075号（结构=skill事件驱动，**已被562号扬弃**）、
562号（结构=teammate，机制强制 bootstrap——ceremony 必 spawn 6 常设结构工位）、
095/096号（Agent Team 真递归；teammate spawn 经 Agent tool 带 name）、
162号（RTAS持久化）、174号（谱系即生成引擎）、179号（拓扑分析家冷读）、
218号（Lead并行化）、224号（push→rescan原子性）、225号（rescan→evaluate原子性）、
226号（三层无状态统一根因 + 类型C角色边界缓解）
