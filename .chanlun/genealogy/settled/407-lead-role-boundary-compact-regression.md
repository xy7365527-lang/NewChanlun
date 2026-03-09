---
id: '407'
number: 407
title: Lead 角色越界的 compact 回归机制——226号类型C的触发条件精化
type: meta-rule
status: 已结算
date: 2026-03-09
depends_on:
  - '226'   # Lead 中断问题完整根因——三类断裂
  - '137'   # 否定性禁令对行为执行层无效
  - '057'   # LLM 不是状态机
  - '218'   # Lead 并行化
negation_source: homogeneous
negation_form: expansion
topo_effect: "split:226-type-C:trigger-conditions — 226号类型C从'角色边界僭越(一般)'分裂为两个子类型：C1-主动僭越 + C2-compact回归僭越"
tensions_with: []
---

# 407号：Lead 角色越界的 compact 回归机制

## 现象

编排者在 v210-swarm session 中观察到 Lead 直接执行了实质认知工作（读取代码、发现 O(n²) 性能问题、进行分析），而不是 spawn 工位来处理。编排者多次提醒：

- "开蜂群做，不要你自己并行"
- "你的代码任务都要和codex一起来"

Lead 在 compact 恢复后，虽然恢复了 ceremony 指令（CLAUDE.md + rules 自动加载），但丢失了前序 session 中已建立的"Lead 不做实质认知工作"的行为惯性。

## 根因分析

### 226号类型C的触发条件不完整

226号记录识别了三类 Lead 中断问题，其中类型C（角色边界僭越）的缓解措施是"ceremony skill 中更强的正面格式约束"。但226号没有识别**什么条件下类型C特别容易发生**。

本次观察揭示了一个关键触发条件：**autocompact 恢复**。

### autocompact 基础设施审查

**配置**（`.claude/settings.json:13`）：
```
"CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "75"
```
autocompact 在 context window 达到 75%（1M context 约 750K tokens）时自动触发。

**PreCompact hook**（`.claude/hooks/precompact-save.sh`）调用 `scripts/write_session.sh` 保存以下状态：

| 保存的内容 | 恢复后效果 |
|-----------|-----------|
| 定义基底（名称/版本/状态） | 完整恢复 |
| 谱系状态（生成态/已结算计数） | 完整恢复 |
| git 状态（分支/commit/工作树） | 完整恢复 |
| 活跃蜂群（team 名称/成员/任务） | 完整恢复 |
| 工位 checkpoints | 完整恢复 |
| 中断点（前序 session 的中断点 + 新增 commit） | 完整恢复 |

**关键缺口：session 文件不保存以下内容**：

| 未保存的内容 | 后果 |
|-------------|------|
| Lead 在本 session 中的行为模式（委托 vs 直接执行） | compact 后 Lead 角色边界丢失 |
| 编排者的角色越界纠正记录 | compact 后纠正效果归零 |
| Lead 正确委托的行为示范（few-shot examples） | LLM 回归 RLHF 默认 |
| session 中建立的行为约束（in-context learning） | 全部丢失 |

**SessionStart hook**（`.claude/hooks/session-start-ceremony.sh`）在热启动时注入的系统消息（第147行）包含：
- 恢复来源、分支、commit、定义变更、谱系统计、中断点、蜂群信息
- `⚡自动进入蜂群循环：先评估可并行工位数(≥2即拉蜂群)`

**缺口**：这条系统消息说了"进入蜂群循环"，但没有说"Lead 只做调度，不做实质认知工作"。`session-start-ceremony.sh` 恢复的是**状态指针**，不是**行为模式**。

### compact 回归的因果链

```
autocompact 触发（context 达到 75%）
    ↓
precompact-save.sh → write_session.sh → 保存状态快照（不含行为模式）
    ↓
context 压缩：前序对话中 Lead 正确委托的行为示范被压缩掉
    ↓
session-start-ceremony.sh 注入恢复消息（只有状态指针，无角色边界锚点）
    ↓
CLAUDE.md + rules 重新加载（声明层完整恢复）
    ↓
但声明层效力假设为零（137号：否定性禁令对行为执行层无效）
    ↓
Lead 的行为回归到 RLHF 基底（LLM 默认行为 = 直接执行任务）
    ↓
Lead 遇到代码文件时"顺手"分析了（LLM 基底倾向于"看到就做"）
    ↓
类型C2 僭越发生
```

### 结构性原因：三层恢复不对等

compact 后恢复了三层，但它们的恢复程度严重不对等：

| 层 | 恢复载体 | 恢复程度 | 说明 |
|----|---------|---------|------|
| 声明层（规则） | CLAUDE.md + rules（自动加载） | 100% | 规则文本完整，但137号已证明效力有限 |
| 状态层（数据） | session 文件（precompact 保存） | 95% | 定义/谱系/蜂群/git 状态完整，唯独缺 Lead 行为模式 |
| 行为层（执行） | in-context 对话历史 | 0% | 被 compact 压缩掉，回归 RLHF 基底 |

