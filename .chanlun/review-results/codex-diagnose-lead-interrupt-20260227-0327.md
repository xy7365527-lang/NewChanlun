# Lead 中断诊断报告（Codex 工位）

日期：2026-02-27
工位：interrupt-diagnosis
诊断模型：claude-opus-4-6

---

## 一、现象清单

| # | 现象 | 发生窗口 | 224/225 是否覆盖 |
|---|------|---------|-----------------|
| P1 | push 后不 rescan | Bash(git push) → 下一步 | ✅ 224号 hook 覆盖 |
| P2 | rescan 后不 evaluate workstations | Bash(ceremony_scan.py) → 下一步 | ✅ 225号 hook 覆盖 |
| P3 | 质询收敛分析后问编排者"你要我修还是先讨论" | Read/分析 → 下一步 | ❌ 无守卫 |
| P4 | Lead 直接修改 v4 文本（僭越白名单） | Write/Edit 调用 | ❌ lead-audit 记录但不阻断 |

P1/P2 已有 hook 层修复。P3/P4 是当前活跃问题。

---

## 二、根因分析

### 2.1 表层原因：hook 覆盖不完整

flow-continuity-guard.sh 只匹配 Bash 工具调用中的三种命令（git commit / git push / ceremony_scan.py）。Lead 的"断掉"不只发生在 Bash 后——P3 发生在 Read/Glob/Grep 后（分析完成后的决策点），P4 发生在 Write/Edit 调用时。这两个窗口没有 ceremony 感知的守卫。

### 2.2 结构性原因：守卫架构与 ceremony 协议的范畴错配

**这不是 hook 覆盖不够的问题，而是守卫架构与 ceremony 协议之间存在范畴错配。**

ceremony 是一个 11 步顺序协议（ceremony.md 步骤 1-11），要求 Lead 维护步骤间的状态转移。但当前守卫架构是**无状态的事件驱动系统**——每个 hook 独立触发，没有共享的"当前处于 ceremony 第几步"状态。

具体表现：

1. **hook 系统无 ceremony 状态**：flow-continuity-guard.sh 通过命令字符串匹配（`"git push" in command`）判断触发条件，不知道 Lead 当前处于 ceremony 的哪一步。它只能在特定 Bash 命令后注入指令，无法在 Read/分析后注入。

2. **`.ceremony-in-progress` 文件未被 PostToolUse hook 使用**：ceremony-completion-guard.sh（Stop hook）检查 `.ceremony-in-progress` 文件，但 PostToolUse hook 不检查。ceremony 状态信息存在但未被守卫链利用。

3. **lead-audit.sh 的设计选择与 ceremony 白名单冲突**：173号-1 方法论反转选择了"默认放行，黑名单记录"——这对常规操作是正确的，但 ceremony 期间 Lead 的白名单更严格（只有调度/路由/持久化三类）。ceremony 期间的 Write/Edit 应该被阻断，不只是记录。

### 2.3 根本原因：ceremony 协议假设 LLM 是状态机

ceremony.md 定义了步骤 1-11 的严格顺序，隐含假设执行者能维护步骤间状态。但 057号谱系已确立：**LLM 不是状态机**。每次 LLM 输出都是一个独立决策点，上下文窗口中的"当前步骤"信息会被新到达的信号（编排者消息、hook 注入、工位汇报）稀释或覆盖。

224/225 的 hook 注入是对这个根本问题的局部补偿——在特定 Bash 调用后注入正面指令，强制 LLM 的下一个决策朝正确方向。但补偿只覆盖了 Bash 边界，没有覆盖 Read/分析边界和 Write/Edit 边界。

### 2.4 四个现象的统一根因

四个现象是同一个结构性缺陷的四种投影：

```
ceremony 协议（顺序状态机）
    ↓ 执行者
LLM（无状态决策器，057号）
    ↓ 守卫
hook 系统（无状态事件驱动，只覆盖 Bash 边界）
```

