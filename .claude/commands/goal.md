---
name: goal
description: 自动建 GOAL_SET 写 events.jsonl 并进入 goal 持续运行循环
argument-hint: [目标描述]
allowed-tools: Bash(python:*)
---

# /goal — 目标驱动的持续运行模式（Lead runtime 承载层）

## 自动写入 GOAL_DRAFT（#2 两阶段，阶段一）

!`python scripts/goal_events.py GOAL_SET --description "$ARGUMENTS"`

上方是脚本执行结果——它写的是 **GOAL_DRAFT**（非 active 占位，含 goal_id + base_head + skeleton acceptance），已 append 到 `.chanlun/goals/events.jsonl`。GOAL_DRAFT **不进 active 集**：reducer 不把它当 current_goal，ceremony_scan 不产 goal 工位。这防止 skeleton 伪验收进入 active goal（codex #2）。

以 Lead 身份完成**阶段二（转正）**：把 skeleton 替换为真实可证伪 acceptance，二选一（均合法）：
1. **直接写正式 GOAL_SET**（同 goal_id，带真实 acceptance）——最简路径。`append_event("GOAL_SET", goal_id=<draft 的 goal_id>, description=..., acceptance=[真实可证伪项], base_head=<draft 的 base_head>)`。
2. **先 GOAL_AMEND ACCEPTANCE_REPLACE 补全 DRAFT 的 acceptance**（带 `old_acceptance_vector_hash` 匹配 skeleton vector），再写正式 GOAL_SET。适用于想保留 amend 审计链时。

每个 acceptance 项须 falsifiable 且可机器判定闭合。转正后 GOAL_SET 进 active。然后调 `python scripts/ceremony_scan.py` 取 ready 工位，进入下方运行协议循环（评估→scan→spawn→监控→真封→commit→回步骤1，不每步等确认）。

---

设定一个**收敛目标**，让蜂群在该目标驱动下**持续自主运行蜂群循环**，直到目标达成、遇真实矛盾、或资源耗尽——**不每步等待编排者确认**。

## 用法

```
/goal <目标描述>
/goal            # 无参时从当前路线选择 + 中断点推导 goal
```

## 存在论位置（624号 / teach-0004a 推论）

624 号揭示：结构能力四象限中 **AGENT×持续监控** 象限在 CC 平台**无承载者**（结构工位工具仅 Read/Grep/Glob，无主动投递通道；hook 不能 spawn=097；task-notification 无 hook=353/621）。teach-0004a 裁定：**唯一修 = Lead 操作化 DAG 解释器纪律**（033号：Lead=DAG 解释器）。

`/goal` 就是这个承载机制的显式化：**Lead runtime 层在 goal 驱动下持续轮询工位 + 推进蜂群循环**，承载"持续监控型"能力——不是 hook/teammate 承载，而是 Lead 自身的运行时纪律承载。

## 契约写入（D′）

`/goal <目标>` 设定时（**两阶段**，#2 codex 修复）：
1. **阶段一（草稿）**：CLI 自动 append **GOAL_DRAFT** 事件（含 goal_id, description, skeleton acceptance[], base_head=当前 git HEAD, ts）。GOAL_DRAFT 非 active——reducer 不计入 active 集，不进 current_goal（防 skeleton 伪验收进 active）。
2. **阶段二（转正）**：Lead 把 skeleton 替换为真实可证伪 acceptance（每项 falsifiable=true，否则 writer/reducer raise——lesson 0011 锋利问题②），二选一：直接写正式 GOAL_SET（同 goal_id + 真 acceptance），或先 GOAL_AMEND ACCEPTANCE_REPLACE 补 DRAFT 再写 GOAL_SET。GOAL_SET 进 active。
3. 调 `python scripts/ceremony_scan.py`（其 `_load_current_goal` 读 events.jsonl → `goal_reducer.reduce_goal` 纯函数 reduce）→ scan JSON 输出 current_goal projection + ready_workstations。**注**：`goal_reducer` 是纯函数（无 IO），reduce 结果仅以 projection 形式出现在 scan JSON 的 `current_goal` 字段中；`current.yaml` 文件物化当前**尚未实装**，由后续任务/Lead 承载——本协议不声称已写 current.yaml。
4. 进入运行协议循环（下方）

幂等（#8/655）：所有写事件可带 `idempotency_key`——CLI/Lead 重试时同 key+同 payload 是 noop（不重复 append），同 key+异 payload raise；同 goal_id 的 GOAL_SET/GOAL_DRAFT 不可重复声明（防重复 active）。

恢复时（无参 /goal 或 /ceremony）：reduce events → 若有 active goal 继续；无则从 roadmap/中断点推导候选 GOAL_SET。
**state 降 projection**：session/interrupt 仅 base_head 匹配时作 hint，不匹配则 reduce 重算（消除状态过时；reduce 输出 current_goal.base_head_stale 标记不匹配）。

## 运行协议（持续循环，不等确认）

设定 goal 后，Lead 进入循环（每轮原子链，不可被总结段落/确认信号打断——137号/post-commit-flow）：

1. **评估**：当前 goal 是否达成？达成 → 输出交付物 + 停（格式B）。未达成 → 继续。
2. **scan**：`python scripts/ceremony_scan.py` 取可并行工位（已定义递归，不 ad-hoc）。
3. **spawn**：无依赖工位**全部并行** spawn（218/275号）；有依赖工位由前置完成回调触发。结构工位+督导按 562 bootstrap 常设。
4. **监控**：轮询在跑工位（这是 Lead runtime 承载持续监控的实质）。
5. **真封**：工位报 done → Lead **全量** build/test 真封（lake build + cargo，不认单文件声明，#93教训）→ 异质验证（codex/gemini challenger）。
6. **commit**：真封通过 → 统一 commit（携谱系意义=rebase 保留，012号）。
7. **回到步骤 1**（不输出总结段落，不问编排者——no-unnecessary-escalation）。

## 终止条件（仅以下三种合法停止）

| 条件 | 动作 |
|------|------|
| **goal 达成** | 输出交付物（可证伪产出/明确交付物）+ 格式B（状态快照+排除理由） |
| **真实矛盾** | `/escalate`（定义冲突/需编排者价值判断/缺外部数据权限） |
| **资源耗尽** | 报告进度 + 剩余工位 + 精确 blocker |

**不存在第四种停止。** "总结后停下来"、"等确认"、以问号结尾——都是违规（137号）。

## 与编排者的关系

- goal 驱动运行中收到的编排者消息：在当前原子轮（步骤1-7）到达边界后统一回应，不中断原子链（224/225号）。
- 编排者可随时 INTERRUPT 改变 goal 或方向——Lead 立即收敛到新 goal。
- goal 是**收敛的**（单一可验收交付物），不是宏大愿景。宏大 goal 拆成里程碑序列，/goal 一次只锚一个里程碑。

## 谱系依据

- 624号：AGENT×持续象限无承载者（CC 平台硬墙）
- teach-0004a：唯一修=Lead 操作化 DAG 解释器纪律
- 033号：Lead=DAG 解释器
- 218/275号：Lead 并行调度 + 局部依赖
- 137号/post-commit-flow：原子链不可被总结/确认打断
- no-unnecessary-escalation：不问可自决问题
