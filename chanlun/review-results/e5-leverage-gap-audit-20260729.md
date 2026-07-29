# E5 杠杆完全分类缺口核对报告（#651）

> research 票 #651（map #530「Lean 形式化残余收口」雾团：E5 杠杆完全分类缺失）
> 审计日期：2026-07-29 · 只读核查，未改任何代码/Lean 文件 · 090 纪律：查不到照实写
> 核查 HEAD：`039cf86974`（main，2026-07-29）· 产出分支 `research/e5-leverage-gap`（worktree `/private/tmp/research-651/wt`）

---

## 0. 一句话结论

票体前提**已过时**：:149 所引「E5 缺、仅 `leverage : Type` 占位」是审计文档的**补全前**段落；同一文档顶部即有同日（2026-06-27）「5层真封后更新」节自陈 E5 已封闭，且 formal/ 现状（HEAD `039cf86974`）核证：G_t/N_t/约束代数已实装于 `Origin/LeverageCapital.lean`，E2 M0-M4 已绑定具体阈值于 `Origin/RootSelDisambig.lean:250-255`，四模块 `lake build` 全绿（17 jobs）。**残余缺口只剩四档小项（R1-R4）+ 一个死槽（R5）**，详见 §2。

---

## 1. 新鲜度

### 1.1 审计文档成文时间线（git 真相）

| commit | 时间（-04:00） | 动作 |
|---|---|---|
| `9e14f06220` | 2026-06-27 08:06:55 | 文档**首次入库**（对照矩阵 4 份之一，完全分类 69%） |
| `ce325b1548` | 2026-06-27 10:46:26 | 文档**最后改动**——顶部插入「5层真封后更新」节（:9-:29），把 E5/A5/A6/B4/B5/F2/F6/C4 全部改标「已实装」 |

票体引用的 :149（缺口清单第 1 条）与 :110（E 区 E5 行）、:112（E2 边界注）均属**补全前矩阵**；文档 :11 自陈「下方 §1-§3 是补全前缺口矩阵（69%/20%/11%）」。同日的 :174「E5+A5/A6 补全则 69%→~94%」同样是补全前语境——该条件句在当日 10:46 已兑现为 97%（34/35，文档 :28）。

### 1.2 E5/E2 相关 formal/ 文件时间线（git log --follow 全史）

| 文件 | 首个 commit | 时间 | 06-28 后变动 |
|---|---|---|---|
| `formal/Origin/RootSelDisambig.lean` | `cefb29df69` | 06-27 09:21 | **无** |
| `formal/Origin/LeverageCapital.lean` | `c570d2ebf6` | 06-27 10:21 | **无** |
| `formal/Origin/DynamicCongruence.lean` | `d0fbc74088` | 06-27 10:38 | **无** |
| `formal/Origin/DecisionSufficiency.lean` | `d0fbc74088` → `29d0360851` | 06-27 10:38 → 11:35 | **无** |
| `formal/Origin/SourceAxioms.lean` | （更早） | — | **无** |

2026-06-28 以来 formal/ 有 14 个 commit（含 07-29 #614 并轨 merge `79a2715070`），但**无一触碰上述五文件**（`git log --since=2026-06-28 -- <五文件>` 为空）。#614 merge 对 formal/ 的改动面 = ActiveSet/AncestorClosure/CertGatedExit(新)/ParityFixtureExport(新)/SegmentFeatureSeq/lakefile.toml，与 E5/A5/A6 无交。四模块均在 lakefile.toml:104 roots 现役名单内。

### 1.3 当前 HEAD 构建验证（本 worktree 内实测）

```
cd formal && lake build Origin.LeverageCapital Origin.RootSelDisambig \
  Origin.DecisionSufficiency Origin.DynamicCongruence
→ Build completed successfully (17 jobs)，零 error
```

即：报告引用的全部定理签名在 HEAD `039cf86974` 上**当前可编译、machine-checked 有效**。

### 1.4 两个票体点名问题的照实回答

- **「`leverage` 占位是否仍抽象？」**——`SourceAxioms.lean:77` 的 `leverage : Type` **仍在**，但它是 `FixedTheta` 结构的 Θ 参数槽，且 `FixedTheta` 在全 formal/ **零引用**（grep 全树仅 :68 定义一处命中）——是**死槽**，不是 E5 算式的载体。E5 算式已另立门户于 `LeverageCapital.lean`。
- **「E2 RiskMode 绑定是否仍缺？」**——**不再缺**。`RootSelDisambig.lean:250-255` `riskMode` 把 M0-M4 绑定到具体阈值：M0 `E_t≤0`、M1 `LiqFlag∨E_t<MM_t`、M2 `E_t<MM_t+B1`、M3 `E_t<MM_t+B2`、M4 else（Int tick 算术，自注 bit-exact 对齐 Rust `risk::risk_mode`）；`:261` `riskMode_complete` 证五态穷尽互斥；`:293` `globalRiskClose_iff` 刻画全局平仓 = {Insolvent, Liquidation}。（`FullDefinitionStrategy.lean:108/136` 的 Bool 骨架版仍在，与 doc E2 行锚 `:108/:136` 精确吻合；具体阈值绑定版是 RootSelDisambig 这份。）

