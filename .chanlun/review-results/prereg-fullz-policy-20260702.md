# full-z×残差 + 完整策略级回测 预注册冻结 v0（编排者解除 codex #85 延后）

- **工位**：swarm/ws-p3 | task #96 | 基线 HEAD=be4f31edd5
- **冻结时刻**：2026-07-02（**看任何 full-z 残差逐桶结果 / 策略级 equity 之前**——防数据挖掘，acc-alpha-estimand-prereg §0 纪律）
- **授权依据**：编排者裁定「我要的是完整的策略实装」优先于 codex #85 的排期裁量（codex #85 裁 C 延后 full-z，本冻结解除延后但**继承 codex #85 §3-§4 对 full-z 的全部数学约束与三阻塞清单**——不是推翻 codex，是在其约束下把延后项做实）。
- **认识论**：本文件 **L0/L1**（纯规格冻结，零 L2 信息增量）。正式跑批产出：(A) full-z 残差逐桶 = **L2**（真实 BTC 单标的 walk-forward OOS，可否证）；(B) 策略级 π 回测 = **L2**（真实 BTC+CL，policy 组合级 equity，walk-forward OOS）。
- **不复用**：`acc-alpha-estimand-prereg-20260701.md`（BTC 4 元组冻结）、`prereg-l3-cross-symbol-20260702.md`（7 品种 4 元组冻结）——本冻结是**新 estimand**（估计对象从「4 元组投影桶」升到「完整 MuClass 7 维桶」，见 §6 改 estimand 须新冻结）。

---

## 0. 两个交付维度（本冻结覆盖两者，硬门：冻结先于跑数）

| 维度 | 对象 | 与既有工作的本质区别 | 认识论 |
|------|------|---------------------|--------|
| **(A) full-z 残差逐桶判定**（信号级） | 逐信号 μ̂(z) 三态，桶键=完整 MuClass 7 维 | s3 用 4 元组投影 `(ℓ,bsp,δ,σ^H)`；本维展开被投影掉的 `i_class`(未压 6-bit)/`short_swing`/`position`/`horizontal` | L2（BTC 单标的，隔离 full-z 单一变量，不叠加跨标的） |
| **(B) 完整策略级回测**（组合级 P&L） | 生产 π `run_theta_v0_pi_chi` 全链 equity 曲线 | s3/A 只测逐信号 μ̂；B 测完整 π 组合级盈亏（净持仓账本 + LexArgmin + χ 门 μ̂ 注入） | L2（BTC 全历史+CL，walk-forward OOS） |

**A 与 B 各自隔离单一变量**（no-workaround / 231）：A 只改桶键维数（BTC 单标的不动），B 只测策略层组合效应（桶键沿用生产 MuClass）。不把「full-z」「跨标的」「策略层」三个 estimand 变化叠加进一次跑批——否则否定性结果无法归因。

---

## (A) full-z 残差逐桶判定

## A1. 冻结的 estimand（桶键 + 估计域 + 判据口径）

### A1.1 桶键（完整 MuClass 7 维，**走 z_of_candidate 生产路径**）

```
z = MuClass{ level, delta, i_class, parent_dir, short_swing, position, horizontal }   # 完整 7 维
  level       = 缠论级别 ∈ {L0..L5}
  delta       = 持仓方向 ∈ {+1(买),−1(卖)}
  i_class     = BspBits.class_index() 6-bit **不压扁**（2B/3B 重合保留，s3 投影为 bsp_class(1/2/3) 丢此细分）
  parent_dir  = σ^H 父声部方向 ∈ {−1,0,+1}
  short_swing = (parent_dir≠0 ∧ delta=−parent_dir)     **δ 派生量**（见 A3.2 置换处理）
  position    = Root / Child
  horizontal  = Some(c.role.h) ∈ {First,SameFollow,SameReverse}   # 残差路径 z_of_candidate 填（selector.rs:158）
```

