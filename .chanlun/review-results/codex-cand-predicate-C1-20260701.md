---
task: "#129 C1 矛盾 — 区间套候选谓词 Cand^δ_ℓ(x) 定义式裁决"
negation_source: heterogeneous
model: codex-cli
date: 2026-07-01
verdict: 其他（既非 A 破中枢，也非 B 次级别一类）
epistemic_level: L0（纯定义/权威文本判读，不依赖数据）
persisted_raw: .chanlun/review-results/codex-decide-20260701-1614.md
---

# C1 裁决：Cand^δ_ℓ(x) = δ 方向 ℓ 级别的候选背驰段区间存在性谓词

## 结论（codex 裁决 + 本工位质询判定：否定成立）

Codex 裁决 **其他**——`Cand^δ_ℓ(x)` 既不是 **A（破中枢）** 也不是 **B（次级别第一类构成）**。
忠实语义：

> **δ 方向、ℓ 级别上可被区间套递归消费的「背驰段候选区间存在性」谓词**。
> Cand 负责在当前中间级别选出候选背驰段/候选区间 `J^δ_ℓ`；递归的 `J^δ_{ℓ+1} ⊆ J^δ_ℓ`
> 和最低级 `Confirm_e`（三类买卖点析取）再完成逐级收缩与最终买卖点确认。

### 三问逐条

**① canonical/Lean 里 Cand^δ 有无显式独立定义式？**
无。S_Θ §6 与 FULL §六都只在 `N^δ_{ℓ↓e}` 递归里**引用** `Candidate` 符号，无独立展开式。
Lean（NestingCertificate.lean §9 / CandidateSet.lean §6）已诚实标注为**抽象 Bool**
（candBuy/candSell），"PDF 仅作符号，未给独立定义式，[需人工确认]，不臆造"。
→ **矛盾属实**：这是 spec 真缺口，非实现错误。Lean 侧未臆造 A/B，做法正确。

**② §11.3 理论依据据此判 A 还是 B？**
§11.3 原文（逐字）："**低级别背驰是本级别背驰的必要条件（非充分）**"。
—— 注意是"本级别**背驰**"，**不是** Lead 消息里引用的"本级别买卖点"。这排除 B（一买）。
—— 也不是"破中枢"。这排除 A。
§11.1 明确区间套对象是"逐级寻找**背驰点**（即买卖点）"，§11.4 "从大级别背驰段逐级缩到小级别背驰段"。
→ Cand 的对象是**背驰段候选区间**，A/B 都误判对象。

**③ 从区间套数学本质判：为何是"其他"？**
基例 `Confirm_e = B1∨B2∨B3`（三类买卖点析取）已承担**买卖点确认**。
`Candidate` 出现在 `ℓ_j > e_v` 的**中间级别**分支——它的职责是"产生当前级别可继续下钻的 `J^δ_ℓ`"，
不是直接触发买卖点。
- **A/B 共同的类型错误**：把中间级候选谓词混淆成基例买卖点触发（把 Confirm 的角色提前塞进中间层）。
- **B 额外错误**：只取第一类，错误排除了 B2/B3（基例三类都算）。

## 本工位质询（简化否定质询，判定否定成立）

1. **问题真实存在？** 是——两份 canonical + 两个 Lean 文件**独立**标注 Cand^δ 无定义式，非 codex 误读上下文。
2. **A/B 是否被合理排除？** 是——排除依据是 §11.3/§11.1 逐字原文，不是 codex 的训练语料常识。
3. **codex 是否臆造了 Cand 的具体判据（越权）？** 否——codex 只给**结构约束**（候选背驰段区间存在性 +
   Sel_Θ 唯一选择 + J 嵌套 + 无候选返 0），未取定"背驰段的精确识别判据"，且明确保留"spec 后补定义式则推翻"。
   与 Lean 诚实标注一致，**不违反 no-workaround**（未硬编码特例，未模糊化）。

→ **否定成立**。这是**定义层缺口的裁决**（Cand 的结构语义），不是定义冲突上浮——因为 codex 给的语义
   与现有 canonical 递归结构自洽（Cand=中间层候选区间，Confirm=基例买卖点），无需改任何已有定义，
   只是把 A/B 两个**错误候选**都否掉，并给出忠实的结构约束。

## 对 W1 区间套实装的指令（codex 原文）

实现 `Cand` 为不可变证据对象的派生 Bool（**非** break_center，**非** sublevel_B1）：

```text
CandidateEvidence {
  dir: δ,
  level: ℓ,
  interval: J^δ_ℓ,
  basis: BackchiSegment / NestedBackchiSegment,   // 背驰段，非破中枢，非一买
  selector: Sel_Θ version,
  source_ids: backing segment ids
}
Cand^δ_ℓ,t = (selected CandidateEvidence exists)
J^δ_ℓ,t    = selected.interval
```

递归执行约束：
1. 非基例 `ℓ > e`：只从 **δ 方向兼容的背驰段/可下钻候选区间**生成 CandidateEvidence。
2. 多候选必须经固定 `Sel_Θ` 唯一选择。
3. 无候选返回 `0/false`，**不是** undefined（canonical 硬约束）。
4. 递归层检查 `J^δ_{ℓ+1,t} ⊆ J^δ_{ℓ,t}`。
5. 基例 `ℓ = e` 才调用 `Confirm_e^δ = B1∨B2∨B3`（买）/`S1∨S2∨S3`（卖）。
6. **禁止**把 `break_center`（A）或 `sublevel_B1`（B）直接命名为 `Cand`；若要保留，必须作为
   外部启发式字段，不能作为忠实谓词。

**前置依赖**：若 W1 没有可用的背驰段区间 `J` 数据结构（BackchiSegment → interval 证据链），
则不能假装忠实实现，须**先补** `BackchiSegment -> interval` 证据链。

## 边界条件（结论翻转）

- 若后续 authoritative spec/PDF 明确给出 `Cand^δ_ℓ` 独立定义式 → **推翻本裁决**，以新定义为准。
- 若旧测试 oracle 按 A 或 B 写 → 信号集会变，须记录为**语义修正**，不得回退到 A/B。

## 影响声明

- 不改任何已有定义/spec/Lean/rust。
- 裁决 W1（#130）区间套实装的 Cand 判据：用背驰段候选区间存在性，非 A 非 B。
- 解锁 W1 补完整 Cand（前置：需先确认/补背驰段→区间 J 证据链是否已在 rust 侧存在）。
- 认识论等级 L0（权威文本判读 + 结构约束，不依赖数据；实盘有效性属 W-VERIFY 的 L2/L3）。
