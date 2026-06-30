# /escalate: acc-delta-r-alpha CHECK_PASS 过早 + reducer 无法撤销（编排者裁决待）

> 2026-06-30 自主推进中发现。编排者睡眠中，本报告durable记录待醒后裁决。

## 矛盾

**acc-delta-r-alpha 已 passed=True（commit adce1d93dd），但约束4异质审计判 UNDERPOWERED（非有效falsification）。两者冲突，且 event-sourcing reducer 的 CHECK_PASS 单调（无 CHECK_FAIL/revoke 事件），无法撤销。**

## 双方依据

**工位 CHECK_PASS 依据**：acc-delta-r-alpha 验收文 = "L3实证 + 分层诊断"。工位跑了 8品种 walk-forward（commit 230691e0bd，l3_delta_r_alpha.rs 528行，Lead真封 1256 passed），产出否定结果（符号检验 p=0.5，χ无系统性alpha），声称"实证达成=验收满足"。

**codex 异质审依据（UNDERPOWERED 判定）**：
1. n=5 符号检验 p=0.5 仅"不显著"非"无效应"，功效近零=样本饥饿，连 4/5 正也不达单边显著——不构成 falsification
2. 3 退化品种(BTC/ES/QQQ χ空仓)须审作实装结果(μ坍缩/成本符号bug/θ=0过严/holdout类未见于IS)，非静默当"无alpha"
3. walk-forward 方向正确但仅当 θ=0/截断/universe 预先指定才无泄漏
4. 诚实结论="此实装下未检测到系统性alpha" ≠ "alpha不存在"

## 矛盾的本质（两个层面）

**层面A：验收判据太弱**。原文"≥1/8品种 ΔSharpe≠0 且 n_beats>0"会在噪声上通过（codex：8品种≥1偶然正99.6%）。真严格判据=跨品种系统性显著，工位用了更强标准得否定，但否定本身又 underpowered。判据需 GOAL_AMEND 收紧（但 GOAL_AMEND 仅支持 ACCEPTANCE_ID_BINDING，不支持判据重写）。

**层面B：reducer 无法表达"passed→invalidated"**。CHECK_PASS 单调累积，无撤销事件。acc-delta-r-alpha 一旦 passed 永久 passed，goal 会在 martingale-guard 通过后假闭合（5/5）。

## 选择（待编排者裁决）

1. **判 acc-delta-r-alpha 未达成**：需补(a)退化品种诊断(μ估计 vs bug) (b)功效分析/更大n (c)pre-registration确认。但 reducer 无法 un-pass——需加 CHECK_FAIL/revoke 事件类型或 SCHEMA 改判据。
2. **判 underpowered 结果"诚实完成实证"=达成**：验收文是"L3实证"非"证明alpha存在/不存在"，underpowered 是诚实科学结果。则 acc-delta-r-alpha passed 合理，但须在 goal 结论标注"未检测到≠不存在"。
3. **GOAL_AMEND 收紧判据 + 重跑**：把 acc-delta-r-alpha 改为"跨品种系统性 + 退化品种审 + 功效达标"，重置 passed。

## 关联谱系
- 658（reducer acceptance rollup SCHEMA 缺口）——同源:reducer 验收语义未完备(无撤销+rollup未定义)
- 645/656（覆盖≠盈利 / 细分类≠盈利）——本否定结果同域,但 underpowered 削弱其作为"印证"的力度
- 231（有效域）——underpowered 结果不缩小有效域(codex:"inconclusive")，伪否定风险

## Lead 自主可推进部分（不待裁决）
- martingale-guard(#48)运行中=独立 null-baseline 交叉验证同问题
- 退化品种诊断 + 功效分析 = codex 要求的补强，可 spawn 工位先做（不预判裁决）
