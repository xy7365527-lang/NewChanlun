# 预注册：多重赋格回测（f3，goal g-20260703T0100Z-nesting-fugue 终点）

> 工位 swarm/ws-f3 | task #114 | 冻结先于跑数（本文件 commit 后方可跑数，acc f3-fugue-backtest 硬约束）
> 前情：f1（nesting-fugue-conformance-20260703）+ f2（f2-impl-20260703，overlay_net_delta/NestTrigger 落地）+ p3 基线（fullz-policy-backtest-20260702，冻结 9a19bb5775 / 跑数 1c977a5c23）
> 认识论预承诺：本回测是**结构回测**（多级别声部并行持仓的行为与账本量化），**非再证信号级 alpha**。全战役无 confirmed alpha（p3 坐实 V=0），f3 不预期翻转该结论——诚实预承诺 (b) 残差 μ̂ 大概率仍 INCONCLUSIVE。

---

## 0. 冻结对象（看结果前锁定）

三个量化对象 + 一个桶键定义 + 三态判据 + 功效门。跑数产出与本预注册逐条对照，偏离照实入册（231）。

- **A（结构，L0/L1）**：多声部并行持仓统计——同时活跃声部数分布 / ShortDiff 腿占比 / ‖ΔN‖_1 逐 bar 分布（overlay 净额可见性）。
- **B（信号级，L2）**：按桶残差 μ̂ 三态，桶键 = (σ_p × L0 bsp_class 压缩 × δ)，复用 s2 残差管线。
- **C（策略级，L2）**：π equity 对照——有 ShortDiff 声部 vs 剔除 ShortDiff 的反事实差分（多重赋格的增量价值）。
- **标的**：BTC 全历史 + CL（p3 同口径）。

---

## 1. 桶键定义 + δ-共线检查（acc 硬约束）

### 1.1 桶键

B 的分层置换与逐桶三态用**现有 4 元组引擎** `perm_test::stratified_delta_perm_p`（perm_test.rs:190），桶键三维：

| 维 | 语义 | 代码 | 值域 |
|----|------|------|------|
| σ_p | 父声部方向（σ^H=parent_dir，根声部=0） | `MuClass::parent_dir` | {−1, 0, +1} |
| L0 bsp_class 压缩 | 买卖点结构类别（压缩版，非完整 i_class） | `MuClass::bsp_class()` | 压缩位集 |
| δ | 仓位方向（买=+1/卖=−1，被检验的 Z 维） | `MuClass::delta` | {−1, +1} |

**关键**：level 冻结 L0（下沉入场落点，f2 通道恒 L0），故桶键退化为 (σ_p, bsp_class压缩, δ) 三维——与 p3 的 s3 4 元组（含 level 维）在 L0 切片上一致，**零新增引擎**（ponytail：复用而非新建）。

### 1.2 δ-共线检查（为何本桶键不重演 full-z 的 i_class⊥δ 退化）

p3 §3.3 坐实的退化机制：full-z 的**聚合基**含完整 `i_class`（`stratified_delta_perm_p_fullz` base=`(level,i_class,parent_dir,position,horizontal)`，perm_test.rs:215），而 buy3=`i_class 4`、sell3=`i_class 32` 是**不同 i_class** ⟹ 分入不同聚合基桶 ⟹ 每桶内只有单一 δ 方向 ⟹ δ-置换检验结构性退化（主 root buy 子桶 perm_p 从 0.005 退化到 1.000，n=1612 大样本仍退化，根因是 i_class⊥δ 共线非样本量）。

本桶键**结构性免疫**该退化，逐维论证：

1. **聚合基 δ-free**（防退化的机制核心）：`stratified_delta_perm_p` 的聚合基 `base_of=|c|(c.level, c.bsp_class(), c.parent_dir)`（perm_test.rs:199）**不含 δ**。δ 仅进 `out_key`（reporting，行 200），不进聚合基。故每个聚合基桶内 δ 双向共存，置换非平凡。

2. **bsp_class 压缩方向对称**：`bsp_class()`（压缩版）把 buy3 与 sell3 **映到同一 bsp_class 值**（对照 full i_class 的 4 vs 32 分裂）。故聚合基桶 (level0, bsp_class=type3, parent_dir) **同时含 buy3(δ=+1) 与 sell3(δ=−1)** 两方向——这正是 p3 中 4 元组主桶 perm_p 保住 0.005（有方向对比）而 full-z 主子桶退化 1.000（buy3-only 无对比）的机制差。压缩版 ⟹ 方向对比保留 ⟹ 非退化。

