---
id: '459'
number: 459
title: "K_active 写入路径审计——5类写入路径的器官原则合规性判定"
type: structural-audit
status: 已结算
date: 2026-03-16
source: "[元编排] v246-swarm R2 batch——438号下游推论1审计"
negation_source: ""
negation_form: ""
topo_effect: ""
depends_on:
  - '438'   # S_net 唯一界面原则（器官原则）——本号审计其推论1
  - '439'   # 存在论架构（图=身体，S_net=器官）
epistemological_level: "L0（代码审计——从代码事实推导）"
tensions_with: []
---

# 459号：K_active 写入路径审计

**认识论等级**: L0（代码审计）

## 审计目标

438号下游推论1：系统性扫描所有向 K_active 添加边/顶点的代码路径，判断哪些经过了穿越（合规）、哪些直接注入（可能违规）。

## 审计范围

扫描 `topological-computation/` 目录下所有包含 `k_active.add_edge` 或 `k_active.add_vertex` 的文件：

1. traversal.py
2. daemon.py
3. daemon_api.py
4. execution_callback.py

ceremony.py 不包含 K_active 写入——逢亮创世由 daemon.py 的 bootstrap 逻辑处理。

## 审计结果

### 类别1：穿越事件产物（合规——经由 S_net 器官）

| 文件 | 行号 | 边/顶点类型 | 合规性 | 理由 |
|------|------|------------|--------|------|
| traversal.py:1075-1112 | COOCCURRENCE 边 + 新顶点 | 合规 | S_net 超边激活产出共现边——物质层边，经由 S_net 器官 |
| traversal.py:1148 | TRAVERSAL_ASSOCIATION 边 | 合规 | 穿越步进副产物——物质层沉积 |
| traversal.py:1313 | ARTICULATED 边 | 合规 | ARTICULATE 遭遇产出——经由穿越→经由 S_net |
| traversal.py:1616-1655 | REFERENCE 边 + bridge 顶点 | 合规 | articulation_feedback 产出——S_net EdgeSuggestion 经由穿越消费 |
| traversal.py:752-850 | k_full 写入 | N/A | 仅写 k_full（全量记录），不写 k_active 概念层 |

### 类别2：辩证操作产物（合规——经由穿越事件触发）

traversal.py 中的 fold/negate/sublate 操作（engine.py 中实装）产出的 FOLD/NEGATION/SUBLATION 边通过 engine.py 的操作函数写入。这些操作由穿越引擎的 `_resolve_encounter` 触发（traversal.py:783-873），间接经由 S_net（穿越在 S_net 层运动后遭遇）。合规。

### 类别3：系统拓扑标记（合规——自反性规范，非外部语言材料）

| 文件 | 行号 | 写入内容 | 合规性 | 理由 |
|------|------|---------|--------|------|
| daemon_api.py:640-646 | source:{session_id} 顶点 | 合规 | 已标注审计意见（daemon_api.py:634-636）：系统拓扑标记（proprioception），不携带用户语义内容 |
| daemon_api.py:679 | REFERENCE 边（概念→source 顶点） | 合规 | 已标注审计意见（daemon_api.py:659-661）：K_active 内部拓扑重连，不引入新概念 |

### 类别4：测试回调注入（边界情况——execution_callback.py）

| 文件 | 行号 | 写入内容 | 合规性 | 理由 |
|------|------|---------|--------|------|
| execution_callback.py:234 | test_run 顶点 | **边界** | 测试结果作为系统反馈写入 K_active。类似于 proprioception——系统自我观测。当前实装注入测试通过/失败信息，不携带外部语义 |
| execution_callback.py:245 | 测试关联边 | **边界** | 同上——测试结果与概念节点的关联。语义来源是代码执行结果，不是外部语言材料 |

判定：execution_callback.py 的写入属于**系统自反性规范**（类似 daemon_api.py 的 source vertex），不违反器官原则。但与 daemon_api.py 不同的是，没有明确的审计标注——建议补充。

### 类别5：daemon.py 批量写入

daemon.py 中的 `add_vertices_and_edges_batch` 调用（1865-1904行）用于从持久化恢复、跨域边注入、S_net 语料摄入回调。这些路径的上游来源均为：
- S_net 共现边（经由 S_net 器官）→ 合规
- 穿越事件副产物 → 合规
- 跨域边检测（daemon 自反性回路）→ 系统拓扑标记，合规

## bootstrap 例外评估（438号下游推论2）

ceremony.py 中不包含 K_active 写入。逢亮创世由 `daemon.py:_bootstrap_graph()` 处理，从持久化文件恢复图状态，或从空图开始。空图创世时的初始节点来自 S_net 首次摄入（ceremony 文本经 S_net 器官处理后产出共现边）。

**结论**：不需要"创世例外"条款。ceremony.py 触发 daemon 启动，daemon 从 S_net 摄入开始建图——初始状态经由 S_net 器官。

## 审计总结

| 类别 | 路径数 | 合规性 |
|------|--------|--------|
| 穿越事件产物 | 5 | 全部合规 |
| 辩证操作产物 | 3 | 全部合规 |
| 系统拓扑标记 | 2 | 全部合规（已标注） |
| 测试回调注入 | 2 | 边界合规（建议补充审计标注） |
| daemon 批量写入 | 3 | 全部合规 |

**0 个违规路径。438号器官原则在当前代码中成立。**

## 边界条件

| 条件 | 当前 | 翻转阈值 |
|------|------|----------|
| execution_callback.py 审计标注 | 缺失 | 建议补充（低优先级——功能合规，仅缺标注） |
| 新增 K_active 写入路径 | 0 违规 | 任何新的直接 K_active 写入需经过器官原则审查 |
| bootstrap 例外 | 不需要 | 若 ceremony.py 未来添加直接 K_active 写入，需要重新评估 |

## 影响声明

- **438号下游推论1**: 审计完成——0 个违规路径，器官原则成立
- **438号下游推论2**: bootstrap 例外不需要——创世经由 S_net 器官
- **438号 tensions_with '实装审计待定'**: 已解决——审计完成，无实装层面的器官原则违反
