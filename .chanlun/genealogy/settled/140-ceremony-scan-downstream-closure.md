---
id: '140'
title: ceremony_scan 闭合下游推论→工位的数据通路（蜂群持久化修复）
type: 定理
status: 已结算
date: 2026-02-22
depends_on:
  - '138'   # 强制输出格式A/B/C
  - '134'   # 声明-能力缺口同构模式
related:
  - '137'   # Lead停顿RLHF根因
  - '139'   # 三级映射+约束型充分性
tensions_with: []
---

## 事件

138号修复后 Lead 仍然停顿——输出格式B（"无待做行动"）而实际上有三个下游推论可推进。Gemini 诊断：问题不在规则层（Lead 遵守了138号格式），而在数据层（ceremony_scan.py 不把 downstream_audit 的 unresolved 项转化为工位）。

## 推导链

1. 138号修复了输出格式（否定性禁令→结构化模板）
2. 139号结算后 Lead 选择格式B → ceremony_scan 返回空工位 → Lead 合法判断"无待做行动"
3. ceremony_scan.py 第292行调用了 downstream_audit → 但只将结果写入统计字段，不转化为 workstation
4. dispatch-dag.yaml ceremony_sequence.scan_sources 声明了"谱系下游行动未执行"为扫描来源 → 声明存在，实现缺失
5. 这是134号声明-能力缺口的又一实例
6. 修复：ceremony_scan 将 downstream_audit 的 recent unresolved 项（最近20条谱系）转化为 P2 工位

## 关键区分

**两种停顿原因**：
- RLHF 驱动的停顿：Lead 有信息但不执行 → 需要格式约束（137/138号）
- 数据源缺口的停顿：Lead 没有信息所以不执行 → 需要工程修复（本号）

蜂群持久化的本质不是"行为约束"问题而是"信息通路"问题——行动队列已隐式存在于谱系文件的"下游推论"节中，只需让 ceremony_scan 正确读取。

## 边界条件

- 如果修复后 Lead 仍在有工位时选择格式B → 问题回归137号（RLHF驱动），需更强格式约束
- 如果 downstream_audit 误报率过高 → 需调整判定逻辑或 overrides
- 随谱系增长（147→500+），全量扫描可能变慢 → 性能优化（增量/缓存），不影响正确性

## 下游推论

1. 体系中可能存在其他"声明了但未实现"的 scan_sources → 需做完整 spec-execution gap 审计
2. 未来类似问题应先检查数据流是否闭合，再检查行为规则

## 影响声明

- 修改 `scripts/ceremony_scan.py`：downstream_audit unresolved → workstation 转化
- 闭合 dispatch-dag.yaml ceremony_sequence.scan_sources 中"谱系下游行动未执行"的声明-实现缺口
