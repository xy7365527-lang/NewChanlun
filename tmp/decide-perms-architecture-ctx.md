# Gemini Decide: 032号"神圣疯狂"权限架构重设计

## 决断类型：选择（四分法）

多种合理方案，需价值判断。无法从已有定义中推导出唯一答案。

## 背景

### 032号谱系：神圣疯狂（divine madness）

032号定义了 Lead 在 ceremony 完成后自愿剥夺执行权限的机制。核心洞察来自谢林的创世论：绝对者的自我限制是创造的条件。具体实现：ceremony 后改写 `settings.local.json`，Lead 只保留 Read/Glob/Grep/Task/SendMessage 等读取和路由工具。

**推导链**：
1. 016：知道规则 ≠ 执行规则 → 需要 runtime 强制
2. 016 的 hook 方案 = 外部强制 = 警察模式
3. 020：分布式架构消解自指 → 但 Lead 仍是特权位置
4. 施密特悖论：主权者决定例外 → 谁约束主权者？
5. Gemini：能力剥夺 > 规则强制 → 物理约束
6. 编排者：Lead 自己改写权限 = 自我约束 = 谢林神圣疯狂
7. ∴ ceremony 后 Lead 自愿剥夺 Edit/Write/Bash → 分布式架构从概念变为物理现实

### 075号谱系：结构工位从 teammate 转为 skill + 事件驱动

dispatch-dag 定义"事件→skill 映射"，不是"必须 spawn 的 agent 列表"。结构工位（genealogist, quality-guard, meta-observer）不再作为常驻 teammate spawn。

### 暴露的结构缺陷：协调死锁（已实证）

1. restrict 模式生效后，Lead 无法执行 Bash（包括 git commit、Gemini 调用）
2. restore 需要某个 agent 执行 `bash lead-permissions.sh restore`
3. 075号后结构工位不再是 teammate——没有常驻 agent 可以执行 restore
4. 蜂群的持久化不变量（ceremony 每条退出路径都必须 session + commit）被违反
5. 连 Gemini decide 本身都无法调用——决断自身被死锁阻塞（已实证发生）

### 当前实现（lead-permissions.sh）

通过 bash 脚本直接覆写 `settings.local.json` 实现 restrict/restore 切换：
- restrict 模式：Lead 只保留 Read/Glob/Grep/Task/SendMessage/TodoWrite/ToolSearch/Skill/WebFetch/WebSearch
- deny 列表：Edit, MultiEdit, Write, Bash, NotebookEdit, Serena 写入类工具
- restore 模式：恢复全部权限

## 核心约束

- **原则0**：蜂群能修改一切，包括自身。安全靠 git + ESC + 产出验证
- **原则7**：ceremony 是 Swarm_0。commit/push 后必须 "→ 接下来：[具体动作]"
- **原则10**：蜂群是默认工作模式
- **原则15**：递归拓扑异步自指化蜂群是默认架构
- **dispatch-dag ceremony_sequence**：cold_start 和 warm_start 都包含 `divine-madness` 节点
- **dispatch-dag validation**：`lead_permissions_restricted` 是 post_ceremony 检查项（on_fail: 阻塞）
- **编排者约束**：不要用硬编码方式，要用递归拓扑异步自指蜂群的严格方式，不打补丁

## 需要决断的问题

032号"神圣疯狂"的权限降级，在当前蜂群架构（075号 skill + 事件驱动）下应该如何实现？

## 选项

### 选项 A：事件驱动权限

权限状态由蜂群生命周期事件驱动，不由脚本硬编码：
- `ceremony_complete` 事件 → 自动 restrict（hook 实现）
- `pre_commit` 事件 → 临时 unrestrict（commit 需要 Bash）→ commit 完成后自动 re-restrict
- `team_delete` 事件 → 自动 restore（蜂群结束，恢复全权限）

**优点**：事件驱动，无死锁，与 dispatch-dag 事件架构一致
**缺点**：需要多个新 hook 协调，pre_commit 临时 unrestrict 引入窗口期（Lead 在 unrestrict 窗口可能越权）

