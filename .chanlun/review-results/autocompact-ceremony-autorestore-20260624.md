# autocompact 后 ceremony 自动恢复中断点修复

工位：infra（task #55）| 日期：2026-06-24 | 认识论等级：L2（真实 transcript 实证 + 平台已知 bug 交叉确认）

---

## 1. 结论

**根因不是"autocompact 不触发 SessionStart"，而是 Claude Code 平台已知 bug #15174：SessionStart(source=compact) hook 会执行，但其 `additionalContext` / stdout 不被注入压缩后的模型 context（auto 与 manual compact 均如此）。**

task 的根因假设（"autocompact 不 fire SessionStart"）被官方文档 + GitHub issue 双向否证：
- 官方文档（code.claude.com/docs/en/hooks）明确：SessionStart 的 `source="compact"` 覆盖 **"Auto or manual compaction"**，PreCompact 的 `trigger` ∈ {manual, auto} 两路都 fire。
- 真正的失效是 **#15174**：hook 执行了但输出"discretely added"，不进 compact 后 context。

修复（三处协同，覆盖 auto + manual 两条路径，绕过 #15174）：

| 文件 | 改动 | 角色 |
|------|------|------|
| `.claude/hooks/precompact-save.sh` | 读 `trigger` 字段；compact 时落 `.chanlun/.pending-ceremony-restore` 标记（记 trigger） | **触发前提**（PreCompact auto+manual 都 fire，未受 #15174 影响） |
| `.claude/hooks/userpromptsubmit-ceremony-restore.sh`（**新增**） | 检测标记 → 经 `hookSpecificOutput.additionalContext` 注入 ceremony 恢复指令 → 删标记（一次性） | **注入前提**（UserPromptSubmit "always fires"，additionalContext 经 system-reminder 进下一次 model request，未受 #15174 影响） |
| `.claude/settings.json` | 注册 `UserPromptSubmit` → 新 hook | 接线 |

两前提共同构成确定性：compact 后用户的**下一条消息**必定可靠触发 ceremony 恢复（invoke /ceremony 序列：`ceremony_state.py write 1 initial` → `ceremony_scan.py` → 并行 spawn 工位恢复中断点）。

### 验证证据（L2）

实证 transcript `14c95478`（30M Lead session，Stop-Guard 注释引用的同一 session）：**7 次 compact_boundary，0 次 ceremony 注入痕迹**，尽管 `session-start-ceremony.sh` 的 compact 分支输出正确（已实测 `additionalContext` 含 `[Ceremony/compact恢复]` + `ceremony_scan`）。这是 #15174 的直接证据——hook 输出正确，但从未到达 compact 后 context。

新机制端到端测试（4/4 通过）：
- T1 autocompact：PreCompact(trigger=auto) 落标记 → UserPromptSubmit 注入含 `ceremony_scan` + `trigger=auto` + `bug#15174` 标注 + 角色边界 + 中断点快照，标记被消费删除 ✓
- T2 手动 compact：trigger=manual 同样注入成功 ✓
- T3 幂等：无标记静默 exit 0，不重复注入 ✓
- T4 非蜂群目录：无 `.chanlun` 静默 ✓
- 语法：`bash -n` 三文件全 OK；settings.json 合法 JSON，UserPromptSubmit 已注册。

---

## 2. 定义依据

权威 hook 行为（出处 + 原文）：

- **SessionStart `source`**（code.claude.com/docs/en/hooks）："startup / resume / clear / compact"；`compact` = **"Auto or manual compaction"**。→ autocompact 后 SessionStart **确实 fire**，否证 task 假设。
- **PreCompact `trigger`**：{manual, auto}，"PreCompact fires on both automatic and manual compaction"。→ PreCompact 是覆盖两路的确定性触发点。
- **PreCompact 输出能力**：仅支持 top-level `decision`/`reason` + 通用 `systemMessage`，**不支持 `additionalContext`**（文档明确 additionalContext 仅 Stop/SubagentStop 在 PreCompact 同组中被点名，PreCompact 不在其列）。→ PreCompact **不能**直接注入 compact 后 context，只能落标记。
- **UserPromptSubmit 输出**："String passed from your hook into Claude's context window. Claude Code wraps the string in a system reminder and inserts it into the conversation at the point where the hook fired. Claude reads the reminder on the next model request"；"always fires on every occurrence"（无 matcher）。→ 可靠注入通道。
- **平台 bug #15174**（github.com/anthropics/claude-code/issues/15174）："SessionStart hook with 'compact' matcher output not injected into context"——hook 执行但 stdout/additionalContext 不进 compact 后 context，JSON 与纯 stdout 两种格式皆然。

输入数据满足定义的哪些条件：transcript 的 `compactMetadata.trigger` 字段（实测全 7 次 = "manual"，结构完整含 preTokens/postTokens）证实 compact 真实发生；compact_boundary 后序列只有 PreCompact stdout 痕迹、无 SessionStart additionalContext 痕迹，精确匹配 #15174 的报告症状。

---

## 3. 边界条件（结论翻转条件）