3. **σ_p 三值非方向决定性**：σ_p=parent_dir 是**父声部**方向，不决定子信号的 δ——父升声部可同时托管买信号（δ+1，同向下沉）与卖信号（δ−1，短差/反手）。故按 σ_p 分层不塌缩 δ（对照：若某维 v 满足「v 固定 ⟹ δ 固定」则 v⊥δ 共线，σ_p 不满足此条件，∵ σ_p∈{−1,0,+1} 每值下 δ 双向）。分层键 `(ℓ,h_bucket,time_block,σ^H)`（perm_test.rs:68）含 σ^H，层内仍双 δ。

**检查结论**：三维皆不引入 i_class⊥δ 共线 ⟹ δ-置换非退化 ⟹ 桶键**过共线检查**。跑数时以主桶 perm_p 是否退化到 1.000 作**事后验证锚**（若退化则本检查证伪，照实上浮）。

### 1.3 赋格分解维（signal-provenance，非分层键）

额外报告维（不入置换分层，仅逐桶归因）：`NestTrigger`（econ_positive.rs:1046，三值 Type1TrendDivergence/Type23SublevelType1/XiaoZhuanDa）。用于回答「三类买卖点通道各自的信号质量」——纯质量对照（L2 含选择偏差，非 OOS alpha），复用 `acc_nest_trigger_quality_probe`（econ_positive.rs:4185）。

---

## 2. A：多声部并行持仓统计（L0/L1）

**量化对象**（跑数逐 bar 从生产 π 链采集，BTC+CL）：

| 指标 | 定义 | 代码锚 | 等级 |
|------|------|--------|------|
| 同时活跃声部数分布 | 逐 bar `active_legs.len()` 直方图 | coverage.rs 生产步 `next_active` | L0（结构计数） |
| ShortDiff 腿占比 | Σ`ShortDiff` 腿 / Σ全腿 | `Vertical::ShortDiff`（voice.rs） | L0 |
| ‖ΔN‖_1 逐 bar 分布 | Σ_t\|ΔN_t\|，ΔN_t=`overlay_net_delta(legs)` | coverage.rs:1290 | L0（净额可见层） |

**判据（可证伪）**：
- **净额可见性**：若 ∀t ‖ΔN_t‖=0 ⟹ depth>0 子声部**不改净头寸**（多空对冲.pdf b2「净额不可见」）⟹ overlay 账本无经济意义，C 的 ShortDiff 反事实差分预期 ≈0。若 ∃t ΔN_t≠0 ⟹ 子声部真改净头寸，C 差分可非零。
- **并行度非平凡**：若活跃声部数分布恒 ≤1 ⟹ 「多重赋格」在生产链**未实际并行持仓**（退化为单声部），A 证伪多声部并行前提，C/B 的赋格增量归零并照实入册。

**边界条件**：A 是结构算术（L0），无 alpha 断言。‖ΔN‖_1 用于真实数据 depth>0 净头寸检验时升 L2（本工位承载）。

---

## 3. B：按桶残差 μ̂ 三态（L2）

**管线**：s2 残差口径 `Y_i=δ·(H−B̂)−C` 不动摇（`ResidualTrade`），走 `z_of_candidate` 生产路径（horizontal=Some，675 非 fork）→ `stratified_delta_perm_p`（§1.1 桶键）→ `decontam` 三态判据（VALIDATED/FALSIFIED/INCONCLUSIVE）+ 逐桶功效门 `n_eff ≥ (1.645·CV)²`（decontam.rs:115，667 号）。复用 `wverify_run::verdict_by` 泛化三态。

**跑批**：`cargo test --release --lib theta_v0::backtest::wverify_run::<f3 入口> -- --ignored`（BTC 全历史 walk-forward OOS + CL）。

**判据（三态，可证伪）**：
- 逐桶：VALIDATED ⟺ n_eff≥功效门 ∧ perm_p<0.05 ∧ LCB>0 **同时**；否则 FALSIFIED（LCB 上界<0 且 powered）或 INCONCLUSIVE（欠功效/未达显著）。
- 全局：任一桶 VALIDATED ⟹ 该桶报可交易 alpha；全桶无 VALIDATED ⟹ 维持 p3 「无 confirmed alpha」。

**诚实预承诺**：p3 主桶（L0/type3/买）欠功效卡 INCONCLUSIVE（n_eff≈167<门 290），f3 用同标的同口径，**不预期**翻 VALIDATED；σ_p 细分只会进一步碎裂样本降功效。B 的价值 = 赋格分解维（NestTrigger）下的质量图景 + 主桶 perm_p 非退化的事后共线验证，**非**新 alpha。

**赋格分解**：按 NestTrigger 三通道各自报 (n, μ̂)，验证 codex-f2 #1 裁决「Xzd 通道不受高级别背驰门一票否决」——若 Xzd/Type23 桶 μ̂ 非负/可观 ⟹ 单一 bool 背驰门误杀有质量信号（L2 证据）。

