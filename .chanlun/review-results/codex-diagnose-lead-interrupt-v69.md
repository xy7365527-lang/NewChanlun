# Codex 诊断报告：Lead 中断 + 蜂群工位卡死

**模式**: diagnose（严格诊断）
**日期**: 2026-02-27 03:35
**诊断者**: codex-reviewer（异质审查工位）
**目标**: Lead 反复"断掉" + v69-swarm 工位卡死

---

## 一、现象清单

### 现象1：Lead 反复中断 ceremony 序列

| # | 具体断裂点 | 224/225号是否覆盖 |
|---|-----------|-----------------|
| 1a | push 后不 rescan | 覆盖（PostToolUse/Bash 注入） |
| 1b | rescan 后不 evaluate | 覆盖（PostToolUse/Bash 注入） |
| 1c | 质询收敛后问编排者"你要我修还是先讨论" | **未覆盖** |
| 1d | Lead 直接修改 v4 文本（僭越 ceremony 白名单） | **未覆盖**（pre-write-edit-dispatcher 是白名单检查，不是 ceremony 状态检查） |
| 1e | 224/225号修复后仍在发生 | 说明存在更深层根因 |

### 现象2：v69-swarm 工位卡死

| 工位 | 症状 |
|------|------|
| interrupt-diagnosis（general-purpose, in-process） | 无产出文件、不响应 SendMessage、不响应 shutdown_request（×2）、TeamDelete 被阻塞 |
| v4-text-fix | 完成 1/4 修改后退出，未完成任务 |

---

## 二、根因分析（分层）

### 现象1 根因

#### 层1（直接原因）：hook 覆盖存在结构性盲区

`flow-continuity-guard.sh` 是 **PostToolUse/Bash** hook——它只在 Bash 工具调用完成后触发。

RLHF 停顿的实际路径：

```
Lead 输出纯文本（问题/总结/分析）→ 模型停止（Stop 事件）→
PostToolUse/Bash hook 不触发（没有 Bash 调用）→
只有 Stop hook（ceremony-completion-guard.sh）触发
```

224/225号修复的是：**Bash 调用成功后 Lead 不继续**。
但断裂也发生在：**Lead 输出纯文本后直接停止，根本没有调用 Bash**。

这两条路径不同：
- 路径A（224/225覆盖）：`git push` Bash → PostToolUse hook 注入 → Lead 应继续但没继续
- 路径B（未覆盖）：Lead 输出文本 → Stop → Stop hook 检查任务队列/谱系 → 如果检查通过则放行

路径B 的 Stop hook 检查的是"有没有活跃任务"，不是"ceremony 序列是否完整"。

#### 层2（结构原因）：ceremony 状态对 Stop hook 不可见

`ceremony-completion-guard.sh` 检查：
1. session 有无下一轮方向 + 无活跃蜂群（死寂检测）
2. 任务队列（pending/in_progress）
3. 生成态谱系矛盾
4. proof-required 标签检查
5. ceremony 确认请求（检查 `.chanlun/.ceremony-in-progress` 文件 + 输出内容模式匹配）

检查5 依赖两个条件同时成立：
- `.chanlun/.ceremony-in-progress` 文件存在
- 输出内容匹配 `待确认|以上理解是否正确|...` 等模式

**缺口**：
- 如果 Lead 的问题不匹配这些模式（如"你要我修还是先讨论"），检查5 不触发
- 如果 `.chanlun/.ceremony-in-progress` 文件不存在或已被清除，检查5 完全跳过
- ceremony 当前处于哪个步骤（step 1-10）对 Stop hook 完全不可见

#### 层3（根本原因）：137号问题在 ceremony 序列中的系统性失效

137号已结算：否定性禁令对行为执行层无效（RLHF 基底约束）。

hook 注入系统的设计假设：**模型总会调用工具，hook 在工具调用后注入指令**。

