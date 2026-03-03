---
id: '316'
number: 316
type: meta-rule
title: "元观察——上下文丢失中断恢复 + 蜂群残留状态处理模式"
date: "2026-03-03"
depends_on: ['315', '226', '137', '090']
status: 已结算
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-03-01 00:27:49 +0000"
---

# 316号：元观察——上下文丢失中断恢复 + 蜂群残留状态处理模式

## 递归判断

任务不可分解：meta-observer 是单一观察角色，观察过程不可并行分割。扁平退化特例。

## 规则版本基线

- `claude_md_commit`: 97f3ba3（与 308-315号相同——CLAUDE.md 未变更）
- `rules_dir_mtime`: 2026-03-01 00:27:49 +0000（与 308-315号相同——规则目录未变更）

结论：规则版本**八连稳定**。

## 观察对象

session 断点恢复过程。v137-swarm-r2 蜂群在 concept-fixer 工位 in_progress 时上下文丢失（session 崩溃/溢出），新 session 从文件系统状态重建。

### 前置状态

- v137-swarm-r2：meta-observer completed, topology-analyst completed, concept-fixer in_progress
- 三个历史蜂群目录残留：v128-swarm, v132-swarm, v137-swarm-r2
- 315号已结算（v137-swarm meta-observation）
- 折叠内在性检验 Agent A/B 产出已存在但未交叉比对

## 观察结果

### 观察1（定理）：上下文丢失是 226号"三层无状态"的又一实例

226号结算了 Lead 中断问题的结构性分类——根因是"三层无状态"（session/team/task 三层各自维护状态，无单一持久化层）。本次上下文丢失后的恢复过程再次验证了这个诊断：

恢复路径：session 文件 → team config → task JSON → git log/diff → 重建工作状态。这条路径能工作，但有信息损失：concept-fixer 的执行进度（它分析到哪一步、初步结论是什么）在上下文中但未持久化到文件系统。

与 226号的关系：不是新发现，是 226号的又一实例。226号的 topo_effect 是"Lead 中断问题从个案修复升级为结构性分类"——本次中断确认了分类的正确性。

**四分法分类**：定理——226号的逻辑延伸。

### 观察2（定理）：Stop-Guard + meta-observer-guard 双 hook 在上下文丢失场景正确触发

两个 hook 在新 session 启动后正确识别了残留状态：
- Stop-Guard：检测到 concept-fixer in_progress，阻止停止
- meta-observer-guard：检测到二阶反馈未执行，要求执行

这是 hook 系统设计意图的正确实现——hook 读取文件系统状态而非 session 内存，因此跨 session 存活。这与 315号观察1（规则版本稳定性）形成互补：规则层不变 + hook 跨 session 存活 = 蜂群在中断后可恢复。

**四分法分类**：定理——hook 设计的逻辑推论。

### 观察3（发散信号）：蜂群残留目录未清理——v128-swarm 和 v132-swarm

三个蜂群目录残留在 `~/.claude/teams/`：v128-swarm、v132-swarm、v137-swarm-r2。其中 v128-swarm 和 v132-swarm 的所有任务都已 completed，蜂群目录本应在 TeamDelete 中被清理。

可能原因：
1. Lead 在 ceremony 的 TeamDelete 步骤之前上下文丢失
2. TeamDelete 失败但未被捕获
3. 新 session 继承了旧 session 的 team 目录但未执行清理

这不是概念层问题——是蜂群生命周期管理的工程缺口。在当前架构下，蜂群目录是无害的残留（不会干扰新蜂群创建），但 session 文件中引用已完成蜂群的信息可能误导恢复逻辑。

**四分法分类**：发散信号。315号观察2（meta.json 计数漂移）的同构模式——"操作完成但元数据未同步清理"。

### 观察4（定理）：concept-fixer 恢复策略——重新 spawn 而非继续

上下文丢失后，concept-fixer 的恢复策略是重新 spawn 一个独立 agent 执行相同任务，而非尝试恢复中断的 agent 状态。这是正确的——concept-fixer 的任务是幂等的（读取 block-topology 当前状态 → 诊断 → 修复），重新执行不会产生错误的副作用。

一般化：幂等任务在中断后应重新执行而非恢复。非幂等任务（如已执行了部分文件写入）需要更复杂的恢复策略。当前蜂群中的大部分任务是幂等的（读取 → 分析 → 写入），因此"重新 spawn"是合理的默认恢复策略。

**四分法分类**：定理——幂等性 + 226号的逻辑推论。

### 观察5（自环检查）：与历史 meta-observation 的交叉对比

| 编号 | 核心发现 | 本轮状态 |
|------|---------|---------|
| 315 观察1 | 规则版本七连稳定 | 推进至八连稳定 |
| 315 观察2 | meta.json 计数漂移 | 观察3 识别同构模式（元数据未同步清理） |
| 314 | stagnation 误报 | 休眠——本轮无 stagnation 检测 |
| 226 | Lead 中断——三层无状态 | 观察1 确认又一实例 |
| 313 观察3 | 总方针-执行漂移追踪缺位 | 休眠 |

**新发现**：
- 观察2（hook 跨 session 存活）——新维度，无历史先例
- 观察3（蜂群残留目录）——315号观察2 的同构

## 结论

上下文丢失中断恢复。关键观察：

1. 226号"三层无状态"的又一实例——恢复可行但有信息损失（定理）
2. Stop-Guard + meta-observer-guard 双 hook 跨 session 正确触发（定理）
3. 蜂群残留目录未清理——元数据未同步的同构模式（发散信号）
4. 幂等任务的恢复策略——重新 spawn 是合理默认（定理）
5. 规则版本八连稳定

无规则触发/违反异常。无语法记录候选。

## 边界条件

1. 如果 concept-fixer 任务是非幂等的（如已部分修改了 relations.jsonl），重新 spawn 可能产生不一致状态——观察4 的结论翻转
2. 如果蜂群残留目录导致新蜂群创建冲突（team name 碰撞），观察3 从"无害残留"升级为需要修复的 bug

## 影响声明

本谱系不改动任何代码、定义或规则。仅记录上下文丢失恢复过程的二阶观察。
