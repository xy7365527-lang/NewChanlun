# s3 严格口径 alpha 重测 — 结果包（goal g-full-mutex-impl acceptance s3-strict-alpha-retest）

**工位**：swarm/ws-s3 | 基线 HEAD=122c918da9（含 s1 桶键扩维 + s2 残差管线）| 不 git
**原文权威**：`docs/formal-chain/alpha分离.pdf`（残差 p1/§4.1、分层 p5 §4.2）+ `alpha检验.pdf`（删尾 p4 §6、方向不对称 p5 §7-§8）
**预注册**：`acc-alpha-estimand-prereg-20260701.md`（冻结 v0，估计量 estimand 冻结于看结果前）
**认识论等级**：残差判定 **L2**（真实 BTC 单标的 walk-forward OOS，可否证）；full-z×残差组合判定 **未产出（结构阻塞，见 §8）**；跨标的 **未做（L2 单标的限定，231）**。

---

## 1. 结论（一句话）

**在严格残差口径（Y_i=δ(H−B̂)−C）下，预注册冻结 estimand `(ℓ, bsp_class, δ, σ^H)` 四元组的全局裁定 = INCONCLUSIVE（V=0 / F=3 / I=25，28 桶 / 3427 残差 / 5 walk-forward OOS 窗）。无任何桶达 VALIDATED。** 主桶 L0/type3/买/σ^H=0 是「统计上欠功效的正点估计」（mean +178.66，LCB +102.49，perm_p 0.005，删尾 +148.70 不翻转，但 n_eff=166 < 功效门 290）——未被否证，也未达 confirmed，精确落在原文预言的 "positive point estimate, statistically inconclusive"。

**任务指令 #1（完整 z:MuClass 桶键）应用于残差判定路径，本工位判定为结构阻塞**——它要求改动 `perm_test.rs` 统计生产机器（perm_p 生产者的桶键类型）+ **新预注册**（预注册 §5：改 estimand 须新冻结，不得复用），且 full-z 细分把已稀疏的高级别桶碎裂到 n=1（perm/LCB 在单-δ 层退化为无鉴别力）。已按 no-workaround 走 STOP+TaskCreate 上浮（§8），不静默改统计机器伪造 full-z 判定。

---

## 2. 定义依据

- **可交易 alpha 定义**：`alpha分离.pdf` p1「α(z,a)=E[Y_i(a)|Z_i=z]，可交易 iff α>0」，Y_i 是 beta 剥离后的残差收益（非原始 X_i）。`alpha检验.pdf` 三态判据（预注册 §3.1）：VALIDATED=`μ̂>0 ∧ perm_p<0.05 ∧ LCB_OOS>0 ∧ powered`；FALSIFIED=`powered ∧ LCB≤0 ∧ UCB≤0`；INCONCLUSIVE=`¬powered ∨ (LCB≤0<UCB)`。功效门 `powered ⟺ n_eff≥(1.645·CV)²`（667）。
- **残差口径**：μ̂ 与置换检验跑在 `ResidualTrade::y()=δ·resid_base−cost` 上（`resid_base=H−B̂`，B̂=因果扩张窗漂移），非原始 X_γ。管线走 s2 已 commit 的 `wverify_run.rs`+`perm_test.rs`（残差分层 δ 置换）。
- **桶键生产路径（675 不 fork）**：残差记录 `ResidualTrade.class` 由 `build_mu_from_bars`（l3_delta_r_alpha.rs:194→254）填 `z_of_candidate(c)`——即残差数据**已携带完整 z:MuClass**（含 `horizontal=Some(role.h)`、未压缩 `i_class`、`position`），走的正是 s1 生产路径，非坐标 fork。当前 harness（wverify_run.rs:84）在消费侧把它**投影到 4 维** `(level, bsp_class(), delta, parent_dir)`——full-z 判定的阻塞在消费侧（perm_test 桶键），不在数据侧。
- **代码锚点**：`wverify_run.rs:71 wverify_full`（残差跑批入口，`#[ignore]`）；`perm_test.rs:37 BucketKey=(u32,u8,i8,i8)` + `:80 stratified_delta_perm_p`（perm_p 生产者，4 维桶键硬编码）；`decontam::{classify_bucket, global_verdict}`（三态判据）。

---

## 3. 残差口径逐桶判定（L2，预注册 4 元组 estimand）

跑批：`cargo test --release --lib theta_v0::backtest::wverify_run::wverify_full -- --ignored`（确定性，14.76s，当前 HEAD=122c918）。全 28 桶表见 `/tmp/wv_full_rows.md`。关键桶：

