# /ceremony — Swarm₀：递归蜂群的第0层

蜂群启动。Python 脚本做确定性推导，LLM 只负责 spawn。

## 设计原则

- ceremony 是蜂群的第0层递归（Swarm₀），不是前置阶段（058号）
- LLM 不是状态机（057号）——确定性逻辑由 Python 脚本执行
- 结构工位是蜂群的拓扑前提，由 ceremony 自动注入（Gemini decide 方案D）
- 结构工位数量从 dispatch-dag 动态读取，不硬编码

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
- `structural_nodes`: 从 dispatch-dag 读取的 mandatory 结构节点

**这是 ceremony 中唯一允许的 Bash 调用。**

## 步骤 2：输出摘要

根据 JSON 输出一行摘要：
```
[ceremony] {mode} | 定义 {definitions} 条 | 谱系 {settled} settled / {pending} pending | HEAD {head}
[ceremony] 工位 {len(workstations)} 个 | 结构节点 {len(structural_nodes)} 个
```

## 步骤 3：TeamCreate

```
TeamCreate(team_name="v{N}-swarm", description="...")
```

## 步骤 4：并行 spawn 结构工位

遍历 JSON 中的 `structural_nodes`，为每个节点发出一个 Task 调用（全部并行）：

```
Task(name="{node.id}", subagent_type="general-purpose", team_name="{蜂群名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="读取 {node.agent} 并按其指令执行。蜂群: {蜂群名}。")
```

## 步骤 5：并行 spawn 业务工位

遍历 JSON 中的 `workstations`，为每个工位发出一个 Task 调用（全部并行）：

```
Task(name="{workstation.name 简写}", subagent_type="general-purpose", team_name="{蜂群名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="{workstation.name}: {具体任务描述}")
```

如果 `workstations` 为空：输出 `[020号反转] 无新区分可产出——系统干净终止`，不执行步骤 3-5。

## 步骤 6：输出行动声明

```
→ 接下来：监控 N 个工位运行
```

## 步骤 7：汇报循环

蜂群运行期间，Lead 持续向编排者汇报：
- 收到 teammate 消息时：汇总该工位的产出/问题/状态变更
- 所有工位完成时：输出完整状态快照（每个工位的结果、新增谱系、遗留项）
- 发现矛盾时：立即汇报并走 `/escalate`

汇报格式：
```
[蜂群名] 工位 {name}: {状态}
  产出: {简述}
  问题: {如有}
```

## 绝对禁止

- **除步骤 1 外不运行任何 Bash**（不运行 pytest、git、cat、grep）
- **不运行额外的 Read/Glob**（所有信息已在 JSON 中）
- **不输出确认请求**（不问"是否正确"、"待确认"）
- **不输出等待信号**
- **不用 Explore agent 替代 Task**——Explore 是只读搜索工具，不是蜂群节点

## 谱系引用

- 058号：ceremony 是 Swarm₀
- 057号：LLM 不是状态机
- 056号：蜂群递归是默认模式
- 069号：递归拓扑异步自指蜂群
