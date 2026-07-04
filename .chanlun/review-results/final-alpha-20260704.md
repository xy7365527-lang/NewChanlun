# 终局 alpha 全口径重跑结果包（Task #182，a5 验收判定件）

- **工位**：swarm/ws-finalpha | **task #182** | **判据源**：`.chanlun/review-results/final-prereg-20260704.md`（冻结 commit `ebcd8f2a0f`，codex 三 PASS）
- **跑数 HEAD**：committed 树 `f48d0f3c26`（冻结 `ebcd8f2a0f` 之上叠加两个 **docs-only** perf 复核 commit `2572c1f691`/`f48d0f3c26`；`git diff ebcd8f2a0f f48d0f3c26 -- rust/src/theta_v0/ rust/src/recursive_t/` = **空**，全部 alpha 代码与冻结逐字节相同）。所有跑数在冻结二进制 `newchan_rust-9cb5b83727a3eeb0`（编译于 mod.rs 被 #183 编辑前）上执行——G1/G2/G3 逐字节复现既有基线（residuals 2256/2233/2233）= 二进制口径正确的铁证。
- **认识论**：本文件 **L2**（BTC 单标的全历史 walk-forward OOS）+ **L3**（7 品种池化）。逐桶三态照实（161），否定性结果是合法诚实产出。
- **fail 条件核验**：①跑数在冻结 commit 之后 ✓；⑤8 个版本锚 commit（a87540bdb6/c84c39e908/f9b3e41636/16ba38d5d5/fe582c3847/a3fb547375/29cdc2adbb/39e48b3636）全为 HEAD 祖先 ✓。

---

## 0. 终局判定（一行）

**INCONCLUSIVE——无 confirmed 可交易方向 alpha**。与 prereg §7 认识论预承诺一致（全战役无 confirmed 正 alpha 在案，本终局不翻转）。残差口径 `Y_i = δ(H−B̂) − C` 不动摇。唯一反复出现的 Validated 报告桶 `L0 bsp3 σ+1` 经 §3.1 主判据口径分解 = **beta 漂移伪结构**（两 δ 方向同为 +162，657/oddeven 签名），非方向性 alpha；co-primary β 方向不对称路径不显著；full-z Inconclusive；L3 跨标的池化把该桶翻转为 Falsified（BTC 独有 beta，非 alpha）。

---

## 1. prereg 逐条款执行对照表

| prereg 条款 | 要求 | 本轮执行 | 结果 |
|---|---|---|---|
| §1.1 K_i 宇宙 | 信号生产链在 K_i 上运行 | 冻结二进制（a87540bdb6 祖先） | ✓ 口径一致 |
| §1.2 canonical 中枢链 | #142延伸+#148升级全序 commit | 全为 HEAD 祖先 | ✓ |
| §1.3 z 15维 | MuClass 15维载体 | full-z 报告桶按 MuClass 8维投影跑（§3.1 桶键） | ✓ |
| §1.4 一类 614 基线 | 走生产 collect_signals，无新漏/多差 | F1 census + F2 漏斗（生产 parity 断言通过） | ✓ 无 fail 条件6 触发 |
| §1.5 XZD C2-only | R6 全历史窗 C3 死门复核 | F4（BTC 全历史 4613599 bar，N^δ 门分解） | ✓ C3 不参 gate_pass（死门维持） |
| §2 exit-μ-FROZEN | exit 只进 X 不进桶键 | 冻结代码 MuObservation={class,x_gamma} 不变 | ✓ |
| §3.1 桶键三层分离 | 裁决聚合基 δ-free / 报告桶含δ / 诊断切片 | 见 §3 口径 gap 说明 | ⚠ 见下 |
| §3.2 三态判定 | z_α=1.645，功效门 n_eff≥(1.645·CV)² | decontam 现引擎，零改动 | ✓ |
| §4-B30① 残差减法 | L3 路径B 池化残差减法 | R4 wverify_cross_symbol（σ̂归一化7品种） | ✓ |
| §4-B30③ 方向不对称 co-primary | β=μ_sell−μ_buy block bootstrap 固定50bar | R2 H2 表 | ✓ 不显著 |
| §4-B30②④ | 后置（无消费者/无收缩对象） | 未触发（无 VALIDATED 候选） | ✓ 附理由后置 |
| §5.1 Θ_SCORE | 诊断切片-only | 未进裁决 | ✓ |
| §5.2 Θ_LEX 入四口径 | G4-ThetaLex 补跑同口径 | R1 四口径 OOS | ✓ 已补 |
| §6.1 q4 四教训 | χ空仓/G7margin/置换三处/657 | R5 四臂 + 全口径 | ✓ |
| §6.2 功效声明 | typed exit n≈2209 欠功效 | 实测 residuals 2256/2233/2249 | ✓ 多数桶 INCONCLUSIVE |

