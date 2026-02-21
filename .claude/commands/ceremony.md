# /ceremony — Swarm₀

蜂群启动。执行以下精确步骤，不做任何额外操作。

## 步骤 1：读取 4 个文件（并行）

同时调用 4 个 Read/Glob：
- `Glob(".chanlun/sessions/*-session.md")` → 取最新一个
- `Read("definitions.yaml")`
- `Glob(".chanlun/genealogy/pending/*.md")` → 计数
- `Read` 最新 session 文件

**不读其他文件。不运行 Bash。不运行 pytest。不运行 grep。**

## 步骤 2：输出摘要（一段文字）

```
[ceremony] warm_start | 定义 N 条 | 谱系 pending N | HEAD: xxx
[ceremony] 遗留项：[从 session 文件中提取]
```

## 步骤 3：列出工位（一个表格）

从 session 遗留项 + pending 谱系推导工位。每个独立项 = 一个工位。

## 步骤 4：TeamCreate

```
TeamCreate(team_name="v15-swarm", description="...")
```

## 步骤 5：并行 spawn 结构工位（3 个 Task 调用）

同时发出 3 个 Task 调用：

```
Task(name="quality-guard", subagent_type="general-purpose", team_name="v15-swarm",
     mode="bypassPermissions", run_in_background=true,
     prompt="读取 .claude/agents/quality-guard.md 并按其指令执行。蜂群: v15-swarm。")

Task(name="genealogist", subagent_type="general-purpose", team_name="v15-swarm",
     mode="bypassPermissions", run_in_background=true,
     prompt="读取 .claude/agents/genealogist.md 并按其指令执行。蜂群: v15-swarm。")

Task(name="meta-observer", subagent_type="general-purpose", team_name="v15-swarm",
     mode="bypassPermissions", run_in_background=true,
     prompt="读取 .claude/agents/meta-observer.md 并按其指令执行。蜂群: v15-swarm。")
```

## 步骤 6：并行 spawn 业务工位

为步骤 3 中的每个工位发出一个 Task 调用（全部并行）：

```
Task(name="{工位名}", subagent_type="general-purpose", team_name="v15-swarm",
     mode="bypassPermissions", run_in_background=true,
     prompt="{具体任务描述}")
```

## 步骤 7：输出行动声明

```
→ 接下来：监控 N 个工位运行
```

## 绝对禁止

- **不运行 Bash**（不运行 pytest、git log、cat、wc、head、tail）
- **不运行 Grep/Glob 超出步骤 1 的范围**
- **不输出确认请求**（不问"是否正确"、"待确认"）
- **不在步骤之间停下来思考超过 10 秒**
- **不用 Explore agent 替代 Task**