**生产路径无 fork（675）**：残差记录 `ResidualTrade.class` 由 `build_mu_from_bars`(l3_delta_r_alpha.rs:194 `z_of_candidate(c)`) 填 —— 即残差数据**已携带完整 7 维 z**（`horizontal=Some`，`i_class` 未压，`position` 真值）。s3 阻塞不在数据侧，只在**消费侧** perm_test/wverify 把它投影到 4 维。本冻结的实装 = 在消费侧保留完整 7 维，不动数据侧、不动 z_of_candidate。

### A1.2 估计域（**BTC 单标的，隔离 full-z 单一变量**）

- 标的 = **BTC 全历史 OOS**（btc_1m_full.json，461 万 bar），walk-forward OOS 窗口沿用 `PREREG_WINDOWS` BTC anchored 中 `test_start≥OOS_START(2023-01-01)` 子集（与 s3/wverify_full 逐窗口径 bit-exact 同）。
- **不跨标的**（隔离原则）：跨标的池化是 `prereg-l3-cross-symbol` 的独立 estimand；本冻结只改桶键维数。full-z×跨标的的组合留待两者各自出结果后（no-workaround：一次一个变量）。

### A1.3 估计量 / LCB / 功效门（引用已实装，不重造）

- μ̂(z) = full-z 桶内 `ResidualTrade::y()=δ·(H−B̂)−C` 的 Welford 均值。
- LCB/UCB = `μ̂ ∓ 1.645·std/√n`（`decontam` 现口径）。
- **功效门逐桶 CV 化（codex #85 阻塞③ —— 现有代码已解决，本冻结确认无需改）**：`decontam::powered(n_eff, cv, z_alpha)=n_eff≥(z_α·CV)²` 是**逐桶** cv 的确定性函数（decontam.rs:115）。s3 报告的「290 特化门」不是硬编码门，是主桶 CV=10.357 代入公式的**值**。full-z 每个子桶用**自己的 CV** 算门 —— 阻塞③在现有 `classify_bucket` 里天然满足，实装零改动。

## A2. beta 剥离口径（★冻结——沿用 s2 残差管线，逐字不变）

```
Y_i = δ_i·(H_i − B̂_i) − C_i                          # alpha分离.pdf §1（p1-2）、§4.1（p5）
  H_i = P_out − P_in                                  δ-free 原始持有窗涨跌
  B̂_i = ĝ·(exit_bar − entry_bar)                      持有窗市场漂移
  ĝ   = (P_entry − P_first)/(entry_bar − first_bar)    因果扩张窗漂移（只用 ≤entry 价，无前视）
  C_i = fee_rate·(P_in + P_out)                        双边费，ThetaConfig::default() 统一
```

**残差口径不动摇**（任务硬约束）：Y_i 与置换检验都跑在 `ResidualTrade::y()` 上（δ-free 基 `resid_base=H−B̂` 重新赋 δ），非原始 X_γ。原文页码：残差减法 `alpha分离.pdf` p1 §1 / p5 §4.1；分层置换 p5-6 §4.2。full-z **不改残差减法**，只改消费侧分桶键。

## A3. ★perm_test 扩维实装（codex #85 阻塞①，本冻结核心）

现状：`perm_test::stratified_delta_perm_p` 输出键 `BucketKey=(u32,u8,i8,i8)=(ℓ,bsp,δ,σ^H)` 4 维硬编码（perm_test.rs:37/84）。full-z 判定须输出键=完整 MuClass。

### A3.1 分层键**不变**（§4.2 明令）

分层键 = `(ℓ, h_bucket, time_block, σ^H)`（perm_test.rs:95），层内 shuffle δ、层间 beta 被分层吸收。**bsp_class 及全部 Z 维不入分层键**——它们是被检验的结构变量，不是要控制的混杂量（§4.2 H0：给定分层，δ 是否携带残差解释力）。full-z 扩维**只动输出桶键，不动分层键**。

### A3.2 输出桶键扩维 + δ 派生量（short_swing）的置换处理（★no-workaround 精确落点）