每一层都是无状态的。ceremony 协议要求状态维护，但执行者和守卫都不维护状态。224/225 在 Bash 边界注入了"伪状态"（正面指令），但 Read/Write/Edit 边界没有对应的伪状态注入。

---

## 三、224/225 为什么不够

### 3.1 224/225 做对了什么

- 正确识别了 137号约束（否定性禁令无效）→ 使用正面指令注入
- 正确覆盖了 Bash 工具边界的两个关键转移点
- hook 返回 `{"decision": "block", "reason": "..."}` 强制 LLM 看到下一步指令

### 3.2 224/225 的结构性局限

| 局限 | 说明 |
|------|------|
| 只覆盖 Bash PostToolUse | Read/Glob/Grep/Write/Edit 后无守卫 |
| 无 ceremony 步骤状态 | hook 不知道当前是第几步，只能匹配命令字符串 |
| 不区分 ceremony 内外 | push 后的 rescan 指令在非 ceremony 场景也会触发（虽然无害但不精确） |
| 不阻断白名单违规 | lead-audit.sh 记录但不阻断 ceremony 期间的 Write/Edit |

### 3.3 未覆盖的断裂窗口

| 窗口 | 触发场景 | 当前守卫 |
|------|---------|---------|
| Read → 行动 | Lead 读取文件后应执行下一步，但转而提问 | 无 |
| Glob/Grep → 行动 | Lead 搜索后应执行下一步，但转而总结 | 无 |
| 分析完成 → 行动 | Lead 完成质询分析后应直接执行，但问"你要我修还是先讨论" | 无 |
| Write/Edit 白名单 | Lead 在 ceremony 期间直接修改内容文件 | lead-audit 记录，不阻断 |
| SendMessage → 行动 | Lead 发送消息后应继续 ceremony，但停下来 | 无 |

---

## 四、修复方案（三层，非补丁）

### 层 1：ceremony 状态文件（新增基础设施）

**问题**：hook 系统无 ceremony 状态，无法判断"当前是第几步"。
**修复**：引入 `.chanlun/.ceremony-step` 状态文件。

```yaml
# .chanlun/.ceremony-step（ceremony 执行期间存在，结束后删除）
step: 8          # 当前步骤编号（1-11）
phase: rescan    # 当前阶段
round: 1         # rescan 循环轮次
prev_workstations: ["ws1", "ws2"]  # 上轮工位列表（步骤9比较用）
```

- ceremony_scan.py 在 `--phase initial` 时创建此文件（step=1）
- 每个步骤完成时由 hook 或脚本更新 step 字段
- ceremony 终止时（步骤2干净终止 / 步骤10不动点 / 步骤11安全阀）删除此文件
- 文件存在 = ceremony 进行中；文件不存在 = 非 ceremony 状态

**与 `.ceremony-in-progress` 的关系**：`.ceremony-step` 取代 `.ceremony-in-progress`（后者只是布尔标记，无步骤信息）。

### 层 2：ceremony 全工具守卫（新增 PostToolUse hook）

**问题**：flow-continuity-guard.sh 只覆盖 Bash，Read/Write/Edit/SendMessage 后无守卫。
**修复**：新增 `ceremony-step-guard.sh`，注册为所有关键工具类型的 PostToolUse hook。

设计：
```
触发条件：PostToolUse，匹配 Read | Write | Edit | Glob | Grep | SendMessage | Task
前置检查：.ceremony-step 文件是否存在
  - 不存在 → exit 0（非 ceremony，不干预）
  - 存在 → 读取当前 step，执行以下逻辑
```

逻辑矩阵（ceremony 期间）：

