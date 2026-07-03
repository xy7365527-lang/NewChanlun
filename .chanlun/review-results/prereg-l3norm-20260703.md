# 跨品种残差归一化 L3 v2 预注册冻结 v0（主桶功效收口，编排者令：候选后续全做）

- **工位**：swarm/ws-l3norm | task #102 | 前身冻结 `prereg-l3-cross-symbol-20260702.md`（v1，commit 4cceee3fd6）
- **冻结时刻**：2026-07-03（**看任何归一化池化结果之前**——防数据挖掘，protocol §0/§2.3；v1 §5「改 estimand 须新冻结」的履行）
- **触发依据**：v1 结果包 `l3-cross-symbol-alpha-20260702.md`——原始-$ 拼接主桶正 mean(+39.67) = BTC 大美元尺度伪影（per-symbol 仅 BTC 正，6/7 负，ES/GC/QQQ powered-Falsified）；CV 10.36→22.58 反超功效门。v1 §5 明令「跨品种归一化到通约单位 = 新 estimand 须新预注册」，编排者启动本冻结。
- **认识论**：本文件 **L0/L1**（纯规格冻结，零 L2 信息增量，`formalization-validity-domain`）。正式归一化池化跑批产出为 **L3**（跨标的真去污 + 尺度通约），逐桶如实标注三态。
- **不复用**：v1 冻结（原始-$ 拼接 estimand）——本冻结**换 judged 量**：从 `Y_i`（原始-$ 残差）改为 `Ỹ_i = Y_i / σ̂_symbol`（σ̂-归一化残差）。桶键/池化域/beta 剥离/置换隔离/三态判据**全部逐字沿用 v1**，唯一新增 = 归一化变换。

---

## 0. 归一化口径二选一——冻结决断（★硬门，写死不动摇）

编排者令给两候选：**(A) per-symbol 残差除以品种波动率尺度 σ̂_symbol（因果窗估计，无前视）** vs **(B) 对数收益率化**。

**冻结选 (A) σ̂_symbol 除法。** 选择理由（写死）：

1. **硬约束「残差 Y_i 口径不动摇」直接淘汰 (B)**。对数收益率化必须把 `build_mu_from_bars` 内的 `H_i = P_out − P_in`（l3_delta_r_alpha.rs:241）重写为 `log P_out − log P_in`，连带 `B̂_i`（扩张窗 $ 漂移）与 `C_i = fee·(P_in+P_out)`（名义额费，不自然 log 化）全部改口径 ⟹ **破 `ResidualTrade::y()` bit-exact ⟹ 破护航 1402 基线 + BTC 单标的 wverify_full 结果漂移**（no-patch-mentality：不在受保护的残差逻辑上动刀）。(B) 与硬约束架构不相容。
2. **(A) 是纯下游变换，Y_i 逐字不动**。`Ỹ_i = Y_i / σ̂_symbol`，σ̂ 是**池化阶段**对每品种残差记录的常数缩放。`build_mu_from_bars` / `perm_test` / `decontam` / `ResidualTrade` 结构**一律不改**（bit-exact），只在 `wverify_cross_symbol` 逐品种循环里对该品种 records 施加 σ̂ 缩放（实现见 §5）。这正是 v1 §5 预期的「新 estimand 层叠在 Y_i 之上」。
3. **(A) 直击尺度伪影**。BTC 的大-$ 残差（价 $30–100k，逐笔涨跌数百 $）除以 BTC 的大 σ̂，ES 点/DX 小残差除以其小 σ̂ ⟹ 全品种同量纲（无量纲 σ 单位）⟹ v1 的「BTC 支配池化均值」被消除。这是 codex #85 边界 (d)「跨品种尺度不可通约」的对症解。
4. **(A) 降 CV 的机制**：池化 std 在原始-$ 下被品种间尺度异质性抬高（v1 CV 22.58 主因）。σ̂-归一化把品种间尺度成分从 std 中除去，同时按品种重加权 mean ⟹ 池化 CV 有望降到 `n_eff ≥ (1.645·CV)²` 内 ⟹ 主桶功效收口。**注意（诚实）**：归一化也可能**揭示池化 mean 转负/近零**（6/7 品种 σ-单位下为负），坐实「主桶正 alpha 是 BTC-idiosyncratic 非缠论普适 edge」——这是**有价值的否证**（231），本冻结无条件如实报告，不 post-hoc 回滚到原始-$（预注册铁律）。