`short_swing = (parent_dir≠0 ∧ δ=−parent_dir)` 是 **δ 的派生量**——置换 δ 时 short_swing 随之翻转，故不能作为「δ-free 输出基」的分量。冻结解法（与现有 4 元组 obs_plus/obs_minus 结构同构，非新机制）：

```
δ-free 输出基 base = (level, i_class, parent_dir, position, horizontal)     # 全 Ord ⟹ BTreeMap 确定序
每个 base 恰对应两个 full-z 输出桶：
  δ=+1 桶：MuClass{...base, delta:+1, short_swing:(parent_dir≠0 ∧ +1=−parent_dir)}
  δ=−1 桶：MuClass{...base, delta:−1, short_swing:(parent_dir≠0 ∧ −1=−parent_dir)}
obs_plus  = base 内原始 δ=+1 成员的 mean(r−C)
obs_minus = base 内原始 δ=−1 成员的 mean(−r−C)
置换：层内 shuffle δ → 逐 base 按 perm-δ 重算 +/− 均值比观测（short_swing 由 perm-δ 重构，落入对应 full-z 桶）
```

这与现有代码 `OutBucket.base=(u32,u8,i8)` + obs_plus/obs_minus **逐字同构**，只是 base 从 3 维升到 5 维（`Horizontal` 派生 `Ord`，coverage.rs:758）。permute δ 移动信号于同一 base 的两个子桶间 —— 是干净双射，无成员越 base 迁移（no-workaround：不是把 δ-依赖字段硬塞进 base，而是从 perm-δ 确定性重构）。

### A3.3 跨进程可复现（预注册 §4 硬约束）

- 置换序列由 `(ℓ,h,time,σ^H)` 分层键的 **BTreeMap 确定序** + 冻结种子 `PERM_SEED=20260701` 决定 —— 分层键不变 ⟹ RNG 消耗序与现有 4 元组**逐字节一致**（full-z 只改输出聚合，不改分层遍历序）。
- 输出 `HashMap<MuClass,f64>` 序列化前按 `MuClass`(Ord) 排序键（跨进程逐字节可比，镜像现有 `perm_xproc_child` 的 `keys.sort_unstable()`）。
- **实装形态（ponytail，最小改动 + 护航）**：**新增** `stratified_delta_perm_p_fullz(records) -> HashMap<MuClass,f64>` 与 `_uclass(records) -> HashMap<UClass,f64>`，**不改**现有 4 元组 `stratified_delta_perm_p`（wverify_full/bucket_verdict 的 1401 基线 bit-exact 不退化）。三者共用同一分层遍历序 + 同种子。新增 L1 自检：full-z/UClass 同种子跨进程逐字节复现 + 红 demo（扰动种子改输出）。

### A3.4 UClass 降维并列（codex #85 阻塞②，winner's curse / 桶碎裂防护）

full-z 把 BTC 高级别桶（s1 oracle 已示 L0 Y 桶碎裂到 n=4/19/1）进一步碎裂 ⟹ 单-δ 层置换退化 perm_p=1 ⟹ 逐桶 full-z 无鉴别力。冻结**两口径并列报告**（不二选一）：

- **full-z 口径**：桶键=完整 MuClass 7 维（细，碎裂，多数 INCONCLUSIVE-underpowered）。
- **UClass 口径**：桶键=`UClass::project_to_u`(mu_estimator.rs:182) = `(level_bucket, δ, role, divergence)`（粗，聚合，抗碎裂；丢 H、折叠 (parent_dir,short_swing,position)→role、i_class→divergence bool、level→level/2 桶）。UClass 的 role 同样 δ-派生（ChildTrend/ChildSwing 依 δ vs parent_dir）⟹ δ-free base=`(level_bucket, position, parent_dir, divergence)`，A3.2 同法处理。

