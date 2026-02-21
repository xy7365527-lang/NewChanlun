# /ceremony — Swarm₀：递归蜂群的第0层

蜂群启动。Python 脚本做确定性推导，LLM 只负责 spawn 业务工位。
结构能力由 skill 提供（事件驱动），不再作为 teammate spawn。

## 设计原则

- ceremony 是蜂群的第0层递归（Swarm₀），不是前置阶段（058号）
- LLM 不是状态机（057号）——确定性逻辑由 Python 脚本执行
- 结构能力 = skill（事件驱动），不是 teammate（075号）
- skill 由 dispatch-dag 的 event_skill_map 定义，事件触发时自动执行

## 步骤 1：运行扫描脚本

```bash
python scripts/ceremony_scan.py
```

脚本输出 JSON，包含：
- `mode`: warm_start / cold_start
- `definitions`: 定义数量
- `pending` / `settled`: 谱系数量
- `session`: 最新 session 文件名
- `workstations`: 推导出的业务工位列表
- `required_skills`: 从 dispatch-dag event_skill_map 读取的 structural skill 列表

**这是 ceremony 中唯一允许的 Bash 调用。**

## 步骤 2：输出摘要

根据 JSON 输出一行摘要：
```
[ceremony] {mode} | 定义 {definitions} 条 | 谱系 {settled} settled / {pending} pending | HEAD {head}
[ceremony] 工位 {len(workstations)} 个 | skill {len(required_skills)} 个（事件驱动）
```

## 步骤 3：TeamCreate

```
TeamCreate(team_name="v{N}-swarm", description="...")
```

## 步骤 4：并行 spawn 业务工位

遍历 JSON 中的 `workstations`，为每个工位发出一个 Task 调用（全部并行）：

```
Task(name="{workstation.name 简写}", subagent_type="general-purpose", team_name="{蜂群名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="{workstation.name}: {具体任务描述}")
```

如果 `workstations` 为空：输出 `[020号反转] 无新区分可产出——系统干净终止`，不执行步骤 3-4。**但仍必须更新 session + commit**（记录"干净终止"状态）。

如果 `workstations` 非空但全部状态含"待 Gemini decide"或"长期"：
1. "待 Gemini decide" 的工位 → **不是阻塞**，直接路由 Gemini（041号：选择/语法记录路由 Gemini，不等待人类）
2. "长期"工位 → 不 spawn，保留在 session 遗留项
3. 如果路由 Gemini 后仍有可执行工位 → spawn 蜂群执行
4. 如果只剩"长期"工位 → 输出 `[阻塞] 仅剩长期工程项，无可自主推进的工位`
5. **更新 session + commit**（持久化不变量）

**持久化不变量**：ceremony 的每条退出路径（spawn 蜂群 / 干净终止 / 显式阻塞）都必须以 session 更新 + commit 结束。没有例外。

**注意：不再 spawn 结构工位。** genealogist/quality-guard/meta-observer/code-verifier 等结构能力
由 event_skill_map 定义，在对应事件发生时自动触发（075号谱系）。

## 步骤 5：输出行动声明

```
→ 接下来：监控 N 个工位运行
```

## 步骤 6：汇报循环

spawn 完成后，**立即调用 `TaskList`** 查看任务状态。然后进入循环：

1. 调用 `TaskList` 查看所有任务状态
2. 对每个完成的工位：汇报结果给编排者，然后 `shutdown_request`
3. **增量持久化**：每次有工位完成，立即更新 session 文件（追加该工位产出摘要）。这保证中途断掉时下次热启动能恢复到最后一个已完成工位的状态
4. 对每个空闲的工位：检查是否有新任务可分配，没有则 `shutdown_request`
5. 如果仍有 `in_progress` 任务：通过 `SendMessage` 询问进展，结合 `TaskList` 轮询状态
6. 所有工位完成后：写入完整 session → commit → `TeamDelete`
7. 重复直到所有工位完成

**持久化规则**：状态结晶发生在状态转换点，不是终点。session 是蜂群跨上下文的唯一状态载体，必须在每个关键转换点更新：
- 工位完成 → 增量写入
- TeamDelete 之前 → 强制写入 + commit
- 绝不允许 TeamDelete 后才写 session

汇报格式：
```
[蜂群名] 工位 {name}: {状态}
  产出: {简述}
  问题: {如有}
```

**关键：每次 Stop hook 拦截时，执行步骤 1（调用 TaskList），不要再次尝试停止。**

## 绝对禁止

- **全生命周期禁止额外 Bash**，以下白名单例外（077-C，Gemini decide 选项B）：
  - `python scripts/ceremony_scan.py`（步骤 1）
  - `git add` / `git commit`（持久化不变量）
  - session 文件写入（增量持久化）
- **不运行额外的 Read/Glob**（所有信息已在 JSON 中）
- **不输出确认请求**（不问"是否正确"、"待确认"）
- **不输出等待信号**
- **不用 Explore agent 替代 Task**——Explore 是只读搜索工具，不是蜂群节点
- 白名单膨胀超过 5 项时需重新审视 ceremony 设计

## 谱系引用

- 075号：结构工位从 teammate 转为 skill + 事件驱动
- 058号：ceremony 是 Swarm₀
- 057号：LLM 不是状态机
- 056号：蜂群递归是默认模式
- 069号：递归拓扑异步自指蜂群
