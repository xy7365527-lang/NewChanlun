# 全面体系孤岛和声明-能力缺口审计 — Gemini 质询上下文

## 任务说明

对新缠论蜂群体系的 20 个 hooks、19 个 agents、12 个 skills 进行完全严格审计，发现孤岛（声明存在但未连接的组件）和声明-能力缺口（宣称有某功能但实际未实现）。

## 已确认的关键事实（预扫描结论）

### 关键文件存在性
- `.chanlun/.crystallization-debt.json`: **不存在**（crystallization-guard 的债务机制无法激活）
- `src/newchan/skills/manifest.yaml`: **不存在**（topology-guard.sh 的 ROOT_NODES 包含此路径，BFS 从不可达根节点出发）
- `.chanlun/manifest.yaml`: **存在**（正确路径）
- `.chanlun/dispatch-dag.yaml`: **存在**
- `.chanlun/pattern-buffer.yaml`: **存在**
- `.chanlun/genealogy/dag.yaml`: **存在**
- `scripts/chanlun/validate_dag.py`: **存在**
- `scripts/downstream_audit.py`: **存在**
- `.claude/settings.local.json`: **存在**（当前内容为 restrict 模式，只允许 WebFetch/WebSearch，deny 为空）

### Windows 平台问题
- `python`: 可用（Python 3.14.3，`/c/Python314/python`）
- `python3`: **不可用**（Windows Store alias，实际无法执行）
- `precompact-save.sh` 在第 15 行使用 `python3`，会**静默失败**，fallback 到 `echo "."` 作为 cwd
- `precompact-save.sh` 第 18 行硬编码 `cd /home/user/NewChanlun`，这是 Linux 路径，在 Windows 上不存在

### settings.json 中注册的 hooks（已确认）
```
SessionStart: session-start-ceremony.sh
PreCompact: precompact-save.sh
PreToolUse(Task): ceremony-guard.sh
PreToolUse(Bash): double-helix-verify.sh
PreToolUse(Write): definition-write-guard.sh, genealogy-write-guard.sh, spec-write-guard.sh, hub-node-impact-guard.sh
PreToolUse(Edit): definition-write-guard.sh, genealogy-write-guard.sh, spec-write-guard.sh, hub-node-impact-guard.sh
PostToolUse(TaskUpdate): recursive-guard.sh
PostToolUse(Bash): flow-continuity-guard.sh, crystallization-guard.sh, topology-guard.sh
PostToolUse(Write): result-package-guard.sh, downstream-action-guard.sh
PostToolUse(Edit): result-package-guard.sh, downstream-action-guard.sh
PostToolUse(TeamCreate): team-structural-inject.sh
Stop: ceremony-completion-guard.sh, post-session-pattern-detect.sh, meta-observer-guard.sh
```

### 未注册 hooks（不在 settings.json 中）
- `lead-permissions.sh`: 不在自动 hook 注册中，需手动调用（设计如此，但功能实际被 settings.json 覆盖）
- `dag-validation-guard.sh`: 不在 settings.json 的任何 hook 触发器中！实际上是一个孤岛文件

**注意**: dag-validation-guard.sh 文件存在于 .claude/hooks/ 目录，但 settings.json 的 PostToolUse(Write/Edit) 列表中没有它——它只有 result-package-guard.sh 和 downstream-action-guard.sh。

### 输出格式问题
- `flow-continuity-guard.sh`: PostToolUse hook 输出 `{"decision": "block", "reason": "..."}` — 这是 PreToolUse 的格式，PostToolUse 的正确格式应为 `{"continue": true/false, "suppressOutput": bool, "systemMessage": "..."}`
- `hub-node-impact-guard.sh`: PreToolUse hook，但输出的是纯文本（`echo "⚠ Hub 节点修改..."` 直接 echo，不是 JSON），然后 `exit 0`。没有输出任何 JSON，hooks 框架可能忽略这些输出。
- `ceremony-completion-guard.sh`（Stop hook）: 输出 `{"decision": "block", "reason": "..."}` — Stop hook 应该用 `{"continue": false}` 格式

