# 检验管线严格化四件套 — 结果包（task #2 / s2，续 #82）

**工位**：swarm/ws-residual | 文件域 mu_estimator/perm_test/wverify_run/l3_delta_r_alpha | 不 git
**对照审计**：`.chanlun/review-results/dlpdf-b-bsp-alpha-20260702.md`（§3 缺口清单 1/2/4/5）
**原文权威**：`docs/formal-chain/alpha分离.pdf`（残差/分层）+ `alpha检验.pdf`（删尾/方向不对称）
**认识论**：管线正确性 L1（单测，零信息增量）；BTC 全 OOS 残差跑批 **L2**（真实数据，可否证）。

---

## 1. 结论

四件套全部实装并接线，护航测试全绿，并在真实 BTC 全 OOS 定义域产出 **L2 残差口径**裁定。

**核心 L2 发现（可否证结果）**：把 retest 的 raw-X_γ 口径换成原文要求的**残差 Y_i=δ(H−B̂)−C + 完整分层
(ℓ,h桶,time block,σ^H)** 后，全局裁定从 retest 的 PASS **降为 Inconclusive（V=0 / F=3 / I=25，28 桶 / 3427 残差 / 5 walk-forward 窗）**。retest 原「唯一 PASS」主桶 **L0/type3/买/σ^H=0** 的命运：

| 口径 | mean | LCB | perm_p | 删尾(−3) | 功效 | state |
|---|---|---|---|---|---|---|
| retest raw X_γ（旧） | +262.49 | +183.86 | 0.000 | — | powered | PASS/Validated |
| **本次 残差 Y_i（新）** | **+178.66** | **+102.49** | **0.005** | **+148.70（未翻转）** | **n_eff=166 < 290=（1.645·CV）²** | **Inconclusive（欠功效）** |

去 beta 把主桶均值从 +262→+179（beta 贡献 ~84/笔），但 LCB 仍 >0、perm_p 仍显著、删尾不翻转——**点估计方向稳健**；然而残差 CV=10.36 抬高功效门槛到 n_eff≥290，实测 n_eff=166 ⟹ **欠功效 ⟹ Inconclusive**。即：去 beta 后主桶**未被否证**，但也**未达 confirmed**——精确落在原文预言的 "positive point estimate, statistically inconclusive"（审计 §1.4）。**无任何桶在残差口径达 Validated。**

---

## 2. 定义依据（每件套溯原文页）

| 件 | 原文规定 | 实装位置 | 状态 |
|---|---|---|---|
| ①残差减法 | `Y_i=δ(H_i−B̂_i)−C_i`（alpha分离 **p1**「定义 beta 剥离后的残差收益」+ **p4 §4.1**「应构造残差或置换检验」，B̂=**p4** `bℓ`「该级别持有窗的共同市场漂移」，只用过去信息） | `l3_delta_r_alpha.rs:239-259`：H=P_out−P_in，C=fee·(P_in+P_out)，B̂=ĝ·hold，ĝ=(P_entry−P_first)/(entry−first)**因果扩张窗漂移**；`mu_estimator.rs::ResidualTrade::y()`=δ·resid_base−cost | ✅ |
| ②分层键补全 | 分层键（alpha分离 **p5 §4.2** 逐字 `s(i)=(ℓi, hi bucket, time block, σhigher,i)`「只在同一置换分层内打乱方向标签 δi」+ **p6**「σhigher 是下一步最重要变量」） | `perm_test.rs:92-99` 分层键=(ℓ,h_bucket,time_block,σ^H)，**解耦**输出桶(ℓ,bsp,δ,σ^H)；h 桶=`mu_estimator::h_bucket(exit−entry)`，time block=`entry_bar/43200+win.i·10000` | ✅ |
| ③删尾稳健 | 删前 3 最大赢家重估符号（alpha检验 **p4**「剔除前 3 个最大赢家后转负，说明收益依赖少数尾部事件」） | `perm_test.rs::drop_top_k_mean` + `wverify_run.rs` 逐桶 TRIM_K=3，报告(删尾mean,翻转) | ✅ |
| ④方向不对称 | `β=μ_sell−μ_buy>0`（alpha检验 **p5 §7**「问题2：方向不对称」`Hasym:μ0,sell−μ0,buy>0`、`Xi=α+βDi+εi` + **p6** `β=μ0,sell−μ0,buy` + **p8**「block bootstrap / cluster SE」） | `perm_test.rs::direction_asymmetry_beta_pvalue`（H0:β≤0 单边）+ `wverify_run.rs` 逐级别 | ✅ |