---

## 1. σ̂_symbol 定义（★冻结——因果、无前视、单品种标量）

**σ̂_symbol = 该品种 OOS 期之前（date < OOS_START=2023-01-01）全体可交易 bar 的逐 bar close-to-close $ 涨跌的样本标准差。**

```
σ̂_symbol = std({ P_{j+1} − P_j : j, j+1 ∈ pre-OOS tradable bars })     # $ 量纲，与 Y_i 同量纲 ⟹ Ỹ 无量纲
  P_j = bars[j].close · tick_size                                       # 与残差管线同价口径（l3_delta_r_alpha.rs:206）
  pre-OOS = ds.slice_date_window("1900-01-01", "2022-12-31")            # 严格早于 OOS（闭区间 ≤2022-12-31）
  仅计 !untradable ∧ close>0 的 bar（与管线一致，跳 untradable）
```

**为何无前视（F-可测）**：所有池化残差来自 `test_start ≥ OOS_START` 的窗（v1 §1.2，wverify_run.rs:58），entry_bar 全在 2023-01-01 之后。σ̂ 由**严格早于 2023** 的数据一次性估定 ⟹ 在每笔 OOS entry 时点 `F_entry`-可测 ⟹ 零前视。与残差管线的 `B̂=ĝ·h`（因果扩张窗漂移，只用 ≤entry 价）同一「只用过去信息」纪律。

**为何单品种标量（非逐笔）**：编排者令口径 = 「品种波动率尺度 σ̂_symbol」（per-symbol），一品种一标量，最简、最少活动部件、跨品种通约目标充分（品种间主导尺度差 = 价位级差，pre-OOS $ 波动率已捕获）。

**ponytail 简化 + 升级路径**：固定 pre-OOS 标量假设品种波动 regime 在 pre-2023 与 OOS(2023–25) 间大体平稳。若某品种 regime 剧变使 pre-OOS σ̂ 误缩放 OOS 残差成为翻转因素 ⟹ 升级为**逐笔因果扩张窗 σ̂_i**（镜像 ĝ，只用 ≤entry 的 bar 涨跌估 std）。当前冻结 = pre-OOS 固定标量（差分洁净、活动部件最少）。

**诚实剔除守卫（镜像 v1 OKLO）**：若某品种 pre-OOS 可交易 bar < `SIGMA_MIN_BARS=1000` 或 σ̂ ≤ 0（退化）⟹ 无法因果估尺度 ⟹ **该品种诚实剔除并入册**（非静默跳过），阶段2 报告记录实际入池品种集。7 品种（BTC/ES/CL/GC/BRN/DX/QQQ）均有数年 pre-2023 1-min 数据（百万级 bar），守卫实测预期不触发；触发即如实缩小 universe（σ̂-归一化对 universe 大小无关，不重启尺度伪影）。

---

## 2. 归一化 estimand（桶键 + 池化域 + 估计量 + 判据，除归一化外全沿用 v1）

### 2.1 桶键（4 元组，**逐字沿用 v1 §1.1，维数不变**）

```
z = (ℓ, bsp_class, δ, σ^H)     ∈ wverify_run.rs:93 现口径，bit-exact
```
full-z 不接入（v1 裁定 C 明令延后，只增功效不加维）。过 δ-共线检查：桶键不含尺度信息，σ̂ 缩放不改桶归属。

### 2.2 池化域（**沿用 v1 §1.2 冻结的 7 品种**）

`L3_UNIVERSE = {BTC, ES, CL, GC, BRN, DX, QQQ}`（wverify_run.rs:38，OKLO 已剔除）。窗口口径沿用 v1：各品种 `wf_anchored` 中 `test_start ≥ OOS_START` 子集。**唯一叠加** = §1 诚实剔除守卫（pre-OOS σ̂ 不可估者出池，实测预期空）。

### 2.3 归一化残差 + 池化规则（★本冻结核心）

```
Ỹ_i = Y_i / σ̂_symbol(i)                                # symbol(i) = 残差 i 所属品种
    = (δ_i·(H_i − B̂_i) − C_i) / σ̂_symbol(i)            # Y_i 逐字 = v1 / l3_delta_r_alpha.rs:255+ResidualTrade::y()
```