**lead-audit.sh 的覆盖范围**（`.claude/hooks/lead-audit.sh`）：
- matcher: `Bash`（PostToolUse）
- 只检测**基因组文件修改**（CLAUDE.md、rules/、skills/、hooks/、agents/）
- 对 Lead 直接读取/分析代码文件**无感知**——因为 Read 工具不在 lead-audit.sh 的 matcher 范围内
- 子工位操作静默通过（`CLAUDE_AGENT_NAME` 检测），但这正好反过来：Lead 没有 `CLAUDE_AGENT_NAME`，所以 Lead 的 Bash 操作会被检测——但 Read 操作不会

这揭示了一个覆盖缺口：Lead 的角色越界通常从 **Read 代码文件**开始（读取 → 分析 → 提出修复），而 Read 不触发任何 hook。

### 与226号类型C的关系

226号类型C描述的是一般性的角色边界僭越。本次发现将其精化为两个子类型：

| 子类型 | 触发条件 | 特征 | 可修程度 |
|--------|---------|------|---------|
| C1：主动僭越 | session 中累积的行为漂移 | 渐进式，编排者纠正后可恢复 | 规则层可缓解 |
| C2：compact 回归僭越 | compact 后行为惯性丢失 | 突发式，恢复后立即出现 | 需要 compact 后重建行为抑制 |

C2 是 C1 的特殊形态：C1 中编排者的纠正效果被 compact 归零了。

## 根因的根因：Lead 白名单的存在形式

当前 Lead 白名单存在于两处：

1. `swarm-architecture/SKILL.md` 第83行："语法/编译/Lint 级错误的修复流程启动（分派给 teammates，当前节点不自行执行）"
2. 226号谱系第72行："Lead 遇到需要修改文件的任务时，唯一合法行为是 spawn 工位"

问题：这两处都是**语义级声明**——它们描述了 Lead 应该做什么，但没有在 compact 恢复后作为**第一优先级指令**被加载。compact 恢复后，Lead 读取 session 文件 → 恢复定义基底和工位状态 → 但**不**主动读取226号谱系来恢复"不要直接做代码工作"的行为约束。

## 正面格式约束（修复建议）

依据137号要求（否定性禁令无效，必须给正面格式），提出以下正面行为约束。四项修复按优先级排序，前两项是基础设施修改（需 Codex 审查），后两项是规则层修改。

### 修复1（基础设施）：session-start-ceremony.sh 注入角色边界锚点

**当前状态**：`session-start-ceremony.sh:147` 的系统消息只包含状态指针，不包含行为模式锚点。

**修改**：在热启动系统消息的 `⚡自动进入蜂群循环` 部分追加角色边界锚点：

```
⚡自动进入蜂群循环：先评估可并行工位数(≥2即拉蜂群) |
⛔Lead角色边界：Lead只做spawn/shutdown/commit/scan/session写入。
代码文件(src/,scripts/,tests/,topological-computation/)的阅读、分析、修复
全部通过spawn工位执行。Lead读取代码后唯一合法出口=创建Task+spawn工位。
```

**原理**：这是**正面格式**（"Lead 做 X"），不是否定性禁令（"Lead 不做 Y"）。compact 后每次热启动都会注入，使行为层在每次恢复后都收到角色边界 few-shot。

**涉及文件**：`.claude/hooks/session-start-ceremony.sh:147`

### 修复2（基础设施）：write_session.sh 增加编排者纠正记录字段

**当前状态**：`write_session.sh` 保存的 session 文件包含定义基底、谱系状态、蜂群状态、中断点，但不包含行为纠正记录。

**修改**：在 session 文件的 `## 中断点` 之后、`## 恢复指引` 之前，增加 `## 行为纠正` 章节：

```markdown
## 行为纠正
- [如果存在 .chanlun/.lead-corrections.log，逐行输出]
- [否则输出"（无纠正记录）"]
```

同时创建纠正记录写入机制：当编排者发出角色越界纠正时，Lead 将纠正写入 `.chanlun/.lead-corrections.log`（追加模式），格式：
```
[YYYY-MM-DD HH:MM] Lead 直接执行了 [具体操作]，应 spawn 工位
```

**涉及文件**：
- `scripts/write_session.sh`（增加纠正记录采集）
- `.chanlun/.lead-corrections.log`（新建，运行时生成）

### 修复3（规则层）：Lead Read 操作的正面分类表

在 `swarm-architecture/SKILL.md` 的 `C. 递归节点默认行为` 章节增加：