**残差置换的关键正确性**：`ResidualTrade` 存 **δ-free 基** `resid_base=H−B̂` + `cost`，置换 δ 时**重算** Y=δ·resid_base−cost（非重排 δ-baked X_γ；alpha分离 **p4 §4.1**「构造残差或置换检验」）。单测 `residual_permutation_finds_no_structure_when_r_constant` 证：resid 恒定时 δ 置换无鉴别力 ⟹ perm_p≈1（残差检验正确判「无 δ-选择结构」，区别于 X_γ 置换的虚假显著）。

---

## 3. 四件套逐项 L2 状态（真实 BTC）

- **①残差减法**：主桶均值 +262→+179，beta 被剥离 ~84/笔；LCB 仍 >0。分层置换在残差上做（perm_p 逐桶）。
- **②分层补全**：分层维从 (ℓ,bsp,σ^H) 补到 (ℓ,**h桶**,**time block**,σ^H)——细分层抬高功效门（主桶 n_eff 从 raw 口径的更高值降到 166），如实反映而非缺陷。
- **③删尾稳健**：主桶 L0/3/买/σ0 删前 3 赢家仍 +148.70（**未翻转**，非尾部依赖）；小样本桶多翻转（L0/3/买/σ+1: +502→−877 翻转 n=18；L2/L3 多个 tiny-n 翻转）——高 CV 桶的正号确为尾部主导。
- **④方向不对称**（原文认为「更有希望」的 H2）：
  - L0：β=μ_sell−μ_buy=**−224.65**, boot_p=0.956 ⟹ **买 > 卖**（否证 PDF 在 L0 的 H2；retest 主桶恰是买，与此一致）。
  - L1：β=**+356.35**, boot_p=**0.010** ⟹ **卖 > 买 显著**（**新发现**：L1 方向不对称支持原文 H2，此前 retest 未做故未见）。
  - L3：β=+171.74, boot_p=0.000（但 n_buy=10/n_sell=7 tiny，L3 功效不足）。

---

## 4. 边界条件（结论翻转条件）

- **主桶 Inconclusive→Validated**：若 n_eff 增至 ≥290（更长 OOS/更多样本降 CV 或更同质分层降方差），主桶残差 LCB>0 已满足，只差功效 ⟹ 会翻为 Validated。故当前 Inconclusive 是**功效受限**非**否证**。
- **主桶→Falsified**：若残差 mean 转负或 UCB≤0——当前 mean=+179 UCB=+255，远未触。
- **B̂ 口径**：ĝ 用扩张窗均漂移（causal）。若改滚动窗/因子模型（alpha分离 §4.1 允许）且 BTC 窗内 regime 切换强，B̂ 估计变化 ⟹ 残差 mean 可能再移；当前扩张窗是保守单因子近似。
- **L1 H2 显著性**：n=126/125，block=20 bootstrap；若块长/样本变动，boot_p 可能移出 0.05（当前 0.010 有余量）。

---

## 5. 下游推论

