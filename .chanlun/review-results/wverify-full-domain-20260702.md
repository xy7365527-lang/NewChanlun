# W-VERIFY acc-alpha 全定义域定版回测（BTC OOS 全窗）

- **工位**：swarm/ws-wverify2 | 任务 #3（定版执行，前任 ws-wverify 配方）
- **锚点 SHA**：`d906648041`（P2+P4+P7+补桥+Cand^δ 段1+段2 完整实装）
- **数据**：BTC `btc_1m_full.json`（461万 bar 全历史）→ `slice_date_window("2023-01-01","2025-06-30")` **全 OOS 窗**（无 MAX_BARS 截断；Holdout 2025-07-01+ 隔离）
- **认识论 L2**（BTC 单标的真实数据，路径 A 分层内 δ 置换去污，可否证不交叉验证）
- **跑批耗时** 155.77s（全 OOS ~1.3M bar 逐 bar 因果分类；生成快路使全窗可行，非 O(n²) 阻塞）
- **驱动**：`build_mu_from_bars`（从 `l3_delta_r_alpha::build_walk_forward_mu` 提取，无 N^δ 门——μ 估计覆盖全候选集，选择器是下游）→ `est.trades()` 投影 `(ℓ,bsp_class,δ,X_γ)` → `perm_test::stratified_delta_perm_p`（N_PERM=200 种子 20260701）→ 逐桶 Welford → `decontam::effective_n`（Σρk 自相关校正 n_eff，665 neff/nraw）→ `classify_bucket` 三态 → `global_verdict`

---

## 全局裁决

```
trades=3602  buckets=11  levels=[0,1,2,3,4,5]
global_verdict = Pass       （V=1  F=0  I=10）
```

**Pass**：存在 VALIDATED 桶（占比>0，预注册 §3.2）⟹ BTC OOS 上存在超 beta 可交易 alpha。**全 6 级别解封成功**（基线 level0-only 1188 trades → 定版全域 3602 trades，level1-5 由 by-construction 门归零变为真实检验，见 §有效域）。

## 逐桶三态表（μ̂ 单位=绝对 PnL，tick 名义单位）

| ℓ | bsp_class | δ | n | n_eff | μ̂ | LCB | UCB | CV | perm_p | 三态 |
|---|-----------|---|---|-------|-----|-----|-----|-----|--------|------|
| L0 | 3 | −1 | 1503 | 355.24 | −73.49 | −129.21 | −17.77 | 17.87 | 1.000 | Inconclusive |
| **L0** | **3** | **+1** | **1714** | **158.71** | **+259.64** | **+183.95** | **+335.34** | **7.34** | **0.000** | **Validated** |
| L1 | 2 | −1 | 132 | 116.09 | +75.07 | −96.19 | +246.33 | 15.93 | 0.190 | Inconclusive |
| L1 | 2 | +1 | 132 | 92.78 | −67.47 | −258.59 | +123.65 | 19.78 | 0.810 | Inconclusive |
| L2 | 2 | −1 | 46 | 28.44 | +62.08 | −192.57 | +316.74 | 16.91 | 0.645 | Inconclusive |
| L2 | 2 | +1 | 48 | 48.00 | +190.58 | −111.85 | +493.01 | 6.68 | 0.355 | Inconclusive |
| L3 | 2 | −1 | 9 | 9.00 | −90.39 | −355.83 | +175.05 | 5.36 | 0.525 | Inconclusive |
| L3 | 2 | +1 | 11 | 10.86 | −69.36 | −621.08 | +482.36 | 16.04 | 0.475 | Inconclusive |
| L4 | 2 | −1 | 4 | 4.00 | −149.60 | −308.44 | +9.24 | 1.29 | 0.770 | Inconclusive |
| L4 | 2 | +1 | 2 | 2.00 | +589.19 | −1017.26 | +2195.63 | 2.34 | 0.270 | Inconclusive |
| L5 | 2 | −1 | 1 | 1.00 | −9.93 | NaN | NaN | NaN | 1.000 | Inconclusive |

### VALIDATED 桶（唯一）：level0 第三类买点（δ=+1）

四条件全满足（预注册 §3.1）：`μ̂=259.64>0` ∧ `perm_p=0.000<0.05`（层内 δ 置换去污后方向信息超随机）∧ `LCB=183.95>0` ∧ `powered`（`n_eff=158.71 ≥ (1.645·CV)²=(1.645·7.337)²≈145.7`）。这是全定义域内**唯一**携超 beta 可交易 alpha 的桶。

## 231号有效域标注（强制）

1. **有效域 = 全定义域（全 6 级别 L0–L5 均有样本）**，区别基线的 level0-only 有效域。基线（`wverify-level0-baseline-20260702.md`）的 level1-4 由 N^δ 门 by-construction 归零（codex H1，#7）；本定版用 `build_mu_from_bars`（无 N^δ 门）+ 锚 d906648041（Cand^δ 段1+段2）⟹ level1-5 真实进入检验，not 门后归零。

2. **中间级样本分布（段2 小转大门口径——如实报告）**：μ 估计**不施加** N^δ/段2 小转大门（该门是下游选择器 χ，非 μ 估计环节）。故各级 n 是**原始候选确认密度**，非门后选择集：L0=3217（1503+1714）、L1=264、L2=94、L3=20、L4=6、L5=1。高级别 n 随级别指数衰减——这是**区间套嵌套的结构必然**（高级别买卖点稀疏），非段2 门致稀。段2 小转大门若下沉到 μ 估计将进一步削减中间级样本，本定版未施加以保全定义域覆盖。