**诚实预承诺（090/231，冻结于看结果前）**：full-z 大概率把多数桶推向 INCONCLUSIVE（欠功效/单-δ 退化）——这是 codex #85 数学核验的预期，**如实入册不掩饰**。full-z 的价值不在「产新 VALIDATED」（同质假设下功效必降，codex #85 §7），而在：(1) 冻结先于结果 ⟹ 无论碎裂到什么程度都不是数据挖掘；(2) UClass 并列给出聚合口径的功效对照；(3) 若**某** full-z 子桶因剥离异质子群使 CV 骤降、功效反升（codex #85 判定的理论可能性），则如实标注为 full-z 相对 4 元组的**信息增量**（须同时满足 n_eff 达标 + CV 大幅低于父桶 + perm_p 未退化，A5 边界条件 c）。

## A4. 三态判据（冻结，`decontam` 不改，与 s3/BTC 冻结逐字一致）

逐桶 `decontam::classify_bucket(mean,lcb,ucb,perm_p,n_eff,cv,1.645,0.05)`：VALIDATED=`μ̂>0∧perm_p<0.05∧LCB>0∧powered`；FALSIFIED=`powered∧LCB≤0∧UCB≤0`；INCONCLUSIVE=`¬powered ∨ LCB≤0<UCB`。全局 `decontam::global_verdict`。判据链不改（只改分桶键维数）。

**level≥2 frontier 污染条件效力域（231，Lead 强制）**：区间套问题② `frontier-bt-consumed-20260702.md`（增量/全量 level≥2 真发散，#87 在审）未排除 ⟹ **ℓ≥2 桶所有结论条件化于「frontier-bug 污染未排除」**，挂标注不作独立否证/确认。ℓ∈{0,1} 不受影响（50K bit-exact）。

---

## (B) 完整策略级回测

## B1. 冻结的回测配置

| 项 | 冻结值 | 来源/理由 |
|----|--------|-----------|
| 标的 | **BTC 全历史 + CL**（数据在池） | 任务指定；CL 数据 `cl_1m_databento_10y.json` 物理存在 |
| π 入口 | `run_theta_v0_pi_chi`（χ 门 μ̂ 注入生产 π） | runner.rs:287；完整 π=`run_theta_v0_pi_inner`（净账本+LexArgmin+K_Θ），χ 门=`Γ_t^trade={γ:LCB(μ)>θ}` |
| χ 门 μ̂ 表 `est` | **walk-forward OOS 训练**（train 段建 est，test 段应用；无 in-sample 泄漏） | runner.rs:285「因果性由调用方负责」；全窗 in-sample est=泄漏 L1（禁）；walk-forward=L2 |
| χ 阈值 θ / z_α | `chi_theta=0.0`（LCB>0 门）/ `chi_z_alpha=1.645` | 生产 χ=1[LCB(μ)>0]（p8-9 选择器）；θ=0 = 「LCB 严格为正才交易」最诚实门 |
| treat_empty_as_pass | `false`（空类不交易） | codex Q3：空 μ̂ 类不交易最诚实 |
| 基线（差分对象1） | **无 χ 门** `run_theta_v0_pi`（χ≡1 全覆盖） | 隔离 μ̂ 选择器的增量价值（π 结构相同，唯一差 = χ 门是否开） |
| 成本 | `ThetaConfig::default()` 统一费率 | 与既有可比（s3/#13 同口径） |
| initial_nav | 与品种首价量级匹配（BTC/CL 各自 sizing 非零门） | runner.rs:124 sizing 需 NAV 够买 1 lot；权益曲线归一化输出 |

**walk-forward 原子性**：χ 门 est 逐窗独立（train 段 `build_walk_forward_mu` 建 est → test 段 `run_theta_v0_pi_chi` 应用），test 段 equity 拼接为整条曲线。基线（无 χ）同窗口跑 `run_theta_v0_pi`。**不用全窗单 est**（in-sample 泄漏）。

## B2. 输出规格（看结果前冻结结构）

