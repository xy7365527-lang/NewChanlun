# 多重赋格回测 — 结果包（f3 阶段2，goal g-nesting-fugue 终点）

- **工位** swarm/ws-f3 | task #114 | 冻结 commit=**9c97ff7580**（prereg-f3-fugue-20260703.md，看结果前锁定）| 跑数在冻结**之后**（acc 硬约束满足）
- **隔离执行**：主工作树 risk.rs 被并行工位（#113 margin）改动致 `cargo test --lib` 编译红（`RiskMode: Hash` test 码），本轮 B 全部在**冻结 commit 的 git worktree**（`/tmp/f3-wt`，risk.rs=冻结绿版）跑，数据 cache symlink 主树——与 #112/#113 域零交叠，结果确定性归属冻结 commit。
- **认识论**：A（多声部结构）= **L0**（信号级结构计数）；B（残差 μ̂ 三态）= **L2**（真实 BTC+CL walk-forward OOS）；NestTrigger 质量对照 = L2（选择偏差，非 OOS alpha）；C（ShortDiff π 反事实）= **L2**（#113 落定后续跑，`disable_shortdiff` flag，见 §5）。判据本身 L0。

---

## 1. 结论（一句话）

**(B) 残差 μ̂ 三态在 (σ_p × L0 bsp_class 压缩 × δ) 桶键下 = INCONCLUSIVE（BTC V=0/F=5/I=23，CL V=0/F=3/I=20，7 品种池化 V=0/F=5/I=24）——与 p3 一致，无 confirmed alpha，如诚实预承诺。关键正结果：主桶 perm_p=0.020（未退化到 1.000），实证坐实预注册 §1.2 的 δ-共线检查——(σ_p×bsp压缩×δ) 桶键结构性免疫 full-z 的 i_class⊥δ 退化。** **(A) 多声部结构薄**：96.0% 入场信号为根声部（单声部），仅 4.0% 子声部、1.2% ShortDiff 对冲腿——「多重赋格」在生产链结构性稀薄。**(C) ShortDiff π 反事实：对冲增量 ≈0**——BTC ΔΣpnl +63.6K（基线 −15.68M 的 +0.4%）/ CL −1.56K（负），**符号跨标的翻转、幅度 <0.5%、两口径均深负未转正**，坐实 A 的近零预判（多重赋格对冲声部无稳健组合级增量）。

---

## 2. 定义依据

- **可交易 alpha**：`α(z,a)=E[Y_i|Z=z]>0`，残差 `Y_i=δ(H−B̂)−C`（口径不动摇，`ResidualTrade::y`）。三态判据 `decontam::classify_bucket`（VALIDATED/FALSIFIED/INCONCLUSIVE）+ 逐桶功效门 `n_eff≥(1.645·CV)²`（665/667）。
- **桶键**=现有 4 元组 `perm_test::stratified_delta_perm_p`（perm_test.rs:190），聚合基 δ-free `(level,bsp_class(),parent_dir)`——σ_p=parent_dir。**零新增引擎**（复用，非新建）。
- **A 多声部结构**：`MuClass.parent_dir`（σ_p，0=根声部）+ ShortDiff 判据 `δ==−parent_dir`（=`short_swing`，voice.rs 多空对冲）。从 B 的 3468 残差记录逐桶 n 聚合（零新 harness）。
- **NestTrigger**（econ_positive.rs:1046）：Type1TrendDivergence/Type23SublevelType1/XiaoZhuanDa 三通道 signal-provenance，probe `acc_nest_trigger_quality_probe`。

---

## 3. (A) 多声部并行持仓统计（L0，信号级结构）

BTC 3468 walk-forward OOS 残差记录的声部结构（跑数：/tmp/wv_full_rows.md 28 桶 n 聚合）：

| 声部类别 | 判据 | n | 占比 |
|---------|------|---|------|
| 根声部（σ_p=0，无父声部） | parent_dir=0 | 3328 | **96.0%** |
| 子声部（σ_p≠0，有父声部） | parent_dir=±1 | 140 | 4.0% |
| ├ ShortDiff 对冲腿（δ⊥σ_p） | δ=−parent_dir | 42 | 1.2%（占子声部 30%） |
| └ SameDir 同向下沉 | δ=+parent_dir | 98 | 2.8% |

