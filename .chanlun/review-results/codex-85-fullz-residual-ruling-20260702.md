# codex #85 全权裁定：full-z×残差 alpha 判定的估计量/预注册决断

**工位**：codex-challenger（代码层异质审查代理）| task #85 | 2026-07-02
**裁定路径**：`.venv/bin/python -m newchan.codex decide` → codex CLI，完整交互见 `.chanlun/review-results/codex-decide-20260702-230123-cedf.md`（CLI 自动持久化，本文件为该裁定的落地摘要 + 工位质询判定）

---

## 1. 结论

**codex 全权裁定：选 (C)**——先补跨标的 L3 池化提升功效，full-z 残差判定延后。桶键继续用预注册冻结的 4 元组 `(level, bsp_class, delta, σ^H)`，不升级 `perm_test.rs` 消费完整 z。

**关键数学核验结果**（本次 #85 的核心质询目标）：s3 提出的「full-z 细化在残差口径只会降功效，不可能产生新 VALIDATED」这一论断，**codex 判定不是数学严格命题**，只在"细分子桶内部 CV 不显著低于父桶（同质性假设）"下成立。功效门 `n_eff≥(1.645·CV)²` 同时依赖 n 和 CV，partition refinement 必然单调降低 n，但 CV 可能因剥离异质子群而骤降——若 CV 降幅快于 √(n_j/n) 降幅，细化理论上可能把欠功效桶变为功效达标。这与"细分类优势定理"（oracle value V(Z)≥V(Y)）在有限样本下的统计直觉一致。

**尽管如此**，codex 裁定实践选择仍是 (C) 而非 (B)：这不是因为 full-z 无价值，而是因为——(1) 当前已确认的根因是**功效不足**（主桶 n_eff=166<290），跨标的池化直接对症（增 n）；(2) full-z 需要新预注册（多重比较/winner's curse 风险），且 s1 oracle 已示的碎裂样例（n=4/19/1）经验上强烈暗示会退化；(3) 工程上应先做"最小、诚实、对症"的下一步，而非在理论可能性未经验证前就承担 full-z 的复杂度代价。

## 2. 工位质询判定（简化质询，155号本体论职责）

对 codex 本轮裁定执行三项质询：

- **裁定是否真实存在数学依据？** 是。功效门公式 `(1.645·CV)²` 与 n 的关系是精确的代数关系，codex 对"只降功效"论断的反驳（CV 可能降更快）在数学上无漏洞，站得住。
- **该否定是否已被其他机制覆盖？** 否——s3 §7 的"关键推论"原文正是被质询的对象，codex 判定该推论应订正为"assumption-dependent"而非当作既定事实写入下游推论。**s3 结果包 §7 措辞需要订正**（见下游推论）。
- **候选选择的严重性判定是否合理？** 合理。(C) 相对 (A)/(B) 的优势论证扎实：(A) 把"实践低收益"包装成"数学终态"是声明膨胀（codex 明确指出"过强"）；(B) 概念合法但当前证据不支持优先做。

**质询结论：codex 裁定成立，无翻转依据。**

## 3. 定义依据

- 功效门：`powered ⟺ n_eff≥(1.645·CV)²`（667号谱系）。
- partition refinement 的 n 单调性：细分桶是父桶的不相交子集，`n_j ≤ n` 恒成立（集合论事实）。
- partition refinement 的 CV 非单调性：CV_j 相对父桶 CV 的变化取决于子群异质性剥离效果，无先验单调方向——这是本次核验的核心发现，**s3 原文缺失此区分**。
- 预注册冻结规则：`acc-alpha-estimand-prereg-20260701.md` §5，改变 estimand（加维或换标的范围）须新冻结，不得复用 4 元组冻结。

## 4. 边界条件（裁定翻转条件，codex 原文）

- 若能提供 full-z 子桶的**预注册外探索**数据，显示至少一个残差子桶同时满足：n_eff 达标 + CV 大幅低于父桶 + 置换检验不退化（n 未碎裂到无法检验）+ 跨 walk-forward 稳定——则裁定翻向 (B)。
- 若跨标的池化引入不可接受的 symbol 异质性（beta/成本模型无法跨品种一致定义），(C) 也应暂停，退回 (A) 或另寻路径。

## 5. 下游推论

- **M1 里程碑**：alpha 存在性判定在残差口径下维持 INCONCLUSIVE，跨标的 L3 池化是下一个可推进的、根因对症的 goal 候选。
- **s3 结果包措辞订正（行动项）**：`.chanlun/review-results/strict-alpha-retest-20260702.md` §7 "full-z 细化在残差口径只会降功效"一句需订正为"assumption-dependent——仅在子桶 CV 不显著低于父桶时成立；当前证据（碎裂样例 n=4/19/1、s1 oracle）不足以证伪该假设但实践上优先级让位于跨标的池化"。避免下游误读为已证明的数学定理（231号形式化有效域规则：有效域<定义域，此处"降功效"论断的有效域被 codex 限定为同质性假设下，不应无条件泛化）。

## 6. 落地顺序

**裁 (C) 的具体要求**：

- **跨标的 L3 是新 estimand，须新冻结预注册**（不得复用 BTC 4 元组冻结）——codex 明确：`symbol universe`（哪些品种池化）、窗口规格、成本模型、beta 剥离方法、置换种子、三态判据须重新冻结，但**桶键维数不变，仍是 4 元组** `(level, bsp_class, delta, σ^H)`（只是 estimand 从"BTC 4元组"扩为"多品种池化 4元组"，不加维）。
- **技术前提**：`wverify_run.rs:73/77` 当前硬编码 `symbol="BTC"`，需改造为循环 `PREREG_WINDOWS`（8 品种）或新增跨品种入口。CL/ES/QQQ 数据已在池中可用（`cl_1m_databento_10y.json` 等），非阻塞性数据缺口。
- **full-z 残差判定延后**：不关闭 (B) 路径，只是排期在跨标的 L3 出功效证据之后。

## 7. 影响声明

- **涉及文件**：`rust/src/theta_v0/backtest/wverify_run.rs`（symbol 硬编码需改造，L3 池化跑批入口）、`rust/src/theta_v0/backtest/perm_test.rs`（本次裁定不改，`BucketKey` 维持 4 维硬编码）、新预注册文件（跨标的 L3，待新工位创建，不得复用 `acc-alpha-estimand-prereg-20260701.md`）。
- **不改**：`perm_test.rs` 桶键结构、`acc-alpha-estimand-prereg-20260701.md` 冻结内容、`econ_positive.rs`（task #83 已完成的 z 桶键接入，与本裁定路径独立无冲突）。
- **待办**：s3 结果包 §7 措辞订正（订正"只降功效"表述为条件性）；新开跨标的 L3 工位（新 task，遵循本裁定 §6 落地顺序）。

---

## 完整 codex 交互原文

见 `.chanlun/review-results/codex-decide-20260702-230123-cedf.md`（prompt + response 完整持久化，包含立场声明 YAML）。

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