| 当前 step | 工具调用 | 守卫行为 |
|-----------|---------|---------|
| 任意 | Write/Edit 目标不在白名单 | **block**："ceremony 期间 Lead 不执行实质认知工作。委派给工位。" |
| 7 (push 阶段) | 非 git 相关 | **block**："当前步骤是 push，立即执行 git push。" |
| 8 (rescan 阶段) | 非 ceremony_scan | **block**："当前步骤是 rescan，立即执行 ceremony_scan.py --phase rescan。" |
| 9-10 (evaluate) | Read（读取 scan 结果） | 放行（合法的评估操作） |
| 9-10 (evaluate) | SendMessage/Task（spawn/shutdown） | 放行（合法的路由操作） |
| 6 (RTAS consume) | Read/SendMessage/Task | 放行（合法的消费操作） |

**白名单定义**（ceremony 期间 Write/Edit 的合法目标）：
- `.chanlun/sessions/` — session 写入
- `.chanlun/.ceremony-step` — 状态文件更新
- `.chanlun/.stop-guard-counter` — 守卫计数器

其他路径的 Write/Edit 一律 block。这比 lead-audit.sh 的"记录不阻断"更严格，但仅限 ceremony 期间。

### 层 3：ceremony 原子链脚本化（架构级修复）

**问题**：步骤 7→8 之间有 LLM 决策间隙（push 完成后 LLM 可能不执行 rescan）。
**修复**：将机械性步骤合并为单个脚本调用，消除间隙。

新增 `scripts/ceremony_push_and_rescan.sh`：
```bash
#!/bin/bash
# 原子执行：push → rescan（消除步骤7→8之间的 LLM 决策间隙）
set -euo pipefail
git add -A && git commit -m "$1" && git push
python scripts/ceremony_scan.py --phase rescan
```

Lead 在步骤 7 调用此脚本而非分别调用 git push 和 ceremony_scan.py。这样：
- push→rescan 之间没有 LLM 决策点
- flow-continuity-guard.sh 在脚本完成后触发（匹配 ceremony_scan.py），注入 evaluate 指令
- 步骤 7 和 8 合并为一个 Bash 调用

**ceremony.md 对应修改**：
```
步骤 7-8（合并）：bash scripts/ceremony_push_and_rescan.sh "commit message"
步骤 9：评估 rescan 输出（LLM 决策，有 ceremony-step-guard 守卫）
步骤 10：spawn 或 terminate（LLM 决策，有 ceremony-step-guard 守卫）
```

---

## 五、P3 专项分析：分析后提问

P3（"质询收敛分析后问编排者'你要我修还是先讨论'"）不是 ceremony 步骤间的断裂，而是**四分法违规**——Lead 把定理类/行动类事项当作选择类上浮。

### 根因

no-unnecessary-escalation.md 的四分法过滤是否定性禁令（"不允许提可自决的问题"）。137号已确立否定性禁令对行为执行层无效。四分法的正面格式约束（格式A/B/C）存在，但只约束输出结尾格式，不约束输出中间的提问。

Lead 可以在输出中间插入一个问题（"你要我修还是先讨论？"），然后以格式A结尾（"→ 接下来：等待编排者回复"）——这在形式上符合格式约束，但实质上是四分法违规。

### 修复

ceremony-step-guard.sh 在 ceremony 期间检测 Stop 事件时的输出内容。如果输出包含问号且不在 `/escalate` 报告内，block 并注入："四分法违规——定理类/行动类直接执行，不提问。立即执行下一步。"

但这仍然是 Stop hook 层面的守卫（只在 agent 即将结束 turn 时触发）。更严格的修复是：**ceremony 期间 Lead 的每次输出都经过格式检查**——这需要一个 PostToolUse hook 在每次工具调用后检查 Lead 的上一段输出是否包含违规提问。

当前 hook 架构不支持检查 LLM 的文本输出（hook 只能看到工具调用的输入/输出）。这是一个**平台层限制**：Claude Code 的 hook 系统无法拦截 LLM 的自然语言输出，只能拦截工具调用。

**在平台限制下的严格形式**：P3 的守卫只能在 Stop hook 中实现（agent 即将结束 turn 时检查）。ceremony-completion-guard.sh 的检查5已经做了确认请求检测，但匹配模式不够全面。扩展匹配模式：