**池化规则（沿用 v1 §1.3 残差拼接 n 加权）**：7 品种全体**归一化后**残差 `Ỹ_i` 拼接进单一 vec，按 4 元组桶键分桶，桶内 μ̃ = 桶内 Ỹ_i 等权均值。与 v1 唯一差异 = 拼接的是 Ỹ（σ-单位）而非 Y（$ 单位）。

### 2.4 估计量 / LCB / 功效门（**沿用 v1 §1.4，施于 Ỹ**）

- μ̃(z) = 池化桶内 `Ỹ_i` 的 Welford 均值。
- LCB/UCB = `μ̃ ∓ 1.645·std(Ỹ)/√n`。
- CV(z) = std(Ỹ)/|μ̃|；n_eff(z) = `decontam::effective_n`（665，事件聚集自相关校正，**尺度不变** ⟹ n_eff 对 σ̂ 缩放不敏感，仅随聚集结构变）。
- **功效门**：`powered ⟺ n_eff ≥ (1.645·CV)²`（667）。**注意**：主桶特化门 290 是 v1 **原始-$ 的 CV=10.357** 派生（(1.645·10.357)²）——归一化后 CV 变化 ⟹ 功效门逐桶随新 CV 重算，**不复用 290 常数**（290 是 v1 口径的伪迹，本冻结主判据 = 逐桶 `n_eff ≥ (1.645·CV_归一化)²`）。

---

## 3. beta 剥离 + 跨品种置换隔离（**逐字沿用 v1 §2/§3，均不改**）

- **beta 剥离**：per-symbol `B̂_i = ĝ·h` 扩张窗（l3_delta_r_alpha.rs:243-252），s2 管线不改。σ̂ 缩放在 B̂ 剥离**之后**施加（先算干净残差 Y_i 再除 σ̂），减法非退化性不受影响。
- **跨品种置换隔离**：`time_block_base = symbol_index·SYMBOL_STRIDE + win.i·WF_TIME_STRIDE`（wverify_run.rs:67，SYMBOL_STRIDE=10^7），stratum 按品种隔离，`perm_test.rs` **不改**。
- **σ̂ 缩放与置换的相容性（★关键，无需改 perm_test）**：σ̂_symbol 是**品种常数**。stratum 已按品种隔离（SYMBOL_STRIDE）⟹ 同一 stratum 内全体残差共享同一 σ̂_symbol ⟹ 层内 δ shuffle 对 `Ỹ=Y/σ̂` 与对 `Y` 的相对极端性**同构**（正常数缩放不改层内排序）。跨桶（跨品种）聚合时不同 σ̂ 产生的重加权 = 归一化的**目的本身**，由「喂归一化后的 records 给 perm_test」透明承载，`perm_test.rs` 代码零改动（scaling 在 records 构造侧上游施加，见 §5）。

---

## 4. 三态判据（**逐字沿用 v1 §4，`decontam` 不改，施于 Ỹ 桶统计**）

| 桶态 | 条件 |
|------|------|
| VALIDATED | `μ̃>0 ∧ perm_p<0.05 ∧ LCB(Ỹ)>0 ∧ powered` |
| FALSIFIED | `powered ∧ LCB(Ỹ)≤0 ∧ UCB(Ỹ)≤0` |
| INCONCLUSIVE | `¬powered ∨ (LCB≤0<UCB)` |

全局 `decontam::global_verdict` 不改。**level≥2 frontier 污染标注**：wverify_run.rs 现状已记「frontier 已于 c546b5633c 修复解除」（bucket_verdict 列内字符串）——沿用现状标注，ℓ≥2 桶按代码现口径报告。ℓ∈{0,1}（含主桶 L0）不受任何 frontier 影响。

---

## 5. 阶段2 实装口径（冻结——bit-exact 护航，最小 diff）

**唯一新增** = `wverify_cross_symbol`（wverify_run.rs:241）逐品种循环里，`walk_forward_oos_residuals` 返回该品种 records 后、`pooled.extend(recs)` 前，插入 σ̂ 归一化：