但 RLHF 基底行为是：**在"自然停顿点"（问题、总结、分析完成后）输出纯文本并停止**。这个停止不经过任何 PostToolUse hook，只经过 Stop hook。

Stop hook 的注入能力有限：它只能 block（阻止停止）并注入 reason 文本。但如果 Lead 的 RLHF 停顿是因为它认为"已完成当前阶段"，Stop hook 的 block 会触发新一轮输出，而新一轮输出可能再次以纯文本结束——形成 block→输出→stop→block 的循环，直到熔断（3次）放行。

**结论**：224/225号修复了 Bash 调用后的断裂，但没有修复"Lead 根本不调用 Bash 就停止"的断裂。这是更深层的结构性问题。

---

### 现象2 根因

#### 假设排序（按可能性）

**假设A（最可能）：context 窗口耗尽**

`interrupt-diagnosis` 工位被分配了诊断任务，任务描述本身可能很长（包含现象描述、要求读取的文件列表等）。加上：
- 系统 prompt（CLAUDE.md + 所有 rules + agent 指令）
- 已读取的文件内容（ceremony-completion-guard.sh 等大文件）
- 对话历史

当 context 接近上限时：
- 模型无法生成新 token
- 无法处理新消息（包括 SendMessage、shutdown_request）
- 进程仍在运行（等待 API 响应）但不产出
- TeamDelete 被阻塞（进程未退出）

**假设B（次可能）：Stop hook 循环耗尽 context**

Stop hook 反复 block → 每次 block 注入 reason → Lead 生成新输出尝试遵守 → 再次 Stop → 再次 block → context 增长 → 最终耗尽。

熔断机制（3次相同状态放行）理论上应该阻止这个循环，但如果每次 block 后 ACTIVE_TASKS 数量发生变化（其他工位完成/新建），熔断计数器会重置，循环可以无限持续。

**假设C（可能）：API 超时循环**

`API_TIMEOUT_MS: 120000`（2分钟）。如果工位在等待一个长时间的 API 调用（如读取大文件、调用 Codex），超时后重试，重试再超时，形成循环。但这种情况通常会有错误输出，与"无产出文件"不完全吻合。

**假设D（较低）：hook 阻塞**

`pre-write-edit-dispatcher.sh` 或其他 PreToolUse hook 返回 block，工位无法执行写操作，卡在等待状态。但这通常会有错误消息。

#### v4-text-fix 完成 1/4 后退出

这是不同的失败模式——不是卡死，而是提前退出。可能原因：
- context 接近上限，模型判断"已完成"（实际未完成）
- 任务描述不够精确，模型认为 1/4 就是全部
- Stop hook 放行（熔断或检查通过），工位正常退出但任务未完成

---

### 两个现象的共同根因

**共同根因：hook 系统对"纯文本输出后停止"这条路径的覆盖是结构性不完整的**

```
现象1：Lead 输出纯文本（问题/总结）→ Stop → Stop hook 检查不到 ceremony 状态 → 放行
现象2：工位输出纯文本（或无输出）→ Stop hook 循环 → context 耗尽 → 卡死
```

两者都是 RLHF 基底行为（在自然停顿点停止）与 hook 系统（只能在工具调用后注入）之间的结构性张力的不同表现。

---

## 三、修复方案（严格，非补丁）

### 修复1：ceremony 步骤状态持久化（针对现象1）

**问题**：Stop hook 不知道 ceremony 当前在哪个步骤。

**修复**：在 ceremony skill 的每个步骤执行时，写入步骤状态文件：

```
.chanlun/.ceremony-step  # 内容：当前步骤编号（1-10）
```

Stop hook 增加检查：
- 如果 `.chanlun/.ceremony-step` 存在且内容不是 `2`（干净终止）或 `10`（不动点）
- 则 block，注入：`[Stop-Guard] ceremony 序列未完成（当前步骤: N）。不允许停止。继续执行步骤 N+1。`

这覆盖了所有 ceremony 中断路径，包括纯文本输出后停止。

