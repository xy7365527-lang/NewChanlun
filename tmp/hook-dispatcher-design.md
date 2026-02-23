# Hook 脚本统一 Dispatcher 设计方案

## 来源
173号-3 下游推论：当前多个 hook 脚本各自启动独立 Python 进程，评估统一 dispatcher 可行性

## 现状分析

### 已有的部分统一
- `pre-write-edit-dispatcher.sh`：已合并 4 个 PreToolUse Write/Edit hook（definition-write-guard + genealogy-write-guard + spec-write-guard + hub-node-impact-guard）
- `post-write-edit-dispatcher.sh`：已合并若干 PostToolUse Write/Edit hook

### 当前 hook 清单（25 个 .sh 文件）

| Hook | 触发类型 | 是否已被 dispatcher 合并 |
|------|---------|------------------------|
| pre-write-edit-dispatcher.sh | PreToolUse Write/Edit | 本身就是 dispatcher |
| post-write-edit-dispatcher.sh | PostToolUse Write/Edit | 本身就是 dispatcher |
| definition-write-guard.sh | PreToolUse Write/Edit | 已合并入 pre-write-edit-dispatcher |
| genealogy-write-guard.sh | PreToolUse Write/Edit | 已合并入 pre-write-edit-dispatcher |
| spec-write-guard.sh | PreToolUse Write/Edit | 已合并入 pre-write-edit-dispatcher |
| hub-node-impact-guard.sh | PreToolUse Write/Edit | 已合并入 pre-write-edit-dispatcher |
| agent-team-bootstrap.sh | SessionStart | 独立 |
| agent-team-enforce.sh | PreToolUse Task | 独立 |
| ceremony-completion-guard.sh | Stop | 独立 |
| crystallization-guard.sh | PostToolUse Write/Edit | 可能已合并入 post-dispatcher |
| dag-validation-guard.sh | PostToolUse Bash | 独立 |
| double-helix-verify.sh | PostToolUse Write/Edit | 可能已合并入 post-dispatcher |
| downstream-action-guard.sh | Bash (git commit) | 独立 |
| flow-continuity-guard.sh | PostToolUse Write/Edit | 可能已合并入 post-dispatcher |
| function-length-guard.sh | PostToolUse Write/Edit | 可能已合并入 post-dispatcher |
| genealogy-gemini-verify.sh | PostToolUse TaskUpdate | 独立 |
| lead-audit.sh | PostToolUse Write/Edit/Bash | 独立 |
| meta-observer-guard.sh | Stop | 独立 |
| post-session-pattern-detect.sh | Stop | 独立 |
| precompact-save.sh | Stop | 独立 |
| result-package-guard.sh | PostToolUse Write/Edit | 可能已合并入 post-dispatcher |
| session-start-ceremony.sh | SessionStart | 独立 |
| source-auditor-prompt.sh | PostToolUse Write/Edit | 可能已合并入 post-dispatcher |
| topology-guard.sh | PostToolUse Bash (git commit) | 独立 |
| topology-mutator-prompt.sh | PostToolUse Bash | 独立 |

### 分类统计
- **已合并（Pre Write/Edit）**: 4 个 → 1 个 dispatcher
- **已合并（Post Write/Edit）**: ~6 个 → 1 个 dispatcher
- **独立运行**: ~14 个
- **触发类型分布**: SessionStart(2), PreToolUse(2 dispatchers), PostToolUse(~10 independent), Stop(4)

## 可行性评估

### 方案A：全局单一 dispatcher.py
- 所有 hook 合并为一个 Python 入口
- bash 只做路由（根据 hook_event + tool_name 调用不同 Python 函数）
- **优势**：消除 25 个 bash 脚本 + 多次 Python 启动开销
- **劣势**：单点故障；Claude Code 的 hook 配置（`.claude/settings.local.json`）需要每个 hook 独立注册，全局 dispatcher 不改变注册数量

### 方案B：按触发类型分组 dispatcher
- 已有的 pre-write-edit-dispatcher 和 post-write-edit-dispatcher 模式扩展到其他类型：
  - `pre-bash-dispatcher.sh`：合并 downstream-action-guard
  - `post-bash-dispatcher.sh`：合并 dag-validation-guard + topology-guard + topology-mutator-prompt + lead-audit(Bash部分)
  - `stop-dispatcher.sh`：合并 ceremony-completion-guard + meta-observer-guard + post-session-pattern-detect + precompact-save
- **优势**：渐进式，每个 dispatcher 独立可测试
- **劣势**：仍有多个 bash 入口

### 方案C：保持现状 + 增量优化
- 已有的两个 dispatcher 已覆盖了最高频的 Write/Edit hook
- Stop 类 hook 每 session 只执行一次，无性能压力
- 独立 Bash hook 触发条件已通过快速前置检查（case 语句）过滤
- **优势**：无迁移风险
- **劣势**：代码重复（resolve_python 等）

## 建议

**方案B**，优先级：
1. `stop-dispatcher.sh`（4 个 Stop hook 合并——每个都启动 Python，session 结束时串行执行 4 次 Python 启动可优化为 1 次）
2. `post-bash-dispatcher.sh`（4 个 PostToolUse Bash hook 合并——git commit 后的 hook 串行执行开销可感知）
3. Pre Bash hook 目前只有 1 个（downstream-action-guard），无需 dispatcher

## 实现前置条件
- 确认 Claude Code hook 配置允许同一个 hook 脚本处理多个子检查
- 当前 pre-write-edit-dispatcher.sh 的模式已验证可行

## 影响范围
- 减少 Python 启动次数（Stop: 4→1, Post Bash: 4→1）
- 不影响 hook 功能（纯粹的性能优化）
- 需要更新 `.claude/settings.local.json` 的 hook 注册
