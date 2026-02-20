---
id: "061"
title: "孤岛审计——拓扑结构的向下区间套"
status: "已结算"
type: "语法记录"
date: "2026-02-20"
depends_on: ["059", "060"]
related: ["036"]
negated_by: []
negates: []
---

# 061: 孤岛审计——拓扑结构的向下区间套

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-20
**前置**: 059, 060

## 语法记录陈述

059号审计是宏观结构性诊断（线性/拓扑/递归三维）。本条是微观连通性诊断（孤岛扫描）。两者构成拓扑维度的区间套——大级别看结构，小级别看连通。

## 审计方法论（经 Gemini 异质否定修正）

原始方法论被 Gemini 否定 5 条，全部成立后修正：
1. 扫描全部 21 个 agent（不只 dispatch-spec 的 7 个）
2. 连通性 = SendMessage + FSMC（文件系统中介通信）
3. 谱系孤岛 = 死分支（老+未引用），不是叶节点
4. Hook 只审关键路径覆盖率
5. 语义孤岛降级为人工审计

## 审计发现

### Agent 连通性（21 个）
- 连通：13（8 dispatch-spec + 4 CLAUDE.md immediate + 1 orchestration_protocol）
- 疑似孤岛：6（有 CLAUDE.md 指导但无自动触发路径）
- 孤岛：2（go-build-resolver, go-reviewer——Go 专用，本项目 Python）

### Hook 关键路径覆盖率（5/7）
- 直接覆盖：3（definitions/、ceremony、commit）
- 部分覆盖：2（genealogy/ 缺 Edit guard、session 缺 Stop 时写入检查）
- 未覆盖：2（dispatch-spec.yaml、CLAUDE.md 无写入前置守卫）

### 谱系 DAG（68 节点）
- Hub Top 3：020(21次)、016(19次)、005b(17次)
- 死分支：5（019a/b/c 被 019 吸收、040/050 概念启动未续接）
- 孤立节点：0

## 下游行动

| # | 行动 | 优先级 |
|---|------|--------|
| A1 | 删除 go-build-resolver.md 和 go-reviewer.md | 低（清理） |
| A2 | genealogy-write-guard 注册 Edit matcher + 修改脚本支持 Edit | 高 |
| A3 | 新增 dispatch-spec.yaml 写入守卫 hook | 高 |
| A4 | 新增 CLAUDE.md 写入守卫 hook | 高 |
| A5 | 评估 6 个疑似孤岛是否需要 dispatch-spec 条目或清理 | 中 |
| A6 | 评估 040/050 死分支是否需要续接或归档 | 低 |

## 边界条件

- 如果 ECC worker agents 被纳入 dispatch-spec（作为 task_station 模板），疑似孤岛数量将降为 0
- 如果项目增加 Go 代码，go-* agents 将从孤岛变为连通

## 异质否定来源

Gemini 3.1 Pro Preview（challenge 模式）否定审计方法论 5 条，全部成立。

## 谱系链接

- 059号（structural-audit）：宏观结构审计，本条是其向下区间套
- 060号（three-plus-one-architecture）：拓扑是四支柱之一，本条是拓扑维度的深入
- 036号（spec-execution-gap）：Hook 缺口是 036 模式的又一实例
