# 严格双向讨论：Stop-Guard 错误循环 + meta-observer advisory 噪音

## 问题一：Stop-Guard 错误循环

### 现象

v144-swarm 工位全部完成后，Lead 试图停止时，ceremony-completion-guard.sh（Stop-Guard）反复阻断：
```
[Stop-Guard] 蜂群任务队列有 2 个活跃任务。不允许停止。
```
重复多次，直到熔断（COUNT >= 5）才放行。

### 根因诊断

Stop-Guard 的检查逻辑（ceremony-completion-guard.sh:90-165）：
1. 扫描 `$HOME/.claude/tasks/*/` 下所有 JSON 文件
2. 统计 `pending` 和 `in_progress` 状态的任务数
3. 如果 ACTIVE_TASKS > 0，阻断并注入路由指令

**缺口**：工位（teammate）完成工作后，通过 SendMessage 向 Lead 发送结果，然后进入 idle 或退出。但这个流程中 **task 状态不一定被更新为 completed**。具体：
- 工位完成工作 → SendMessage → idle
- Lead 收到结果 → shutdown_request → 工位退出
- **但 task.json 的 status 可能仍然是 in_progress**

这创造了一个**僵尸任务**状态：工位已经不存在了，但 task 文件还是 in_progress，Stop-Guard 检测到它，反复阻断。

### 已知的 TeamDelete 清理逻辑

TeamDelete 应该清理 `~/.claude/teams/{name}/` 和 `~/.claude/tasks/{name}/`。但如果 TeamDelete 没有被调用（Lead 在 shutdown 工位和 TeamDelete 之间停顿了），残留目录就会一直存在。

### 问题

#### A. 谁的职责更新 task 状态？

当前流程中，task 状态有三个可能的更新者：
1. **工位自己**：完成后调用 TaskUpdate(taskId, status=completed)
2. **Lead**：收到工位结果后调用 TaskUpdate
3. **系统**：TeamDelete 时批量清理

如果（1）不可靠（工位可能忘记），（2）取决于 Lead 是否停顿，那 Stop-Guard 就永远有可能遇到僵尸任务。

#### B. Stop-Guard 是否应该检查工位存活性而非仅看 task 状态？

当前 Stop-Guard 只看 task.json 的 status 字段。它不检查对应的工位进程是否还活着。如果增加"工位存活性检查"（team config 的 members 列表 vs task 的 owner），能否消除僵尸任务问题？

#### C. 熔断是否是正确的止损机制？

当前熔断（连续5次阻断后放行）是唯一的逃逸路径。但这意味着每次蜂群结束时 Lead 都要经历5次无意义的 Stop-Guard 阻断。这个数字是否可以基于任务实际状态进行智能判断？

## 问题二：meta-observer advisory 噪音

### 现象

每次 session 结束时，meta-observer-guard.sh 输出：
```
[meta-observer advisory] 本 session 二阶观察已跳过（STRICT_MODE=0）。如需执行：...
```

### 根因诊断

2026-02-21 的 hotfix 将 `META_OBSERVER_GUARD_STRICT` 默认从 1 改为 0，因为 STRICT_MODE=1 触发了 "Invalid signature in thinking block" 的上游平台错误。

hotfix 的逻辑：
- STRICT_MODE=0 → 不阻断，只输出 advisory（systemMessage）
- advisory 包含一个可操作的提示（"如需执行：读取 meta-observer.md..."）
- 但 Lead 实际上从不执行这个 advisory

### 问题

#### D. advisory 模式是"降级"还是"失效"？

meta-observer 的 087号谱系修复说"advisory 模式产生实际的 systemMessage 提示（D策略：hooks 提示 + Lead 认领）"。但在实践中，Lead 从未基于这个 advisory 主动执行二阶观察。这意味着 advisory 模式 = 完全失效，不是降级。

#### E. 上游平台错误修复后，应该恢复 STRICT_MODE=1 吗？

"Invalid signature in thinking block" 是否已经修复？如果已修复，直接恢复 STRICT_MODE=1 是最简修复。如果未修复，需要找到替代方案（例如：在 ceremony 循环的特定点而非 Stop hook 中触发二阶观察）。

#### F. 二阶观察的触发点是否应该从 Stop hook 移到 ceremony 循环？

Stop hook 是"即将停止"时触发——但这正是 context 最紧张的时刻。如果二阶观察在 ceremony 循环中作为一个工位被 spawn（而非在 Stop hook 中强制），可能更合理。但这与当前架构（结构能力由 skill 事件驱动，不 spawn 结构工位——075号）矛盾。

## 体系文件

请基于以下文件回答：
- .claude/hooks/ceremony-completion-guard.sh（Stop-Guard 完整逻辑）
- .claude/hooks/meta-observer-guard.sh（meta-observer guard 完整逻辑）
- .chanlun/genealogy/settled/075-structural-skill-event-driven.md（结构能力从 teammate 转为 skill）
- .chanlun/genealogy/settled/087-xxx.md（如有，087号严格性否定补丁思维）
- .chanlun/genealogy/settled/016-code-enforcement.md（如有，规则需代码强制）

简体中文，严格直接。