| 桶 (ℓ,bsp,δ,σ^H) | n | n_eff | mean(Y) | LCB | UCB | CV | perm_p | 删尾(−3) | 翻转 | state |
|---|---|---|---|---|---|---|---|---|---|---|
| **L0,type3,买,σ0**（主桶/卖腿重心对照的买腿） | 1597 | 166.42 | **+178.66** | **+102.49** | +254.82 | 10.357 | **0.005** | +148.70 | 否 | **Inconclusive**（欠功效：166<290） |
| L0,type3,卖,σ0 | 1413 | 361.85 | −40.07 | −99.78 | +19.64 | 34.05 | 0.005 | −55.71 | 否 | Inconclusive |
| L0,type3,卖,σ+1 | 3 | 3.00 | −648.76 | −945.98 | −351.54 | 0.482 | 1.000 | — | 否 | **Falsified** |
| L1,type2,卖,σ−1 | 14 | 14.00 | +733.82 | +157.73 | +1309.9 | 1.786 | 1.000 | +241.61 | 否 | Inconclusive（perm_p=1 单-δ 层退化） |
| L3,type2,卖,σ+1 | 2 | 2.00 | −387.86 | −740.39 | −35.34 | 0.781 | 1.000 | — | 否 | **Falsified** |
| L4,type2,卖,σ0 | 3 | 3.00 | −189.93 | −378.52 | −1.35 | 1.045 | 1.000 | — | 否 | **Falsified** |

**全局裁定 = INCONCLUSIVE**（`decontam::global_verdict`：无 VALIDATED 桶 ∧ 存在 INCONCLUSIVE ⟹ 非 Pass 非 161）。V=0 / F=3 / I=25。

### 3.1 卖腿重心（原文预注册方向，alpha检验.pdf §7 H2）
- **卖腿逐桶无 alpha**：所有 δ=−1（卖）桶均 Inconclusive 或 Falsified，无一 VALIDATED。3 个 Falsified 桶全是卖腿（L0σ+1 / L3σ+1 / L4σ0），但 n=2/3 极小，powered 由 CV<1 得，实质是「弱否证」（判据在 n<10 的有效性边界，#13 §3.3 开放问题）。
- **买腿**：唯一强点估计在 L0 买桶（+178.66），但欠功效 Inconclusive。

### 3.2 稳健性（任务 #4）
- **删尾（drop_top_k=3，§6）**：主桶 L0 买 +178.66→+148.70 **不翻转**（正号非前 3 尾部赢家依赖）；高级别小样本桶多翻转（尾部主导，如实标注）。
- **H2 方向不对称（β=μ_sell−μ_buy，block bootstrap，§7-§8）**，见 `/tmp/wv_full_h2_asymmetry.md`：

| 级别 | n_buy | n_sell | β=μ_sell−μ_buy | boot_p(H0:β≤0) | 读出 |
|---|---|---|---|---|---|
| L0 | 1623 | 1441 | **−224.65** | 0.956 | 买>卖（否证原文 H2 在 L0；与主桶是买腿一致） |
| L1 | 126 | 125 | **+356.35** | **0.010** | 卖>买 **显著**（支持原文 H2，新方向候选） |
| L2 | 43 | 46 | +24.48 | 0.477 | 不显著 |
| L3 | 10 | 7 | +171.74 | 0.000 | 卖>买 但 n tiny，功效不足 |
- **walk-forward 逐窗 LCB**：主桶跨 5 窗（2023-02→2025-06）见 #13 报告（+66.8/+155.9/+98.5/+653.6/+270.1 全正）；残差口径 LCB=+102.49（逐窗聚合）。

---

## 4. 基线差分（任务 #5，X_γ→Y_i 部分完成，full-z 部分阻塞）

**可完成的差分：粗投影 4 元组的 X_γ raw → Y_i 残差**（coarse-projection-baseline-0702 = #13）：

| 口径 | 主桶 mean | LCB | perm_p | 删尾 | 全局 | 判定 |
|---|---|---|---|---|---|---|
| #13 X_γ raw（4 元组） | +262.49 | +183.86 | 0.000 | — | V=2/F=2/I=24 | **PASS** |
| **s3 Y_i 残差（4 元组）** | **+178.66** | **+102.49** | **0.005** | +148.70 不翻转 | **V=0/F=3/I=25** | **INCONCLUSIVE** |

