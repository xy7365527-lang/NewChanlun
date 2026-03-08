---
id: '398'
number: 398
title: "元观察——v197热修复session（397号结算+重复bug修复+二阶观察）"
type: meta-rule
status: 已结算
date: 2026-03-08
source: Lead 直接执行（session内二阶观察，meta-observer guard 触发）
depends_on:
  - '397'   # v197-swarm 元观察
rule_version_baseline:
  claude_md_commit: "09bc3999062d55d369dde9ebedd37c9efe4ce0e3"
  rules_dir_mtime: "2026-03-04 19:53:04 +0000"
epistemological_level: L0
---

# 398号：元观察——v197热修复session（397号结算 + 重复bug修复）

## 递归判断

任务不可分解：单一session内的热修复+二阶观察。扁平退化特例。

## 规则版本基线

- `claude_md_commit`: 09bc399（CLAUDE.md 从 97f3ba3 变更为 09bc399——v194 ceremony.py commit）
- `rules_dir_mtime`: 2026-03-04 19:53:04 +0000（与 356-397号相同——未变更）

结论：CLAUDE.md commit 发生了变更（97f3ba3→09bc399），但 rules_dir 未变。CLAUDE.md 变更是 v194 逢亮 ceremony.py 功能的 commit，不是元规则变更。规则层实质静止。

## 观察对象

本session（非蜂群，Lead 直接执行）：
1. 397号谱系从 pending 结算到 settled
2. 重复 bug 诊断与修复（expression_pressure_ws_message 重复推送）

## 观察结果

### 观察1（定理）：WS watermark 与对话 watermark 的分离——两个消费者共享状态的典型缺口

重复 bug 的根因：`expression_pressure_ws_message()` 和 `/present` endpoint 共用 `_reported_step_watermark`，但只有 `/present` 推进它。WS 推送路径读取 watermark 却从不推进——导致同一事件在每次后续事件触发时被反复选中。

这是 397号-BC6（instance_tension 无消费者）的同构问题：**写入与消费不对称**。instance_tension 是"有写入无消费"，WS watermark 是"有消费无推进"。两者的共同模式：共享状态的生产-消费协议不完整。

修复：引入独立的 `_ws_last_reported_step` watermark，WS 推送路径独立推进。

**四分法分类**：定理——共享状态的生产-消费协议不完整是已知工程反模式的实例。

### 观察2（定理）：穿越层的 fold 重复检测——更深层的结构性问题

探索 agent 发现：`traversal.py` 中 `_detect_encounter_topo()` 每步都扫描 visit_history 寻找共邻居的顶点对，可以在多个步骤中反复检测到同一结构相似性。如果 fold 被 settled cycle 阻断，结构条件不被消耗，下次经过时会再次检测。

WS watermark 修复解决了"同一事件被反复推送"，但"同一结构被反复检测"是穿越层的独立问题。后者需要在 `_detect_encounter_topo` 中引入"已尝试 fold 对"的记忆——但这涉及穿越引擎的记忆模型设计，不是热修复范畴。

**四分法分类**：定理——已诊断但修复涉及架构决策（穿越引擎记忆模型），留作后续 ceremony 工位。

### 观察3（行动）：397号结算——谱系持久化的正常操作

397号从 pending 移到 settled，状态改为"已结算"。无概念信息差。

**四分法分类**：行动。

## 自环检查：与 397号交叉对比

| 编号 | 核心发现 | 本轮状态 |
|------|---------|---------|
| 397-BC1 | 谱系编号冲突 382/383/384 | **未解决**。继续继承 |
| 397-BC4 | operator_ruling regex 窄 | **未解决**。继续继承 |
| 397-BC6 | instance_tension 无消费者 | **观察1扩展**：共享状态生产-消费不对称是更一般的模式 |
| 346 | 规则版本稳定 | 推进中。CLAUDE.md commit 变更但实质为功能 commit，rules_dir 静止 |

**收敛信号**：
- 规则版本持续稳定
- 共享状态生产-消费不对称被识别为可复用模式

**发散信号**：
- 穿越层 fold 重复检测——新发现，需要穿越引擎记忆模型设计

## 边界条件

1. **穿越层 fold 重复检测**（新增）：`_detect_encounter_topo` 无"已尝试对"记忆，被阻断的 fold 会被反复检测。WS 层修复掩盖了此问题的用户可见症状但未消除根因
2. **谱系编号冲突**（继承自 397-BC1）：382/383/384 双文件共存
3. **operator_ruling regex**（继承自 397-BC4）：审批表述不匹配会静默失败

## 影响声明

本谱系不改动定义或规则。记录 v197 热修复 session 的二阶观察。确认 WS watermark 修复（daemon_api.py 两处修改），识别穿越层 fold 重复检测为独立问题。继承 3 条未解决边界条件。
