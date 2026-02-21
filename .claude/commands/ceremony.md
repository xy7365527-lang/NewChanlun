# /ceremony — Swarm₀：递归蜂群的第0层

蜂群启动。执行以下精确步骤，不做任何额外操作。

## 设计原则（058号谱系）

- ceremony 不是蜂群的前置阶段，是蜂群的第0层递归（Swarm₀）
- Agent 无"等待确认"中间态——要么在计算区分，要么已终止
- 结构工位是蜂群的拓扑前提，由 ceremony 自动注入（Gemini decide 方案D：拓扑属于仪式不属于业务）
- 业务 teammate 不需要知道结构工位的存在，prompt 保持极简

## 步骤 1：读取 4 个文件（并行）

同时调用 4 个 Read/Glob：
- `Glob(".chanlun/sessions/*-session.md")` → 取最新一个
- `Read("definitions.yaml")`
- `Glob(".chanlun/genealogy/pending/*.md")` → 计数
- `Read` 最新 session 文件

**不读其他文件。不运行 Bash。不运行 pytest。不运行 grep。**

不扫描的内容（按需检索，不在启动时加载）：
- settled/ 下的已结算谱系（已固化，通过 dag.yaml 索引）
- definitions/*.md 逐个文件（已通过 definitions.yaml 汇总）
- CLAUDE.md 全文（已在系统 prompt 中加载）
- dispatch-dag.yaml 全文（按需读取具体节）

## 步骤 2：输出摘要（一段文字）

```
[ceremony] warm_start | 定义 N 条 | 谱系 pending N | HEAD: xxx
[ceremony] 遗留项：[从 session 文件中提取]
```

如果没有 session 文件，输出 `[ceremony] cold_start`。

## 步骤 3：列出工位（一个表格）

从以下来源推导（**只读取已有文件，不执行任何命令**）：

| 来源 | 推导方式 |
|------|---------|
| session 遗留项 | 每个独立项 = 一个工位 |
| pending 谱系 | 每个生成态矛盾 = 一个工位 |
| session 记录的测试失败 | 上次记录中的失败项 = 一个工位 |
| 谱系下游行动未执行 | spec-execution gap = 一个工位 |

**无工位可派生时**（所有来源均无工作）：
输出 `[020号反转] 无新区分可产出——系统干净终止`，然后停止。不执行步骤 4-7。

## 步骤 4：TeamCreate

```
TeamCreate(team_name="v{N}-swarm", description="...")
```

## 步骤 5：并行 spawn 结构工位（3 个 Task 调用，mandatory dominator nodes）

同时发出 3 个 Task 调用（dispatch-dag mandatory=true，fractal_template inherited）：

```
Task(name="quality-guard", subagent_type="general-purpose", team_name="{蜂群名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="读取 .claude/agents/quality-guard.md 并按其指令执行。蜂群: {蜂群名}。")

Task(name="genealogist", subagent_type="general-purpose", team_name="{蜂群名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="读取 .claude/agents/genealogist.md 并按其指令执行。蜂群: {蜂群名}。")

Task(name="meta-observer", subagent_type="general-purpose", team_name="{蜂群名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="读取 .claude/agents/meta-observer.md 并按其指令执行。蜂群: {蜂群名}。")
```

子蜂群递归时同样继承三个结构工位（fractal_template, parent_interface 三条边）。

## 步骤 6：并行 spawn 业务工位

为步骤 3 中的每个工位发出一个 Task 调用（全部并行）：

```
Task(name="{工位名}", subagent_type="general-purpose", team_name="{蜂群名}",
     mode="bypassPermissions", run_in_background=true,
     prompt="{具体任务描述}")
```

## 步骤 7：输出行动声明

```
→ 接下来：监控 N 个工位运行
```

## 绝对禁止（违反任何一条 = ceremony 失败，必须重做）

- **不运行 Bash**（不运行 pytest、git log、cat、wc、head、tail、grep）
- **不运行 Grep/Glob 超出步骤 1 的范围**
- **不输出确认请求**（不问"是否正确"、"待确认"、"如有偏差请指出"）
- **不输出等待信号**（不说"等待新任务输入"）
- **不在步骤之间停下来过度思考**
- **不用 Explore agent 替代 Task**——Explore 是只读搜索工具，不是蜂群节点
- **热启动时不等待编排者确认**——直接进入蜂群循环（058号）

## 谱系引用

- 058号：ceremony 是 Swarm₀
- 056号：蜂群递归是默认模式
- 057号：LLM 不是状态机
- 059号：ceremony 线性协议是结构性缺陷
- 060号：3+1 架构原则
- 069号：递归拓扑异步自指蜂群
