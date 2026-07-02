# 跨标的 L3 alpha 池化 预注册冻结 v0（codex #85 裁定 C 落地）

- **工位**：swarm/ws-l3 | task #86 | 基线 HEAD=7e39e70b42
- **冻结时刻**：2026-07-02（**看任何跨标的 L2/L3 池化结果之前**——防数据挖掘，protocol §0/§2.3）
- **裁定依据**：`codex-85-fullz-residual-ruling-20260702.md`（裁 C：先补跨标的 L3 池化提功效，full-z 延后；**新 estimand 须新冻结，桶键维数不变仍 4 元组**）。
- **认识论**：本文件 **L0/L1**（纯规格冻结，零 L2 信息增量）。正式池化跑批产出为 **L3**（跨标的真去污），逐桶如实标注。
- **不复用**：`acc-alpha-estimand-prereg-20260701.md`（BTC 4 元组冻结）——本冻结是**新 estimand**（估计对象从「BTC 单标的 4 元组」扩为「7 品种池化 4 元组」，见 §5 预注册 §5 改 estimand 须新冻结）。

---

## 1. 冻结的 estimand（桶键 + 池化域 + 估计量 + 判据口径）

### 1.1 桶键（4 元组，**维数不变**，裁定明令不加维）

```
z = (ℓ, bsp_class, δ, σ^H)                       # wverify_run.rs:84 现口径，bit-exact
  ℓ         = MuClass.level        ∈ {L0..L5}     缠论级别
  bsp_class = MuClass.bsp_class()  ∈ {1,2,3}      一/二/三类买卖点
  δ         = MuClass.delta        ∈ {+1(买),−1(卖)}
  σ^H       = MuClass.parent_dir   ∈ {−1,0,+1}    父声部方向（根声部=0；perm_test.rs:36 σ^H≡parent_dir）
```

**与 BTC 冻结的唯一差异 = 池化域**（§1.2），桶键结构逐字不变。full-z（完整 MuClass 7 维）**不接入**——裁定 C 明令延后，只增 n 不加维。

### 1.2 池化域（symbol universe，**新冻结核心**）

**候选 = `prereg_windows.rs::PREREG_WINDOWS` 8 品种**，按 pool 角色筛：

| symbol | pool | 数据文件（analysis/data_cache/） | wf_anchored 窗数 | 入池？ |
|--------|------|--------------------------------|-----------------|-------|
| BTC | Core | btc_1m_full.json | 12 | ✓ |
| ES  | Core | es_1m_databento_10y.json | 15 | ✓ |
| CL  | Core | cl_1m_databento_10y.json | 15 | ✓ |
| GC  | Core | gc_1m_databento_10y.json | 15 | ✓ |
| BRN | Extended | brn_1m_databento_10y.json | 10 | ✓ |
| DX  | Extended | dx_1m_databento_10y.json | 10 | ✓ |
| QQQ | Extended | qqq_1m_databento_full.json | 11 | ✓ |
| OKLO | Observation | oklo_1m_databento.json | **0** | ✗ 剔除 |

**冻结池化域 = {BTC, ES, CL, GC, BRN, DX, QQQ}（7 品种）**。OKLO **诚实剔除**（非数据缺失）：`wf_anchored=[]`（Observation 池无 walk-forward 窗，无法产 walk-forward OOS 残差）+ OOS 跨越全局 Holdout 边界（OK1 张力，protocol §2.4 特例，不进主判据）。8 个数据文件**全部物理存在**（已核，无数据缺口）。

**窗口口径**：每品种取其 `wf_anchored` 中 `test_start ≥ OOS_START(2023-01-01)` 的窗子集（与 BTC 单标的口径同，wverify_run.rs:51），逐窗对 test 段独立 `build_mu_from_bars` 后聚合。clipped 末窗残差计入池（样本外真实，与 BTC 口径一致；W1 裁定「clipped 不进时间稳定性主判据」不影响 alpha 存在性池化）。

### 1.3 池化规则（跨品种同桶合并方式，**冻结**）

**残差拼接（residual concatenation）= n 加权池化**：把 7 品种全部 `ResidualTrade` 记录**拼接进单一 vec**，按 4 元组桶键分桶。桶内 μ̂ = 跨品种全体残差 Y_i 的等权均值（等价按 n 加权各品种桶均值）。

- **选择理由**：裁定 C 的目标是**增 n 过功效门**（BTC 主桶 n_eff=166<290）。拼接直接增 n，是池化条件期望 `E[Y|Z=z]`（总体=「7 品种全体信号」）的自然估计量；置换检验也需拼接后的原始残差记录。
- **不选** meta-analysis 式「逐品种桶均值再加权平均」——不同样地增 n，且掩盖品种异质性（异质性用 §3.2 per-symbol 表暴露）。

### 1.4 估计量 / LCB / 功效门（引用已实装，不重造）

