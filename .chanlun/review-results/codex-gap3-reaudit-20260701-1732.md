# Codex 异质复审 — GAP3 返工三致命修复（复审轮）

## 元数据

- **mode**: review（复审）
- **subject**: GAP3 返工 commit 5e92b99380 三致命修复——Δ驱动schedule/phase由stage派生/退本金cash-tight
- **model**: codex-cli
- **timestamp**: 2026-07-01 17:32 UTC
- **被审 commit**: 5e92b99380（分支 main）
- **前一轮**: codex-review-20260701-2055.md（codex #9，判 FAIL，三致命）

---

## Codex 本轮输出（摘要）

codex 本轮判 fail，六点否定：

1. **PhaseIII/Add 意图下 Noop（判"致命"）**：risk_adapter 对 Add 返 positions.max(1)，仓位已=1 则 Δ=0，"增股数"仍是 Noop。
2. **pub transition_adapter + pub OrderOut 外部注入（判"致命"）**：外部可构造不一致 OrderOut 绕过 schedule_adapter 重现幽灵买入。
3. **RecoverCapital is_legal_from 恒 true（判"致命"）**：外部注入 RecoverCapital(w>free) 可制造负 free。
4. **kappa 私有化 in-module 测试仍可直接构造（判"重要"）**。
5. **debug_assert 仍非 assert（判"重要"）**。
6. **u64→i64 as 截断（判"建议"）**。

---

## agent 质询判定

### 质询1：PhaseIII Add Noop（Codex 判"致命"）

**否定不成立——误读有效域标注**。

codex #9 原判"致命2"是"stage 已到 EarningShares，intent 仍是 Buy"——即 phase 恒 PhaseI，intent 读不到正确相位。修复后 phase 由 stage 派生，stage=EarningShares → PhaseIII → intent=Add。intent 类型同步已解决。

Add 在 L0 有限网格结构层 Δ=0 Noop 是**已声明的有效域边界**：risk_adapter 是有限网格确定选择器（网格 {0,1}），真实 sizing（BuyCore，PDF §10 步骤4）在 L2 runner 层 plan_orders 完成。commit message 明确标注此为有效域边界（formalization-validity-domain 规则合规）。Codex 把 L0 网格粒度限制当成实现缺陷。

**判定：误判。撤回"致命"定级。**

### 质询2：pub transition_adapter + pub OrderOut 注入（Codex 判"致命"）

**否定成立，降级为"重要"**。

外部可构造不一致 OrderOut 绕过 schedule_adapter 是真实架构边界问题。但：
- 当前所有执行路径（hybrid_step → policy_output → schedule_adapter → transition_adapter）都经 schedule_adapter；
- run_closed_loop 闭合路径无外部 OrderOut 注入；
- 这不是 codex #9 三致命之一，是新发现的架构脆弱性。

不阻断三致命验收，但需归档为后续修复项。

**判定：降为"重要"，非致命。**

### 质询3：RecoverCapital is_legal_from 恒 true（Codex 判"致命"）

**原致命3（codex #9 负 free 退本金）已真解；Codex 新发现是不同的边界问题，降级为"重要"**。

codex #9 致命3指 stage_progression 自身无 w≤free 约束——现已修复（`w = min(recover_target, s.free)`，`w>0` 才派）。生产路径上 RecoverCapital 只由 stage_progression 派生，free 不会变负。

Codex 本轮的批评是"外部构造 RecoverCapital(w>free) 可制造负 free"——这与质询2同类（外部注入架构脆弱性），与原致命3不同。is_legal_from 对 RecoverCapital 恒 true 是已知设计（raw 层全函数），legal 层边界宽松是后续强化点。

**判定：原致命3已真解。新发现降为"重要"。**

### 质询4：kappa 私有化（Codex 判"重要"）

**否定成立，但属已知诚实标注**。

`kappa` 是模块私有（非 crate 私有），同模块测试可直接构造 `RiskPolicy { kappa: -1 }`。代码注释已明确声明：

> 「不可构造」严格口径 = crate 外部不可构造（try_new 是唯一外部构造闸），非「任何位置不可构造」。同模块反向见证是故意的（证 kappa_nonneg() 谓词对负值返 false）。