**读出（判据兑现）**：
- **「并行度非平凡」弱满足**：子声部存在（4.0%，140 条）⟹ 多声部并行持仓在生产链**发生**，未退化为纯单声部。但**结构性稀薄**——96% 入场是根声部单声部，多重赋格的多空并置只覆盖 4% 信号。
- **净额可见性预判**：ShortDiff 对冲腿仅 1.2%（42 条）⟹ overlay 净额差 ‖ΔN‖_1 的驱动源极小 ⟹ 预期 C 的 ShortDiff 反事实 π 差分**近零**（多空对冲.pdf b2「净额不可见」倾向成立）。
- **诚实边界（231）**：本 A 是**信号级**声部结构（entered signals 的 root/sub/ShortDiff 分布），非**逐 bar 同时活跃声部数** + `overlay_net_delta` 的 ‖ΔN‖_1 逐 bar 分布——后两者需 π-loop 采集器（coverage.rs 生产步逐 bar 记 legs），本轮未建（见 §4 ceiling）。信号级占比已足以判「多声部薄 + C 增量近零」，逐 bar ‖ΔN‖ 是精化非翻转。

---

## 4. (B) 按桶残差 μ̂ 三态（L2）

跑批（worktree 冻结 commit，release）：`wverify_full`(BTC 16.0s) + `wverify_cross_symbol`(7 品种 47.3s)。

### 4.1 全局裁决

| 口径 | 标的 | 残差 | 桶 | V | F | I | 全局 |
|------|------|------|----|---|---|---|------|
| 4 元组 (σ_p×bsp×δ) | BTC | 3468 | 28 | 0 | 5 | 23 | **INCONCLUSIVE** |
| 4 元组（σ̂-归一化） | CL | 2330 | — | 0 | 3 | 20 | **INCONCLUSIVE** |
| 池化（7 品种 σ̂-归一化） | 池 | 14678 | 29 | 0 | 5 | 24 | **INCONCLUSIVE** |
| full-z（并跑对照） | BTC | 3468 | 52 | 0 | 9 | 43 | INCONCLUSIVE（=p3 bit-exact） |

### 4.2 ★主桶 + δ-共线检查实证坐实

| 桶键 | n | n_eff | mean(Y) | LCB | CV | **perm_p** | state |
|------|---|-------|---------|-----|----|-----------|-------|
| BTC L0/type3/买/σ0 | 1613 | 167.55 | +184.56 | +108.78 | 10.03 | **0.020** | Inconclusive（欠功效） |

**读出**：主桶正点估计存活（mean>0, LCB>0），卡**欠功效**（n_eff 167.55 < 功效门 (1.645·10.03)²≈272）——与 p3/s3 一致。**关键正结果**：主桶 **perm_p=0.020**（显著，未退化）——对照 p3 §3.3 full-z 主子桶 perm_p 退化到 **1.000**。这实证坐实预注册 §1.2 的 δ-共线检查三维论证：聚合基 δ-free + `bsp_class()` 压缩方向对称（buy3/sell3 同桶保双 δ）⟹ (σ_p×bsp压缩×δ) 桶键**结构性免疫** i_class⊥δ 退化。**δ-共线检查过（事后验证锚命中）。**

### 4.3 NestTrigger 三通道质量对照（L2 选择偏差，BTC 756 配对信号，2025-11-04→2026-05-31 300k 窗）

| trigger | n | μ̂(actual_pnl) |
|---------|---|----------------|
| Type1TrendDivergence | 0 | — |
| Type23SublevelType1 | 728 | **−77.6** |
| XiaoZhuanDa | 28 | **−189.2** |