```rust
// σ̂_symbol：pre-OOS 逐 bar close-to-close $ 涨跌 std（因果，无前视）。
let sigma = sigma_pre_oos(&ds, &cfg);                 // 新 helper：slice_date_window("1900-01-01","2022-12-31") 上算 std
assert!(sigma > 0.0 && n_pre_oos >= SIGMA_MIN_BARS, "{sym} σ̂ 不可估——诚实剔除，不静默兜底");
for r in &mut recs { r.resid_base /= sigma; r.cost /= sigma; }   // Ỹ = Y/σ̂：y()=δ·resid_base−cost 自动归一化
```

- **不改**：`build_mu_from_bars`（残差逻辑 bit-exact）、`ResidualTrade` 结构（缩放 resid_base+cost 两字段即得 Ỹ，y() 无需改）、`perm_test.rs`、`decontam`、`prereg_windows.rs`、`bucket_verdict`（透明消费归一化 records）、`wverify_full`（BTC 单标的不走 cross_symbol 路径，bit-exact）。
- **护航**：`cargo test --release --lib` 须维持 **1402 passed / 0 failed**（既有测试全绿）。
- **新增 L1 自检**（testing-override）：`sigma_pre_oos` 对 BTC 返回 >0 有限值 + pre-OOS bar 数 ≥ SIGMA_MIN_BARS（管线正确性，非假设验证——L1 零信息增量，仅防 σ̂ 估计 bug）。
- **代码基线确认（硬门）**：跑数**前**与 team-lead 确认 HEAD——ws-bottomup 若在改 econ/nest 生产信号集，本工位跑数须用其定稿 HEAD（避免混版本），bit-exact 差分才洁净。

---

## 6. 报告规格（阶段2 产出，看结果前冻结结构）

结果包 `.chanlun/review-results/l3norm-alpha-20260703.md`，含：

1. **σ̂ 表**：逐品种 σ̂_symbol 值 + pre-OOS bar 数——使读者直观看到 BTC σ̂ 远大于 ES/DX，归一化如何除去尺度。
2. **per-symbol 归一化表**：逐品种逐桶 `(n, μ̃, LCB(Ỹ), perm_p, state)`——暴露归一化后品种异质性是否仍 BTC 独占正号（v1 强烈暗示）。
3. **池化归一化表**：7 品种 Ỹ 拼接逐桶三态 + 全局裁决（主判据）。
4. **★v1↔v2 差分（本冻结核心报告项）**：主桶 `L0/type3/买/σ0` 三口径对照——
   | 口径 | n | n_eff | mean | CV | LCB | perm_p | powered? | state |
   |---|---|---|---|---|---|---|---|---|
   | v1 原始-$ 拼接 | 6865 | 699.74 | +39.67 | 22.582 | +21.88 | 0.040 | 否(门1380) | Inconclusive |
   | **v2 σ̂-归一化** | (待跑) | … | … | … | … | … | … | … |
   
   核心判读：(a) CV 是否降到 `n_eff ≥ (1.645·CV)²` 内 ⟹ 主桶 powered？(b) 归一化后 mean 符号/LCB ⟹ INCONCLUSIVE→VALIDATED（功效收口）还是转 FALSIFIED（坐实 BTC-idiosyncratic）？两个方向都是合法产出。
5. **认识论 L3 声明**（231）：per-symbol B̂ 独立 + σ̂ 因果无前视 ⟹ 真去污 + 通约；否定性结果（若池化转负/FALSIFIED）缩小有效域，比确认信息量高。

---

## 7. 冻结常量表（看结果前锁定）

| 常量 | 值 | 来源 |
|------|-----|------|
| 归一化口径 | **(A) σ̂_symbol 除法**（(B) 对数收益率化否决——破 Y_i bit-exact） | 本冻结 §0 |
| σ̂_symbol | pre-OOS(date<2023-01-01) 逐 bar close-to-close $ 涨跌 std | 本冻结 §1 |
| SIGMA_MIN_BARS | 1000（pre-OOS bar 下限，不足则诚实剔除） | 本冻结 §1 |
| Ỹ_i | `Y_i / σ̂_symbol`（Y_i 逐字沿用 v1，缩放 resid_base+cost） | 本冻结 §2.3 |
| 桶键 | `(ℓ,bsp_class,δ,σ^H)` 4 元组不加维 | v1 §1.1 |
| 池化域 | {BTC,ES,CL,GC,BRN,DX,QQQ}（7 品种，OKLO 剔除；σ̂ 守卫叠加） | v1 §1.2 |
| 池化规则 | 归一化残差拼接（n 加权，拼 Ỹ） | 本冻结 §2.3 |
| 功效门 | `n_eff ≥ (1.645·CV_归一化)²`（逐桶重算，**不复用 290**） | 本冻结 §2.4 |
| z_α / perm 种子 / N_PERM | 1.645 / `PERM_SEED`=20260701 / 200 | v1 §5 |
| SYMBOL_STRIDE / WF_TIME_STRIDE | 10^7 / 10^4 | v1 §3（不改） |
| beta 剥离 | B̂=ĝ·h 因果扩张窗 per-symbol（σ̂ 缩放在剥离后） | v1 §2 |
| 成本 | ThetaConfig::default() 统一费率（缩放前算 C，再同除 σ̂） | v1 §2 |

