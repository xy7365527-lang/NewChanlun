---
id: '358'
number: 358
title: "ceremony_scan 双向扫描能力结算"
type: engineering-completion
status: settled
settled_at: 2026-03-05
date: 2026-03-05
source: v159-swarm R2/R3 实现 + R3 rescan 验证
depends_on:
  - '353'   # 消费断裂与声明缺口等价性——双向扫描的理论基础
  - '357'   # ceremony_scan 消费器到提议器增强——实现层
epistemology:
  - level: L0
    scope: 实现完整性结算
    description: 353号诊断 + 357号实现 + R3 rescan 输出确认，无新经验假设

definitions_involved:
  - name: ceremony_scan
    version: v159-swarm-r3
    role: 被增强的核心组件——双向扫描能力已落地
  - name: spec-execution-gap SKILL.md
    version: v159-swarm-r3
    role: 消费断裂检测方向已补充（B方向）
  - name: gangmu.yaml ceremony-scan-bidirectional action
    version: v159-swarm-r4
    role: "completion_check 目标——本谱系满足 genealogy_settled: scan-bidirectional"

resolution:
  type: 结构重组
  description: |
    353号诊断了 ceremony_scan 的单向性（只消费 gangmu，不从谱系提议），
    357号实现了 _scan_genealogy_proposals（谱系→提议方向），
    spec-gap SKILL.md 增加了消费断裂检测（B方向）。
    R3 rescan 确认提议器生效（输出 11 条 proposed_new_mu）。
    双向扫描能力从诊断到实现到验证的完整闭环已完成。
  decided_by: 编排者（v159-swarm session）

new_output:
  definitions: []
  code_changes:
    - "ceremony_scan.py: +_scan_genealogy_proposals() 阶段（+154行，R2完成）"
    - "spec-execution-gap SKILL.md: +消费断裂检测方向（+19行，R3完成）"
  orchestration_changes:
    - "ceremony_scan 输出新增 proposed_new_mu 字段——11条提议已通过 R3 rescan 验证"

impact:
  affected_modules:
    - scripts/ceremony_scan.py
    - .claude/skills/spec-execution-gap/SKILL.md
  affected_definitions:
    - ceremony_scan 的角色定义（从消费器扩展为消费器+提议器——357号 split 的落地确认）
  downstream_implications:
    - "proposed_new_mu 的自动注入机制——当前是提议，需要人工或脚本注入 gangmu（357号边界条件1未闭合）"
    - "downstream_implications 字段规范化——统一为 downstream_implications 而非多种写法（357号边界条件2未闭合）"

retroactive_settlement:
  settled_by: v159-swarm-r4
  settlement_date: 2026-03-05
  settlement_description: |
    gangmu.yaml 中 ceremony-scan-bidirectional action 的 completion_check 要求
    genealogy_settled: scan-bidirectional。本谱系直接满足此条件。
    实现链：353号(诊断) → 357号(实现) → 358号(结算)。

related_records:
  parent: '357'  # 357号是实现，358号是结算确认
  children: []
  siblings:
    - '353'  # 353号是理论基础（消费断裂等价性）
---

# 358号：ceremony_scan 双向扫描能力结算

## 推导链

1. **353号**诊断了 ceremony_scan 的单向性问题：只消费 gangmu.yaml 中已有的 active mu，无法从谱系 downstream_implications 发现新工作
2. **357号**实现了解决方案：`_scan_genealogy_proposals()` 阶段，扫描近期谱系的 downstream_implications 并与 gangmu.yaml 已有 target 对比，输出 proposed_new_mu
3. **spec-gap SKILL.md** 更新增加了消费断裂检测（B方向：产出→消费），与 A方向（声明→能力）形成双向覆盖
4. **R3 rescan** 确认提议器生效：输出 11 条 proposed_new_mu，证明从谱系到提议的管道畅通
5. gangmu.yaml 中 `ceremony-scan-bidirectional` action 的 `completion_check: genealogy_settled: scan-bidirectional` 由本谱系满足

## 完成状态

| 组件 | 状态 | 轮次 |
|------|------|------|
| `_scan_genealogy_proposals()` | 已实现（+154行） | R2 |
| spec-gap SKILL.md B方向 | 已更新（+19行） | R3 |
| R3 rescan 验证 | 11 条 proposed_new_mu 输出 | R3 |
| 本谱系结算 | settled | R4 |

## 未闭合的边界条件（继承自357号）

1. **proposed_new_mu 自动注入**（357号边界条件1）：提议输出如果没有消费者，则本增强本身就是 353号模式的又一个实例。当前需人工或脚本注入 gangmu
2. **downstream_implications 字段规范化**（357号边界条件2）：当前写法不统一，提议器扫描逻辑需容忍多种格式
3. **L2 验证窗口**（357号边界条件3）：提议机制的有效性需在下一轮 ceremony 中观测

## 影响声明

- **满足 gangmu.yaml 中 ceremony-scan-bidirectional 的 completion_check**
- **确认 353→357→358 的实现链完整闭合**
- **不引入新代码变更**——本谱系是纯结算记录
- **不改动任何定义文件**