- **equity 曲线**：`RunResult.equity_curve`（归一化绝对权益 E_t=(cash+units·P)/nav0）——χ-gated 与 baseline 两条并列。
- **最大回撤**：`RunResult.metrics.max_drawdown`（metrics.rs:66，峰谷）。
- **按 z 桶归因的 P&L 分解**：逐笔平仓 P&L 归到其入场 z 桶。**实装缺口（诚实标注）**：`TradeRecord`（metrics.rs:198）当前不携带 z——冻结**最小 plumbing**：在 π fill loop 开仓处记 `entry_z: Option<MuClass>`（χ 门放行的候选的 z），平仓时连同 `trade_pnls` 落桶。桶键用 4 元组投影 `(ℓ,bsp,δ,σ^H)`（与 A 的 full-z 报告分离——B 的归因目的是 P&L 可读性，非假设检验，用粗桶足够；若需 full-z 归因在 A 出结果后再评估）。
- **含浮盈 / 已实现双口径并列**：`trade_pnls_with_forced`（终点强平实现浮盈）与 `trade_pnls`（已实现）都报（231，编排者铁律「浮盈算上」）。

## B3. 差分（看结果前冻结）

1. **χ-gated vs 无 χ 基线**（μ̂ 选择器增量价值）：Δequity、Δmax_drawdown、ΔP&L —— μ̂ 门是否改善组合级风险调整后收益。
2. **与逐信号 μ̂ 判定的一致性**：B 的 z 桶 P&L 符号 vs A/s3 的逐桶 μ̂ 三态 —— 组合级盈亏是否与信号级 alpha 判定同向（背离则暴露 π 结构层的额外损益源，如 sizing/K_Θ/LexArgmin）。

---

## 5. 冻结常量表（看结果前锁定）

| 常量 | 值 | 来源 |
|------|-----|------|
| 置换种子 | `perm_test::PERM_SEED`=20260701 | 666 |
| 置换次数 | `perm_test::N_PERM`=200 | 666 |
| z_α | 1.645（单边 95%） | mu_estimator |
| 功效门 | `n_eff≥(1.645·CV)²`**逐桶 cv** | 667 / decontam.rs:115（阻塞③已满足） |
| A 桶键 | 完整 `MuClass` 7 维（`horizontal=Some`） | 本冻结 A1.1 |
| A UClass 并列键 | `(level_bucket,δ,role,divergence)` | mu_estimator.rs:182 |
| A 估计域 | BTC 单标的 walk-forward OOS（隔离 full-z） | 本冻结 A1.2 |
| B π 入口 | `run_theta_v0_pi_chi`（θ=0,z_α=1.645,empty=false） | runner.rs:287 |
| B 标的 | BTC 全历史 + CL | 任务 |
| B 基线 | `run_theta_v0_pi`（χ≡1） | runner.rs:272 |
| beta 剥离 | B̂=ĝ·h 因果扩张窗，per-symbol | s2/#82 |
| 成本 | `ThetaConfig::default()` 统一 | 本冻结 |
| 分层键 | `(ℓ,h_bucket,time_block,σ^H)` **不变** | perm_test.rs:95 |

## 6. 谱系依据 / 改 estimand 须新冻结

- **codex #85**（`codex-85-fullz-residual-ruling`）：三阻塞清单（perm_test 桶键扩维 / UClass 降维并列 / 功效门逐桶 CV）—— 本冻结逐条落地（①A3、②A3.4、③A1.3 确认已解决）。裁 C 的延后被编排者解除，但 codex §7 的「full-z 同质假设下降功效、异质剥离下个别子桶可翻」数学约束**继承**（A3.4 诚实预承诺）。
- **acc-alpha-estimand-prereg §5**：改 estimand（加维）须新冻结不得复用 —— 本冻结即履行（4 元组→7 维是加维）。
- **231 / formalization-validity-domain**：full-z=新 estimand 须新有效域；否定性结果（多数 INCONCLUSIVE）缩小边界，比 s3 4 元组更严格。有效域声明须与验证等大（A=L2 BTC 单标的，B=L2 BTC+CL）。
- **675**：残差数据走 z_of_candidate 生产路径（非 fork）；full-z 阻塞只在消费侧 perm_test。
- **663/665/666/667**：判据/去污/功效门。s3（`strict-alpha-retest`）：BTC 4 元组 INCONCLUSIVE 基线（A 的差分对象）。#13（`wverify-alpha-retest`，coarse-projection）：粗投影基线（A 的第二差分对象）。
- **full-strategy-pi-conformance**：π 已实装且被回测消费（`run_theta_v0_pi_inner`→χ 门），B 的入口依据。

