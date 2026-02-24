---
id: '191'
title: 空 consensus 区块审计——零让步共识合法性 + refs 悬空节点
type: 矛盾发现
status: 结算态
date: 2026-02-24
source: gangju_analysis.py → Gemini verify R1
settlement: 吸收
depends_on:
  - '188'
  - '178'
related:
  - '187'
audit_targets:
  - '多轮质询管道'
---

# 191号：空 consensus 区块审计——零让步共识合法性 + refs 悬空节点

## 来源标注

[gangju_analysis.py 自动生成骨架] audit_needed=true, new_mu=1 项
→ 由 Gemini verify R1 填充

## 审计发现

### 触发原因

consensus 区块 1 个，但 residue 内容全为空——质询仅走形式未产出实质让步。

### Gemini verify 审计（R1）— conditional

**三个关键发现**:

1. **归因逻辑自相矛盾**（stance: contradictory）
   - consensus 区块的 `refs: ["unknown"]` 与 `scan_and_trigger()` 的空仪式防护逻辑上不可同时成立
   - 可能解释：A. Python 隐式布尔求值漏洞（`[None]` 绕过 `not` 检查）B. 代码演进时间差（写入发生在防护代码提交之前）

2. **inquiry_loop.py 缺少防护 ≠ 设计缺陷**（stance: reject_defect_assumption）
   - 零让步共识（初始立场即一致）时 concession 为空但共识仍成立
   - 强加空仪式防护会导致"初始即达成的共识"无法记录到区块拓扑
   - `scan_and_trigger()` 的强行跳过反而可能导致拓扑丢失

3. **`refs: ["unknown"]` 是拓扑坍塌**（stance: needs_work）
   - consensus 区块成为悬空节点（孤岛），切断因果链条
   - 不仅是完整性问题，是区块拓扑的局部坍塌

**三个隐藏假设**:

| # | 隐藏假设 | 问题 |
|---|---------|------|
| 1 | "共识必须建立在让步之上" | 忽略了零让步共识的合法性 |
| 2 | "内存状态与序列化后完全一致" | 隐式布尔求值 vs JSON 清洗 |
| 3 | "代码静态快照能解释历史数据" | 代码演进时间差 |

## 边界条件

- 审计基于当前 block-topology 中仅有的 1 组三区块
- 如果该三区块由旧版 scan_and_trigger()（无防护版本）写入，则问题1的矛盾消解
- inquiry_loop.py 的"无防护"在零让步共识场景下正确，但 stance 解析全部失败（所有 stance=None）时也会产出空区块——此边界未被测试覆盖

## 下游推论

1. ~~根因调查：确认空三区块的实际写入时间点和当时代码版本~~ **已完成**
   - 三区块由 commit bc6d1a4（v46-swarm "三层架构首次实质激活"）写入
   - 该版本的 scan_and_trigger() 无空仪式防护、无 trigger_id 查找逻辑
   - Gemini R1 解释 B（代码演进时间差）确认——问题1的矛盾消解
2. ~~防护逻辑一致性：先回答"零让步共识是否应写入区块"的概念层问题~~ **已结算（定理类推导）**
   - §21 共识仪式三区块的触发条件是质询循环收敛
   - 零让步共识 = 初始立场即一致 → 不存在质询循环 → 不存在收敛 → 不触发仪式
   - 183号目C（stance<2轮不触发）已正确实现此逻辑
   - Gemini R1 发现2的 reject_defect_assumption 结论正确：inquiry_loop 不加空仪式防护是正确设计（零让步场景由 scan_and_trigger 层面防护）
3. ~~`refs: ["unknown"]` 修复：补全悬空 consensus 区块的来源引用~~ **已完成**
   - consensus dc9e4c7f... 的 refs 已更新为 182号 event block (4f49811c...)
   - consensus_trigger.py 新增 refs 悬空防护（191号目3）：trigger_id 为 None 时拒绝写入
   - 同时移除 735/742 行的 `or "unknown"` 兜底
4. 声明-能力一致性：inquiry_loop.py 仅 mock 通过，完整管道声明待验证

## 谱系引用

- 188号：多轮质询管道收敛算法
- 178号：topology_operator（区块写入）
- 187号：架构分离伪诊断