---

## 4. C：π equity ShortDiff 反事实对照（L2）

**对象**：完整策略 π 回测（BTC+CL，单折有界 train→OOS，p3 §4 同配置：χ门 `run_theta_v0_pi_chi`，成本 `ThetaConfig::default()`），两口径差分：

- **口径1（全赋格）**：含 ShortDiff 反向子声部腿（多空对冲.pdf H_t=−σ_parent·h_t）。
- **口径2（反事实）**：剔除 ShortDiff 腿（`Vertical::ShortDiff` 声部禁用，最小 harness flag）。

**度量**（well-defined 净账本模型，p3 §4.1 诚实局限继承——不做假单-z 归因）：Σpnl(含浮盈) / max_dd / n_orders / strat_return，逐标的报 口径1−口径2 差分。

**判据（可证伪，多重赋格增量价值）**：
- ΔΣpnl(口径1−口径2)>0 ∧ Δmax_dd≤0 ⟹ ShortDiff 声部（多重赋格对冲）有组合级增量价值。
- ΔΣpnl≤0 ⟹ 无正增量，照实入册（多重赋格在当前 estimand 下不产净收益）。
- 若 A 判 ‖ΔN‖_1≡0 ⟹ 预期 C 差分≈0（净额不可见，自洽交叉验证 A↔C）。

**诚实预承诺**：p3 两口径均净负（χ门减损但不转正）。C 测的是「ShortDiff 声部**相对**全赋格基线的增量」，非绝对盈利——增量可正（减损/减回撤）而绝对仍负。

---

## 5. 硬约束 + 与 p3 差分

- **残差口径不动摇**：Y_i=δ(H−B̂)−C，不改减法。
- **零生产改动优先**：B 全复用现有引擎（`stratified_delta_perm_p`/`decontam`/`verdict_by`，零新增）。A 复用 `overlay_net_delta`（f2 已落）。仅 C 口径2 需最小 harness flag（禁 ShortDiff 声部）+ A 需逐 bar 采集器——两者若须改 harness，最小 diff + L1 自检 + 护航 1407 基线（bit-exact 现有测试）。
- **p3 差分锚**：B 对 p3 fullz-policy §3（52 桶 INCONCLUSIVE）；C 对 p3 §4（χ门 vs 无χ，1c977a5c23）。f3 新增维 = σ_p 分层 + NestTrigger 分解 + ShortDiff 反事实。
- **否定性结果照实入册**（231）：A 退化（单声部/‖ΔN‖≡0）、B 无 VALIDATED、C 差分≤0 均照实，不美化。

---

## 6. 认识论等级

| 对象 | 等级 | 有效域 | 否证性 |
|------|------|--------|--------|
| A 并行持仓统计 | L0/L1 | 结构算术，BTC+CL 生产链 | 可否证多声部并行前提（活跃度≤1）/净额可见性（‖ΔN‖≡0） |
| B 残差 μ̂ 三态 | L2 | BTC 单标的 walk-forward OOS + CL；残差口径；(σ_p,bsp压缩,δ) 桶键 | 可否证（三态判据两分支合法收口） |
| C π ShortDiff 反事实 | L2 | 单折有界 train（逐窗 walk-forward 是升级路径） | 可否证（ΔΣpnl 符号） |

判据/去污逻辑本身 L0。

---

## 7. 谱系引用

- p3（fullz-policy-backtest-20260702，冻结 9a19bb5775 / 跑数 1c977a5c23）：B/C 差分对象；§3.3 i_class⊥δ 共线退化机制（§1.2 检查依据）；§4.1 净账本归因诚实局限（C 继承）。
- f2（f2-impl-20260703，f7c60de5da）：`overlay_net_delta`（A P0-3）、`NestTrigger`（B 分解维 P0-1）；codex-f2-design-ruling（#1 Xzd 不受一票否决）。
- 674 号：R/TW 不同构，overlay 第三范畴（A 只读诊断，不建 OverlayState）。
- 663/665/667：decontam 三态/去污/功效门。675：残差走 z_of_candidate 生产路径。
- 231/formalization-validity-domain：A/B/C 认识论标注 + 否定性结果缩小有效域。
- memory（oddeven_mu_identity / gap3）：桶键×δ 共线自毁的先例警示（§1.2 检查动机）。

## 8. 影响声明

本预注册冻结 f3 三对象（A/B/C）+ 桶键 + δ-共线检查 + 三态判据 + 功效门。跑数阶段（f3-fugue-backtest-20260703.md）逐条兑现。预期代码改动：C 口径2 的 ShortDiff 禁用 flag + A 逐 bar 采集器（最小 harness，护航 1407）；B 零改动。冻结后不改本文件（改则谱系断裂）。