## 7. 结果包六要素（本冻结）

1. **结论**：冻结 full-z×残差（A，桶键完整 MuClass 7 维，BTC 单标的，UClass 并列）+ 完整策略级 π 回测（B，`run_theta_v0_pi_chi` 走 walk-forward χ 门，BTC+CL，vs 无 χ 基线）的 estimand + 判据 + 输出规格，看结果前锁定。perm_test 扩维方案冻结（新增 full-z/UClass 函数，分层键不变，δ-派生 short_swing 从 perm-δ 重构，跨进程复现）；功效门逐桶 CV 现有代码已满足。
2. **定义依据**：可交易 μ(z)>0（663，alpha分离.pdf p1）；残差 Y_i=δ(H−B̂)−C（alpha分离.pdf §1 p1/§4.1 p5，不动摇）；桶键先验给定 z_of_candidate（675，非 fork）；full-z 7 维=MuClass 全分量（mu_estimator.rs:79）；功效门逐桶 CV（667/decontam.rs:115）。
3. **边界条件（翻转）**：(a) 若残差记录 `horizontal=None`（z_of_candidate 未填）⟹ full-z 退化为 4 元组无信息增量，诚实标注非静默；(b) 若 full-z 扩维破坏跨进程复现（分层键遍历序变）⟹ 实装违约，L1 自检必挡（红 demo）；(c) 若**某** full-z 子桶同时 n_eff≥(1.645·CV)²+CV 大幅低于 4 元组父桶+perm_p 未退化+walk-forward 稳定 ⟹ full-z 相对 4 元组产**信息增量**（codex #85 理论可能性坐实）；否则维持「full-z 同质降功效」预期；(d) B 若 χ-gated equity ≤ 无 χ 基线 ⟹ μ̂ 选择器在组合级无增量价值（照实入册，非隐藏）。
4. **下游推论**：A 产新 VALIDATED ⟹ full-z 是残差口径的更优 estimand，M1 推进；A 全 INCONCLUSIVE ⟹ 坐实 codex #85「4 元组是残差口径当前诚实 estimand」，full-z 仅作 oracle 上界。B 正 Δequity ⟹ μ̂ 门有组合级 alpha；B 非正 ⟹ π 结构层（sizing/K_Θ）损益需单独诊断。
5. **谱系引用**：见 §6（codex #85 / acc-alpha-prereg §5 / 231 / 675 / 663-667 / s3 / #13 / full-strategy-pi-conformance）。
6. **影响声明**：新增本预注册文件（L0/L1 规格）。阶段2 待改：`perm_test.rs`（**新增** full-z/UClass 函数，不改 4 元组现函数，1401 基线 bit-exact 护航）；`wverify_run.rs`（新增 full-z/UClass 逐桶报告入口）；`metrics::TradeRecord`（加 `entry_z` 最小 plumbing，B 的 z 桶归因）；π fill loop（开仓处记 entry_z）。**不改**：残差减法逻辑（`l3_delta_r_alpha`/`ResidualTrade::y`）、z_of_candidate、`decontam` 三态判据、分层键、`run_theta_v0_pi_inner` 主链、`acc-alpha-estimand-prereg`/`prereg-l3-cross-symbol` 冻结文件。护航：`cargo test --release --lib` 全绿基线不退化 + 新增 full-z/UClass 跨进程复现 L1 自检。