```python
# 当前模式
CONFIRM_PATTERNS = '待确认|以上理解是否正确|如有偏差请指出|是否现在处理|是否有新的|请确认|等待.*确认'

# 扩展模式（覆盖 P3 场景）
CONFIRM_PATTERNS = '待确认|以上理解是否正确|如有偏差请指出|是否现在处理|是否有新的|请确认|等待.*确认|你要我|先讨论|还是先|要不要|需不需要|是否需要|你觉得|你认为|你希望'
```

---

## 六、P4 专项分析：Lead 僭越白名单

### 根因

lead-audit.sh 的 173号-1 方法论反转选择了"默认放行，黑名单记录"。这对常规操作是正确的（Lead 需要灵活性），但 ceremony 期间 Lead 的角色边界更严格——ceremony.md 白名单明确限定 Lead 只做调度/路由/持久化。

当前 lead-audit.sh 对 ceremony 内外不做区分。ceremony 期间的 Write/Edit 应该是 block，不是 systemMessage。

### 修复

在 lead-audit.sh 中增加 ceremony 状态检查：

```python
# 如果 .ceremony-step 文件存在（ceremony 进行中）
# 且 Write/Edit 目标不在 ceremony 白名单中
# → 返回 {"decision": "block", "reason": "..."}
# 否则保持现有逻辑（记录不阻断）
```

或者更干净的方案：将此逻辑放在层2的 ceremony-step-guard.sh 中（ceremony 专用守卫），lead-audit.sh 保持不变（非 ceremony 场景的审计）。两个 hook 职责分离：
- lead-audit.sh：非 ceremony 场景的拓扑异常记录（173号-1）
- ceremony-step-guard.sh：ceremony 场景的白名单强制执行

---

## 七、边界条件

1. **INTERRUPT 优先级**：编排者显式 INTERRUPT 可中断 ceremony（224号已声明）。ceremony-step-guard 检测到 INTERRUPT 时删除 `.ceremony-step` 文件，退出 ceremony 状态。
2. **熔断**：ceremony-step-guard 连续 block 3 次且步骤无推进 → 放行（与 ceremony-completion-guard 的 145号熔断一致）。
3. **非 ceremony 场景不受影响**：所有新增守卫以 `.ceremony-step` 文件存在为前提。文件不存在时静默退出。
4. **子蜂群不受影响**：ceremony-step-guard 检查 `CLAUDE_AGENT_NAME` 环境变量，子工位调用不触发。
5. **平台限制**：hook 无法拦截 LLM 自然语言输出（只能拦截工具调用）。P3 的完整修复需要平台层支持"输出内容检查"hook 类型。在当前平台下，P3 只能在 Stop hook 中部分覆盖。

---

## 八、修复优先级

| 优先级 | 修复项 | 覆盖现象 | 依赖 |
|--------|--------|---------|------|
| P0 | 层1：`.ceremony-step` 状态文件 | 所有 | 无（基础设施） |
| P0 | 层2：ceremony-step-guard.sh | P3, P4 | 层1 |
| P1 | 层3：ceremony_push_and_rescan.sh | P1 加固 | 无 |
| P1 | ceremony-completion-guard.sh 扩展匹配模式 | P3 | 无 |
| P2 | ceremony.md 步骤合并（7-8 → 单步） | P1 加固 | 层3 |

---

## 九、谱系影响

本诊断如果被接受，产生以下谱系条目：

- **新谱系**：Lead ceremony 中断的统一根因——守卫架构与 ceremony 协议的范畴错配（ceremony 要求状态维护，守卫系统无状态）
- **依赖**：224号、225号、137号、057号、173号-1
- **下游推论**：
  1. ceremony 状态文件成为 hook 系统的共享状态载体
  2. ceremony 期间的 Lead 白名单从"记录"升级为"阻断"
  3. 机械性步骤序列应脚本化为单个调用（消除 LLM 决策间隙）
  4. P3 的完整修复受平台限制（hook 无法拦截自然语言输出），需要记录为已知 Gap