- **审计 §1.4 主桶降级坐实**：retest 的「唯一 PASS」在原文严格口径（残差+完整分层）下**不达 confirmed**，落到 Inconclusive（欠功效），而非 Validated。acceptance a4 效力域须按审计 §5 收窄——「L2 单标的、残差口径、欠功效正点估计」，非「confirmed structural alpha」。
- **beta 未完全解释主桶**：去 beta 后主桶仍 +179/LCB+102/删尾不翻转 ⟹ 主桶正号**不是纯 beta 伪影**，但样本功效不足以 confirmed。审计的「可能是 BTC 做多 beta 细分投影」被**部分**反驳（beta 只吃掉 ~1/3 均值），部分保留（剩余正号 statistically inconclusive）。
- **新方向假设 L1 卖腿**：H2 在 L1 显著（卖>买, p=0.01）——原文认为方向不对称「更有希望」，L1 是候选。下游可对 L1/type2/卖 桶做 L3 跨标的复验。
- **审计缺口 3（跨标的 L3）仍未做**：本工位 L2 单标的 BTC；confirmed 需 8 品种层级模型（审计 §3.3，硬前置）。

---

## 6. 谱系引用

- **231（形式化有效域）**：残差跑批 L2，主桶结论「Inconclusive」有效域=BTC 单标的/5 窗/扩张窗 B̂；不外推。否定性结果（V=0）缩小有效域边界，比确认更有价值。
- **663/665/666/667**：功效门 n_eff≥(1.645·CV)² 沿用；本次残差 CV 抬升使主桶落 underpowered——667「LCB underpowered≠证伪」直接适用（主桶 Inconclusive 非 Falsified）。
- **#51（σ^H 桶键 + G-A2 h 缺口）**：h 缺口本次**闭合**——h 桶接入 `ResidualTrade` 并入分层键，审计 §1.2 的「beta 从分层漏进 perm_p」路径被堵。
- **审计 dlpdf-b（§3 缺口 1/2/4/5）**：四件套正是其列的前置检验步骤，本次落地缺口 1（残差）/2（分层）/4（删尾）/5（不对称）；缺口 3（L3 跨标的）留独立工位。

---

## 7. 影响声明

- **产出**：四件套代码（残差减法 l3、分层置换+删尾+不对称 perm_test、残差报告 wverify_run、ResidualTrade+h_bucket mu_estimator）+ 本结果包 + `/tmp/wv_full_rows.md`（28 桶残差表）/`/tmp/wv_full_h2_asymmetry.md`（逐级别 β）/`/tmp/wv_full_timeblocks.md`。
- **改动的声明**：`wverify_run.rs` 从 raw-X_γ 口径改为**残差 Y_i 口径**——mean/lcb/ucb/perm_p 全在 Y 上算。这使 wverify 裁定语义从「原始收益是否显著」变为「去 beta 残差是否显著」，与 alpha分离.pdf **p1**「定义 beta 剥离后的残差收益」/ **p4 §4.1** 对齐。
- **护航验证**：`cargo test --lib --release theta_v0::backtest` = **178 passed / 0 failed / 50 ignored**；perm_test 12/12（含 D1 跨进程 `perm_xproc_reproducible` 绿）；mu_estimator 27/27；`wverify_full`（ignored，真实 BTC 461万 bar）EXIT=0，15.15s。
- **不改**：selector μ 表口径（仍 X_γ，选择器预测真实可交易 PnL 含 beta——残差化只用于 alpha 显著性检验，非选择器信号，见 l3 build_mu_from_bars 双产出注释）；任何 estimand/prereg 冻结文件。
- **并发观察**：本工位执行期间同文件域被另一实例并发编辑（#82 续跑重复 spawn），四件套单测修订由该实例落地并将 task #2 标 completed；本工位补齐其缺失的 L2 残差跑批 + 结果包（新文件，无 clobber）。

---

## 附：认识论等级标注

- 单测（perm/mu_estimator/backtest 全绿）：**L1**（管线正确性，零信息增量）。
- BTC 全 OOS 残差跑批（V=0/F=3/I=25，主桶降级）：**L2**（真实数据，否证性结果）。
- 「主桶去 beta 后 Inconclusive 非 Validated」：**L2 逻辑推论**（残差 mean/LCB/功效门实测，非预判）。
- 跨标的鲁棒性（是否 8 品种同降级）：**待 L3**（本工位未做）。
