# 严格双向讨论：Context Limit 根因的严格修复

## 现象

Lead 在每个 session 中反复遇到 "Context limit reached"，需要 /compact 恢复。
上一轮诊断（141号）确定根因不是 hook system-reminder 注入，而是 Lead 反复读取大文件：
- dag.yaml（55KB）
- Gemini 讨论来回（~40KB/轮）
- pattern-buffer.yaml（膨胀到 831 条，已清理到 324 条）

## 已尝试的修复

1. **dag_add_node.py**：避免 Lead 读取完整 dag.yaml 来添加节点
2. **precompact-save.sh**：compact 前保存蜂群状态，减少 compact 后的上下文恢复成本
3. **session-start-ceremony.sh**：热启动时注入蜂群状态，避免 Lead 重新扫描

## 未解决的根因

这些修复都是**缓解措施**，没有解决核心问题：

### A. Lead 为什么反复读取大文件？

dag.yaml 被读取的场景：
1. ceremony_scan.py 已经输出了工位列表，Lead 不需要读 dag.yaml
2. 但 Lead 在谱系结算时需要读 dag.yaml 来添加节点/边
3. dag_add_node.py 解决了添加，但 Lead 在验证边时仍然可能读取
4. validate_dag.py 也需要完整读取

**问题**：如何在架构层面让 Lead 永远不需要读取 dag.yaml 的完整内容？

### B. Gemini 讨论的 context 成本

每轮 Gemini 讨论 = 写上下文文件 + 工位读取 + 工位产出报告 + Lead 读取报告。
两轮收敛 = ~80KB context 消耗。

但这是**设计意图**（异质碰撞需要完整的推理链传递），不是浪费。

**问题**：如何在保持异质碰撞质量的同时降低 context 成本？是否应该限制单轮上下文的大小？

### C. compact 是否应该更早触发？

当前 compact 在 "Context limit reached" 时由用户手动触发。如果系统自动检测到 context 使用率 > 80% 时自动 compact，能否避免"撞墙"？

**问题**：Claude Code 平台是否支持自动 compact？如果不支持，Lead 能否在每个 ceremony cycle 结束时主动检查 context 使用率？

### D. ceremony 循环的 context 预算

一个完整的 ceremony 循环（ceremony_scan → spawn 工位 → 汇报 → 谱系结算 → commit → push）消耗多少 context？如果超过单次 session 上限的 50%，则每个 session 最多只能完成一个循环。

**问题**：是否应该为 ceremony 循环设定 context 预算上限？超出时强制 compact + 热重启？

简体中文，严格直接。
