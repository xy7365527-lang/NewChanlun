---
id: "083"
title: "干净终止过早诊断：D策略——确认阶段性里程碑，基于对象矛盾派生两个收尾任务"
type: 选择
status: 已结算
date: "2026-02-21"
negation_source: heterogeneous
negation_model: gemini-3.1-pro-preview
negation_form: expansion
depends_on: ["081", "076", "079", "018", "041"]
related: ["069", "001", "058"]
negated_by: []
negates: []
provenance: "[新缠论]"
---

# 083 — 干净终止过早诊断：D策略决断

**类型**: 选择（多路径价值判断，路由 Gemini decide）
**状态**: 已结算
**日期**: 2026-02-21
**negation_source**: heterogeneous（Gemini 编排者代理，041号）
**negation_model**: gemini-3.1-pro-preview
**negation_form**: expansion（"干净终止合法性"概念的精确化）
**前置**: 081-swarm-continuity, 076-fractal-execution-gap, 079-downstream-suggestions, 018-four-way-classification, 041-orchestrator-proxy
**关联**: 069-recursive-topology, 001-degenerate-segment, 058-ceremony-as-swarm-zero

## 背景

系统在定义偏序链全节点实现完毕（14 定义、1435 测试全通过、92 谱系已结算）后，
roadmap.yaml 中全部 5 个任务 status=completed，ceremony_scan.py 判定 clean_terminate=true，
系统停机。

人类编排者指出这不应发生，并提出四个选项（A/B/C/D）。

ceremony_scan.py 的 downstream_actions 字段显示：total=73，unresolved=8，execution_rate=67%。

## 决断问题

1. downstream_actions.unresolved=8 是否应阻止 clean_terminate？
2. 选项 A/B/C/D 哪个最符合系统原则？
3. 如选 D，从 10 个可能来源中选哪 2-3 个写入 roadmap？

## Gemini 决断

**选项 D（混合方案）**：确认阶段性里程碑合法，但基于对象矛盾（不是阈值）派生 2 个任务。

### 决断内容

1. **downstream_actions.unresolved=8 不阻止 clean_terminate**。
   根据 076号（下游推论靠主动认领）和 079号（长期阻塞项是背景噪音），
   这 8 项（全为长期工程项或平台阻塞）不构成强制阻塞。
   ceremony_scan.py 当前的 clean_terminate 逻辑正确。

2. **roadmap "为空" = active 任务为 0**（不是文件为空）。
   已 completed 的任务是已结算对象，保留在文件中。

3. **派生 2 个 active 任务**：
   - **bi_mode_new_default**（P1）：笔引擎 mode='new' 切换为默认值
     前置条件（real_data_validation）已满足，bi.md #4 明确记录此状态跃迁待执行
   - **recursion_termination_consistency**（P1）：统一 dispatch-dag 递归终止表述
     069号谱系 unresolved 项 4：第197-198行用"背驰+分型"，第377行用"区间套收敛"

### 否定的方案

- **A（接受干净终止）**：概念层遗留明确矛盾（递归终止表述不一致）和未完成的状态跃迁（bi mode 切换）。接受 A = 绕过已暴露的矛盾，违反原则 2（no-workaround）
- **B（扩展 scan 扫描覆盖率/代码质量）**：覆盖率 80% 和代码行数 800 是阈值否定，违反原则 8（对象否定对象）。系统不能由人为数值阈值驱动工作
- **C（自动从 TODO 派生）**：TODO 缺乏严格上下文，会导致系统被低价值琐碎任务淹没，丧失主线方向

## 推理链（Gemini 原始推理摘要）

1. **为什么拒绝 A**：概念层还有遗留矛盾（递归终止表述不一致 + bi mode 状态跃迁未完成），接受 A = 绕过矛盾
2. **为什么拒绝 B**：覆盖率/代码行数是阈值否定，违反"对象否定对象"原则
3. **为什么拒绝 C**：TODO 驱动导致系统方向失控，丧失主线
4. **为什么选这 2 个任务**：
   - bi_mode_new_default：前置已满足的状态跃迁，是核心对象（笔）的明确未完成项
   - recursion_termination_consistency：已记录于 069号谱系的概念矛盾，是系统自己发现并记录的问题

## 同质质询判定（Claude 执行）

**定义回溯**：
- Gemini 引用 076号（下游推论靠主动认领）：正确，已结算
- Gemini 引用 079号（背景噪音不是工位）：正确，已结算
- Gemini 引用原则 2（不绕过矛盾）：正确，CLAUDE.md 元编排原则
- Gemini 引用原则 8（对象否定对象）到编排层：有轻微过度推广（该原则原本是走势描述语言的语法规则，但扩展到编排层结论方向仍然正确）

**反例构造**：
- 若 bi_mode_new_default 导致大面积测试失败 → 可能揭示核心定义缺陷，不是简单收尾任务。边界有效，Gemini 也已记录此边界
- 若 recursion_termination_consistency 分析发现两种表述不等价 → 走矛盾上浮，任务目标从"统一表述"变成"上浮矛盾"。边界有效，不构成反例

**推论检验**：
两个任务都是对象驱动（状态跃迁 + 已记录矛盾），不是阈值驱动。与 081/079/076 体系一致。

**判定：决断成立。**

## 执行结果

roadmap.yaml 已写入两个 active 任务（version 1.3 → 1.4）：
- id: bi_mode_new_default（P1, active）
- id: recursion_termination_consistency（P1, active）

## 边界条件

- 若 bi_mode_new_default 导致大面积回归失败且无法不修改定义就修复 → 停止，走矛盾上浮
- 若 recursion_termination_consistency 分析发现两种表述不可调和 → 生成新谱系，上浮
- 若人类编排者提供新的战略级 roadmap → 当前 2 个任务作为子任务并入新 roadmap
- 2 个任务完成后，系统将再次 clean_terminate。后续方向（回测框架/多品种验证等）需要人类战略级输入

## 下游推论

1. **立即执行**：roadmap.yaml 写入两个 active 任务（已完成）
2. **立即执行**：ceremony_scan.py 发现 2 个 active 任务，系统不再干净终止
3. **中期**：2 个任务完成后，ceremony_scan.py 再次判定 clean_terminate=true。此时为真正的阶段性里程碑，需要人类提供下一战略方向
4. **语法记录候选**：dispatch-dag 中"谱系下游行动未执行"作为强制阻塞来源的表述应修正——根据 076/079，它应为建议来源而非强制阻塞

## 影响声明

- 新增 083 号谱系（本文件）
- 修改 `.chanlun/roadmap.yaml`（version 1.3→1.4，新增 2 个 active 任务）
- 涉及模块：ceremony 任务发现机制、bi 引擎默认模式、dispatch-dag 递归终止表述
