# Gemini verify 模式：计算过程 vs 对话过程的架构分离审计（v48）

## 纲（审计基准）

**递归运动在计算质料上产生它自己的亏格。**

从这个纲推导：什么是"计算过程"（确定性、可无人值守），什么是"对话过程"（需要推理引擎、context window、inter-agent 协商）？

---

## 系统当前状态（物质证据）

### block-topology 统计（2026-02-24）

- block_count: 197（186 迁移 + 15 cc产出）
- relation_count: 960+
- type分布：event 191, rewrite 3, consensus 1, residue 1, tension 1
- relations类型：depends_on 主要，negates 12, negated_by 13, freezes 少量
- source分布：migration 182, cc 15

### 谱系状态

- 186 settled / 0 pending
- 最近谱系（183-186号）全部关于动力学缺失诊断

### 代码管道状态

确定性管道（可无人运行）：
- ceremony_scan.py: 确定性扫描
- gangju_analysis.py: 确定性推导
- topology_operator.py: 确定性写入
- consensus_ceremony.py: scan_and_trigger + write_consensus_ceremony
- block_topology.py: 确定性读写

尚不存在的管道：
- 异步自指（t审查t-1）：零代码实现
- 多轮质询循环：单轮提取存在，跨轮聚合不存在
- Nachträglichkeit 触发链路：schema有，调用者无
- gangju_analysis 的 Gemini/Codex 审计调用：脚本输出 audit_needed 但不调用 API

---

## 关键谱系引用（用于定义回溯）

### 183号核心判断（Gemini+Codex 双向审计收敛）

**系统有拓扑结构但没有动力学。** 三层架构的"激活"是形式上的——consensus/residue/tension 区块存在但为空壳，因为产生它们的过程（多轮质询循环 + 异步自指）从未执行。

### 184号核心判断

"空 residue 被当成正常写入"——在没有异步自指和多轮质询的情况下，系统不可能自发产生 tension。

### 182号第二个下游推论（部分修正）

"三层架构从首批区块起实质激活"被降级为"部分解决——形式激活但动力学未启动"。

---

## 审计问题（五个）

### 问题1：计算/对话分类

当前体系中，哪些模块属于"计算过程"（确定性、可无人值守）？哪些属于"对话过程"（需要推理引擎、context window、inter-agent 协商）？

给出完整清单，并说明分类依据是什么（从纲推导，不是从工程便利性推导）。

### 问题2：降级可能性

"对话过程"中，哪些实际上可以降级为"计算过程"？

已知线索：立场差分计算（parse_stance_sequence）已经是确定性的 Python 代码。质询触发（scan_and_trigger）也是确定性的。但质询本身（Gemini 推理、Codex 审查）显然不能降级。

### 问题3：主体介入门槛（核心问题）

编排者的核心问题："体系达到了不需要主体介入就能维持辩证推进的程度了吗？"

从纲推导，答案的严格形式是什么？

注意：183号已诊断"动力学机制缺失"。在动力学机制（多轮质询+异步自指）尚未实现的前提下，这个问题的答案能确定吗？

### 问题4：半自主架构的严格定义

"半自主架构"的严格定义是什么？哪些关键节点必须暂停等待人类介入？判断标准是什么？

从纲推导：如果"递归运动在计算质料上产生它自己的亏格"，那么"产生亏格"的环节（质询循环）是否必须有人类介入？还是 Gemini API 调用就够了？

### 问题5：183号诊断对架构分离的含义

183号诊断（动力学缺失）对这个计算/对话架构分离意味着什么？

如果动力学机制（多轮质询+异步自指）实现了，架构分离的答案会改变吗？具体改变什么？

---

## 对审要求

- verify 模式：找缺口，找否定，不是辅助
- 从纲推导，不从工程便利性推导
- stance-declaration YAML 是必须输出
- 输出总长度 ≤ 8KB