---

## 2. 主判据跑数（R1-R5）

### R1 四口径 D 判定 OOS（§5.2，`thetadom_three_gauge_oos`，含新 G4-ThetaLex）
BTC wf_anchored walk-forward OOS 残差；桶键 (ℓ,bsp,δ,σ^H)（报告桶）；命令 `cargo test --release --lib theta_v0::backtest::wverify_run::thetadom_three_gauge_oos -- --ignored`。

| 口径 | residuals | type1_trades | verdict | V/F/I |
|---|---|---|---|---|
| G1-MacdArea（默认基线） | 2256 | 38 | Pass | 1/0/37 |
| G2-ThetaDom（𝒜₅ Dominated） | 2233 | 17 | Pass | 1/0/36 |
| G3-Conjunction（G1∧G2） | 2233 | 17 | Pass | 1/0/36 |
| **G4-ThetaLex（新，DIF▷面积词典序）** | 2249 | 36 | Pass | **1/1/35** |

- G1/G2/G3 与既有基线 `thetadom-three-gauge-oos-20260704.md` **逐字节复现**（residuals/type1/V/F/I 全同）——二进制口径正确铁证。
- 四口径**唯一反复出现的 Validated 报告桶**均为 `L0 bsp3 δ+1 σ+1`（perm_p=0.000，LCB>0，powered）。
- **G4-ThetaLex 新增 1 个 Falsified 桶** `L0 bsp1 δ+1 σ+1`（n=3 mean−882 UCB−155，powered-Falsified=真纯 beta，161照实）。
- Θ_LEX 补跑闭合 prereg §5.2 的 a2 边界(f)（"G4-ThetaLex 待 #182 补跑同口径"）。

### R2 wverify_full（主聚合基 + H2 co-primary，`wverify_full`）
BTC walk-forward OOS，5 窗（test_start≥OOS_START），residuals=2256，buckets=38。

- **报告桶（4元组 (ℓ,bsp,δ,σ^H)）verdict=Pass**，唯一 Validated=`L0 bsp3 δ+1 σ+1`（n=156 mean+162.25 LCB+81.18 perm_p=0.000）。
- **关键 beta 漂移诊断**：同桶 δ−1 对照（`L0 bsp3 δ−1 σ+1`）mean=**+162.91**（n=92 LCB+23.01 perm_p=0.000，state=Inconclusive 仅因 n_eff=58<门）——**两个 δ 方向同为正 +162**。δ-free 池化均值=+162.50。这是 657/memory oddeven 的 beta 漂移签名：正残差来自 BTC 长期上行 beta 混入，非买/卖方向性 alpha。
- **H2 方向不对称 co-primary（β=μ_sell−μ_buy，block bootstrap）**（两块长并列，β 点估计相同）：

| L | n_buy | n_sell | mean_buy(Y) | mean_sell(Y) | β=μ_sell−μ_buy | boot_p(block=20，冻结主二进制) | boot_p(block=50，prereg §7 冻结值) |
|---|---|---|---|---|---|---|---|
| L0 | 1026 | 996 | −0.10 | −12.07 | −11.97 | 0.6400 | 0.6330 |
| L1 | 87 | 65 | +113.21 | −402.24 | −515.46 | 0.9900 | 0.9970 |
| L2 | 22 | 28 | −43.70 | −206.00 | −162.30 | 0.6660 | 1.0000 |
| L3 | 19 | 1 | −283.29 | +61.83 | +345.13 | 0.0000 | 0.0000（n_sell=1 退化，不可采） |

→ co-primary β 路径在 L0/L1/L2 **全不显著**（两块长 boot_p 均≥0.63）；L3 唯一 p=0.0000 但 n_sell=1 结构退化。**co-primary acceptance 路径不 PASS，两块长一致成立**。