3. **高级别（L2–L5）全 Inconclusive = underpowered，不判纯 beta（161/667）**：L2 n_eff≤48、L3≤11、L4≤4、L5=1，远不过功效门（CV≈5–20 ⟹ 门槛 n≈145–1083）。`LCB≤0 ⊬ μ≤0`——高级别**无检出力**，不得声称"高级别缠论买卖点无 alpha"，须走预注册 §3.3 提功效链（收益率尺度/L0 聚合/L3 跨标的/多窗）。

4. **level0 三类卖点（δ=−1）：UCB=−17.77<0 但 underpowered**（n_eff=355.24 < 门槛 (1.645·17.87)²≈864）⟹ Inconclusive 非 Falsified。raw-n 口径曾误判 Falsified（F=1）；Σρk 自相关校正 n_eff 后降级 Inconclusive（F=0）——underpowered 的 UCB<0 不构成有功效否证（667）。**全域 F=0：无任一 powered-FALSIFIED 桶**。

## 边界条件（结论翻转）

- 若 codex 复审判定层内 δ 置换残留 beta（665 粗吸收）⟹ VALIDATED 桶降 Inconclusive，Pass 翻 Inconclusive。
- 若 L0/bc3/δ+1 的 perm_p 在 BTreeMap 确定序下不可复现（跨进程漂移）⟹ 去污无效（已由 perm_test BTreeMap + 固定种子锁定，multi_stratum_reproducible 单测守卫）。
- 若换收益率/夏普尺度使某高级别桶 powered 且 LCB>0 ⟹ 该桶 Inconclusive→Validated（增 alpha 证据）。
- 若 Holdout 一次性确认 L0/bc3/δ+1 的 LCB≤0 ⟹ OOS VALIDATED 是过拟合，Pass 不外推 Holdout。

## 下游推论

- acceptance `acc-alpha-beta-decontaminated` = **Pass**（占比>0）⟹ 存在可交易 alpha，进 M1 里程碑验证；**但 alpha 集中于单桶（level0 三类买点）**，非全级别系统性——下游 χ 选择器应聚焦此桶，勿声称全域 alpha。
- 高级别 underpowered ⟹ 触发 §3.3 提功效链（路径 B 跨标的 L3 是提 n 主路，667 pending①⑤）。
- 段2 小转大门作为下游选择器的有效性待独立检验（本 μ 估计未施加）。

## 多重检验 family 注记（codex #4 缺口②，ws-neff 补）

本轮同测 **11 桶**（family size=11，非基线 r1 的 5 桶）。逐桶 `classify_bucket` 施加的是**未校正逐桶** perm_α=0.05；以下为报告层 FWER 注记，**未内置于判据**：

- **perm_p 精度**：VALIDATED 桶 perm_p 表 `0.000` = **0/200 次置换超观测，非真 0**。加一平滑 `(0+1)/(200+1)=1/201≈0.004975`——这是 R=200（预注册 §4 冻结）下的**分辨率下限**，真 p 可能远小于此但 R=200 无法进一步分辨（改 R 违反 §4 冻结）。
- **Bonferroni FWER**：11 桶 α/11 = `0.004545`。VALIDATED 桶加一平滑 p = `0.004975 > 0.004545` ⟹ **严格 11 桶 Bonferroni + 加一平滑下临界失守**（差 0.00043，落 R=200 分辨率噪声内）。对比 r1 的 5 桶 α/5=0.01 则通过——桶数 5→11 使 Bonferroni 更严。
- **口径分歧（选择类，待 codex/编排者裁）**：若 FWER 校正**入判据** ⟹ VALIDATED 桶降 Inconclusive ⟹ 全局 Pass→Inconclusive（分辨率受限的诚实收口）；若保持**逐桶 α=0.05**（现 `classify_bucket` 口径，Bonferroni 过保守，可选 BH-FDR）⟹ 维持 Pass。两口径并列如实（161），不擅自定。扩展分析见 `wverify-alpha-r2-20260702.md`（ws-neff task#15 r2）。

## 谱系引用

663（判据=μ>0 非显著性）/665（除偏差⊬alpha + 单标的 beta 退化 + neff/nraw）/666（δ 置换）/667（LCB underpowered，inconclusive≠证伪，`n_eff>(1.645·CV)²`）/231（有效域≠定义域，否定膨胀禁止）/161（underpowered 不判纯 beta）。锚定预注册 `acc-alpha-estimand-prereg-20260701.md`（估 mand 冻结）。task #15（n_eff 自相关校正 = 本定版 n_eff 口径来源）。

## 影响声明

- **改动**：`l3_delta_r_alpha.rs`——提取 `pub fn build_mu_from_bars(bars:&[Bar],&cfg)->MuEstimator`（`build_walk_forward_mu` 降为其 Dataset 薄包装，8 调用点零改动，bit-exact）；`wverify_run.rs`——改接 `build_mu_from_bars` + 真实 API `load_by_symbol`/`slice_date_window` 全 OOS 窗 + `decontam::effective_n` 校正 n_eff。
- **影响模块**：backtest::l3_delta_r_alpha、backtest::wverify_run；不改引擎/分类器/decontam/perm_test 逻辑。
- **测试**：`cargo test --release --lib theta_v0::backtest::perm_test` 5/5 绿；`wverify_full` #[ignore] 跑批通过（trades=3602 非空断言）。
- **未 commit**（Lead 唯一 commit 者）。
</content>