### settings.local.json vs settings.json 的权限冲突
- `settings.json` 的 `permissions.allow` 包含 `["Read", "Edit", "MultiEdit", "Write", "Grep", "Glob", "Task", "Bash(*)", ...]` — **允许一切**
- `settings.local.json` 当前内容为 restrict 模式：只允许 WebFetch/WebSearch，deny 为空
- 这意味着 restrict 模式下：写入、bash、edit 等操作没有被显式 deny（deny 数组为空）
- 实际上 restrict 模式可能不工作：因为 settings.json 中已经 allow 了一切，settings.local.json 的 allow 列表只有 WebFetch/WebSearch，但没有 deny 掉其他
- lead-permissions.sh 的 restrict 模式设计意图（剥夺写入权限）实际上未能实现，因为全局 settings.json allow 列表更宽

### meta-observer-guard.sh hotfix
- 默认模式已改为"自动落标+放行"（STRICT_MODE 默认为 0）
- 效果：二阶反馈机制**形同虚设**，每次 Stop 时自动打标记然后放行

### ceremony-guard.sh（空壳）
- 直接输出 `{"continue": true}` 然后退出
- 注释说"075号后直接放行"，功能已被废弃

### crystallization-guard.sh 债务机制
- 检查 `.chanlun/.crystallization-debt.json`，但该文件**不存在**
- pattern-buffer.yaml 存在，所以 pattern 检查路径（frequency >= 3）会执行
- 但 debt 文件这条路径永远不会触发

### topology-guard.sh ROOT_NODES 错误
```python
ROOT_NODES = [
    '.chanlun/dispatch-dag.yaml',
    'CLAUDE.md',
    'src/newchan/skills/manifest.yaml',  # 这个路径不存在！
]
```
- `src/newchan/skills/manifest.yaml` 不存在，正确路径是 `.chanlun/manifest.yaml`
- BFS 从 3 个根节点出发，其中 1 个（src/newchan/skills/manifest.yaml）永远不在 all_files 集合中
- 实际只从 2 个根节点出发，可达集合偏小，孤岛检测会产生误报

### pattern-buffer 自生长回路（043号）
- post-session-pattern-detect.sh: Stop hook，从 session 文件提取 tool 调用序列
- 但 session 文件（.chanlun/sessions/）是由 precompact-save.sh 写入的
- precompact-save.sh 在 Windows 上失败（python3 不可用）
- 如果 precompact-save.sh 失败 → 没有 session 文件 → post-session-pattern-detect.sh 静默退出 → pattern-buffer 从未更新
- 整个自生长回路在 Windows 上**断裂**

