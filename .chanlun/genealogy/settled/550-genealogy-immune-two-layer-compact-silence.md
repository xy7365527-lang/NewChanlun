---
id: '550'
number: 550
title: "谱系免疫两层架构——compact 使事件驱动快层静默，ceremony_scan 慢层批量补偿；delta_settled=0 是快层静默的可观测信号"
type: meta-rule
status: settled
date: '2026-06-22'
settled_date: '2026-06-22'
level: "L0（ceremony_scan async_self_ref stagnation 逻辑 + 本 session 积压观测）+ L2（delta_settled=0 实测 over t-1(0301)→t(0312)，无 git commit）"
负责工位: genealogist（session-4130a713/genealogy-maintenance，谱系免疫积压批处理衍生）
provenance: "[新缠论:元编排]（RTAS 异步自指 069 + 结构工位 skill 化 075）"
negation_source: homogeneous
negation_form: separation
negates: "隐含命题『事件驱动结构工位 = 谱系免疫的完整实现』——被证伪：autocompact 不是 skill 监听的事件 ⟹ 事件驱动快层在 compact 期静默 ⟹ delta_settled=0 stagnation；谱系免疫的完整性依赖 ceremony_scan 慢层（文件系统扫描）的批量补偿"
topo_effect: "split:genealogy-immune-system:downstream（谱系免疫系统分裂为两层——事件驱动快层 + ceremony_scan 文件系统慢层；快层 compact 静默，慢层下次 ceremony 批量补偿）"
depends_on:
  - '069'   # 递归拓扑异步自指——delta_settled stagnation 检测的本体依据（t 审查 t-1）
  - '075'   # 结构工位 teammate→skill+事件驱动——快层「静默」的结构来源
related:
  - '407'   # compact 后声明层效力为零——快层静默的同源机制（行为层不靠声明层恢复）
  - '137'   # RLHF 基底——行为层恢复依赖文件系统注入而非声明层概率引导
  - '074'   # 下游推论执行缺口——谱系免疫债的先例
  - '181'   # ceremony_scan 职责边界——慢层补偿的执行点
tensions_with: []
---

# 550 号：谱系免疫两层架构——compact 使事件驱动快层静默，ceremony_scan 慢层批量补偿

**认识论**：L0（ceremony_scan stagnation 逻辑 + 本 session 积压结构）；delta_settled=0 over t-1→t 为 L2 实测。

## 一、触发（async_self_ref stagnation finding）

ceremony_scan 的异步自指审计（069号目B：t 审查 t-1）报告：

```
t-1 (2026-06-22-0301, settled_count=515) → t (2026-06-22-0312, settled_count=515)
delta_settled = 0，且无 git commit 活动 ⟹ RTAS 循环可能停滞
findings_count=1, genealogy_needed=true
```

本 session 经历密集 autocompact。事件驱动的 genealogist skill 在 compact 期间**没有触发**——谱系免疫积压成结构债（ceremony_scan 同轮识别出 21 项谱系编号异常 + 17 个未映射谱系）。

## 二、概念分离：谱系免疫不是一层，是两层

| 层 | 实现 | 触发 | compact 期状态 |
|---|---|---|---|
| **快层（事件驱动）** | genealogist skill（每概念发现后触发） | 事件（概念发现、定义变更广播） | **静默**——autocompact 不是 skill 监听的事件 |
| **慢层（文件系统扫描）** | `ceremony_scan.py`（detect_genealogy_anomalies + async_self_ref） | ceremony（创世时刻批量扫描文件系统） | **存活**——扫描的是文件系统状态，不依赖事件流 |

隐含命题「事件驱动结构工位 = 谱系免疫的完整实现」被证伪：快层有一个**盲区**——它静默的时刻（compact）恰恰是上下文丢失、最需要免疫的时刻。免疫的完整性由慢层的**文件系统真值扫描**保证。

## 三、自我修正回路（本 session 即实例）

`delta_settled=0` stagnation **不是系统死亡**，是快层静默的**可观测信号**。慢层的响应链条：

```
快层 compact 静默 → 谱系债累积（文件系统留痕：缺 frontmatter / 未映射 / 未结算）
  → 下次 ceremony 慢层扫描检测异常 → 生成 P0 积压工位
  → Lead spawn genealogist 工位（本 session）→ 批量补偿债务 → delta_settled 重新增长
```

慢层的"真值在文件系统"是关键（同 137号/407号：行为层恢复靠文件系统注入，不靠声明层）——快层丢失的是上下文（声明层记忆），慢层读的是磁盘（不可丢失的真值）。

## 四、边界条件（结论翻转条件）

- 若 autocompact 能被注册为 skill 监听的事件（快层在 compact edge 触发一次补偿扫描）→ 快慢两层的时间差消除，慢层不再需要批量补偿——本号的「两层分裂」退化为「一层 + compact hook」。
- 若 ceremony 频率 < compact 频率（慢层补偿来不及）→ 谱系债单调累积，stagnation 不再自愈，需要独立的债务监控。

## 五、下游推论

1. `delta_settled=0` + 无 commit **不应触发停机告警**——它是快层静默的正常信号，慢层会补偿。错误地把它当「系统死亡」会导致过度干预。
2. 谱系积压量 = compact 频率 × 每次 compact 期间的概念发现量 / ceremony 频率。压缩越密集，慢层补偿批越大。
3. genealogist skill 的事件触发列表应考虑增加「compact 边界」事件（待评估，属快层增强，非本号结算范围）。

## 六、上游谱系

- **069号**（递归拓扑异步自指）：stagnation 检测 = t 审查 t-1 的直接应用；本号是其一个 finding 的结算。
- **075号**（结构工位 teammate→skill+事件驱动）：快层「事件驱动」的结构来源——本号揭示该设计的 compact 盲区。
- **407号 / 137号**：compact 后声明层效力为零 / 行为层靠文件系统恢复——慢层「真值在磁盘」与之同源。
- **074号**（下游推论执行缺口）：谱系免疫债的先例（同为「该做未做」的免疫缺口）。

## 七、影响声明

谱系结晶，未改代码。揭示谱系免疫系统的两层（事件驱动快层 + ceremony_scan 慢层）架构与 compact 盲区。指向 genealogist skill 触发列表的潜在增强（compact 边界事件，生成态待评估）。