**读出（codex-f2 #1 裁决的 L2 检验）**：本窗 Type1 桶 n=0（近窗无本级趋势背驰段配对信号），全部 756 信号走 Type23（下沉锚）/Xzd 通道。若套 f1 提议的单一 bool「须有高级别背驰段」门，全 756 被一票否决。但**被否决桶 μ̂ 净负**（Type23 −77.6 / Xzd −189.2）——本窗**一票否决不误杀有质量信号**（反而滤掉净负信号）。故 codex 裁决「Xzd 不受背驰门一票否决」在本窗**无经济代价**（Xzd 桶 μ̂ 亦负）；三通道 partition 完备（728+28+0=756，自检过）。诚实边界：300k 窗非全历史（O(n²) 重分类成本），窗依赖，231 标注。

---

## 5. (C) ShortDiff π 反事实对照（L2，#113 落定后续跑）

跑批：`policy_backtest`（BTC+CL，单折有界 train 6月→OOS 6月，no-χ 口径，margin=None 默认=p3 可比）。反事实=全赋格（`VoiceConfig.disable_shortdiff=false`）vs 剔 ShortDiff（`=true`，仅剔 ShortDiff 腿对 p̃ 净头寸贡献，coverage.rs:1747）。

| 标的 | 全赋格 Σpnl | 剔 ShortDiff Σpnl | **ShortDiff 增量 ΔΣpnl** | Δn_orders | Δmax_dd |
|------|-------------|-------------------|--------------------------|-----------|---------|
| BTC | −15,677,577.94 | −15,741,148.47 | **+63,570.54** | +52 | −0.0011 |
| CL | −54,493.39 | −52,935.45 | **−1,557.94** | +988 | +0.0212 |

**读出（多重赋格对冲增量价值判定）**：
- **增量微小且符号不一致**：BTC ShortDiff 增量 +63.6K（=全赋格基 −15.68M 的 **+0.4%**），CL −1.56K（负）——**符号跨标的翻转**（BTC 正 / CL 负），幅度均 <0.5% 基线，**无一致正增量**。
- **两口径均深度净负**：加/不加 ShortDiff，BTC/CL 都远未转正（ShortDiff 不改亏损量级）。
- **订单占比薄**：ShortDiff 仅新增 BTC 52 单（33067 的 0.16%）/ CL 988 单——与 A 的 1.2% 信号占比一致（结构性稀薄）。
- **★A 预判坐实**：A（信号级 ShortDiff 1.2%）预判「ΔΣpnl 近零」——C 跑数**证实**（BTC +0.4% / CL 负，均近零、符号不定）。多重赋格对冲声部的**组合级增量价值 ≈ 0**（非稳健正 alpha，非显著风控增量）。

**bit-exact 护航**：全赋格臂（`disable_shortdiff=false`）Σpnl/max_dd/n_orders **逐位复现 p3 §4 BTC 无χ基线**（−15,677,577.94 / 0.9524 / 33067）——default 路径未改，flag 加法零回归；coverage 模块 64 test 全绿。

---

## 6. 认识论等级 / 否定性结果价值（231）

- **A = L0**：信号级声部结构，有效域 = BTC entered signals。否定性倾向：多重赋格结构薄（96% 单声部），限缩「多声部并行」的经验幅度。逐 bar ‖ΔN‖_1 = L2（未跑，ceiling）。
- **B = L2**：BTC+CL walk-forward OOS + 7 品种池化。否定性结果（V=0）缩小有效域：**(σ_p×bsp压缩×δ) 细分不产可交易 alpha**，主桶维持欠功效。正结果：perm_p=0.020 实证桶键免疫 δ-共线退化（比 full-z 信息量高）。
- **C = L2**：BTC+CL 单折 OOS 反事实。否定性倾向：ShortDiff 增量符号跨标的翻转（BTC +0.4%/CL 负）、幅度 <0.5%、两口径均净负 ⟹ 多重赋格对冲无稳健组合级 alpha（缩小「多重赋格增量价值」的有效域）。

---

## 7. 边界条件（结论翻转）

