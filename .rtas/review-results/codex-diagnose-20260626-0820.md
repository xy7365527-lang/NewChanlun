# Codex Diagnose — OQ-9 扩维消解方案审查

**Session:** 019f0303-b9f0-7e21-b249-6738cbc56264  
**Model:** gpt-5.5, reasoning xhigh  
**Tokens used:** 92,208  
**Date:** 2026-06-26 08:20  
**Mode:** diagnose（严格诊断）

---

## Prompt 摘要

审查对象：OQ-9 守恒律相变可逆性矛盾的「扩维消解」方案。

矛盾两端：
- 端A（算术真相）：EarningShares 下闭合旧 ShareConserving 亏损腿 → cost_basis 数值回正
- 端B（INV-2）：LedgerPhase 单向相变，EarningShares 不回退

被审查方案：stage 与 cost_basis 解耦（独立维度），将旧腿亏损闭合回正路径判为 OperationViolation，Lean 证合法转移下 stage 单向是定理。

三个诊断问题：
1. 是 workaround 还是真消解？
2. 两端是否真的同时成立？
3. 非对称触发条件是否引入新不一致？

---

## Codex 完整裁决输出

> 先给总裁决：**扩维方向是对的，但当前表述不是严格真消解；close-time gate 版本仍有 workaround 成分，必须改成 phase-entry 边界约束。**

### 问题1 裁决：当前方案偏 workaround，修正后可成为真消解

缠论语义下，`EarningShares` 不是"成本数值碰巧 ≤ 0"的普通状态，而是"本金已退、之后只用卖出资金回补、目标为筹码增加"的操作范式。第31课和第87课的语义都指向：进入该阶段后，合法操作不应重新消耗本金来修复旧亏损腿。

但这里的漏洞是：**如果还有旧 `ShareConserving` 腿未闭合，它就是未结算负债或风险敞口**。这时仅凭另一个腿的利润把 `cost_basis <= 0`，就宣布进入 `EarningShares`，其实过早。随后再把旧腿亏损闭合定义为 `OperationViolation`，就像是在入口处放过了风险，出口处再宣布它非法。

裁决：
- "旧 `ShareConserving` 亏损腿在 `EarningShares` 下继续按旧律闭合"不是缠论合法操作。
- 但**只在 close 时判违规**不是最忠实的建模。
- 忠实建模应当是：进入 `EarningShares` 前，必须已清算、转换、冻结或储备所有旧 `ShareConserving` 风险敞口。

非法性应从"闭合动作违规"上移为"带未结算旧腿进入 `EarningShares` 的资格不足"。

### 问题2 裁决：不是两端同时成立，端A 被 legal gate 否定

端A 的强表述是：在 `EarningShares` 下，旧 `ShareConserving` 亏损闭合可以把 `cost_basis` 数值回正。

如果 Lean 的合法转移关系把这条路径直接返回 `OperationViolation`，那么在合法系统内部，回正状态根本不可达。因此：

- 在 `rawStep` / Python 算术层，端A 成立。
- 在 `legalStep` / Lean 合法账本层，端A 不成立。
- 说"可以回正，但不允许发生"不是"两端都保留"，而是把端A放到了非法轨迹或反事实轨迹里。

严格表达应改成双层关系：

```lean
rawClose oldShareLeg loss  => cost_basis > 0
legalClose oldShareLeg loss => OperationViolation
```

这样可以说"算术事实被保留为 violation witness"，但不能说"合法账本同时保留端A和端B"。

### 问题3 裁决：非对称本身自洽，但当前触发条件太弱

"入口依赖 `cost_basis <= 0`，进入后 stage 不随 cost_basis 回退"本身不是矛盾。这是典型的 hysteresis / latch：阈值触发相变，相变后由历史相位保持，不再由瞬时数值反投影。

真正的问题是：入口判据不能只是裸 `cost_basis <= 0`。它必须是更强的"本金已退且无未结算旧律负债"的证书：

```text
effective_cost_basis <= 0
and no_open_share_conserving_legs
```

或：

```text
effective_cost_basis = raw_basis + reserved_legacy_liability <= 0
```

否则就会出现新的不一致：`cost_basis` 在入口处被当作本体判据，进入后又被宣布为无关变量。这不是相位解耦的问题，而是入口证书不完整的问题。

### 精确修正方向

1. 保留 `stage` 独立维度，不再把 `stage = f(cost_basis)` 当作恒等式。
2. 引入 `rawStep` 与 `legalStep` 分层：raw 层证明旧腿亏损闭合会回正；legal 层把该 trace 判为违规。
3. 把 `enterEarning` 的前提加强为：无未闭合旧 `ShareConserving` 腿，或所有旧腿风险已被 reserve / marked / converted。
4. `OperationViolation` 不应是"为了 stage 不回退而禁止回正"，而应是"违反进入 `EarningShares` 所需证书后暴露出的非法 trace"。

**最终裁决：扩维方案需要修正。** 方向正确，当前论证过强；严格版本不是"端A和端B在同一合法系统内并存"，而是"raw 算术端A作为非法轨迹见证被保留，legal 系统用更强入口证书保证端B"。
