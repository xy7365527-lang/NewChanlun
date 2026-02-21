---
id: "081"
title: "蜂群持续自动化断裂：D策略 — roadmap对象化 + scan补全 + 终止逻辑修正"
type: 选择
status: 已结算
date: "2026-02-21"
negation_source: heterogeneous
negation_model: gemini-3.1-pro-preview
negation_form: expansion
depends_on: ["041", "078", "079", "018"]
related: ["058", "075", "076"]
negated_by: []
negates: []
provenance: "[新缠论]"
---

# 081 — 蜂群持续自动化断裂：D策略决断

**类型**: 选择（多路径价值判断，路由 Gemini decide）
**状态**: 已结算
**日期**: 2026-02-21
**negation_source**: heterogeneous（Gemini 编排者代理，041号）
**negation_model**: gemini-3.1-pro-preview
**negation_form**: expansion（"干净终止"概念的精确化）
**前置**: 041-orchestrator-proxy, 078-break-blocking-loop-strategy-d, 079-downstream-suggestions-not-blockers, 018-four-way-classification
**关联**: 058-ceremony-as-swarm-zero, 075-structural-to-skill, 076-fractal-execution-gap

## 背景

蜂群在 078号 D 策略解除历史积压后，ceremony 修复了 23 个测试失败（v18-swarm），随后再次停机。session 遗留项为空，pending 谱系为 0，测试全通过，ceremony_scan.py 判定"干净终止"并停机。

但实际上系统有一个明确的 P2 工位（level_recursion）未被 scan 发现，原因是 ceremony_scan.py 只实现了 dispatch-dag 定义的 7 个 scan_sources 中的 2 个。

## 根因诊断

ceremony_scan.py 产生了**假阴性"干净终止"**：
- 扫描了：session 遗留项（0）+ 测试失败（0）+ pending 谱系（0）
- 未扫描：CLAUDE.md 目标、spec-execution gap、pattern-buffer、TODO、覆盖率、谱系张力

dispatch-dag 定义的 terminate_condition 要求"扫描全部来源后仍无工位"，但实现只扫描了 3/7 来源。这是 spec-execution gap（076号结构，不是临时 bug）。

## Gemini 决断

**选项 D（混合方案）**。

### 决断内容

1. **概念确立**：引入 `.chanlun/roadmap.yaml` 作为业务目标的结构化载体，使"长期目标"对象化，系统可以直接读取，无需从叙述性文本猜测。

2. **消除矛盾**：将 `level_recursion` 写入 roadmap.yaml，立刻解决当前无工位可派生的阻塞。

3. **修正终止逻辑**：将 `terminate_condition` 的严格定义改为：
   ```
   roadmap 为空 AND pending 谱系为空 AND 测试全过 AND pattern-buffer 无达标
   ```
   只有对象级别的全面枯竭，才能触发真阴性干净终止。

4. **scan 补全**（中期）：ceremony_scan.py 优先新增 roadmap 扫描，中期补全 pattern-buffer 和谱系张力扫描。

### 否定的方案

- **C（接受干净终止）**：违反原则7/10，系统停机是感知器官（scan）残疾的症状，不能用"合法"掩盖
- **A（独立方案：正则解析 CLAUDE.md）**：用正则解析叙述性文本提取任务极其脆弱，违反"概念优先于代码"（原则1）
- **B（独立方案：仅 roadmap 文件）**：不修正终止逻辑，假阴性问题仍存在

## 推理链（Gemini 原始推理摘要）

1. 否定 C：接受干净终止等于承认系统感知器官残疾是合法状态，这与原则7/10直接矛盾
2. 否定 A（独立）：正则解析叙述性文本是工程层暴力解法，违反概念优先
3. 否定 B（独立）：仅 roadmap 不修正 spec-execution gap，还有其他 scan_sources 未实现
4. D 的必然性：用对象化（roadmap.yaml）+ 终止逻辑修正，同时解决根因（scan 残疾）和症状（无工位）

## 同质质询判定（Claude 执行）

**定义回溯**：Gemini 引用原则1/7/10均对应 CLAUDE.md 已结算原则，引用正确。"假阴性干净终止"诊断与 ceremony_scan.py 实际代码一致（只扫描了 3 个 scan_sources）。

**反例构造**：若 roadmap.yaml 本身为空，系统仍然终止——这是合法的真阴性终止，不是假阴性。边界条件有效，不翻转。

**推论检验**：D 与 041/079/018 结构一致。roadmap.yaml 可由 Gemini decide 追加，人类 INTERRUPT 覆盖。"干净终止严格定义"补全了 spec-execution gap（dispatch-dag 定义 vs 实现），是补全而非绕过矛盾。

**判定：决断成立。**

## 边界条件

- 若 level_recursion 写入 roadmap 后，连续 3 次派生工位失败（目标过于宏大无法拆解）→ 停止，人类介入拆解
- 若 roadmap.yaml 维护变成人类专属负担（系统无法自行追加新发现的长期目标）→ 重新评估读写权限设计
- 若 NegationObject 定义修正（080号）与 005b 产生新矛盾 → 停止，启动质询序列

## 下游推论

1. **立即执行**：创建 `.chanlun/roadmap.yaml`，写入 `level_recursion`（P2）
2. **立即执行**：ceremony_scan.py 新增 roadmap 扫描（最高优先级任务来源）
3. **立即执行**：修正 dispatch-dag.yaml 的 terminate_condition 文字说明（加入 roadmap 为空条件）
4. **中期**：补全 ceremony_scan.py 的 pattern-buffer 扫描
5. **中期**：补全 ceremony_scan.py 的谱系张力扫描

## 影响声明

- 新增 081 号谱系（本文件）
- 新增 `.chanlun/roadmap.yaml`
- 修改 `scripts/ceremony_scan.py`（新增 roadmap 扫描）
- 修改 `dispatch-dag.yaml`（terminate_condition 说明修正）
- 涉及模块：ceremony 入口、任务发现机制
