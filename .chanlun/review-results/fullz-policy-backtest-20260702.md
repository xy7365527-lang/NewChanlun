# full-z×残差 + 完整策略级回测 — 结果包（prereg-fullz-policy 阶段2）

- **工位**：swarm/ws-p3 | task #96 | 冻结 commit=9a19bb5775（prereg-fullz-policy-20260702.md，看结果前锁定）| 实装 HEAD=be4f31edd5+
- **原文权威**：`docs/formal-chain/alpha分离.pdf`（残差 p1 §1 / 分层 p5 §4.2）
- **认识论**：(A) full-z 残差逐桶 = **L2**（真实 BTC 全历史 walk-forward OOS，可否证）；(B) 策略级 π 回测 = **L2**（真实 BTC+CL，单折有界 train→OOS）。判据/去污逻辑本身 L0。

---

## 1. 结论（一句话）

**(A) full-z 细化残差判定 = INCONCLUSIVE（V=0/F=9/I=43，52 桶）；UClass 降维并列 = INCONCLUSIVE（V=0/F=2/I=13，15 桶）——两口径均无 VALIDATED，坐实 codex #85 预测与预注册诚实预承诺（full-z 不产新可交易 alpha）。** 新发现：full-z 的 `i_class` 维与被检验的 δ **共线**（buy3=i_class4 与 sell3=i_class32 分属不同桶），故按 full i_class 分桶使 δ-置换检验结构性退化（主 root buy 子桶 perm_p 从 s3 4 元组的 0.005 退化到 1.000）——这是比「小样本碎裂」更根本的退化机制。

**(B) 完整策略级 π 回测：μ̂ χ 门大幅减损/减回撤/减过度交易，但未转正。** BTC 6 月 OOS：χ门 Σpnl −1.13M（dd 6.9%）vs 无χ基线 −15.68M（dd 95.2%），ΔΣpnl **+14.5M**、Δdd **−88pp**、订单 33067→838（~40× 收窄）；CL 同向（−1.8K vs −54.5K，dd 2.2% vs 68.5%）。**两口径均净负**——μ̂ 门减损但不转正，与 (A) 无 VALIDATED alpha 一致：μ̂ 选择器阻止灾难性过度交易，但底层信号级 alpha INCONCLUSIVE ⟹ 门后组合仍不盈利。

---

## 2. 定义依据

- **可交易 alpha**：`alpha分离.pdf` p1「α(z,a)=E[Y_i|Z=z]>0」，Y_i 残差（beta 剥离后）。三态判据 `decontam`（VALIDATED/FALSIFIED/INCONCLUSIVE）+ 功效门 `n_eff≥(1.645·CV)²` 逐桶 cv（667/decontam.rs:115）。
- **残差口径**：`Y_i=δ·(H−B̂)−C`（不动摇），走 s2 残差管线（`ResidualTrade::y`）。
- **full-z 桶键**：完整 `MuClass` 7 维，走 `z_of_candidate` 生产路径（`horizontal=Some`，675 非 fork）。perm_test 扩维 = 新增 `stratified_delta_perm_p_fullz`/`_uclass`（通用引擎 `stratified_delta_perm_p_by`，4 元组现函数 bit-exact 不变，δ-派生 short_swing 从 perm-δ 重构，A3.2）。
- **UClass 并列**：`UClass::project_to_u`（`(level_bucket,δ,role,divergence)`，divergence 用 bool 非方向性 i_class，A3.4 桶碎裂防护）。
- **代码锚点**：`perm_test.rs::stratified_delta_perm_p_by`（通用置换引擎）+ `_fullz`/`_uclass`；`wverify_run.rs::wverify_fullz`（BTC full-z+UClass 逐桶报告，`#[ignore]`）+ `verdict_by`（泛化键三态）；`policy_backtest`（B 入口，`#[ignore]`）。

---

## 3. (A) full-z 残差逐桶判定（L2，BTC 单标的 walk-forward OOS，残差 3468 笔）