**边界条件**：
- ceremony 正常终止时（步骤2或步骤10），删除 `.chanlun/.ceremony-step`
- INTERRUPT 时，删除 `.chanlun/.ceremony-step`（允许停止）
- 文件存在但内容无效（非数字），视为步骤1，block

### 修复2：ceremony 确认请求检测扩展（针对现象1c）

**问题**：Lead 问"你要我修还是先讨论"不匹配现有模式。

**修复**：扩展 `ceremony-completion-guard.sh` 检查5的模式：

```bash
CONFIRM_PATTERNS='待确认|以上理解是否正确|如有偏差请指出|是否现在处理|是否有新的|请确认|等待.*确认|你要我|先讨论|还是先|要我.*还是|是否.*修'
```

但这是补丁思维——模式匹配永远不完整。严格修复是修复1（步骤状态持久化），它不依赖内容模式匹配。

### 修复3：工位 context 预算控制（针对现象2）

**问题**：工位 context 耗尽导致卡死。

**修复**：
1. 工位 spawn 时限制任务描述长度（不超过 500 字）
2. 需要读取大量文件的诊断任务，拆分为多个小工位（每个工位读取一个文件子集）
3. 在 `agent-team-enforce.sh` 中检查 Task 工具的 prompt 长度，超过阈值时 block 并要求拆分

**边界条件**：
- 某些任务本质上需要大量上下文（如全局审计），这类任务需要分阶段执行
- 拆分粒度由任务的数据依赖决定，不是任意拆分

### 修复4：卡死工位强制回收机制（针对现象2）

**问题**：卡死工位无法响应 shutdown_request，TeamDelete 被阻塞。

**修复**：
- 在 `ceremony-completion-guard.sh` 的 Stop hook 中，检测 in_progress 任务的最后活跃时间
- 如果某个 in_progress 任务超过 N 分钟无更新，标记为 `zombie`，不计入 ACTIVE_TASKS
- TeamDelete 时跳过 zombie 工位

**边界条件**：
- N 的选择：`API_TIMEOUT_MS` 是 120 秒，合理的 zombie 阈值是 5-10 分钟
- 任务文件中需要有 `last_updated` 字段（当前格式是否有此字段需要验证）

---

## 四、边界条件

| 条件 | 影响 |
|------|------|
| INTERRUPT 信号 | 覆盖所有守卫，允许停止——修复1不应阻止 INTERRUPT |
| ceremony 步骤文件损坏 | 视为步骤1，block——保守策略，不会漏放 |
| 工位 context 耗尽但任务已完成 | v4-text-fix 的情况——需要区分"完成"和"耗尽后误判完成" |
| Stop hook 熔断（3次相同状态） | 修复1不受熔断影响（ceremony 步骤状态独立于 ACTIVE_TASKS 计数） |

---

## 五、影响声明

| 文件 | 变更类型 |
|------|---------|
| `.claude/hooks/ceremony-completion-guard.sh` | 新增检查6：ceremony 步骤状态检测 |
| `.claude/skills/*/` (ceremony skill) | 每个步骤写入 `.chanlun/.ceremony-step` |
| `.claude/hooks/agent-team-enforce.sh` | 新增 Task prompt 长度检查 |
| `.chanlun/.ceremony-step` | 新增运行时状态文件 |

---

## 六、诊断者判定

**否定成立**：224/225号修复不完整——它们覆盖了"Bash 调用后断裂"，但没有覆盖"纯文本输出后断裂"。这是结构性盲区，不是边界情况。

**严重性**：
- 现象1（Lead 中断）：HIGH——直接导致 ceremony 序列失效，编排者需要反复干预
- 现象2（工位卡死）：HIGH——卡死工位阻塞 TeamDelete，影响蜂群生命周期管理

**共同根因确认**：两个现象有共同根因（hook 系统对纯文本停止路径的结构性盲区），但修复路径不同（现象1需要 ceremony 状态持久化，现象2需要 context 预算控制 + 卡死回收）。