---

## 2. 缺口矩阵：canonical §13 逐量 × Lean 现状（HEAD）

canonical 源：`~/Downloads/newchanlun-claude-code-result-package/FULL_USER_FORMULA_SOURCE.md` 十三（L975-1063，本次实读核对）。

| canonical §13 条目 | Lean 现状（文件:行 · 定义/定理族） | 状态 |
|---|---|---|
| `n_{v,t}=σ_v·M_v·P_{v,t}·q_{v,t}` | `LeverageCapital.lean:92` `signedNotional` = sign(side)×notionalMag；**M_v·P_v·q_v 乘积由调用方预算**（:83-84 诚实标注），乘积三因子结构未形式化 | **部分**（输入层抽象，刻意） |
| `G_t=∑_v\|n_v\|` | `:109` `grossNotional` | ✅ 已实装 |
| `N_t=\|∑_v n_v\|` | `:116` `netNotional` | ✅ 已实装 |
| N≤G 三角不等式（「双开抬毛不抬净」） | `:131` `net_le_gross` + `:152` `hedged_gross_high_net_zero`（N=0∧G=2k 见证） | ✅ 已实装 |
| `L^G=G/E`、`L^N=N/E`（比值） | **无有理数比值定义**——刻意整数化为名义上限比较 `G≤G̅=L̄^G·E`（`:196` `grossOk` / `:199` `netOk`，:177-181 注明「避免有理数除法，bit-exact 对齐 rust」）；比值级只有名义形式 `:202` `netLeGross` | **部分**（比值形式缺，设计内取舍） |
| 杠杆约束「必须同时约束二者」 | `:211` `gross_cap_implies_net_cap`（G̅≤N̅ 时毛帽⟹净帽）+ `:223` `net_ok_not_imply_gross_ok`（净约束不能替代毛约束，反例见证） | ✅ 已实装 |
| `0<B^{(1)}<B^{(2)}` | **未形式化**——`RootSelDisambig.lean:242-243` buffer1/buffer2 是裸 Int 输入，无排序前提、无衍生引理 | **缺**（小档） |
| `μ_t∈{Insolvent,…,Normal}` 五态 | `RootSelDisambig.lean:226` `RiskMode` | ✅ 已实装 |
| M0-M4 判据（含 ¬M_0∧…∧¬M_{i-1} 前缀） | `:250-255` `riskMode` if/else 短路链（:248 注明前缀语义）；`MM_t(q_t)` 的**持仓依赖被抽象**为 Int 输入 maintMargin | ✅ 判据已绑定 / MM(q) 依赖输入层抽象 |
| `∑1[M_i]=1` 穷尽互斥 | `:261` `riskMode_complete` | ✅ 已实装 |

### 残余缺口精确分档（「哪一族引理的哪一档」+ 若补齐落在哪）

- **R1（比值档）**：`L^G/L^N` 作为 ℚ 比值无量、`E_t>0 ⟹ L^N≤L^G` 保序定理缺。**注意：`LeverageCapital.lean` 头注 :24 点名 `lev_net_le_gross` 为机器可检验命题之一，但文件内并无此定理**——头注与实现不符（文档虫）。补齐落点：`Origin/LeverageCapital.lean` §4 一族，加 `levG/levN : ℚ` 定义 + `lev_net_le_gross`。
- **R2（乘积档）**：`n_v=σ·M·P·q` 三因子乘积未展开。补齐落点：同文件 §2 `VoicePosition` 一族加 multiplier/price/qty 字段。属刻意的输入层抽象（:83-84 在案），补齐价值低。
- **R3（缓冲序档）**：`0<B1<B2` 前提缺失。补齐落点：`Origin/RootSelDisambig.lean` §4 `RiskModeInput` 加排序前提（或独立引理 `B1<B2 ⟹ (E<MM+B1 ⟹ E<MM+B2)` 档）。
- **R4（MM(q) 依赖档）**：`MM_t(q_t)` 持仓→保证金函数未形式化，maintMargin 是输入值。属券商规则/Θ 层开口，与 E5 同源的「给定参数后的代数」口径一致，不建议在 L0 层补。
- **R5（死槽）**：`SourceAxioms.lean:77` `FixedTheta.leverage : Type` 全树零引用。要么 Θ 实例化工位接管（绑定到 `LeverageAccount`），要么清理；与 E5 算式无功能耦合。