跑批：`cargo test --release --lib theta_v0::backtest::wverify_run::wverify_fullz -- --ignored`（确定性 14.69s，HEAD=be4f31+）。全表见 `/tmp/wv_fullz_rows.md`（52 桶）/ `/tmp/wv_uclass_rows.md`（15 桶）。

### 3.1 全局裁决

| 口径 | 桶数 | V | F | I | 全局 |
|------|------|---|---|---|------|
| **full-z**（完整 MuClass 7 维） | 52 | 0 | 9 | 43 | **INCONCLUSIVE** |
| **UClass**（降维并列） | 15 | 0 | 2 | 13 | **INCONCLUSIVE** |
| s3 4 元组基线（对照） | 28 | 0 | 3 | 25 | INCONCLUSIVE |

### 3.2 主桶命运（s3 主桶 L0/type3/买/σ0 的 full-z 分裂 + UClass 聚合）

| 口径 | 桶键 | n | n_eff | mean(Y) | LCB | CV | perm_p | state |
|------|------|---|-------|---------|-----|----|--------|-------|
| s3 4 元组 | (L0,type3,买,σ0) | 1597 | 166 | +178.66 | +102.49 | 10.36 | **0.005** | Inconclusive（欠功效） |
| **full-z 主子桶** | L0,δ+1,i4,pd0,Root,First | 1612 | 167.69 | +184.07 | +108.25 | 10.05 | **1.000** | Inconclusive |
| **UClass 主桶** | lb0,δ+1,Root,¬div | 1727 | 171.92 | +157.16 | +85.33 | 11.55 | **0.050** | Inconclusive（欠功效） |

**读出**：主桶正点估计（mean>0, LCB>0）在三口径都**存活**，但都**未达 VALIDATED**——4 元组/UClass 卡在**欠功效**（n_eff≈167/172 < 功效门 290/361），full-z 主子桶额外卡在 **perm_p 退化到 1.000**。

### 3.3 ★新发现：full-z 的 i_class 维与 δ 共线 ⟹ δ-置换结构性退化

full-z 按完整 `i_class` 分桶：buy3=`i_class 4`、sell3=`i_class 32` **是不同桶**。故一个 full-z 桶（如 buy3-only）内**只有单一 δ 方向**——δ-置换检验（H0：给定分层，δ 是否携带残差解释力）在此**结构性退化**（被检验的 δ 与桶键 i_class 共线）。主 root buy 子桶 perm_p 因此从 s3 4 元组的 0.005（buy3 vs sell3 有方向对比）退化到 **1.000**（buy3-only 无方向对比）。

这比 codex #85 / 预注册预期的「小样本碎裂 n=1 退化」**更根本**：即使 full-z 主子桶 n=1612（大样本、非碎裂），perm_p 仍退化——根因是 **i_class ⊥ δ 共线**，不是样本量。UClass 用 `divergence` bool（非方向性）替代 i_class，故 UClass 主桶保住 perm_p=0.050（有方向对比）——这正是 A3.4 UClass 并列的价值：暴露 full-z 的共线退化。

### 3.4 边界条件 c（full-z 信息增量）未触发

预注册 A5(c)：若某 full-z 子桶同时 n_eff≥功效门 + CV 大幅低于父桶 + perm_p 未退化 + walk-forward 稳定 ⟹ full-z 产信息增量。**52 桶无一满足**——正点估计子桶 perm_p 退化，perm_p<0.05 的子桶（如 n=9 的 σ−1 短差桶 perm_p=0.010）mean 为负（−309）或欠功效。故维持「full-z 不产新 VALIDATED」，full-z 仅作 oracle 上界（s1）。

### 3.5 level≥2 frontier 污染标注

full-z/UClass 表中 level≥2 桶（如 UClass lb2 = level≥4）挂 frontier 污染标注（`frontier-bt-consumed`，#87 在审），不作独立否证/确认（231）。ℓ∈{0,1} 不受影响。全表 frontier 列已标。

---

## 4. (B) 完整策略级 π 回测（L2，BTC+CL，单折有界 train→OOS）