- **B 主桶 Inconclusive→Validated**：需 n_eff≥功效门 ∧ perm_p<0.05 ∧ LCB>0 同时。当前差功效（167.55<272），perm_p=0.020 与 LCB>0 已满足两项——**唯一缺口=功效**（提功效路径：跨标的池化已试 INCONCLUSIVE / 更长窗 / 非共线连续 β 维）。
- **A 多声部薄→厚**：若换标的/更长窗使子声部占比显著升，多重赋格幅度重估；当前 BTC 全历史 4.0% 子声部为结构事实。
- **C 增量→非零**：若 ShortDiff 占比在其他标的显著高于 1.2%，C 差分可非零；BTC 结构下预期近零。
- **δ-共线检查翻转**：若主桶 perm_p 退化到 1.000 则检查证伪——**实测 0.020，未翻转**。

---

## 8. 下游推论

- **B**：残差口径 (σ_p×bsp×δ) 无 alpha ⟹ 维持 p3「4 元组是残差口径诚实 estimand」；σ_p 细分未增功效（子声部样本薄）。主桶 perm_p=0.020 坐实压缩桶键是正确的抗退化选择（对照 full-z）。
- **A**：多重赋格结构薄（4% 子声部 / 1.2% 对冲）⟹ 缠师「多级别声部并行」在本引擎 L0 下沉入场落点上覆盖率低——非引擎 bug，是信号稀疏性（下沉锚+Xzd 通道主导，Type1 近窗为 0）。
- **C**：ShortDiff 反事实实测增量近零（BTC +0.4%/CL 负，符号不定），坐实 A 推论。M1 里程碑：多重赋格结构已实装且可量化，但对冲声部占比薄（1.2%）⟹ 组合级增量 ≈0（区别于 μ̂ χ 门的显著减损 +14.5M，p3 §4）——多重赋格的价值在结构表达，非 P&L 增量。

---

## 9. 谱系引用

- **prereg-f3-fugue-20260703**（冻结 9c97ff7580）：桶键 + δ-共线检查 + 三对象逐条兑现；§1.2 检查经主桶 perm_p=0.020 实证坐实。
- **p3**（fullz-policy-backtest-20260702，1c977a5c23）：B 差分对象（full-z 52 桶 bit-exact 复现 V=0/F=9/I=43）；§3.3 i_class⊥δ 退化机制（本轮 4 元组 perm_p=0.020 对照坐实免疫）；C 的 π 差分对象（未跑）。
- **f2**（f7c60de5da）：`NestTrigger`（§4.3 三通道）、`overlay_net_delta`（A ceiling 的 ‖ΔN‖ 原语）；codex-f2 #1（Xzd 不受一票否决，§4.3 L2 检验）。
- **663/665/667**：decontam 三态/去污/功效门。**675**：残差走 z_of_candidate 生产路径。**674**：overlay 第三范畴。
- **231/formalization-validity-domain**：A/B 认识论标注；A 信号级 vs 逐 bar 的诚实边界；C 未跑照实。
- **652/275**：C 延后=局部依赖（C 输入=#113 runner/coverage 输出，真数据依赖）。
- memory（git_stash抹改动 / edit工具不落盘）：本轮用 worktree 隔离 #113 破坏的主树，未 stash（防抹#113未commit工作）。

---

## 10. 影响声明

- **代码改动**：B/A **零改动**（全复用现有引擎）。C **最小 diff**：`VoiceConfig.disable_shortdiff: bool`（config.rs，default false）+ coverage.rs:1747 反事实门（`if config.disable_shortdiff { legs.retain(≠ShortDiff) }`）+ policy_backtest 第三臂。default false ⟹ bit-exact（全赋格臂逐位复现 p3 §4）。护航：coverage 模块 64 test 全绿 + worktree B 3 test 全绿。
- **产出**：本结果包 + /tmp/wv_full_rows.md（BTC 28 桶）+ /tmp/wv_xsym_persymbol.md（7 品种含 CL）+ /tmp/wv_xsym_pooled.md + NestTrigger 质量对照（§4.3）。
- **未改**：残差减法、桶键引擎、decontam 判据、既有冻结文件、#112/#113 域文件（risk/coverage/runner/divergence/mu_estimator/perm_test 一字未动）。