1. **若平台修复 #15174**：SessionStart compact 注入恢复生效，则本机制与 `session-start-ceremony.sh` 的 compact 分支**双重注入**（无害——UserPromptSubmit 仍一次性消费标记，重复内容由模型去重；可后续删 UserPromptSubmit hook 简化）。结论不翻转，只是冗余。
2. **compact 后无任何用户消息**（纯 teammate 驱动的 Lead 自循环，无 UserPromptSubmit）：本 hook **不 fire**，标记残留至下次用户消息。此场景由 Stop hook（`ceremony-completion-guard.sh`）的常设结构工位强制 + check0 context 临界放行兜底——compact 后蜂群状态由文件系统持久化，Stop-Guard 检测缺失工位会 block 注入 spawn 指令，间接强制 ceremony-等价行为。**本 hook 覆盖"有用户交互"路径，Stop-Guard 覆盖"纯自循环"路径，二者正交互补。** 这是能力边界的诚实声明，不是 workaround。
3. **若 `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`（当前=80）使 autocompact 永不触发**（issue #63015 报告的 statusline 100% 不 compact）：则无 compact 事件，本机制不被调用，但问题也不存在（无 compact = 无中断点丢失）。该场景与本修复正交。
4. **UserPromptSubmit additionalContext 本身的可靠性**：issue #43733 指出"即使注入了，Claude 是否可靠据此行动是 hit-or-miss"。本修复用 137号正面格式（声明"第一动作=invoke /ceremony"）+ 角色边界锚点最大化据此行动的概率，但**无法 100% 保证模型行为**——这是 LLM 行为层的固有概率性（137号：否定性禁令对行为层无效，正面格式是缓解非消除）。

---

## 4. 下游推论

- `session-start-ceremony.sh` 的 compact 分支（第199-203行）**不再是 compact 恢复的唯一依赖**——它仍保留（startup/resume/clear 路径有效 + 若 #15174 修复则 compact 路径恢复），但 compact 后的实际恢复由 UserPromptSubmit 路径承担。该文件第38行注释"空 matcher 已覆盖全部 source（含 compact）"在"覆盖 = hook 被调用"意义上正确，但在"覆盖 = 注入生效"意义上**被 #15174 证伪**——已在新 hook 注释中记录此区分。
- `.chanlun/` 新增一个生命周期标记文件 `.pending-ceremony-restore`（与现有 `.ceremony-in-progress` 等同惯例），由 PreCompact 落、UserPromptSubmit 消费，一次性。
- Lead 需在 git commit 时纳入 `.claude/settings.json`（敏感文件，按 task 约束由 Lead 统一 commit）。

---

## 5. 谱系引用

- **407号**（compact 后行为层恢复依赖 session-start-ceremony 注入角色锚点）：本修复揭示 407 的隐含前提"SessionStart compact 注入有效"被 #15174 证伪——407 的恢复机制在 compact 路径上实际由 UserPromptSubmit 承载，应作为 407 的修订记录。建议 genealogist 评估是否结晶为新谱系条目（"声明的注入通道 ≠ 实际生效通道"，与 spec-execution-gap 016→034 链同构：声明 SessionStart 覆盖 compact，实际能力被平台 bug 阉割）。
- **137号**（否定性禁令对行为层无效 → 正面输出格式）：新 hook 用正面格式"本回合第一个动作=invoke /ceremony"声明，继承 137 的设计。
- **224/225号**（ceremony push→rescan→evaluate 原子链）：本修复保证 compact 后能重新进入 ceremony 序列，是原子链在 compact 边界的延续。
- 不确定是否有"hook 注入通道有效性"的既有谱系——明确声明：未检索到直接对应条目，建议 genealogist 检查是否与 spec-execution-gap 链合并。

---

## 6. 影响声明

**改动**：
- 新增 `.claude/hooks/userpromptsubmit-ceremony-restore.sh`（UserPromptSubmit hook，compact 后 ceremony 恢复注入）
- 修改 `.claude/hooks/precompact-save.sh`（读 trigger + 落 `.pending-ceremony-restore` 标记）
- 修改 `.claude/settings.json`（注册 UserPromptSubmit → 新 hook）

**影响模块**：compact 后恢复链（SessionStart compact 分支不再唯一依赖）、`.chanlun/` 标记文件惯例（+1）、Lead session 的 ceremony 自动恢复行为。

**不改动**：`session-start-ceremony.sh`（保留，双保险）、`ceremony-completion-guard.sh`（Stop-Guard 兜底路径不变）、任何业务/定义代码。

**未 commit**（按 task 约束，settings.json 敏感，Lead 统一 commit）。

---

## 附：这是平台缺口还是真修复？

**是真修复，不是平台缺口的假装修好。** 平台缺口（#15174）是**真实存在**的——SessionStart compact 注入通道被掐死，这条路无法在我们侧修复（不能改平台）。但修复**没有停留在那条死路上**：通过 PreCompact（落标记，未受 bug 影响）+ UserPromptSubmit（注入，未受 bug 影响）的确定性双前提，**绕过**了死通道，在"有用户交互"路径上确定性恢复 ceremony。诚实的能力边界：纯自循环无用户消息的场景由 Stop-Guard 兜底，不在本 hook 覆盖内（已声明，非隐瞒）。