跑批：`cargo test --release --lib theta_v0::backtest::wverify_run::policy_backtest -- --ignored`（51.57s）。配置：χ门 `run_theta_v0_pi_chi`(θ=0=LCB>0 门, z_α=1.645, est=train段 `build_mu_from_bars` 无泄漏) vs 无χ基线 `run_theta_v0_pi`；**单折有界 train=2022-07…12（6 月）→ test=2023-01…06（6 月 OOS）**（ponytail 限 est-build 成本；18 月×2.5 年全窗 est-build 单折 >45min，缩为 6 月单折，逐 wf 窗 walk-forward 是升级路径）；成本 `ThetaConfig::default()` 统一；nav=首可交易价×1000。全表 `/tmp/policy_backtest.md`。

| 标的 | 口径 | Σpnl(含浮盈) | max_dd | n_orders | n_trades | strat_return |
|------|------|-------------|--------|----------|----------|--------------|
| **BTC** | χ门 μ̂ | −1,134,787 | **0.0688** | 838 | 771 | −0.0686 |
| BTC | 无χ基线 | −15,677,578 | 0.9524 | 33,067 | 32,443 | −0.9476 |
| BTC | **差分** | **+14,542,791** | **−0.8837** | −32,229 | — | +0.879 |
| **CL** | χ门 μ̂ | −1,770 | **0.0221** | 282 | 274 | −0.0220 |
| CL | 无χ基线 | −54,493 | 0.6852 | 20,483 | 19,318 | −0.6775 |
| CL | **差分** | **+52,723** | **−0.6631** | −20,201 | — | +0.656 |

**读出（μ̂ 选择器增量价值）**：χ 门（LCB(μ)>0 准入）在 BTC/CL 都**大幅减损、减回撤、减过度交易**——BTC 亏损缩 93%（−15.68M→−1.13M）、max_dd 从 95%→7%、订单 40× 收窄（33067→838）；CL 亏损缩 97%、dd 69%→2%。μ̂ 门滤掉了无χ基线的灾难性过度交易（基线近乎爆仓，strat_return −95%）。**但两口径均净负**——χ 门未使策略转正（BTC −6.9% / CL −2.2% over 6mo OOS）。这与 (A) 逐信号 V=0（无 VALIDATED alpha 桶）**一致自洽**：μ̂ 选择器阻止亏损放大，但底层信号级 alpha INCONCLUSIVE ⟹ 门后组合级仍不盈利。**基线是极低基准**（无风控过度交易），χ 门的增量价值证的是「LCB>0 过滤防灾难」，非「产正 alpha」。

### 4.1 ★z 桶 P&L 归因的诚实局限（净账本模型）

预注册 B2 冻结「按 z 桶归因 P&L」用最小 plumbing。实装时坐实：**净账本 π 模型下单笔净持仓变动聚合多声部，单 trade→单 z 归因本就不干净**（`runner.rs:1013-1015` 作者自注「多声部 depth 推断是近似」；`track_position_transition` 按净 units 跨 0 配对，非按候选 z）。按 no-workaround，**不做假单-z 归因**（伪造「主导 z」= 声明膨胀）。B 报 well-defined 的 Σpnl/max_dd/n_orders χ-vs-基线差分（直接答「μ̂ 选择器增量价值」）；逐 z 的信号级图景由 (A) 承载。这是比冻结时预设更诚实的实装边界（231）。

---

## 5. 认识论等级 / 否定性结果价值（231）

- **A = L2**：有效域 = BTC 单标的、残差口径、full-z/UClass 桶键、walk-forward OOS。否定性结果（V=0）缩小有效域边界：**残差口径下 full-z 细化不产可交易 alpha**，且暴露 i_class⊥δ 共线退化——比 s3 4 元组信息量更高（多一层否证：细化路径也无 alpha，且给出退化机制）。
- **B = L2**：单折有界 train（ponytail 上限；逐窗 walk-forward 是升级路径）。

---

## 6. 边界条件（结论翻转）