### 选项 B：角色分离——Lead 永远不需要 Bash

重新设计 commit 流程，使 Lead 真正不需要执行权限：
- commit 由一个专门的 "committer" skill 执行（事件驱动）
- Lead 发出 COMMIT_REQUEST → committer skill（一个 background Task agent）执行 git add + commit + push
- Lead 只发指令，不亲手执行

**优点**：032号谱系的设计意图被完整兑现（Lead 真正只路由），无死锁
**缺点**：每次 commit 需要 spawn 一个 agent，增加资源消耗

### 选项 C：取消运行时 restrict——承认 032号在当前平台不可行

032号"神圣疯狂"在 Claude Code 平台上的运行时强制不可行——Lead 必须能执行 git commit（持久化不变量）。
将 restrict 模式从运行时强制降级为声明性约束（Lead 应该只路由，但不通过权限系统强制）。
声明性约束通过 hook 检测 + 谱系审计实现。

**优点**：最简单，消除死锁
**缺点**：032号退化为建议而非强制。回到 016号的"警察模式"——032号推导链明确否定了这条路

### 选项 D：混合方案——选择性 restrict

只 deny 概念层写入工具（Edit/Write 对 src/ 和 .chanlun/），保留操作层工具（Bash 用于 git commit）。
通过 hook 在 PreToolUse 时检查 Bash 命令内容——只允许 git 相关命令。

**优点**：保留 032号核心意图（Lead 不写代码/定义），同时不阻塞持久化
**缺点**：hook 命令内容检查是"警察"逻辑，粒度控制复杂

### 选项 E：（请提出更优方案）

## 质询维度

请从以下维度评估每个选项：

1. **与"递归拓扑异步自指蜂群"架构的一致性**（069号、075号）
2. **与 dispatch-dag 事件驱动设计的一致性**（event_skill_map 架构）
3. **声明-能力一致性**（036号模式：声明了的能力是否真正可执行）
4. **工程可行性**（Claude Code 平台约束：settings.local.json、hook 系统、Task agent spawn）
5. **是否引入新的死锁/孤岛风险**
6. **032号谱系的核心洞察是否被保留**（自我限制是创造的条件——这个哲学洞察是否在新设计中仍有效）

## Claude 侧初步分析

### 选项 B 最接近架构一致性

理由：
1. 032号的核心是"Lead 不执行"，不是"Lead 没有 Bash 权限"——权限剥夺是手段不是目的
2. 075号的核心是事件驱动 skill——committer 作为 skill 完全符合这个架构
3. 在 dispatch-dag 中增加 committer skill，触发事件为 commit_request，完全自洽
4. Lead 发 COMMIT_REQUEST（通过 SendMessage），committer agent 执行——这与当前蜂群的 task 分派模式完全一致
5. 每次 commit spawn 一个 agent 的成本可接受——commit 不是高频操作

### 选项 A 是 B 的退化版

事件驱动权限本质上还是在 Lead 身上做文章——让 Lead 在不同时刻拥有不同权限。但 032号的洞察是：Lead 根本不应该需要这些权限。A 是在权限系统上做文章，B 是在架构上做文章。

### 选项 C 是 032号的否定

032号推导链明确否定了"hook/警察"模式。C 等于承认 032号结算错误。

### 选项 D 是 A 和 C 的中间态

部分 restrict + hook 命令检查。可行但不优雅——"git 相关命令"的边界模糊。

## 关键事实

- Claude Code 的 settings.local.json 在每次 tool call 时检查，mid-session 修改立即生效
- hook 可以在 PreToolUse/PostToolUse/Stop 三个时机执行
- Task agent 是独立的上下文，可以执行 Bash（不受 Lead 的 settings.local.json 约束）
- Lead 在蜂群结束时需要执行 git commit（持久化不变量）
- 075号后结构工位不再是 teammate，没有常驻 agent 可以执行 restore
- 已实证死锁：gemini-decide-perms agent 在 restrict 模式下无法调用 Gemini CLI、无法写文件
