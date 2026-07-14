# Codex decide — 2026-07-02 23:01:23 UTC

## 元数据

- **mode**: decide
- **subject**: full-z×残差 alpha 判定的估计量/预注册决断——4元组终态(A) vs 新full-z预注册(B) vs 先补跨标的L3(C)
- **model**: codex-cli
- **timestamp**: 2026-07-02 23:01:23 UTC
- **context-file**: /tmp/codex-85-fullz-residual-ctx.md

## Prompt

## 技术选型请求

full-z×残差 alpha 判定的估计量/预注册决断——4元组终态(A) vs 新full-z预注册(B) vs 先补跨标的L3(C)

## 上下文

# 全权裁定请求：full-z×残差 alpha 判定的估计量/预注册决断（task #85）

## 背景

蜂群正在对 BTC 单标的 alpha 是否存在做严格统计判定（三态判据：VALIDATED / FALSIFIED / INCONCLUSIVE，预注册冻结防数据挖掘）。当前处于"残差口径"（Y_i=δ(H−B̂)−C，beta 剥离后的收益，非原始价格差 X_γ）阶段。

工位 s3 在预注册冻结的 4 元组 estimand `(ℓ, bsp_class, δ, σ^H)` 上跑出全局判定：**INCONCLUSIVE**（V=0/F=3/I=25，28 桶，3427 残差样本，5 个 walk-forward OOS 窗）。

关键桶（唯一强点估计）：

| 桶 | n | n_eff | mean(Y) | LCB | UCB | CV | perm_p | 删尾(-3) 翻转? | 判定 |
|---|---|---|---|---|---|---|---|---|---|
| L0,type3,买,σ0（主桶） | 1597 | 166.42 | +178.66 | +102.49 | +254.82 | 10.357 | 0.005 | 否 | Inconclusive（欠功效：166<290 功效门） |
| L0,type3,卖,σ0 | 1413 | 361.85 | −40.07 | −99.78 | +19.64 | 34.05 | 0.005 | 否 | Inconclusive |
| L0,type3,卖,σ+1 | 3 | 3.00 | −648.76 | −945.98 | −351.54 | 0.482 | 1.000 | — | Falsified |
| L1,type2,卖,σ−1 | 14 | 14.00 | +733.82 | +157.73 | +1309.9 | 1.786 | 1.000 | +241.61 否 | Inconclusive（单-δ 层置换退化 perm_p=1） |
| L3,type2,卖,σ+1 | 2 | 2.00 | −387.86 | −740.39 | −35.34 | 0.781 | 1.000 | — | Falsified |
| L4,type2,卖,σ0 | 3 | 3.00 | −189.93 | −378.52 | −1.35 | 1.045 | 1.000 | — | Falsified |

功效门：`powered ⟺ n_eff≥(1.645·CV)²`。主桶 CV=10.357 ⟹ 功效门=(1.645×10.357)²≈290。

## 待裁定的具体问题

蜂群的分类层（`coverage.rs`/`selector.rs`）已经把每个候选事件的完整状态 `MuClass`（z，含 level/delta/i_class/parent_dir/short_swing/position/horizontal 共 7 维）算出并携带到 `SignalDecomp.z`（task #83 已实装，全链路 1402 测试绿）。但残差统计判定路径 `perm_test.rs::stratified_delta_perm_p`（perm_p 生产者）的桶键 `BucketKey=(u32,u8,i8,i8)` 硬编码只投影 4 维 `(level, bsp_class, delta, parent_dir=σ^H)`——**不消费** z 中已有的 `i_class`（未压缩 6-bit，bsp_class 是其压缩投影）、`short_swing`、`position`、`horizontal` 这些额外维度。

工位 s3 提出一个关键数学/统计推论，请你核验其严格性：

> **「full-z 细化在残差口径只会降功效，不可能产生新的 VALIDATED；预注册 4 元组已是残差口径下最大功效的诚实 estimand。」**

论据：
1. 主桶已经欠功效（n_eff=166 < 290 门槛，只差不到 2 倍）。
2. full-z 若按 s1 oracle（X_γ 口径，非残差）已实测的碎裂样例：单一 4 元组桶 `(L0,δ−1,sell2)` 按 σ_p/H 细分裂为 3 个 Z 桶，n=4/19/1——远低于任何功效门。
3. L1-L5 桶已经 n≤25，若再按 i_class/short_swing/position/horizontal 细分，会碎裂到 n=1，单-δ 层置换恒等 ⟹ perm_p=1（代码 `perm_test.rs:439` 已注此退化）——逐桶判定无鉴别力。

