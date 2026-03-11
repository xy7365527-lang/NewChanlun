---
id: '412'
number: 412
title: "OpenClaw 作为 ceremony 子系统 liveness 节点——非认知常驻心跳"
type: grammar-record
status: 已结算
date: 2026-03-11
source: v216-swarm/openclaw-liveness
depends_on:
  - '409'   # 统一运行时裁决——daemon=liveness节点
  - '411'   # v215-swarm 元观察
negation_source: homogeneous
negation_form: expansion
topo_effect: "expand:409-principle-7:openclaw-liveness → ceremony维度liveness节点实装"
tensions_with: []
epistemological_level: L0
---

# 412号：OpenClaw 作为 ceremony 子系统 liveness 节点

## 背景

409号原则7确立：daemon 的唯一特权是 liveness，不是 authority。逢亮 daemon 承担研究推进维度的 liveness（穿越步进 + handoff schema），但 ceremony 维度（ceremony_scan 定时检查是否有新工位需要处理）缺少对称的 liveness 节点。

CC session 是一次性工作脉冲（409号原则8），session 结束后没有持久进程监控是否有新工位涌现。用户需要手动启动 CC session 才能触发 ceremony_scan。

## 架构决策

OpenClaw 作为 ceremony 维度的 liveness 节点实装。

### RTAS 拓扑中的位置

| 节点 | 维度 | liveness 类型 | 认知能力 |
|------|------|--------------|---------|
| 逢亮 daemon | 研究推进 | 穿越步进 + handoff schema | 无（调度器） |
| OpenClaw | ceremony | ceremony_scan 定时检查 | 无（闹钟） |
| CC 蜂群 | 认知工作 | session 级别（一次性脉冲） | 有（唯一认知主体） |
| Gemini | 异质审计 | 按需调用 | 有（审计视角） |
| Codex | 代码审查 | 按需调用 | 有（代码视角） |

### 耦合接口

OpenClaw 与蜂群的耦合接口是 `ceremony_scan.py --summary` 的 stdout——一个纯文本的工位摘要。这是唯一接口：

- OpenClaw 不读取谱系文件
- OpenClaw 不修改任何项目文件
- OpenClaw 不做认知判断
- OpenClaw 不触发 CC session

### 实装内容

1. `ceremony_scan.py` 新增 `--summary` 参数：输出简洁的工位摘要（工位数 + 名称列表），替代完整 JSON 输出
2. OpenClaw workspace 指向 `G:\NewChanlun`（agent 能读取项目文件执行脚本）
3. OpenClaw cron job：每 30 分钟运行 ceremony_scan，有工位时通知用户
4. BOOTSTRAP.md：描述 agent 的角色和约束

### 与逢亮 daemon 的对称性

| 维度 | 逢亮 daemon | OpenClaw |
|------|-----------|----------|
| liveness 类型 | 研究推进（穿越步进） | ceremony（工位扫描） |
| 物理形态 | Python 进程 | OpenClaw agent（cron job） |
| 认知能力 | 无 | 无 |
| 耦合接口 | handoff schema (JSON) | ceremony_scan stdout |
| 恢复策略 | 读 schema 继续 | 下一个 cron 周期重试 |

## 边界条件

1. 如果用户需要即时通知（不等30分钟），可调整 cron 间隔或手动触发
2. 如果 ceremony_scan.py 运行时间超过 cron 周期，OpenClaw 的 maxConcurrentRuns=1 防止重叠
3. 如果用户配置了 WhatsApp/Telegram channel，通知可以跨平台推送——这是 OpenClaw 的原生能力，不需要额外开发

## 下游推论

1. WhatsApp 通知集成：用户配置 WhatsApp channel 后，ceremony liveness 通知自动通过 WhatsApp 推送 [deferred]
2. 多 channel 扩展：同一 cron job 的 delivery 配置可扩展到 Telegram/Slack 等 [deferred]
3. 逢亮 daemon 与 OpenClaw 的交互：daemon 产出新 handoff schema 后，下一个 cron 周期的 ceremony_scan 会检测到变化并通知用户 [deferred]

## 影响声明

- 新增 `ceremony_scan.py --summary` 参数（代码变更）
- 修改 OpenClaw workspace 配置指向项目根目录
- 新增 BOOTSTRAP.md（OpenClaw workspace）
- 新增 cron job（30分钟间隔 ceremony liveness 检查）
- 不修改任何现有蜂群规则或架构

## 谱系关联

- 父记录：409号（统一运行时裁决——daemon=liveness节点，原则7"唯一特权是liveness"）
- 相关：411号（v215-swarm 元观察——工程密集期方法论回归）
- 相关：069号（递归拓扑异步自指蜂群——DAG 平等性）
- 实装触发：v216-swarm openclaw-liveness 工位