---

## 8. 结果包六要素

1. **结论**：冻结跨品种归一化 L3 v2 estimand——归一化口径**二选一定为 (A) σ̂_symbol 除法**（(B) 对数收益率化因破残差 Y_i bit-exact + 护航 1402 而否决）；`Ỹ_i = Y_i/σ̂_symbol`，σ̂ = pre-OOS 逐 bar $ 涨跌 std（因果无前视单品种标量）；桶键/池化域/beta/置换隔离/三态判据全部逐字沿用 v1，唯一新增 = 池化阶段 σ̂ 缩放（resid_base+cost 两字段，perm_test/decontam/build_mu 零改动 bit-exact）。
2. **定义依据**：可交易 alpha μ(z)>0（663）；三态 + 功效门 `n_eff≥(1.645·CV)²`（667）；桶键先验给定（665）；σ̂ 因果无前视 ⟹ F_entry-可测（B̂ 同纪律）；σ̂ 品种常数 + stratum 品种隔离 ⟹ 层内 shuffle 同构（perm_test 无需改）。
3. **边界条件（翻转）**：(a) 归一化后主桶 CV 降到 `n_eff≥(1.645·CV)²` 内 ∧ LCB(Ỹ)>0 ⟹ INCONCLUSIVE→VALIDATED（功效收口，编排者目标达成）；(b) 归一化后 6/7 品种 σ-单位仍负、池化 mean 转负 ⟹ 主桶 FALSIFIED/INCONCLUSIVE，坐实「正 alpha BTC-idiosyncratic」；(c) 某品种 pre-OOS bar<1000 或 σ̂≤0 ⟹ 诚实剔除入册；(d) 若品种 vol regime 在 pre-2023/OOS 间剧变致 pre-OOS σ̂ 误缩放成翻转因素 ⟹ 升级逐笔因果扩张窗 σ̂_i（本冻结用固定标量）。
4. **下游推论**：v2 主桶 VALIDATED ⟹ 跨品种通约后缠论主桶存在超-beta alpha，M1 推进；v2 主桶仍 INCONCLUSIVE/FALSIFIED ⟹ 缩小有效域（跨品种归一化后亦无 confirmed alpha，主桶正号 BTC 独有被坐实，对照 oddeven_mu_identity beta 漂移伪结构）；full-z 延后不变（无 VALIDATED 则无加维动机）。
5. **谱系引用**：v1 冻结 `prereg-l3-cross-symbol-20260702` §5（改 estimand 须新冻结——本冻结即履行）+ v1 结果包 §5 边界（归一化列为新预注册候选）；codex #85 裁 C + 边界 (d)（尺度不可通约）；663/665/667（判据/去污/功效门）；231（否定性结果缩边界 + L3 声明须 L3 验证）；formalization-validity-domain（L0/L1 冻结零增量，L3 跑批产 L2+ 信息）；no-patch-mentality（(B) 破 bit-exact 否决的依据）；oddeven_mu_identity（BTC beta 漂移伪结构参照）。
6. **影响声明**：新增本预注册文件（L0/L1 规格）+ 待改 `wverify_run.rs::wverify_cross_symbol`（阶段2：新 helper `sigma_pre_oos` + 逐品种 σ̂ 缩放循环 + L1 自检）。**不改**：`build_mu_from_bars`/`ResidualTrade`/`perm_test.rs`/`decontam`/`prereg_windows.rs`/`bucket_verdict`/`wverify_full`（全 bit-exact）、v1 冻结文档。护航：`cargo test --release --lib` 1402 基线不退化。