## 请你核验的数学核心（不要只复述 s3 的论据，要独立判断严格性）

partition refinement 会同时改变两个东西：
(a) 每桶样本量 n（细化必然单调降低，这是集合论事实，无可辩驳）；
(b) 每桶内部方差/CV（细化**可能**降低，如果原桶混合了异质子群——例如主桶 CV=10.357 极高，暗示强异质性，若某个 Z 子桶恰好剥离掉了高方差/负 alpha 的子群，其内部 CV 可能远低于 10.357，从而功效门 `(1.645·CV)²` 大幅下降）。

功效门是否达标取决于 n_eff 和 CV 的**相对**变化率，不是只看 n 单调下降就能断言"只降功效"。用数学语言：细化后子桶 j 的功效门 `(1.645·CV_j)²` 相对样本量 `n_j` 的比值，如果 `CV_j` 的下降速度快于 `√(n_j/n)` 的下降速度，细化反而可能把某个原本欠功效的桶变为功效达标（即使全局其他子桶功效更差）。这不是不可能——这正是"细分类优势定理"（oracle value V(Z)≥V(Y)）在有限样本下的统计对应物：如果异质性真实存在，分桶应该能在某个子桶上锐化信号。

**请你明确回答**：s3"full-z 只会降功效"这一论断，在数学上是否严格成立？还是只是在"假设各 Z 子桶内部 CV 与父桶相近或更高"这一（未经验证的）同质性假设下才成立？如果后者，这个假设在当前数据下（主桶 CV=10.357，极端异质）是否可信，还是恰恰相反——高 CV 暗示细分反而最可能提升功效（这与 s1 oracle 已示的碎裂样例 n=4/19/1 是否矛盾——即使某桶因异质性剥离后 CV 骤降，n=1 桶下也无法做置换检验，属于另一个独立的"n 太小完全无法检验"的硬约束，不属于"细化降低功效"这个连续性论证的范畴）。

## 三条候选路径（供裁定，或提出更优的 D）

- **(A)** 认定预注册 4 元组已是残差口径下最大功效的诚实 estimand，full-z 只保留在 X_γ oracle 侧（s1，非 selection），残差判定不升 full-z——s3 §3 判定即终态。
- **(B)** 新开 full-z 残差预注册 + 改 `perm_test.rs` 桶键至完整 MuClass + 并列 UClass 降维路径（防 winner's curse），承认功效大概率下降。
- **(C)** 先补跨标的 L3（`wverify_run.rs:73/77` 当前硬编码 `symbol="BTC"`，但 CL/ES/QQQ 数据已在 `PREREG_WINDOWS` 8 品种池化可用），跨标的池化提升样本量/功效，full-z 残差判定延后至功效证据出来再裁。

工位 ws-s3 倾向 (C)，理由：INCONCLUSIVE 根因是功效不足，跨标的池化比加维更对症（池化增加 n 而不像加维那样减少 n）。

## 已有的相关裁定（供你参考，不要重复裁定这些已结算的点）

此前 codex（本人上一轮，`codex-review-20260702-220553-0379.md`）已就"H 轴是否进 canonical z"给出裁定：`h_axis_in_canonical_z: accept`（z 应含完整 MuClass 7 维，含 horizontal）、`h_axis_in_default_selection: conditional`（selection/训练选类层 `train_winner_class` 保持粗 Y 键，不无条件升 full z，防 winner's curse）。此裁定已落地（task #83，`econ_positive.rs` 分类层桶键已扩至 `d.z`）。

**本次 #85 裁定的范围不同**：不是"z 该不该含 H"（已裁），而是"**统计判定（perm_test.rs 的 perm_p 生产 + 预注册 estimand）该不该升级到消费完整 z**"——这是下游的、独立的统计功效问题，不是分类完备性问题。

## 预注册规则（约束候选 B/C 的实施细节）

`acc-alpha-estimand-prereg-20260701.md` §5：改变 estimand（无论是对齐 PDF 更完整的 z、加维、或换标的池化范围）都须**新冻结预注册**，不得复用已冻结的 4 元组 estimand（否则=看到结果后改 estimand=数据挖掘/多重比较问题）。

## 请按以下结构裁定（六要素）