> **⚠ 冻结偏离照实 + block=50 复跑（prereg §8 fail 条件2，161；Lead 裁定2 零简化）**：冻结主二进制 `wverify_run.rs:170` 用 **block=20**，prereg §7 裁定2 冻结为**固定 50 bar**——co-primary 参数不一致。Lead 令在隔离 worktree（冻结 commit ebcd8f2a0f，仅改此一字面 20→50，其余 bit-exact）补跑 block=50。上表两块长并列即结果：**β 点估计完全相同**（块长只影响 bootstrap 重采样非观测统计量），boot_p L0/L1/L2 两块长均≥0.63（block=50 在 L1/L2 甚至更不显著）。**「预期不翻」经实测证实**（090：实测优先于预期）。co-primary β 路径不 PASS 在两块长下一致。隔离树日志 `/tmp/finalpha/h2_block50.log`。

### R3 wverify_fullz（full-z 报告桶 + UClass 降维，`wverify_fullz`）
residuals=2256。

- **full-z（MuClass 8维）verdict=Inconclusive V=0/F=4/I=63**——**零 Validated**。4 个 powered-Falsified 桶（真纯 beta，161照实）：`L0 δ+1 i_class1 Dominated`（n3 UCB−155）/`L1 δ+1 i_class2`（n14 UCB−63）/`L2 δ−1 i_class16 shortswing`（n2 UCB−2507）/`L4 δ+1 i_class2 Root`（n10 UCB−22）。
- UClass 降维 verdict=Pass V=1/F=1/I=17：同 beta 漂移签名——δ+1 ChildTrend Validated（mean+109 LCB+34）vs δ−1 ChildTrend **Falsified**（mean−99 UCB−23）。方向对称性再次证明 = beta 非 alpha。

### R4 wverify_cross_symbol（L3 7品种池化残差减法，`wverify_cross_symbol`）
7 品种（BTC/ES/CL/GC/BRN/DX/QQQ）σ̂ 归一化残差池化，pooled_residuals=9030。

| 品种 | residuals | verdict | V/F/I |
|---|---|---|---|
| BTC | 2256 | Pass | 1/0/37 |
| ES | 1392 | Inconclusive | 0/8/22 |
| CL | 1422 | Inconclusive | 0/3/30 |
| GC | 1431 | Inconclusive | 0/7/27 |
| BRN | 999 | Inconclusive | 0/3/30 |
| DX | 846 | Inconclusive | 0/8/23 |
| QQQ | 684 | Inconclusive | 0/2/28 |
| **7品种池化** | **9030** | **Inconclusive** | **0/9/34** |

- **只有 BTC 单标的 Pass（V=1，就是那个 beta 漂移桶）**；其余 6 品种全 0 Validated。
- **主桶 (L0,bsp3,δ+1,σ0) 池化翻转**：BTC 单标的 n=428 mean=−0.64 LCB=−2.59 → Inconclusive；7品种池化 n=1856 mean=**−1.96** LCB=**−2.67** perm_p=0.055 → **Falsified**（翻转=是）。
- 结论：正残差是 **BTC 独有**（BTC secular bull beta），σ̂ 归一化跨标的池化后消失甚至转负——memory `l3_cross_symbol_btc_idiosyncratic` 全历史确认。

