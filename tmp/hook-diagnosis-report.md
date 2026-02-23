# PostToolUse Hook Error 诊断报告

## 诊断结论

**根因是双层的：一个是 Claude Code 平台的已知 cosmetic bug，一个是 hook 脚本使用了非标准的输出格式。**

### 根因 1：Claude Code v2.1.50 cosmetic bug（不可修，平台侧问题）

Claude Code 在 UI 中显示 "hook error" 标签，即使 hook 脚本以 exit 0 成功退出且输出了有效 JSON。
这是一个已知的、被多次报告的 cosmetic bug：

- [#17088](https://github.com/anthropics/claude-code/issues/17088): PreToolUse hook shows 'error' label even for successful (exit 0) hook runs
- [#10463](https://github.com/anthropics/claude-code/issues/10463): "Stop hook error" displayed despite hooks producing zero output

**关键事实**：所有 hook 脚本在手动测试中全部 exit 0，无一报错。Lead 之前的手动测试结论是正确的——hook 脚本本身没有运行时错误。

### 根因 2：PostToolUse hooks 使用了非标准的 `{"decision": "allow"}` 格式

根据 [Claude Code 官方文档](https://code.claude.com/docs/en/hooks)，PostToolUse 的 decision control 只支持：

| 字段 | 有效值 | 说明 |
|------|--------|------|
| `decision` | `"block"` | 向 Claude 反馈 reason |
| （省略 decision） | — | 允许操作继续 |

即：**PostToolUse 不识别 `"decision": "allow"` 这个值**。`"allow"` 只在 PreToolUse 的 `hookSpecificOutput.permissionDecision` 中有效。

当 hook 输出 `{"decision": "allow", "reason": "..."}` 时，Claude Code 将其视为无法识别的结构化输出，在 UI 上显示为 "hook error"。

### 正确的 PostToolUse 输出格式

根据官方文档，PostToolUse hook 的合法输出方式有三种：

1. **静默通过**：`exit 0`，无 stdout 输出
2. **带上下文的通过**：输出 `{"systemMessage": "...", "suppressOutput": false}`
3. **阻断并反馈**：输出 `{"decision": "block", "reason": "..."}`
4. **带附加上下文**：输出 `{"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": "..."}}`

**不合法的输出**：`{"decision": "allow"}` — 这不是 PostToolUse 支持的 decision 值。

## 受影响的 hook 脚本

### PostToolUse hooks 输出 `{"decision": "allow"}` 的位置

| 脚本 | 行号 | 当前输出 | 修复方式 |
|------|------|---------|---------|
| `lead-audit.sh` | 133 | `{"decision": "allow"}` | 改为 `exit 0`（静默通过） |
| `lead-audit.sh` | 211-215 | `{"decision": "allow", "reason": "..."}` | 改为 `{"systemMessage": "..."}` |
| `crystallization-guard.sh` | 164-172 | `{"decision": "allow", "reason": "..."}` | 改为 `{"systemMessage": "..."}` |
| `crystallization-guard.sh` | 205-213 | `{"decision": "allow", "reason": "..."}` | 改为 `{"systemMessage": "..."}` |
| `result-package-guard.sh` | 83-87 | `{"decision": "allow", "reason": "..."}` | 改为 `{"systemMessage": "..."}` |
| `downstream-action-guard.sh` | 86-96 | `{"decision": "allow", "reason": "..."}` | 改为 `{"systemMessage": "..."}` |
| `function-length-guard.sh` | 168-178 | `{"decision": "allow", "reason": "..."}` | 改为 `{"systemMessage": "..."}` |
| `genealogy-gemini-verify.sh` | 146 | `{"decision": "allow", "reason": "..."}` | 改为 `{"systemMessage": "..."}` |

### PostToolUse hooks 使用正确格式的脚本

| 脚本 | 格式 | 状态 |
|------|------|------|
| `source-auditor-prompt.sh` | `{"continue": true, "systemMessage": "..."}` | 正确 |
| `topology-guard.sh` (manifest部分) | `{"systemMessage": "..."}` | 正确 |
| `flow-continuity-guard.sh` | `{"decision": "block", "reason": "..."}` | 正确（阻断场景） |
| `topology-guard.sh` (孤岛部分) | `{"decision": "block", "reason": "..."}` | 正确（阻断场景） |

### PreToolUse hooks 使用旧格式的脚本（也需修复）

PreToolUse 已弃用 top-level `decision`/`reason`，应改用 `hookSpecificOutput`：

| 脚本 | 行号 | 当前输出 | 修复方式 |
|------|------|---------|---------|
| `double-helix-verify.sh` | 88,129,133,134 | `{"decision": "allow"}` | 改为 `hookSpecificOutput` 格式 |
| `definition-write-guard.sh` | 69,158,241 | `{"decision": "allow", "reason": "..."}` | 改为 `hookSpecificOutput` 格式 |
| `spec-write-guard.sh` | 65 | `{"decision": "allow", "reason": "..."}` | 改为 `hookSpecificOutput` 格式 |
| `hub-node-impact-guard.sh` | 32 | `{"decision": "allow", "reason": "..."}` | 改为 `hookSpecificOutput` 格式 |
| `genealogy-write-guard.sh` | 260 | `{"decision": "allow", "reason": "..."}` | 改为 `hookSpecificOutput` 格式 |

## 修复策略

### PostToolUse hooks 修复规则

1. **非阻断场景且有信息要传递给 Claude**：用 `{"systemMessage": "..."}` 替换 `{"decision": "allow", "reason": "..."}`
2. **非阻断场景且无信息要传递**：直接 `exit 0`，不输出任何内容
3. **阻断场景**：保持 `{"decision": "block", "reason": "..."}`（这是正确的）

### PreToolUse hooks 修复规则

将 `{"decision": "allow", "reason": "..."}` 替换为：
```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "permissionDecisionReason": "..."
  }
}
```

### 不修复的部分

Claude Code 平台的 cosmetic bug（exit 0 无输出时仍显示 "hook error" 标签）——这需要 Anthropic 修复平台代码。但修复输出格式后，应能消除大部分 "hook error" 显示。

## 边界条件

- 如果 Claude Code 更新后修复了 cosmetic bug，本次修复仍然有效（使用标准格式是正确的）
- 如果 Claude Code 在某个版本恢复对 `{"decision": "allow"}` 的识别，本次修复仍然有效（`systemMessage` 是文档化的标准字段）
- 本修复不改变任何 hook 的语义（阻断/放行逻辑不变），只改变输出格式

## 影响声明

修改所有 PostToolUse hook 脚本的输出格式，从非标准的 `{"decision": "allow"}` 改为标准格式。
修改所有 PreToolUse hook 脚本的输出格式，从弃用的 top-level `decision`/`reason` 改为 `hookSpecificOutput` 格式。
不影响任何 hook 的触发条件、阻断逻辑或业务语义。

## 修复状态：已完成

### 已修复的文件

**PostToolUse hooks**（`{"decision":"allow"}` → `{"systemMessage":"..."}` 或 `exit 0`）：
1. `.claude/hooks/lead-audit.sh` — 白名单场景改为静默 `exit 0`；审计消息改为 `systemMessage`
2. `.claude/hooks/crystallization-guard.sh` — 两处 warning 改为 `systemMessage`
3. `.claude/hooks/result-package-guard.sh` — 缺失字段警告改为 `systemMessage`
4. `.claude/hooks/downstream-action-guard.sh` — 二阶反馈改为 `systemMessage`
5. `.claude/hooks/function-length-guard.sh` — 熔断放行警告改为 `systemMessage`
6. `.claude/hooks/genealogy-gemini-verify.sh` — allow 消息改为 `systemMessage`

**PreToolUse hooks**（`{"decision":"allow/block"}` → `hookSpecificOutput` 格式）：
7. `.claude/hooks/definition-write-guard.sh` — 三处输出全部改为 `hookSpecificOutput`
8. `.claude/hooks/spec-write-guard.sh` — 核心文件警告改为 `hookSpecificOutput`
9. `.claude/hooks/hub-node-impact-guard.sh` — Hub 节点影响链改为 `hookSpecificOutput`
10. `.claude/hooks/genealogy-write-guard.sh` — 谱系写入警告改为 `hookSpecificOutput`
11. `.claude/hooks/double-helix-verify.sh` — 所有输出改为 `hookSpecificOutput`（block→deny, allow→allow）
12. `.claude/hooks/agent-team-enforce.sh` — Task 阻断改为 `hookSpecificOutput` deny

### 验证结果

所有修复后的 hook 脚本均通过手动测试（JSON 管道输入 + exit code 检查）。