- μ̂(z) = 池化桶内 `ResidualTrade::y()=δ·(H−B̂)−C` 的 Welford 均值（wverify_run.rs:81-92 现口径，扩到多品种）。
- LCB/UCB = `μ̂ ∓ z_α·std/√n`，`z_α=1.645`（单边 95%）。
- CV(z)=std/|μ̂|；n_eff(z)=事件聚集自相关校正（`decontam::effective_n`，665）。
- **功效门**：`powered ⟺ n_eff ≥ (1.645·CV)²`（667）。**主桶特化门 = 290**（BTC 主桶 CV=10.357 ⟹ (1.645·10.357)²≈290）；一般门逐桶随 CV 变。

---

## 2. beta 剥离口径（★冻结——B̂=ĝ·h 扩张窗，s2 规格，per-symbol）

**沿用 s2 残差管线**（`l3_delta_r_alpha::build_mu_from_bars`:131-252，task #82，alpha分离.pdf §1/§4.1）：

```
Y_i = δ_i·(H_i − B̂_i) − C_i
  H_i = P_out − P_in                                   δ-free 原始持有窗涨跌
  B̂_i = ĝ·(exit_bar − entry_bar)                       持有窗市场漂移
  ĝ   = (P_entry − P_first)/(entry_bar − first_bar)     因果扩张窗漂移（只用 ≤entry 价，F_entry 可测，无前视）
  C_i = fee_rate·(P_in + P_out)                          双边费
  fee_rate = (commission_bps+slippage_bps+tax_bps)/10000  ← ThetaConfig::default()，全品种统一
```

**跨品种如何真去污（L3 vs L2 的核心）**：`B̂_i` 由**每个品种自身** `first_tradable` bar 起的扩张窗漂移估，即 per-symbol 市场因子。跨品种池化 = 对 7 品种各自跑 s2 残差管线（各自剥各自 beta）后拼接残差。这坐实了 `acc-alpha-estimand-prereg-20260701.md` §2 路径 B「8 品种独立市场因子」——残差管线**已内建 per-symbol B̂**，路径 B = 循环路径 A 的残差管线跨品种，无需新减法。`raw_i` 与各品种同期 beta 独立，减法非退化（665 单标的退化定理的有效域=单标的直接扣 B_market≡raw，此处 B̂=扩张窗漂移≠逐笔 raw，非退化）。

**成本口径诚实标注**（ponytail 简化）：全品种用同一 `ThetaConfig::default()` 费率——统一成本模型是简化（真实 per-symbol 成本/tick 不同）；若跨品种成交现实性成为翻转因素，改 per-symbol 费率配置。当前冻结 = 统一 default（与 BTC 单标的口径可比，差分洁净）。

---

## 3. 跨品种置换分层口径（★冻结——per-symbol 偏移，perm_test.rs **不改**）

裁定明令 `perm_test.rs` 不改。分层键 = `(ℓ, h_bucket, time_block, σ^H)`（perm_test.rs:95），层内 shuffle δ、层间 beta 被分层吸收。

**跨品种污染风险**：不同品种若共享同一 `time_block` 值 ⟹ 落入同一置换 stratum ⟹ δ 在**跨品种**间 shuffle。但不同品种**不共享持有窗 beta**（各自市场），跨品种同 stratum 置换 = beta 污染。

**冻结解法（只改 harness 传参，不改 perm_test.rs）**：`build_mu_from_bars` 的 `time_block_base` 参数按品种加**大偏移**，使各品种 `time_block` 值域**两两不相交** ⟹ 置换 stratum 天然按品种隔离：

```
time_block_base(symbol_index, win.i) = symbol_index · SYMBOL_STRIDE + win.i · WF_TIME_STRIDE
  WF_TIME_STRIDE  = 10_000       （现有常量，窗间偏移）
  SYMBOL_STRIDE   = 10_000_000   （冻结新常量，品种间偏移）
```

**跨度核算**（保证不相交）：窗内 `time_block = entry_bar/TIME_BLOCK_BARS`，`TIME_BLOCK_BARS=43_200`；6 月 test 窗 1min ≤ ~259K bar ⟹ 窗内块 ≤ ~6 < WF_TIME_STRIDE(10K)。单品种最多 15 窗 ⟹ 品种 time_block 值域 ≤ 15·10_000+6 = 150_006 < SYMBOL_STRIDE(10^7)。7 品种 · 10^7 = 7·10^7 < u32::MAX(4.29·10^9)，不溢出。⟹ 品种间 stratum 严格隔离，置换只在**同品种同窗同层**内 shuffle δ，无跨品种 beta 污染。

---

## 4. 三态判据（冻结，与 BTC 冻结逐字一致，`decontam` 不改）

逐桶 `decontam::classify_bucket(mean,lcb,ucb,perm_p,n_eff,cv,1.645,0.05)`：

| 桶态 | 条件 |
|------|------|
| **VALIDATED** | `μ̂>0 ∧ perm_p<0.05 ∧ LCB>0 ∧ powered` |
| **FALSIFIED** | `powered ∧ LCB≤0 ∧ UCB≤0` |
| **INCONCLUSIVE** | `¬powered ∨ (LCB≤0<UCB)` |