**差分读出**：去 beta 剥离 ~84/笔（+262→+179），主桶点估计仍正、perm_p 仍显著、删尾不翻转（**正号非纯 beta 伪影**）；但残差 CV=10.36 抬高功效门到 n_eff≥290，实测 166 ⟹ 主桶从 #13 的 confirmed 降为 **Inconclusive（欠功效）**。全局从 PASS 降为 INCONCLUSIVE。这与 s2 独立跑批（#82）**逐值一致**——s1 的 MuClass 第 7 维（horizontal）未接入残差 4 元组消费键，故对残差判定零扰动（bit-exact 重现）。

**未完成的差分：full-z（Z 细分）逐桶对照** —— 阻塞（§8）。s1 econ 侧 oracle（X_γ，非残差）已记录 full-z 碎裂样例（z-bucket-impl §4a）：单一 Y 桶 `(L0,δ−1,sell2)` 按 σ_p/H 分裂为 3 个 Z 桶（n=4/19/1）。在残差口径复现此细分需改 perm_test 桶键 + 新预注册。

---

## 5. 认识论等级 / L3 诚实声明（任务 #6，231）

- **L2 单标的限定**：本判定有效域 = **BTC 单标的、残差口径、预注册 4 元组、walk-forward 5 窗（2023-02→2025-06）**。不外推跨品种。
- **L3 跨标的未做（诚实缺口，非虚报）**：数据侧 CL（`cl_1m_databento_10y.json`）/ES/QQQ 可用且在 `PREREG_WINDOWS`（8 品种），但 `wverify_full` 入口**硬编码 symbol="BTC"**（wverify_run.rs:73/77）——跨标的跑批需改 harness（循环品种）或新增入口，属「改代码才能跑」，本工位不 fork（no-workaround）。跨标的 L3 与 full-z 一并列入 §8 上浮。
- 231 否定性结果价值：全局 INCONCLUSIVE（V=0）缩小了有效域边界（严格残差口径下无 confirmed alpha），比 #13 的确认性 PASS 更严格、信息量更高。

---

## 6. 边界条件（结论翻转）

- **主桶 Inconclusive→Validated**：n_eff 增至 ≥290（更长 OOS/降 CV/更同质分层）且 LCB 仍>0 ⟹ 翻 Validated。当前只差功效，非否证。
- **主桶→Falsified**：残差 mean 转负或 UCB≤0——当前 +178.66 / UCB +254.82，远未触。
- **3 个 Falsified 桶→Inconclusive**：若判 n<10 的 powered 不可信（判据有效域边界，#13 §3 待 codex 裁），退 Inconclusive，全局仍 INCONCLUSIVE。
- **全局 INCONCLUSIVE→FALSIFIED**：须**所有**桶 powered-FALSIFIED（含 UCB≤0），25 个 Inconclusive 桶阻止 161（667）。
- **高级别（level≥1）「无 alpha」的条件效力域**（任务硬约束）：L1-L5 桶的 perm_p 多为 1.000，因单-δ 层置换退化（非真无结构），且区间套 frontier 链污染（问题②，#84 未修）**未排除**——高级别桶结论条件化于「污染未排除」，不作独立否证。

---

## 7. 下游推论

- **acceptance s3-strict-alpha-retest = INCONCLUSIVE**：严格残差口径下预注册 estimand 无 confirmed alpha。#13 的 PASS 在残差口径下不成立——M1 里程碑不得以本 estimand 的 alpha 存在性为独立通行凭证。
- **提功效链（667 pending）**：降 CV（收益率尺度归一）、L0 聚合、跨标的 L3 池化——是 INCONCLUSIVE→可判的下一 goal 候选。
- **full-z 细化在残差口径只会降功效**（§8 关键推论）：full-z 把已稀疏的桶进一步碎裂 ⟹ 每桶 n 更小 ⟹ 功效更低 ⟹ 严格 full-z 残差判定结构上被推向 INCONCLUSIVE，**不可能**产生新的 VALIDATED。预注册 4 元组已是残差口径下**最大功效的诚实 estimand**。full-z 的价值在 oracle μ̂ 上界（X_γ，s1），不在残差 alpha 判定。

---

## 8. ★结构阻塞上浮（任务硬约束「必须改代码才能跑 → STOP+TaskCreate」）

**阻塞项**：任务指令 #1（完整 z:MuClass 桶键）∩ 指令 #2（残差口径判定）的组合，无法从既有 harness 产出，须改代码 + 新预注册。三条独立阻塞：

