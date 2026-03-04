# Hooks/配置声明-能力缺口审计报告（v31-swarm 子蜂群A）

**日期**: 2026-02-22
**审计范围**: `.claude/hooks/*.sh` (22 files) + `.claude/settings.json` + `.claude/settings.local.json` + `.chanlun/dispatch-dag.yaml`

## 审计方法

逐一读取每个 hook 文件，验证三个维度的一致性：
1. **声明功能**（文件注释/文件名暗示） vs **实际能力**（脚本逻辑）
2. **输出格式** vs **Claude Code Hook 规范**（PreToolUse/PostToolUse/Stop 各自的标准格式）
3. **settings.json 注册** vs **实际文件存在性/类型匹配**

## 发现缺口与修复

### 已修复（5 项）

| # | 文件 | 缺口描述 | 路径 | 修复内容 |
|---|------|---------|------|---------|
| 1 | `ceremony-guard.sh` | 废弃文件（exit 0）仍注册在 settings.json PreToolUse/Task matcher 中 | B | 从 settings.json 移除注册，更新文件注释 |
| 2 | `hub-node-impact-guard.sh` | 使用 grep/sed 解析 JSON（对含特殊字符的 file_path 不安全） | A | 改用 python json.load 解析，与其他 hooks 统一 |
| 3 | `lead-audit.sh` | (a) `MSG` 变量未通过 export 传递给 python，导致 `os.environ['MSG']` 可能 KeyError；(b) PostToolUse 输出混用 `decision`+`systemMessage` 字段 | A | 改用 `MSG="$MSG" python` 行内赋值语法；输出改为标准 `decision`+`reason` 格式 |
| 4 | `function-length-guard.sh` | PostToolUse hook 输出 `{"continue":true,"suppressOutput":false,"systemMessage":...}` — 这是 PreCompact/SessionStart 格式，PostToolUse 不识别 | A | 非匹配时静默 exit 0；匹配时改用 `{"decision":"block/allow","reason":...}` 格式 |
| 5 | `precompact-save.sh` | 最后一行用 bash 字符串拼接生成 JSON（`echo "{...\"${MSG}\"}"`），但 MSG 可含引号导致畸形 JSON。脚本自身定义了安全的 `emit_json()` 函数却未使用 | A | 改用 `emit_json "$MSG"` |

### 未修复（低优先级/非缺口，3 项）

| # | 文件 | 描述 | 理由 |
|---|------|------|------|
| 6 | Stop hooks (ceremony-completion-guard, meta-observer-guard) | 输出到 stdout 而非 stderr | Claude Code 实践中 stdout 也能工作，不影响功能 |
| 7 | `post-session-pattern-detect.sh` | 无 decision 输出给 Claude Code | 设计意图是静默 side-effect（写 pattern-buffer），Stop hook 不强制输出 |
| 8 | settings.local.json lead-audit matcher `Write|Edit|Bash` | 对 Bash 调用也触发审计 | 设计意图（Lead 直接执行 Bash 也需记录拓扑异常） |

### 无缺口（14 项，通过审计）

| 文件 | 类型 | 声明-能力一致性 |
|------|------|----------------|
| `ceremony-completion-guard.sh` | Stop | 5 项检查逻辑完整，熔断机制正确 |
| `crystallization-guard.sh` | PostToolUse/Bash | git commit 拦截+债务检查+pattern-buffer 检查，strict/light 双模式 |
| `dag-validation-guard.sh` | PostToolUse/Write,Edit | 谱系写入后调用 validate_dag.py，脚本存在 |
| `double-helix-verify.sh` | PreToolUse/Bash | git commit 前 Gemini 验证，熔断+死锁保护完整 |
| `downstream-action-guard.sh` | PostToolUse/Write,Edit | settled 谱系写入后调用 downstream_audit.py --summary，脚本和参数均有效 |
| `result-package-guard.sh` | PostToolUse/Write,Edit | 谱系文件三要素检查，allow + 警告模式 |
| `session-start-ceremony.sh` | SessionStart | 冷/热启动检测+状态注入，emit_json 安全输出 |
| `spec-write-guard.sh` | PreToolUse/Write,Edit | 核心文件写入记录+放行，原则0 合规 |
| `meta-observer-guard.sh` | Stop | advisory/strict 双模式，熔断正确 |
| `source-auditor-prompt.sh` | PostToolUse/Write,Edit | docs/ 路径检测+advisory 提示，环境变量传递语法合法 |
| `definition-write-guard.sh` | PreToolUse/Write,Edit | status/version 字段验证+熔断，Edit 谱系交叉检查 |
| `genealogy-write-guard.sh` | PreToolUse/Write,Edit | 4 类验证（字段/引用/meta-observer/语义），全部 allow + 警告 |
| `genealogy-gemini-verify.sh` | PostToolUse/Write,Edit | 调用 gemini_genealogy_verify_prompt.py 生成审查上下文，脚本存在 |
| `agent-team-enforce.sh` | PreToolUse/Task | Task 调用强制 team_name 参数，096号无例外 |
| `flow-continuity-guard.sh` | PostToolUse/Bash | git commit 后注入继续指令，[FINAL] 边界条件 |
| `topology-guard.sh` | PostToolUse/Bash | BFS 孤岛检测+manifest 注册检查 |
| `post-session-pattern-detect.sh` | Stop | 模式检测+跨 session 合并，幂等 |

## dispatch-dag.yaml 对照

dispatch-dag 声明的 hook 检查点 vs 实际实现：

| dispatch-dag 声明 | 对应 hook | 状态 |
|-------------------|----------|------|
| `immune_system` (.claude/hooks/) | 全部 22 个 hook 文件 | 存在 |
| `quality_gate` (file_write hook 自动触发) | result-package-guard + downstream-action-guard | 已注册 |
| `meta_observation` (session_end 触发) | meta-observer-guard.sh | 已注册 |
| `lead_audit_registered` (PostToolUse hook) | lead-audit.sh (settings.local.json) | 已注册 |
| event: session_start → ceremony | session-start-ceremony.sh | 已注册 |
| event: session_end → pattern_detector | post-session-pattern-detect.sh | 已注册 |

## 发现模式

1. **格式不统一**：PostToolUse hooks 中 3 个文件使用了非标准输出格式（continue/suppressOutput/systemMessage），应统一为 decision/reason。已修复。
2. **废弃文件未清理**：ceremony-guard.sh 已废弃但仍注册。这类"保留避免引用断裂"的模式容易累积技术债。已修复。
3. **JSON 生成安全性**：2 个文件使用不安全的 bash 字符串拼接生成 JSON。应统一使用 python json.dumps 或已有的 emit_json 辅助函数。已修复 1 个（precompact-save），hub-node-impact-guard 的 JSON 生成已通过改用 python 间接解决。
4. **环境变量传递**：lead-audit.sh 使用裸变量赋值而非行内赋值语法，可能在部分 shell 环境下失效。已修复。