全局：`decontam::global_verdict`——有 VALIDATED ⟹ PASS；全桶 powered-FALSIFIED ⟹ 161；否则 INCONCLUSIVE。**判据链不改**（裁定：只增池化域，不改统计机器）。

---

## 5. 冻结常量表（看结果前锁定）

| 常量 | 值 | 来源 |
|------|-----|------|
| 置换种子 | `perm_test::PERM_SEED`（=20260701，看 perm_test.rs 冻结值） | 666 方法1 |
| 置换次数 | `perm_test::N_PERM` = 200 | 666 |
| z_α | 1.645（单边 95%） | mu_estimator |
| 功效门 | `n_eff ≥ (1.645·CV)²`（主桶特化 290） | 667 |
| 桶键 | `(ℓ, bsp_class, δ, σ^H=parent_dir)`，4 元组不加维 | wverify_run.rs:84 |
| 池化域 | {BTC,ES,CL,GC,BRN,DX,QQQ}（7 品种，OKLO 剔除） | 本冻结 §1.2 |
| 池化规则 | 残差拼接（n 加权） | 本冻结 §1.3 |
| 窗口 | 各品种 `wf_anchored` 中 test_start≥2023-01-01 | prereg_windows.rs |
| WF_TIME_STRIDE | 10_000 | 现有常量 |
| SYMBOL_STRIDE | 10_000_000 | 本冻结 §3（新常量，跨品种 stratum 隔离） |
| beta 剥离 | B̂=ĝ·h 因果扩张窗，per-symbol | s2 / #82 |
| 成本 | ThetaConfig::default() 统一费率 | 本冻结 §2 |

---

## 6. 报告规格（阶段2 产出，看结果前冻结结构）

- **per-symbol 表**：逐品种逐桶 (n, μ̂, LCB, perm_p, state)——暴露品种异质性。
- **池化表**：7 品种拼接后逐桶三态 + 全局裁决（主判据）。
- **与 BTC 单标的差分**：池化主桶 vs s3 BTC 主桶（n_eff 166→池化后、mean、LCB、state 是否翻转 INCONCLUSIVE→VALIDATED）。
- **高级别桶（ℓ≥1）**：结论标注 **frontier 污染条件效力域**（区间套问题② #84 在途未修，perm_p=1 单-δ 层退化未排除）——不作独立否证。
- 结果包 `.chanlun/review-results/l3-cross-symbol-alpha-20260702.md`（六要素 + 认识论 L3）。

---

## 7. 结果包六要素

1. **结论**：冻结跨标的 L3 池化 estimand——桶键 4 元组不变 `(ℓ,bsp_class,δ,σ^H)`、池化域 7 品种（OKLO 剔除）、残差拼接 n 加权、beta per-symbol 扩张窗剥离、跨品种置换按 SYMBOL_STRIDE 隔离 stratum（perm_test.rs 不改）、三态判据不改。
2. **定义依据**：可交易性 μ(z)>0（663）；桶键先验给定（665 方向不对称免疫）；跨品种 B̂_i 独立 ⟹ 减法非退化（665 单标的退化定理有效域=单标的扣 B_market）；LCB≤0 underpowered ⊬ μ≤0（667+231）。
3. **边界条件（翻转）**：(a) 若某品种 residual 口径/格式不符 s2 管线 ⟹ 该品种诚实剔除并入册，不静默跳过；(b) 若跨品种 stratum 未隔离（time_block 碰撞）⟹ 置换 beta 污染，SYMBOL_STRIDE 须够大（§3 核算已保证）；(c) 若池化后主桶 n_eff≥290 且 LCB>0 ⟹ INCONCLUSIVE→VALIDATED（裁定 C 目标达成）；(d) 若池化引入不可接受品种异质性（beta/成本无法跨品种一致）⟹ (C) 暂停退 (A)（codex #85 §4）。
4. **下游推论**：池化 PASS ⟹ 跨标的存在超 beta alpha，M1 里程碑推进；池化仍 INCONCLUSIVE ⟹ 缩小有效域边界（跨标的亦欠功效/无检出），full-z 与提功效链后续；全桶 powered-FALSIFIED ⟹ 跨标的无 edge 照实 161。
5. **谱系引用**：codex #85 裁定 C（本冻结的直接依据）；663/665/666/667（判据/去污/功效）；231（有效域<定义域，L3 声明须 L3 验证）；acc-alpha-estimand-prereg §5（改 estimand 须新冻结——本冻结即履行）；s3 strict-alpha-retest（BTC 单标的 INCONCLUSIVE 基线，池化差分对象）。
6. **影响声明**：新增本预注册文件（L0/L1 规格）+ 待改 `wverify_run.rs`（symbol 硬编码→7 品种循环 + SYMBOL_STRIDE 偏移，阶段2）。**不改**：`perm_test.rs`（裁定明令）、`decontam`（三态判据）、`prereg_windows.rs`（窗口冻结）、`acc-alpha-estimand-prereg-20260701.md`（BTC 冻结）、`l3_delta_r_alpha.rs` 残差逻辑（只加 time_block_base 传参）。护航：cargo test --release --lib 1402 基线不退化 + harness 改动加 L1 自检测试。