### 附带发现（文档锚漂移，非代码问题）

- 审计文档更新节（:18）给 A5 的行号锚 `:65/84/94/176` 已**漂移**——`DecisionSufficiency.lean` 当日 11:35 被 `29d0360851` 二次改写，当前实际行号：`:83` `decision_feasible_nonempty`、`:102` `decision_intent_complete`、`:112` `same_class_same_intent`、`:165` `decision_pipeline_exists_unique`、`:194` `same_class_same_policy`。**定理名全对**，仅行号漂移。
- E5 行（:22）的 LeverageCapital 锚 `:109/:116/:92/:202/:211` 与当前**精确吻合**；A6 锚（`:107/:178`）精确吻合。

---

## 3. A5/A6 连带缺口现状与归口建议

### 3.1 现状（HEAD，构建绿）

- **A5 决策充分性四支**（canonical §16，`C(x)=C(y)⟹K=∧Intent=∧J=∧π=`）：`DecisionSufficiency.lean` 四支全在——K 支 `:83`（K_Θ≠∅）、Intent 支 `:102/:112`、J 支 `:132` `decision_lexargmin_exists_unique` + `:144`、π 支 `:176` `decision_policy_total_unique` + `:194`；另 `:272` `complete_classifier_decision_sufficiency_iff` 给出充要综合。gatekeeper `:388` 诚实标注非真完全分类。**已实装（L0 结构综合）**。
- **A6 动态同余算子**（canonical §17/§22，`C_Θ∘T_Θ=T̄_Θ∘(C_Θ,ē)`）：`DynamicCongruence.lean:107` `dynamic_congruence_commutes`（算子级交换）+ `:178` `witness_is_dynamically_congruent`（非空洞见证）+ `:124` `congruence_via_foundation_skeleton`。**已实装（L0）**。

### 3.2 归口建议（裁定权在编排者）

**建议：#651 以「缺口已封闭，残余另立」结案；残余不与 #651 同票。**

理由：
1. E5 主体 + A5/A6 已在 2026-06-27 同批封闭（`c570d2ebf6`/`cefb29df69`/`d0fbc74088`/`29d0360851`），并被审计文档自身更新节记录在案——#651 的侦察前提（:149/:174 补全前段落）已不成立，本票作为 research 票的答案就是「雾团已散」。
2. #651 边界自陈「只查不改」，残余 R1-R5 是**实施**工作，天然不同票。
3. 残余量级小且异质：R1/R3 是值得补的 L0 小档（半日级），建议**另立一张实施小票**「E5 残余：ℚ 比值形式 lev_net_le_gross + 0<B1<B2 前提（兼修 LeverageCapital 头注 :24 文档虫）」；R2/R4 是刻意输入层抽象（诚实标注在案），**不建议立票**；R5 死槽留给 Θ 实例化工位或顺手并入上述小票清理。
4. A5/A6 无连带缺口可立票——仅剩的「schema 前提义务」（文档 :28 所称唯一非已实装剩项）属 L2 经验层口径，不是结构缺口。

---

## 4. 证据锚索引

- 审计文档：`docs/canonical-coverage-classification.md` :9-:29（更新节）/ :110-:112（E 区补全前）/ :149（缺口清单 1）/ :174（覆盖率条件句）
- git：`9e14f06220`（文档首入 06-27 08:06）→ `ce325b1548`（更新节 06-27 10:46）；`cefb29df69`/`c570d2ebf6`/`d0fbc74088`/`29d0360851`（E5/E2/A5/A6 封闭四 commit）；06-28 后五文件零变动
- Lean：`formal/Origin/LeverageCapital.lean` :92/:109/:116/:131/:152/:196/:199/:202/:211/:223/:268；`formal/Origin/RootSelDisambig.lean` :226/:239-244/:250-255/:261/:293；`formal/Origin/DecisionSufficiency.lean` :83/:102/:112/:132/:165/:176/:194/:272；`formal/Origin/DynamicCongruence.lean` :107/:124/:178；`formal/Origin/SourceAxioms.lean` :68/:77
- 构建：本 worktree `lake build` 四目标模块 17 jobs GREEN（2026-07-29，HEAD `039cf86974`）
- canonical：`FULL_USER_FORMULA_SOURCE.md` 十三 L975-1063（实读）