- **A 主桶 Inconclusive→Validated**：需 n_eff≥功效门 **且** perm_p<0.05 **且** LCB>0 同时成立。当前 4 元组/UClass 差功效、full-z 差 perm_p（共线退化）——三口径各差一项，均未触。
- **A full-z→有增量**：若换非共线的连续背驰强度维 β（P1-1，divergence 连续量而非方向性 i_class）细分，可能避开 i_class⊥δ 退化——留待 β 强度分箱（prereg D3/P1）。
- **B χ门增量**：若 ΔΣpnl>0 且 Δmax_dd≤0 ⟹ μ̂ 门有组合级增量价值；若 ΔΣpnl≤0 ⟹ 无（照实入册）。

---

## 7. 下游推论

- **A**：残差口径 full-z 不产 alpha ⟹ 维持 codex #85「4 元组是残差口径当前诚实 estimand」；full-z 的 i_class⊥δ 共线退化提示——若要细化增功效，应走**非方向性细分维**（背驰强度 β 连续量，P1-1），不是完整 i_class。跨标的池化（prereg-l3-cross-symbol）仍是提功效的独立路径。
- **B**：μ̂ χ 门有**显著减损/减回撤增量价值**（防灾难性过度交易），但**不产正 alpha**（门后仍净负）——与 A 自洽。M1 里程碑：完整策略 π 已实装并被回测消费（χ 门 μ̂ 注入生效），但当前 estimand 下无正收益边缘；提正收益需 A 的信号级 alpha 从 INCONCLUSIVE 翻 VALIDATED（提功效链 / 非共线细分维 β），非策略层结构问题。χ 门本身工作正确（40× 收窄交易集，兑现 `Γ_t^trade={γ:LCB>0}`）。

---

## 8. 谱系引用

- **codex #85**（`codex-85-fullz-residual-ruling`）：三阻塞全落地（perm_test 扩维/UClass 并列/功效门逐桶 CV）；§7「full-z 同质降功效」预测坐实，并新增 i_class⊥δ 共线退化机制（比样本碎裂更根本）。
- **prereg-fullz-policy-20260702**（冻结 9a19bb5775）：A/B estimand + 判据 + 输出规格，本结果包逐条兑现；A3.4 UClass 并列的价值（暴露共线退化）经验证实。
- **231/formalization-validity-domain**：A/B 均 L2，否定性结果缩小边界；B 的 z 归因诚实局限（净模型不干净）= 有效域<定义域的实装落点。
- **675**：残差走 z_of_candidate 生产路径（horizontal=Some）；**663/665/667**：判据/去污/功效门。
- **s3**（`strict-alpha-retest`）：4 元组 INCONCLUSIVE 基线（A 差分对象）；**full-strategy-pi-conformance**：π 已实装被回测消费（B 入口依据）。
- **no-workaround / no-patch-mentality**：B 不做假单-z 归因，如实报净模型局限。

---

## 9. 影响声明

- **代码改动**：`perm_test.rs`（重构通用引擎 `stratified_delta_perm_p_by` + 新增 `_fullz`/`_uclass`，4 元组现函数 bit-exact 保留 + L1 自检 `fullz_uclass_reproducible_and_refines`）；`mu_estimator.rs`（UClass/VoiceRole 加 `PartialOrd,Ord` derive，确定序报告）；`wverify_run.rs`（新增 `verdict_by` 泛化三态 + `wverify_fullz`/`policy_backtest` 两 `#[ignore]` 入口）。**未改**：残差减法（`l3_delta_r_alpha`/`ResidualTrade::y`）、`z_of_candidate`、`decontam` 三态判据、分层键、`run_theta_v0_pi_inner` 主链、`TradeRecord`（净模型归因不干净，不加假字段）、既有冻结文件。
- **测试**：`cargo test --release --lib` 基线 +1（全绿，1504）；perm_test 13/13 含跨进程复现 + 新 full-z/UClass 自检。
- **产出**：本结果包 + `/tmp/wv_fullz_rows.md`（52 桶）+ `/tmp/wv_uclass_rows.md`（15 桶）+ `/tmp/policy_backtest.md`（B）。