即：模块内直接构造负 κ 是为了测试谓词行为，而非漏洞。crate 外部无法构造负 κ。codex #9 原判"重要"（κ 负可构造）已通过 try_new + 私有化解决了外部 API 不变量，剩余的测试内直接构造属已声明豁免。

**判定：否定部分成立，但已有诚实豁免声明，无需进一步修复。**

### 质询5：debug_assert（Codex 判"重要"）

**否定成立，继承自 codex #9 原判"重要"**。

OQ-9 gate 的两处 debug_assert 在 release 下失效。这是 codex #9 已判"重要"的已知问题，不在本次三致命修复范围内。

**判定：否定成立，归档为后续修复项（非三致命修复阻断）。**

### 质询6：u64→i64 as 截断（Codex 判"建议"）

**否定成立，建议级**。positions 是 u64，`target_pos as i64 - x.positions as i64` 对 >i64::MAX 的仓位不安全。现实量级（网格 {0,1,2,3}）不会触达，但用 i128 中间域更严格。

**判定：否定成立，建议级，后续可改 i128。**

---

## 综合判定

**三致命：CHECK_PASS**

| 致命编号（codex #9） | 修复内容 | 复审判定 |
|---|---|---|
| 致命1：幽灵买入 | Δ驱动 schedule_adapter；filled_pos 写回；free.max(0) 现金约束 | **真解** |
| 致命2：phase/TStage 脱钩 | phase_from_stage(tw_next.stage) 写回；stage 推进即驱动 intent 变 | **真解** |
| 致命3：负 free 退本金 | w = min(recover_target, free)；w>0 才派 RecoverCapital | **真解** |

**反面对照成立**：
- closed_loop_earning_shares_not_reached_l0_honest_gap3 断言 stage 恒 CostReduction + holding=1（非幽灵Q=128）+ free≥0。
- 未注资 campaign 恒 CostReduction（III 非硬置）。

**Codex 本轮 fail 判定评估**：基于一个误判（Add Noop=L0有效域边界，非实现缺陷）+ 两个降级（pub 注入风险 + is_legal_from 宽松）+ 继承问题（debug_assert）。三致命本身已真解。

---

## 结果包（遵循 result-package.md 简化版——纯技术性产出）

**结论**：GAP3 返工 commit 5e92b99380 的三致命（幽灵买入/phase脱钩/负free退本金）已真解。Codex 本轮否定不阻断三致命验收。

**边界条件**：
- 三致命真解的条件是"所有执行路径都经 schedule_adapter 产生 OrderOut"。若外部直接构造 OrderOut 并注入 transition_adapter，幽灵买入可从架构层复现（Codex 否定2，降为"重要"）。
- PhaseIII Add 执行仍是 L0 Noop，真实增股数（BuyCore，L2 sizing）待后续 runner 层实装。

**影响声明**：
- 修改文件：transition.rs（schedule_adapter/stage_progression/transition_adapter/phase_from_stage）、ledger.rs（kappa 私有化/try_new/eta_star i128/buy_core_legal i128）、runner.rs（honest_gap3 测试/funded_recovery 测试）。
- 后续待修项（不阻断本次）：pub OrderOut 注入边界 + is_legal_from RecoverCapital 宽松 + debug_assert 升 assert。

---

## stance-declaration

```yaml
---stance-declaration---
verdict: conditional
stances:
  phantom_buy_delta_fix: accept
  phase_synced_via_stage_derivation: accept
  cash_tight_recover_capital: accept
  add_noop_l0_boundary: accept（误判撤销——L0有效域边界非实现缺陷）
  public_orderout_injection_risk: needs_work（降为"重要"，非致命）
  recovercapital_legal_boundary: needs_work（降为"重要"，原致命3已解）
  kappa_module_private_test_literal: accept（已有诚实豁免声明）
  debug_assert_release_gate: needs_work（继承待修，非本次阻断）
concessions:
  - phase_synced_but_add_noop（Codex 误判，非致命——撤回）
---end-stance---
```
