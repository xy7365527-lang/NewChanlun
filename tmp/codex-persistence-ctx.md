# Codex 质询：蜂群持久化机制

## 背景

158号谱系第一条下游推论声明："ceremony.md 必须移除 TeamDelete（蜂群持久化——蜂群不再被主动终止，只因物理约束自然终止）"。

这导致了以下实际问题：
1. v42-swarm config 中积累了大量僵尸工位（已退出但未从 config 清理的成员）
2. ceremony-completion-guard（Stop-Guard）检测到"空闲工位"误报——因为已退出的工位仍在 members 列表中
3. 蜂群跨 context 实际上无法"保持存活"——Claude Code session 结束时所有 agent 进程终止
4. 所谓的"持久化模式"实际只是"不执行 TeamDelete"，没有真正的进程持久化

## 编排者的否定

编排者原文："蜂群的持久化不是靠不关闭蜂群达到的，我们需要的是 RTAS 的循环。"
追加："你这样只会让僵尸工位越来越多。"

## RTAS 循环作为持久化机制

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

## 要求 Codex 评审的问题

1. "不执行 TeamDelete"是否是对"持久化"的错误实现？
2. RTAS 循环（ceremony → work → session → TeamDelete → 下次 ceremony 热启动）是否才是持久化的正确形式？
3. 158号的三步战略本身是否受影响，还是只有下游推论1被否定？
4. ceremony.md 应该恢复 TeamDelete 吗？如果是，应该在什么时候执行（所有工位完成后？还是其他时机）？