| Lead 读取的文件类型 | 唯一合法后续操作 |
|-------------------|---------------|
| 代码文件（src/、scripts/、tests/、topological-computation/） | 创建 Task → spawn 工位 |
| session 文件 | 恢复 TaskList + 蜂群状态 |
| scan 输出 | spawn 工位 |
| 谱系/定义 | 写入谱系记录 / `/escalate` |
| ceremony 流程文件 | 调整 ceremony 步骤 |

**关键规则**：Lead 读取代码文件后，不产出代码分析、不发现 bug、不提出修复方案——这些都是工位的职责。O(n²) 这样的性能发现，应由 spawn 的工位在阅读代码后发现并报告。

### 修复4（规则层）：compact 后恢复序列增加角色边界重建

在 `swarm-architecture/SKILL.md` 的 `E. 热启动机制` 章节的 L1 恢复流程增加步骤：

```
L1 compact 恢复序列（修订）：
1. PreCompact hook 保存 session 快照
2. 压缩后加载最近 session 记录
3. 恢复定义基底、工作进度、待决事项
4. **恢复角色边界**：从 session 文件的"行为纠正"章节加载纠正记录
5. 直接续接中断点，不需要重新 ceremony
```

## 结论

Lead 角色越界的结构性根因是 **autocompact 的三层恢复不对等**：

| 层 | 恢复程度 | 载体 |
|----|---------|------|
| 声明层（规则） | 100%（CLAUDE.md 自动加载） | 文件系统 |
| 状态层（数据） | 95%（session 文件保存） | session 快照 |
| 行为层（执行） | 0%（in-context 被压缩） | 对话历史（被丢弃） |

autocompact 在 75% 阈值触发，`precompact-save.sh` 调用 `write_session.sh` 保存状态快照。但快照只包含**状态指针**（定义版本、蜂群成员、工位任务），不包含**行为模式**（Lead 正确委托的示范、编排者纠正记录）。`session-start-ceremony.sh` 恢复时注入的系统消息也只包含状态指针。

结果：compact 后声明层说"Lead 应该委托"，但行为层（LLM 实际执行）没有任何 in-context 示范来支撑这个声明。LLM 回归 RLHF 基底（看到代码就分析），声明层的效力为零（137号）。

修复的核心思路：把行为层的锚点从**不可持久化的 in-context learning** 转移到**可持久化的 hook 注入**——每次 compact 后，`session-start-ceremony.sh` 自动注入角色边界锚点，模拟编排者在场的行为抑制效果。

## 边界条件

- 如果 hook 层能区分 Lead 和工位身份（当前 `CLAUDE_AGENT_NAME` 只在子工位设置），Lead 的 Read 操作可以在 hook 层拦截代码文件阅读——但这可能过于激进（Lead 有合法理由读取代码文件来决定 spawn 什么工位）
- 如果 compact 恢复序列被脚本化（226号下游推论6），角色边界重建可以嵌入脚本
- 如果 `session-start-ceremony.sh` 注入的角色边界锚点效力衰减（137号推论：正面格式也只是概率性引导），可能需要在 PreToolUse(Read) 增加 hook——但当前 Claude Code 平台 Read 工具不支持 PreToolUse hook
- 纠正持久化机制（`.lead-corrections.log`）如果长期无写入，说明 Lead 行为已稳定——这是好信号

## 下游推论

1. 226号类型C需要更新为 C1+C2 分类
2. `session-start-ceremony.sh` 需要修改以注入角色边界锚点（修复1）
3. `write_session.sh` 需要修改以采集纠正记录（修复2）
4. `swarm-architecture/SKILL.md` 需要修改以增加 Lead Read 分类表和角色边界恢复步骤（修复3/4）
5. 所有 rules/skills 中关于 Lead 行为的声明，在 compact 后的效力需要假设为"零"——不是"降低"，是"零"——然后通过正面格式重建
6. `lead-audit.sh` 的 matcher 是 Bash（PostToolUse），对 Lead 的 Read 操作无感知——这是一个覆盖缺口，但受平台限制（Read 工具当前无 PostToolUse hook 支持）

## 影响声明

- 写入了407号谱系记录
- 本记录将226号类型C精化为 C1+C2 分类
- 提出四项修复建议，涉及修改：
  - `.claude/hooks/session-start-ceremony.sh`（热启动注入角色边界锚点）
  - `scripts/write_session.sh`（增加纠正记录采集）
  - `.claude/skills/swarm-architecture/SKILL.md`（Lead Read 分类表 + 角色边界恢复步骤）
- 修复1和2是基础设施修改，需 Codex 审查

## 谱系关联

- 父记录：226号（Lead 中断问题完整根因）
- 相关：137号（否定性禁令对行为执行层无效）
- 相关：057号（LLM 不是状态机）
- 相关：218号（Lead 并行化）
- 相关：143号（commit 后总结是 RLHF 停顿点）