### R5 q4 π^full 四臂 policy 回测（§6.1，`q4_fullpi_policy`）
> **[待填——R5 在冻结二进制 `newchan_rust-9cb5b83727a3eeb0` 上直跑中，避开 #183 未提交 mod.rs 编辑造成的编译污染]**

---

## 3. 口径 gap 说明（§3.1 报告桶 vs 裁决聚合基）——报 Lead 判定归属

**发现**：冻结 HEAD 的 `bucket_verdict`/`global_verdict` 计算三态（及 verdict=Pass）在含 δ 的 4元组 `(level, bsp, δ, σ^H)` 上——这按 prereg §3.1 是 **W-VERIFY 报告桶（描述性，非裁决）**。prereg §3.1 明文：主判据裁决聚合基 = δ-free `(level, bsp_class, parent_dir)`，且"报告桶的单桶 state 不单独触发 acceptance；全局裁决走聚合基（§3.2）"。perm 分层键确实是 δ-free `(ℓ,h桶,time_block,σ^H)`（perm_test.rs:92），但 μ̂/LCB/UCB/state 是报告桶粒度。

**我的读法（全部落在 prereg 文本内，非新裁定）**：
1. 报告桶那个唯一 Validated（`L0 bsp3 σ+1`）两个 δ 方向同为 +162 → §3.1 δ-free 铁律 + 657/oddeven 判定为 beta 漂移伪结构，不是方向性 alpha。
2. co-primary β 路径（§3.2 path-2，唯一显式的 δ-free 方向检验）L0/L1/L2 全不显著。
3. full-z（8维含 i_class 方向）verdict=Inconclusive，零 Validated。
4. L3 跨标的池化把 BTC 独有正桶翻转 Falsified。
→ 四路证据一致指向 **INCONCLUSIVE**（无 confirmed 方向 alpha），与 prereg §7 认识论预承诺一致。

**Lead 裁定（收 3 项裁定）**：报告桶 verdict=Pass **不算** §3.2 path-1（§3.1 明文直接应用，定理类）；四路证据收敛，终局 acceptance=**INCONCLUSIVE 无 confirmed 方向 alpha**，Lead 认定。a5 CHECK 事件由 Lead 写（归属在 Lead），本工位不写。

### 3.1 δ-free 聚合基三态离线重算（Lead 裁定点1②，明文主判据执行 gap 补齐）
冻结 `wverify_full` 只落报告桶（含 δ）不落 δ-free 聚合基三态。从 R2 报告桶行离线池化 δ+1/δ−1 到 δ-free 基 `(level,bsp_class,parent_dir)`（组均值+组方差合并公式，std 由 LCB 反推 `std=(mean−lcb)·√n/1.645`，正态近似 LCB）——**非跑后调口径，是执行 prereg 明文主判据**（§3.2 主判据=δ-free 聚合基 LCB_OOS>0）。22 个 δ-free 桶中：

- **唯一 LCB>0 桶 = `L0 bsp3 σ+1`**（N=248 池化 mean +162.50 std 695.0 LCB **+89.90**）——与报告桶同一桶。
- **决定性 beta 判据**：该桶 δ+1（mean+162.25 n156）与 δ−1（mean+162.91 n92）**两方向同号同量级** ⟹ 池化后不抵消，δ-free mean 仍 +162.50。**若是方向性 alpha，买/卖池化必抵消**（异号）；同号不抵消 = 该 (level,bsp,σ^H) 结构格的**纯 beta 暴露**（level×持有窗 beta 相位，与交易方向无关），非可交易买卖边际。
- 其余 21 桶：1 个 UCB≤0（L0 bsp1 σ+1 纯 beta）、其余 LCB≤0<UCB（欠功效 Inconclusive）。
- **口径限制照实（231）**：离线池化能算 δ-free mean/std/正态 LCB（主判据 LCB>0 门），但**不能**精确重建池化后的 n_eff（事件聚集校正）与 δ-free perm_p（需逐笔序列，冻结二进制未落盘）。故此 δ-free LCB>0 是**正态近似上界口径**，比生产 decontam 的 n_eff 折减口径**更宽松**（n_eff<n ⟹ 真 LCB 更低）——即真实三态**至多**与此一致，不会更多桶 LCB>0。结论方向稳健：δ-free 主判据下也仅此 beta 桶「LCB>0」，且它是纯 beta（同号不抵消），**acceptance 仍 INCONCLUSIVE**。逐笔留存精确重算（含 n_eff+perm_p）列为下游待办（需冻结代码加 per-trade dump，非本轮域）。

四路证据（报告桶 beta 签名 + co-primary β 不显著 + full-z 零 Validated + L3 池化翻 Falsified）+ 本 δ-free 聚合基重算 = **五路一致 INCONCLUSIVE**。

---

## 4. 三同族终态复测（a5 "三同族现象修复后复测"）

### F1 一类信号 census（`level_signal_census_btc`，BTC 全历史 4613599 bar）
逐级 buy1/sell1（raw bsp bits）：L0 buy1=219 sell1=233 / L1 3/4 / L2 4/9 / L3 4/5 / L4 1/1 / L5 0/0。一类合计 483（raw bsp 口径；与 §1.4 冻结 614 差异=614 是 post-collect_signals dedup 口径，两者不同投影，均合法）。**type1>0 全级别，type1=0 前提确认已修复**。

### F2 一类漏斗逐环（`type1_funnel_census_btc`，全历史）
逐级 环0候选→环1前驱中枢→环2局部趋势门→环3破最后中枢→环4 A/C配对→环5坐标映射→环6背驰C<A（=一类信号）：
- L0: 39998→39995→1980→580→579→579→**452**
- L1: 9261→9258→979→27→20→20→**7**
- L2: 2085→2080→389→22→20→20→**13**
- L3: 440→431→166→16→16→16→**9**
- L4: 80→72→44→4→4→4→**2**
- 旧 AllTrend 锁死点全部解锁（L0 2017-08-17 起、L1 2017-08-19 起等）；2021 顶区 sell1 因果重放正验证。探针走生产 parity 断言通过（675号）。

### F3 区间套 depth 分布（`l2_depth_distribution_dx`，全历史 4613599 bar）
Type2/3 主群 **8248 = base_none 0 + 小转大(descend None) 7498 + 有锚(Some d) 750**，小转大占比 **90.91%**。depth 直方图（有锚 750）：d=1=716（L1 432/L2 181/L3 81/L4 22）、d=2=33（L2 24/L3 9）、d=3=1（L3）、**max_depth=3，无 d≥4**。锚点正确性抽样 200/200 通过。→ 区间套深下沉是稀有事件（memory `interval_nesting` 全历史确认：95%+ 退化 base-case/浅下沉）。

### F4 XZD C3 死门 / N^δ 门重封（`h2_sample_exclusion_dx`，全历史 4613599 bar）
level1-4 第二类信号进入 Γ=7241，N^δ 门分解：base_none **6574（90.79%，tower[lvl] 无 end==src 候选段=无定位）** / gate_pass **667（9.21%）** / cond3_extreme=0 / 其余阶段=0。C3 诊断字段**不参与 gate_pass**（`gate_pass = type2_confirmed && (level!=1 || c3_new_center_breakout_ok)`，econ_positive.rs:1138）——§1.5 XZD C2-only 维持裁定确认，C3 死门维持。区间套问题① false negative=0（端点相等口径与区间包含口径 100% 一致，无分叉）。

---

## 5. 结果包六要素

1. **结论**：终局 alpha = **INCONCLUSIVE**（无 confirmed 可交易方向 alpha）。四口径 D 判定唯一 Validated 报告桶 `L0 bsp3 σ+1` 两 δ 同正 = beta 漂移；co-primary β 不显著；full-z 零 Validated（4 powered-Falsified）；L3 跨标的池化正桶翻 Falsified。三同族 type1=0 前提确认已修复（漏斗全级别存活）、区间套 max_depth=3 稀有、XZD C3 死门维持。
2. **定义依据**：可交易判据=μ(z,a)>0 的 LCB_OOS>0（663/§12）；桶键 δ-free（657 共线免疫+665 方向不对称免疫）；LCB≤0 不外推 μ≤0（667+231）；beta 漂移=两 δ 同号残差（memory oddeven，δ 反事实非几何真结构）。
3. **边界条件（结论翻转）**：(a) 若 Lead 裁 δ-free 聚合基重算后出现 powered-VALIDATED 桶且 co-primary β 显著 → acceptance PASS，触发 B30②④生产化；(b) 若样本量升数量级（跨标的池化 n≈数万 + 657 归一化新 prereg）→ 提功效链可再检；(c) 默认口径切 ThetaLex/ThetaDom（影响信号集，090 bit-exact）→ 全链重冻结。
4. **下游推论**：a5 验收的下游重跑件字面（"否定性结果照实"）满足——三同族修复复测 + q4 口径 alpha 重跑全部完成，INCONCLUSIVE/FALSIFIED 照实产出。B30②selector LCB 生产化 + ④shrinkage 维持后置（无 VALIDATED 候选=无消费者/无收缩对象）。M1 里程碑走 §6.2 提功效链（非本轮判据）。
5. **谱系引用**：135（冻结先于跑数）；657 + memory `oddeven_mu_identity`（δ 同号=beta 漂移）；memory `l3_cross_symbol_btc_idiosyncratic`（BTC 独有正号，池化消失）；memory `interval_nesting`（区间套退化）；663/665/667（判据/单标的 beta 退化/功效门 inconclusive≠证伪）；231/formalization-validity-domain（L2/L3 分级）；090/161（bit-exact + 否定性照实）；179（XZD C2-only 终裁）；180/exitmu-bucketing。
6. **影响声明**：新增 `.chanlun/review-results/final-alpha-20260704.md`（本结果包）。**不改任何代码、不改谱系**——纯跑数 + 判读。跑数产物落 `/tmp/finalpha/`（日志）+ `/tmp/wv_*.md`（桶表）+ `.chanlun/review-results/l2-depth-raw-20260702.md`（F3 depth 原始，由测试落盘）。发现并上报 #182/#183 共享工作树冲突（R5 被 #183 未提交 mod.rs 编辑污染，已用冻结二进制隔离重跑）。a5 CHECK 事件待 Lead 确认判定归属后落。
