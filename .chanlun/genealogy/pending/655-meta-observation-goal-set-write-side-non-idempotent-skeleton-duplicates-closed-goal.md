---
id: "655"
number: 655
status: 生成态   # meta-observer 二阶观察：/goal 命令的 GOAL_SET 写入端非幂等——_cli_goal_set 无条件 append 新 skeleton，从不查询是否已存在等价(active/closed)goal，与已 closed 的等价 goal(g-mutex-eat-every-element)机械撞车，靠 Lead 事后 SUPERSEDE 收拾。1 条语法记录/选择候选(写入端是否应去重)。辨认待编排者 /ritual。依赖 353/621/624/636。（待#37 codex 裁定）
date: "2026-06-30"
type: meta-rule
source: meta-observer（二阶观察，compact 恢复 session 触发——/goal ! 预处理重调时自动新建 skeleton goal g-20260630T042951Z-56dfde07 重复已 closed 的 g-mutex-eat-every-element）
negation_source: heterogeneous
negation_form: separation
# separation：『GOAL_SET 写入』这一统一动作在写入端暴露异质性——
#   『首次设立一个新目标』与『重新触发一个已存在(active 或已 closed)的等价目标』在
#   _cli_goal_set 中走同一条无条件 append 路径，写入端不区分两者。
#   636 已识别 Lead 读 notification 信号端的语义歧义(idle≠完成)；本号识别 goal 系统的对偶面：
#   写入端(_cli_goal_set)同样不携带"该目标是否已存在"的语义判断——每次 ! 预处理重调即机械新建 skeleton。

# 代码证据(L0)：scripts/goal_events.py:460 _cli_goal_set
#   - 读 events 仅用于生成 goal_id(_slug_goal_id: sha256(description) 前8位)与 base_head(git HEAD)
#   - 从不调用 _current_goal_set(356行，已有"最后一个未被 SUPERSEDE 的 GOAL_SET"读口径)
#   - 无条件 append_event("GOAL_SET", ...)，acceptance=固定 a0-skeleton 占位
#   - 结果：description 相同→goal_id 相同(sha256 确定性)，但每次仍 append 一条新 GOAL_SET 事件；
#     description 不同但语义等价(如本轮 closed goal 的重述)→goal_id 不同→产生独立 skeleton goal

topo_effect: "split:goal-set-write-action:first-setup-vs-re-trigger-existing-equivalent-goal-cli-conflates-unconditionally-appends"
depends_on:
  - "636"   # 对偶面——636 是 Lead 读 notification 信号端的语义歧义；本号是 goal 系统写入端(_cli_goal_set)的语义缺失(同族:goal/信号系统语义边界)
  - "621"   # 结构工位自动化缺口承重——本号是 /goal 写入路径的自动化缺口(GOAL_SET 无去重检测)
  - "624"   # /goal=Lead runtime 承载层——本号:该承载层的写入入口非幂等
depends_on_unverified:
  - "353"   # 平台不支持语义事件(636 前置，本号同族但未独立核实其与写入端的因果)
related:
  - "275"   # 局部依赖原则——goal 是 Lead runtime 的全局对象，写入端去重涉及"读全局状态"是否违反局部依赖(待 /ritual 辨认)
  - "090"   # 声明膨胀——skeleton goal 的 a0-skeleton 占位 acceptance 声明"待 Lead 补全"，本身可证伪(代码注释已自辩护非假数据)，但重复 skeleton 使 events.jsonl 累积语义噪声
related_records:
  parent: "636"
  children: []

# 认识论等级标注（formalization-validity-domain 强制——本号自指适用）
epistemological_levels:
  - proposition: "_cli_goal_set(goal_events.py:460) 无条件 append 新 GOAL_SET，从不调用 _current_goal_set 查询已存在等价 goal"
    level: "L0(代码静态事实：函数体只读 events 生成 id/base_head，无去重分支)"
    increment: "高：写入端非幂等的结构判定"
  - proposition: "本轮 ! 预处理重调 /goal 自动新建 skeleton g-20260630T042951Z-56dfde07，机械重复已 closed 的 g-mutex-eat-every-element，Lead 用 SUPERSEDE 退役"
    level: "L0(本轮事件转述——Lead 报告的现象；meta-observer 未独立核实 events.jsonl 中两 goal 的 description 是否字面等价或仅语义等价)"
    increment: "中：写入端非幂等的活体实例(本轮)；description 等价类型(字面 vs 语义)未核实，影响去重判据设计"