1. **结论**：s3"full-z 只降功效"论断的数学严格性判定（严格成立 / 仅同质性假设下成立 / 不成立，需具体论证）；A/B/C 三选一的明确裁定，或提出候选 D。
2. **定义依据**：引用功效门公式、partition refinement 的 n/CV 关系、预注册 §5 冻结规则。
3. **边界条件**：在什么条件下你的裁定会翻转（例如：如果 s1 oracle 碎裂样例的 CV 数据被提供，是否会改变判断）。
4. **下游推论**：裁定对 M1 里程碑（alpha 存在性判定）、后续 goal 排期的影响。
5. **落地顺序**：如果裁 (C)，明确 L3 预注册要求（跨标的 estimand 是否沿用 4 元组冻结 vs 新冻结、种子/窗口规格）；如果裁 (B)，明确新预注册的字段范围（完整 7 维 z 还是 UClass 降维子集）。
6. **影响声明**：本裁定涉及哪些文件/模块（`perm_test.rs`、`wverify_run.rs`、预注册文件、`acc-alpha-estimand-prereg-20260701.md`）。

## 立场声明输出协议（必须遵守）

在回复末尾附加以下 YAML 格式的结构化立场声明：

```yaml
---stance-declaration---
verdict: pass | fail | conditional
stances:
  fullz_power_only_claim: <strict_true / assumption_dependent / false>
  candidate_decision: <A / B / C / D>
  cross_symbol_l3_prereg: <same_4tuple / new_prereg / not_applicable>
concessions: []
---end-stance---
```


## 要求

请做出明确的技术选择。

输出格式：
- **决策**：[你的选择]
- **推理链**：[为什么选这个]
- **替代方案**：[被拒绝的方案及理由]
- **边界条件**：[何时应推翻此决策]
- **风险**：[可能的问题]


## Response

**决策**：选 **C**。先做跨标的 L3 池化，但必须新冻结预注册；桶键继续用 4 元组 `(level, bsp_class, delta, σ^H)`，不把残差判定路径升级为 full-z。

**推理链**：s3 的“full-z 只会降功效”不是数学严格命题，只能在“子桶 CV 不显著下降”的同质性假设下成立。partition refinement 确实让每个子桶 `n_j <= n`，但功效门是：

`powered <=> n_eff >= (1.645 * CV)^2`

细分同时改变 `n_eff` 和 `CV`。如果 full-z 子桶显著降低 CV，理论上可能从父桶欠功效变成子桶有功效。所以“只降功效”作为普遍定理不成立。

但代码层技术选择不是证明 full-z 永远没价值，而是选下一步最小、诚实、对症的统计接口。当前主因是功效不足：主桶 `n_eff=166 < 290`，而 L1-L5 多数已经接近不可检验。full-z 会扩大 estimand 空间，增加多重比较和 winner’s curse 风险；即使某些子桶 CV 降低，也需要新预注册，且当前已有碎裂证据说明检验很可能退化。跨标的池化直接增加样本量，和问题根因对齐。

**替代方案**：A 不选。A 把“4 元组最大功效”说成终态，过强；它把实践上的低收益判断误写成数学定理。

B 不选。full-z 残差预注册在概念上合法，但现在样本功效不支持优先做。若要做，必须是新预注册，并且 full 7 维 `MuClass` 与一个预先定义的 UClass 降维路径并列，不能看结果后挑。

**边界条件**：如果提供 full-z 子桶的预注册外探索表明：至少一个残差子桶有足够 `n_eff`、CV 大幅下降、置换检验不退化、且跨 walk-forward 稳定，那么裁定可翻向 B。若跨标的池化引入不可接受的 symbol 异质性，或 beta/成本模型不能跨品种一致定义，则 C 也应暂停。

**风险**：C 的主要风险是跨标的池化把“BTC alpha”变成“多品种 pooled alpha”，所以不能复用旧 BTC 4 元组预注册。必须新冻结：symbol universe、窗口、成本、beta 剥离、置换种子、三态判据、桶键仍为 4 元组。涉及模块：`wverify_run.rs`、`perm_test.rs`、预注册文件、`acc-alpha-estimand-prereg-20260701.md`。

```yaml
---stance-declaration---
verdict: conditional
stances:
  fullz_power_only_claim: assumption_dependent
  candidate_decision: C
  cross_symbol_l3_prereg: new_prereg
concessions: []
---end-stance---
```
