# Codex 诊断：蜂群持久化机制——158号下游推论1是否为"持久化"的错误实现

## 问题

158号谱系（三步战略——让矛盾具有拓扑效力）的下游推论1声明：
> "ceremony.md 必须移除 TeamDelete（蜂群持久化——蜂群不再被主动终止，只因物理约束自然终止）"

编排者否定了这个推论：
> "蜂群的持久化不是靠不关闭蜂群达到的，我们需要的是RTAS的循环。"
> "你这样只会让僵尸工位越来越多。"

## 当前实现（ceremony.md 步骤6第8条）

```
8. 如果步骤1无新工位 → 输出格式B（无待做行动），但**不执行 TeamDelete**——蜂群保持存活
9. 蜂群持续存活，直到编排者显式终止或上下文耗尽
```

## 实际后果

1. v42-swarm config 中积累了大量僵尸工位（已退出但未从 config 清理的成员）
2. ceremony-completion-guard（Stop-Guard）检测到"空闲工位"误报——因为已退出的工位仍在 members 列表中
3. 蜂群跨 context 实际上无法"保持存活"——Claude Code session 结束时所有 agent 进程终止
4. 所谓的"持久化模式"实际只是"不执行 TeamDelete"，没有真正的进程持久化

## RTAS 循环模型（编排者提出的正确持久化形式）

RTAS = 递归拓扑异步自指蜂群。其循环结构：

1. ceremony_scan.py → 推导工位
2. TeamCreate → spawn 工位 → 工位执行
3. session 写入（状态结晶）→ commit → push
4. TeamDelete（清理）
5. 下次 ceremony 从 session 热启动 → 回到步骤1

持久化不在于蜂群实例的存活时间，在于：
- session 文件携带完整状态
- ceremony 热启动恢复到上次断点
- 谱系 DAG 是跨 session 的永久拓扑结构
- 每次 ceremony 是新的 Swarm₀，但继承了前面所有 Swarm 的否定史

## 子蜂群 ceremony skill 中的对比

sub-swarm-ceremony SKILL.md 步骤5（Session 标注与清理）明确包含：
```
2. TeamDelete 清理子 team
```

也就是说子蜂群已经在正确地执行 TeamDelete，只有顶层 ceremony 因为158号下游推论1而移除了 TeamDelete。

## 热启动机制（swarm-architecture SKILL.md Section E）

热启动机制设计：
- L1 compact: PreCompact 保存 → 恢复
- L2 新对话: ceremony 从 session 文件恢复

设计目标："蜂群永远在线。compact 和新对话都不是中断，而是状态快照+恢复的不同级别。"

这个设计**已经实现了持久化**——通过状态结晶和热启动，而不是通过进程存活。

## 诊断问题

1. 158号下游推论1（不执行 TeamDelete）是否是"持久化"的错误实现？
2. RTAS 循环（ceremony → work → session → TeamDelete → 下次 ceremony 热启动）是否才是正确的持久化形式？
3. 158号的三步战略本身是否受影响，还是只有下游推论1被否定？
4. ceremony.md 应该恢复 TeamDelete 吗？应在什么时机执行？