# 四分法分类(meta-observer 不自决，标注候选类型供 /ritual 行使分类权)
quadrant_candidate: "选择 OR 语法记录(二者之一，待辨认)"
quadrant_reasoning: |
  Lead 提出的问题"/goal 自动 GOAL_SET 在已有等价 closed goal 时应否去重/检测重复"是典型的需价值判断的开放问题，
  存在至少三个合理方案，meta-observer 不能臆断答案(no-unnecessary-escalation: 这是"选择"类，允许且应当 /escalate)：

  方案A(写入端去重)：_cli_goal_set 在 append 前调用 _current_goal_set/扫 events，
    若存在等价 active goal→拒绝并提示 GOAL_AMEND；若存在等价 closed goal→提示需显式 REOPEN/确认意图。
    代价：写入端引入"读全局 goal 状态"逻辑，可能触及 275 局部依赖边界(Lead runtime 全局对象，非工位局部依赖)。

  方案B(保持非幂等，Lead 事后 SUPERSEDE)：当前行为即设计——skeleton 是廉价占位，
    Lead 的 command-后审查(注释 464行声明的职责)就是去重责任方。
    代价：每次重调累积 events.jsonl 噪声 + 依赖 Lead 不遗漏审查(行为层概率性，137号 RLHF 基底约束下不可靠)。

  方案C(等价性判据本身需先定义)：去重前提是"goal 等价"有可判定形式。
    当前 goal_id=sha256(description) 仅捕获字面等价，语义等价(本轮 closed goal 的重述)不被捕获。
    若选 A，必须先回答"goal 等价"的形式化定义——这可能是更深的未结算概念。

  辨认方向：若编排者裁定"写入端应幂等"=语法记录(已隐含在 SUPERSEDE/_current_goal_set 设计意图中但未在写入端兑现)；
  若裁定"保持 Lead 审查为去重责任方"=确认现状为选择。两路均需 /escalate → /ritual。
---

# 二阶观察：/goal GOAL_SET 写入端非幂等——skeleton 机械重复已 closed 的等价 goal

## 规则版本基线（强制字段——本工位工具限制说明）

```yaml
rule_version_baseline:
  claude_md_commit: "PENDING — meta-observer 工位无 Bash 工具，无法运行 git log -1 --format='%H' -- CLAUDE.md。待 Lead 或 genealogist 回填精确 hash。诚实标注优于臆造(no-patch-mentality)。"
  rules_dir_mtime: "PENDING — 同上，无法运行 git log -1 --format='%ai' -- .claude/rules/。待回填。"
```

> meta-observer agent 定义授予 Read/Write/Grep/Glob/Task/SendMessage，**无 Bash**。基线 hash 需 git 命令获取，本工位结构性不可达。
> 这不是省略（formalization-validity-domain 禁止等级省略），而是工具边界的诚实声明——
> 141号要求基线字段，但未要求由无 Bash 的工位强行臆造 hash。回填责任在有 Bash 的下游(Lead/genealogist)。
> **影响**：在 hash 回填前，本号与其它观察的"规则版本变化 vs 认知差异"判别暂不可执行（141号下游推论3 的能力暂缺）。

## 现象

compact 恢复后，`/goal` 命令的 `!` 预处理在重调时自动 GOAL_SET 了一个新 skeleton goal
（`g-20260630T042951Z-56dfde07`），它是已 closed goal（`g-mutex-eat-every-element`，"吃到每一个元素"）的机械重复。
Lead 用 SUPERSEDE 退役了这个冗余 skeleton。

## 根因（代码证据，L0）

`scripts/goal_events.py:460 _cli_goal_set`：
- 读 events **仅**用于生成 `goal_id`（`_slug_goal_id` = sha256(description) 前8位）与 `base_head`（git HEAD）
- **从不**调用 `_current_goal_set`（356行——已存在的"最后一个未被 SUPERSEDE 的 GOAL_SET"读口径）
- 无条件 `append_event("GOAL_SET", ...)`，acceptance 固定为 `a0-skeleton` 占位

写入端不携带"目标是否已存在"的判断。读端已有 `_current_goal_set` 提供"未被 SUPERSEDE 的活 goal"口径，
但写入端不消费它——读写两端对"goal 是否已存在"的语义判断不对称。

## 与 636 的对偶关系（发散信号，非收敛）

636 号识别 **Lead 读 notification 信号端** 的语义歧义（idle ≠ 完成）。
本号识别 **goal 系统写入端**（`_cli_goal_set`）的语义缺失（首次设立 ≠ 重触发等价目标，但走同一路径）。
两者同属 goal/信号系统的语义边界族，但分居读/写两端，不重叠。

收敛检查：grep settled/ 全量 meta-rule 无"goal 写入端幂等/去重"等价记录。命中的 idempotent 全是缠论 invariant（I9/I14），与 goal 无关。判定为发散信号（新维度），独立记录。

## 上浮建议（供 Lead）

本观察的核心问题（"应否去重"）是**选择/语法记录**类，meta-observer 不自决（no-unnecessary-escalation 允许此类 /escalate）。
建议 Lead 通过 `/escalate` 上浮 → `/ritual` 辨认。三方案与等价性判据前提见 front matter `quadrant_reasoning`。
