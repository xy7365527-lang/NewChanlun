---
id: '226'
number: 226
title: Lead 中断问题完整根因——三类断裂 + 角色边界僭越
type: meta-rule
status: 已结算
date: 2026-02-27
depends_on:
  - '224'   # push→rescan 原子性断裂
  - '225'   # rescan→evaluate 原子性断裂
  - '137'   # 否定性禁令对行为执行层无效
  - '057'   # LLM 不是状态机
  - '173'   # lead-audit 方法论反转（默认放行+黑名单记录 vs ceremony 白名单阻断）
tensions_with: []
topo_effect: "Lead 中断问题从个案修复升级为结构性分类——统一根因：三层无状态"
---

# 226号：Lead 中断问题完整根因

## 现象

编排者反复观察到 Lead "断掉"，224/225号修复后仍在发生：
- push 后不 rescan（224号已修）
- rescan 后不 evaluate（225号已修）
- 质询收敛后问编排者"你要我修还是先讨论"（四分法违规）
- Lead 直接修改 v4 文本（僭越 ceremony 白名单）

编排者："你怎么又断了"、"你又断了"、"我受不了了，一直在出现"。

## 统一根因：三层无状态（Codex 诊断 §2.4）

四个现象是同一个结构性缺陷的四种投影：

```
ceremony 协议（顺序状态机，要求步骤间状态维护）
    ↓ 执行者
LLM（无状态决策器，057号）
    ↓ 守卫
hook 系统（无状态事件驱动，只覆盖 Bash 边界）
```

每一层都是无状态的。ceremony 协议要求状态维护，但执行者和守卫都不维护状态。224/225 在 Bash 边界注入了"伪状态"（正面指令），但 Read/Write/Edit 边界没有对应的伪状态注入。

守卫架构与 ceremony 协议之间存在**范畴错配**——ceremony 是顺序状态机，守卫是无状态事件驱动系统。

## 根因分类

Lead 的"断掉"不是一个问题，是三类不同的结构性问题：

### 类型 A：Bash 后断裂（已修）

**现象**：git push / ceremony_scan.py 执行后，Lead 被用户消息吸引走。
**根因**：hook 覆盖不足。
**修复**：224号（push→rescan）+ 225号（rescan→evaluate）。flow-continuity-guard.sh 扩展覆盖。
**状态**：✅ 已修复。

### 类型 B：纯文本输出后断裂（平台限制）

**现象**：Lead 完成分析/总结后停下来，不执行格式A的"→ 接下来"。
**根因**：Claude Code 平台的 hook 只在工具调用前后触发。纯文本输出后没有 hook 点——没有 PostOutput hook。规则层（no-unnecessary-escalation.md 格式A/B/C）对行为执行层效力有限（137号）。
**本质**：057号推论——LLM 不是状态机，不能保证每次输出都遵守格式约束。规则是概率性的引导，不是确定性的保证。
**可能的缓解**：
1. Stop hook（ceremony-completion-guard.sh）已存在——在 Lead 停止时检查是否有未完成的 ceremony 步骤
2. 但 Stop hook 只在 session 结束时触发，不在每次输出后触发
3. 平台层面无解——除非 Claude Code 增加 PostOutput hook

### 类型 C：角色边界僭越（407号精化为 C1+C2）

类型C包含两个子类型，触发条件和修复路径不同：

#### C1：主动僭越（session 内累积）

**现象**：Lead 在 session 进行中渐进式漂移，直接执行白名单外的操作（修改 v4 文本、直接 resolve 下游推论而不 spawn 工位）。
**根因**：ceremony 白名单是规则层声明，没有 hook 层强制。Lead 的 PreToolUse hook（double-helix-verify.sh）不检查 ceremony 白名单。
**特征**：渐进式，编排者纠正后可恢复。
**修复方向**：在 PreToolUse 阶段检查 Lead 是否在执行白名单外的 Edit/Write 操作。但这需要 hook 能区分 Lead 和工位——当前 hook 无法获取调用者身份。
**当前可行缓解**：ceremony skill 中更强的正面格式约束——"Lead 遇到需要修改文件的任务时，唯一合法行为是 spawn 工位"。

#### C2：compact 回归僭越（407号新增）

**现象**：autocompact 触发后，Lead 立即执行实质认知工作（读取代码、发现性能问题、进行分析），而非 spawn 工位。
**根因**：compact 压缩掉了 in-context 中 Lead 正确委托的行为示范和编排者纠正记录。session 恢复只恢复状态指针，不恢复行为模式。三层恢复不对等（声明层100%/状态层95%/行为层0%）。
**特征**：突发式，compact 恢复后立即出现。LLM 回归 RLHF 基底（看到代码就分析）。
**触发条件**：autocompact（context 达到75%阈值）→ precompact-save.sh 保存状态快照（不含行为模式）→ 行为层归零。
**修复**：session-start-ceremony.sh 注入角色边界锚点 + write_session.sh 采集纠正记录（407号修复1/2）。
**谱系依据**：407号

## 结论

| 类型 | 可修程度 | 当前状态 |
|------|---------|---------|
| A: Bash 后断裂 | 完全可修（hook 覆盖） | ✅ 已修 |
| B: 纯文本后断裂 | 平台限制，只能缓解 | ⚠ Stop hook 部分覆盖 |
| C1: 主动僭越 | 规则层可强化，hook 层受限 | ⚠ 需要 ceremony skill 强化 |
| C2: compact 回归僭越 | 基础设施可修（hook注入+session采集） | ⚠ 407号修复1-4 |

类型 B 是 137号的直接推论：否定性禁令（"不允许停下来"）对行为执行层无效，正面格式约束（"必须以格式A/B/C结尾"）也只是概率性引导。这是 LLM 作为执行层的固有限制（057号），不是可以通过规则修复的 bug。

类型 C 可以通过 ceremony skill 强化来缓解——但同样受 137号约束。

## 边界条件

- 如果 Claude Code 平台增加 PostOutput hook，类型 B 可以完全修复
- 如果 hook 能获取调用者身份（Lead vs 工位），类型 C 可以在 hook 层强制

## 下游推论

1. 编排者需要接受类型 B 的存在——这是平台限制，不是蜂群 bug。当 Lead 断掉时，编排者发送任何消息即可触发 Lead 继续（消息本身就是 wake-up 信号）
2. ceremony skill 需要增加正面格式："Lead 遇到需要修改文件的任务 → spawn 工位"（类型 C 缓解）
3. 224/225号的 hook 修复模式（Bash 后注入正面指令）是类型 A 的完整解——未来发现新的 Bash 后断裂点，用同样模式修复
4. ceremony 状态文件（`.chanlun/.ceremony-step`）成为 hook 系统的共享状态载体——消除三层无状态的根因（Codex 层1）
5. ceremony 期间的 Lead 白名单从"记录"（173号-1 方法论反转）升级为"阻断"——ceremony-step-guard.sh 与 lead-audit.sh 职责分离（Codex 层2）
6. 机械性步骤序列应脚本化为单个调用，消除 LLM 决策间隙（Codex 层3）
7. P3（分析后提问）的完整修复受平台限制（hook 无法拦截自然语言输出），记录为已知 Gap