1. **改 perm_test.rs 统计生产机器**：`perm_test::stratified_delta_perm_p` 的输出桶键 `BucketKey=(u32,u8,i8,i8)`（4 维硬编码）+ `bucket_members` 键 + `OutBucket.base` 须改为完整 MuClass。perm_test 是 **perm_p 生产者**（decontam 消费），带独立测试套件 + 跨进程复现冻结种子约束——非「harness 一行」，是统计核心改动。任务硬约束「不改生产代码，只跑既有 harness」在此触发。
2. **须新预注册（no-workaround + formalization-validity-domain）**：预注册 §1.1 冻结 estimand = `(ℓ,δ,bsp_class,σ^H)`；#13 §5 明确「改 estimand（对齐 PDF z / 加 h / 跨标的）须**新预注册**，不得复用本冻结」。full-z 是新 estimand，静默切换 = 看结果后改 estimand = 数据挖掘（预注册冻结正为防此）。
3. **统计上 full-z 残差判定退化**：full-z 把 L1-L5 桶（已 n≤25）碎裂到 n=1，单-δ 层置换恒等 ⟹ perm_p=1（perm_test.rs:439 已注此退化）⟹ 逐桶 full-z 判定无鉴别力；只有 L0 桶（n>1400）可细分，但 s1 oracle 已示 L0 亦碎裂到 n=4/19/1。⟹ full-z 残差判定须走 **UClass 降维**（`project_to_u` 丢 H，折叠 role），而 UClass ≠ 4 元组，是**第三个 estimand**（又须预注册）。

**上浮请求**：这是「选择/语法记录」类决断（是否为 full-z 残差判定新开预注册 + 改 perm_test 桶键），非本工位可自决。TaskCreate 已建（见汇报）。三个候选路径供编排者/codex 裁：
- (A) 认定预注册 4 元组已是残差口径最大功效诚实 estimand，full-z 仅保留在 X_γ oracle（s1），残差判定不升 full-z ——则本工位 §3 判定即终态。
- (B) 新开 full-z 残差预注册 + 改 perm_test 桶键 + UClass 降维并列（winner's curse 防护），承认功效必降。
- (C) 先补跨标的 L3（提功效优先于加维），full-z 延后。

---

## 9. 谱系引用

- **231/formalization-validity-domain**：残差判定 L2 有效域=BTC 单标的/4 元组/5 窗；full-z=新 estimand 须新有效域（新预注册）。否定性结果（V=0）缩小边界。
- **预注册冻结 v0**（acc-alpha-estimand-prereg-20260701）§1.1 estimand / §3 三态 / §5 改 estimand 须新预注册。
- **663/665/666/667**：功效门 n_eff≥(1.645·CV)²（667）；主桶欠功效 Inconclusive 非 Falsified（667「LCB underpowered≠证伪」）；残差分层置换 beta 吸收（665/666）。
- **675（探针走生产路径）**：残差数据 `ResidualTrade.class=z_of_candidate` 已走生产路径（非 fork）；阻塞在消费侧 perm_test 桶键。
- **s1（z-bucket-impl-20260702）§3c/§4a**：full-z 是 oracle 上界不作 selection；L0 Y 桶碎裂样例。
- **s2（strict-pipeline-s2-20260702）**：残差四件套 L2 跑批，本工位 bit-exact 重现其 4 元组判定。
- **#13（wverify-alpha-retest-20260702，coarse-projection-baseline-0702）§5**：估计量有效域=4 元组，改 estimand 须新预注册——本工位 §8 阻塞的直接谱系依据。
- **no-workaround / no-patch-mentality**：不静默改 perm_test 伪造 full-z 判定；走 STOP+TaskCreate。

---

## 10. 影响声明

- **产出**：本结果包 + `/tmp/wv_full_rows.md`（28 桶残差表，重生成）+ `/tmp/wv_full_h2_asymmetry.md`（逐级别 β）+ `/tmp/wv_full_timeblocks.md`（逐窗分层）。
- **代码改动**：**零**（只跑既有 `wverify_full` 入口，未改任何 .rs）。full-z×残差组合的必需改动（perm_test 桶键 + 新预注册）**未实施**，上浮 §8。
- **判定语义**：严格残差口径下预注册 estimand 全局 INCONCLUSIVE（V=0/F=3/I=25），无 confirmed alpha。这是 #13 PASS 在残差口径的降级坐实（与 s2 一致），并新增：full-z 细化在残差口径只降功效不产 VALIDATED（§7 推论）。
- **未改**：任何生产代码（econ_positive.rs / selector.rs / perm_test.rs / mu_estimator.rs 分类逻辑）、预注册冻结文件、settled 定理。