### 082号谱系确认的已知问题
- 075号声明事件驱动架构，但没有实际 event dispatcher
- 结构 skill 的 agent 文件存在（.claude/agents/*.md），但从未被自动触发
- D策略已决断：hooks 层检测+提示，Lead 作为调度器响应（诚实降级）

## 各组件详情

### Hooks 详情

**precompact-save.sh**
- 注册：✅ PreCompact hook
- python3 调用（第15行）：Windows 失败
- 硬编码路径 `/home/user/NewChanlun`（第18行）：Windows 不存在
- 实际功能：Windows 上很可能完全失败，无法保存 session

**session-start-ceremony.sh**
- 注册：✅ SessionStart hook
- 使用 `python`（正确，Windows 上有）
- 功能：冷启动/热启动 bootstrap，检测 session 文件

**ceremony-guard.sh**
- 注册：✅ PreToolUse(Task)
- 功能：直接 `{"continue": true}`，完全空壳

**double-helix-verify.sh**
- 注册：✅ PreToolUse(Bash)
- 功能：git commit 前调用 Gemini verify（通过 Python 的 newchan.gemini.modes.decide）
- 依赖：GOOGLE_API_KEY 环境变量，newchan 包

**definition-write-guard.sh**
- 注册：✅ PreToolUse(Write/Edit)
- 功能：验证定义文件格式
- 问题：decision: "allow" 不是 block，实际上不阻断，只警告

**genealogy-write-guard.sh**
- 注册：✅ PreToolUse(Write/Edit)
- 功能：验证谱系强制字段
- 问题：二阶反馈检查查找 ~/.claude/teams/ 的 config.json，这个路径在实际环境中可能不存在

**spec-write-guard.sh**
- 注册：✅ PreToolUse(Write/Edit)
- 功能：记录核心配置文件修改，allow+记录
- 问题：只对已存在的文件有效（os.path.exists 检查）

**hub-node-impact-guard.sh**
- 注册：✅ PreToolUse(Write/Edit)
- 功能：Hub 节点影响链警告
- 格式问题：输出纯文本 echo（不是 JSON），然后 exit 0。Claude Code 的 hooks 框架期望 JSON，纯文本 stdout 可能被忽略

**flow-continuity-guard.sh**
- 注册：✅ PostToolUse(Bash)
- 格式问题：输出 `{"decision": "block", "reason": "..."}` 但这是 PostToolUse hook，正确格式应该是 `{"continue": false, "systemMessage": "..."}`

**crystallization-guard.sh**
- 注册：✅ PostToolUse(Bash)
- debt 机制：`.chanlun/.crystallization-debt.json` 不存在，debt 路径永远不触发
- pattern 路径：pattern-buffer.yaml 存在，但这条路径依赖 session 文件（Windows 上 precompact 失败）

**topology-guard.sh**
- 注册：✅ PostToolUse(Bash)
- ROOT_NODES 错误：`src/newchan/skills/manifest.yaml` 不存在
- manifest 检查路径：使用 `MANIFEST_PATH = '.chanlun/manifest.yaml'`（正确路径！），但 BFS 部分仍用错误路径

**recursive-guard.sh**
- 注册：✅ PostToolUse(TaskUpdate)
- 功能：检查重构任务完成后的函数行数
- 输出格式：正确（continue: true + systemMessage）

**result-package-guard.sh**
- 注册：✅ PostToolUse(Write/Edit)
- 功能：检查谱系文件的六要素字段
- 输出：allow + 警告（不阻断）

**downstream-action-guard.sh**
- 注册：✅ PostToolUse(Write/Edit)
- 功能：运行 downstream_audit.py，报告下游行动
- downstream_audit.py 存在

**team-structural-inject.sh**
- 注册：✅ PostToolUse(TeamCreate)
- 功能：从 dispatch-dag.yaml 读取 event_skill_map 的 structural skill，输出提示
- 这是 D策略的实现：hooks 层提示，Lead 手动执行 skill

**ceremony-completion-guard.sh**
- 注册：✅ Stop
- 格式问题：Stop hook 输出 `{"decision": "block", "reason": "..."}` — Stop hook 正确格式应该是 `{"continue": false}` 或 `{"stopReason": "..."}`

**meta-observer-guard.sh**
- 注册：✅ Stop
- 默认 hotfix 模式：STRICT_MODE=0，自动落标放行，**形同虚设**

**post-session-pattern-detect.sh**
- 注册：✅ Stop
- 功能：提取 session 文件的 tool 序列，写 pattern-buffer
- 依赖 precompact-save.sh 写的 session 文件
- Windows 上整个链路断裂

**lead-permissions.sh**
- 未注册（手动调用）
- 功能设计：修改 settings.local.json 切换权限
- 实际问题：restrict 模式下 deny 数组为空，全局 settings.json allow 一切，实际效果可疑

**dag-validation-guard.sh**
- **未注册**（不在 settings.json 中的任何 hook 触发器）
- 这是一个孤岛文件，永远不会被自动触发

## 审计问题

请对以上事实进行完整分析，回答：

1. 对每个 hook，判断其真实功能状态（正常/退化/孤岛）
2. 孤岛判定：是应删除、应修复、还是应明确废弃
3. precompact → session → pattern-buffer 链路在 Windows 上的断裂影响面
4. topology-guard 的错误 ROOT_NODES 的实际影响（误报率）
5. flow-continuity-guard 和 ceremony-completion-guard 的格式问题是否实际导致功能失效
6. hub-node-impact-guard 的纯文本输出是否被框架识别
7. lead-permissions.sh 的权限控制是否真的有效
8. 整体评估：体系中有多少组件处于"声明存在但实际不工作"状态
