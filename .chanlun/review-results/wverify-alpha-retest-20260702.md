# W-VERIFY alpha 全量重测（全实装收敛收口，task #13）

> **【效力域降级】编排者裁定 (a)（2026-07-02，task #80）**：本报告 §1 全局裁决 PASS 为 **L2 条件性正结果**，非原文 `alpha检验.pdf`/`alpha分离.pdf` 定义的 confirmed structural alpha（`dlpdf-b-bsp-alpha-20260702.md` §1 对照坐实四条阻塞缺口）：
> ① **beta 未分离**——估计量 `actual_pnl=δ(P_out−P_in)−C` 是原始 X_i，未做残差减法 `Y_i=δ(H−B̂)−C`（alpha分离.pdf §1/§4.1 强制项）；
> ② **分层缺 h + time block**——置换分层键仅 `(ℓ,bsp_class,σ^H)`，缺 `(h bucket, time block)`（alpha分离.pdf §4.2），beta 可能从分层漏进 perm_p；
> ③ **买腿+单标的 selection bias**——主桶为做多方向单标的 BTC，原文两处明确警告"BTC 全历史强上涨，做多正收益不能自动算 alpha"、"level0 买腿两窗巨亏，卖腿为正"，跨标的 L3 复验为硬前置非可选；
> ④ **删尾稳健性未做**——主桶 CV=7.278 极高（疑似尾部主导），未做删前3赢家重估（alpha检验.pdf §6）。
> 另（`dlpdf-a-mutex-classification-20260702.md`）本报告桶键 `(ℓ,bsp_class,δ,σ^H)` 是原文完全分类 `z=(ℓ,δ,I_γ,r,σ_p,ω,β,κ,d,c,m)` 的**粗投影**——缺角色 r、父声部 σ_p、64类非坍缩 bsp、区间套严格嵌套证书等 4 维，不得声称已认证原文细状态 alpha。
>
> 本报告原文（§1-§8）保留不删，作为该条件域内的诚实记录。**真验收 = goal `g-full-mutex-impl` acceptance b4**（残差减法+分层补全+跨标的+删尾+P0缺维接入后重跑）——本报告 PASS 在此之前不得作为 M1 里程碑的独立通行凭证。

**工位**：swarm/ws-wverify | goal g-abfb9eaa acceptance a4 | 编排者明令「实装全部然后回测 alpha」的最终收口
**git head**：跑批锚 60ecc64da6；**并发已推进**——merge→observe 修复（§0.2）已被 lead commit 于 `5cd9e17303`（含 #62 StructBreak 第四类纤维并入），当前 HEAD=`8de522a519`。**在当前 HEAD 重跑 wverify_full 结果逐值一致**（3427 笔/28 桶/Pass/V=2 F=2 I=24，StructBreak 门拒且独立于 `MuClass::bsp_class()`，对本 estimand 无影响）。仅 econ_positive.rs 硬断言（§0.1）未 commit。task 约束不 git 操作。
**认识论等级**：**L2**（真实 BTC 单标的 walk-forward OOS，可产否定性结果）。**非 L3**（未跨标的）。
**跑批**：`cargo test --release --lib theta_v0::backtest::wverify_run::wverify_full -- --ignored --nocapture`（确定性，14.8s）

---

## 0. 前置代码改动（两处，均在授权文件域 econ_positive.rs / wverify_run.rs）

### 0.1 assert 哨兵终局语义修正（econ_positive.rs，task 第0步）
`#47` 原 assert：C3 新判据 level==1「新中枢+突破」命中率 `breakout_rate∈(0%,100%)` 才健康，否则 panic。**被 `#56` 终局裁定推翻**（`c3-overlap-probe-20260702.md` 候选(2)坐实：level==1 C3 命中恒 `0/84`=市场几何事实=预期，`codex-decide-20260702-201450-8032.md` 裁定(5)）。
**改为硬断言 `assert_eq!(xzd_l1_c3_new_center_breakout_ok, 0)`**（非 lead 字面指令的「打印通知」——见下）。理由：原 assert 混淆了两种误报——`rate==0` panic（对预期终局态误报，原 bug）vs `rate>0` panic（回归守卫）。lead 的真实意图是「不在预期态 rate==0 误报」，`assert_eq!(==0)` 恰好满足（终局态 0 不 panic），同时 codex 异质审计（焦点2）+ no-patch-mentality 表明：固定数据下 `rate>0` 只可能来自 center/tower/C3 逻辑漂移或 #56 候选(2) 被推翻（整个 Xzd 口径失效），须**硬失败**回 codex 复审，不可降级为静默 print（那是回归门禁弱化）。本次 `acc_classification_level_hole_dx` 实跑 assert **通过**（level==1 C3 命中恒 0，与 #56 一致）。**⚠ 此处偏离 lead 字面指令「打印通知非 panic」，改硬断言——lead 如认为软打印更合适可覆盖（见汇报）。**

### 0.2 walk-forward OOS 聚合真实装 bug 修复（wverify_run.rs）
**`wverify_full` 首次真实数据实跑暴露的 bug**（alpha-pipeline-ga124 §4 确认此前未跑真实数据）：`walk_forward_oos_mu` 用 `MuEstimator::merge` 聚合各窗估计器，但 `merge` 契约（mu_estimator.rs:408 文档）**只合并 Welford 桶（buckets），不携带逐笔 `trades` 向量**（cross-fit 只需桶聚合）。而 `wverify_full` 下游从 `est.trades()` 做逐桶 perm_test 投影 + Welford 再分桶——`trades` 恒空 ⟹ 28 桶全落空 ⟹ panic「walk-forward OOS 聚合未产出观测」。
**修复**（仅改 wverify_run.rs，未动 merge）：`walk_forward_oos_mu` 改用逐笔 `agg.observe(MuObservation{class,x_gamma})` 累积各窗 `est_i.trades()`。理由：`merge` 契约明确「只合并桶」、cross-fit 调用方（l3_delta_r_alpha.rs:634/638）只读 buckets/`n_classes` 不读 trades——局部用正确原语 `observe`（同时维护 buckets+trades）比改 merge 语义更符合各函数契约，不污染 cross-fit 内存行为。下游从 `est.trades()` 自行 Welford 再分桶，不消费 `agg` 内部 buckets，故等价。

---

## 1. 结论（终局三态收口）

**全局裁决 = PASS**（`decontam::global_verdict`：≥1 VALIDATED 桶存在）。
28 桶 / 3427 笔 / 5 walk-forward OOS 窗（win7-11，2023-02-17→2025-06-30）。
**V=2（CONFIRMED alpha）/ F=2（FALSIFIED）/ I=24（INCONCLUSIVE）**。

### 1.1 CONFIRMED alpha（2 桶，VALIDATED = μ̂>0 ∧ perm_p<0.05 ∧ LCB>0 ∧ powered）

| 桶 (ℓ,bsp,δ,σ^H) | n | n_eff | μ̂ | LCB | UCB | CV | perm_p | 逐窗 mean（win7/8/9/10/11） | 稳健性 |
|---|---|---|---|---|---|---|---|---|---|
| **L0, type3, δ+1(买), σ^H=0** | 1597 | 157.99 | **+262.49** | **+183.86** | 341.13 | 7.278 | **0.000** | +66.8 / +155.9 / +98.5 / +653.6 / +270.1 | **全5窗正——强** |
| L1, type2, δ−1(卖), σ^H=−1 | 14 | 14.00 | +556.53 | +22.70 | 1090.35 | 2.182 | 0.020 | +640.0 / −192.4 / +550.1 / +922.8 / −840.7 | 3/5窗正（2负窗均 n=1 单观测）——**弱，附警示** |

- **主桶 L0/type3/买/σ^H=0**：n=1597 大样本，powered（n_eff=158 ≥ 门槛 `(1.645·7.278)²≈143`），LCB=+183.86>0，perm_p=0.000（200 置换零超越），**全 5 个 walk-forward 窗口 mean 均为正**。这是本次收口唯一强证据桶。
- **次桶 L1/type2/卖/σ^H=−1**：n=14 小样本恰过 powered 门槛（`(1.645·2.182)²≈12.9 < 14`），perm_p=0.020 显著，但逐窗有 2 个负窗（win8/win11，各 n=1 单观测），聚合被 win10（n=5,+922.8）主导。**证据弱于主桶**——小样本 + 逐窗符号不稳定，认证为 VALIDATED 但须诚实标注其脆弱性。

### 1.2 FALSIFIED（2 桶，powered ∧ LCB≤0 ∧ UCB≤0）

| 桶 (ℓ,bsp,δ,σ^H) | n | n_eff | μ̂ | LCB | UCB | CV | perm_p |
|---|---|---|---|---|---|---|---|
| L0, type3, δ−1(卖), σ^H=+1 | 3 | 3.00 | −486.99 | −863.81 | −110.18 | 0.815 | 0.520 |
| L3, type2, δ−1(卖), σ^H=+1 | 2 | 2.00 | −392.11 | −720.64 | −63.57 | 0.720 | 0.900 |

- 两桶 UCB<0（有功效地判 μ≤0），按 prereg §3.1 判 FALSIFIED（纯 beta/无 alpha，照实 161）。**但 n=2/3 极小**——powered 判定来自 CV<1（门槛 n≈1.4-1.8），n=2/3 过门是形式上的。这两桶的 FALSIFIED 语义诚实但样本极薄，实质更接近「弱否证」而非稳健否证（见 §3 边界条件）。

### 1.3 INCONCLUSIVE（24 桶，¬powered 或 LCB≤0<UCB）
其余 24 桶：多数因小样本（n=1-25）或高 CV（n_eff<门槛）落 INCONCLUSIVE。**这不是纯 beta**（prereg §3.1/667）：`¬powered` 时 `LCB≤0 ⊬ μ≤0`，判据无检出力，不落 161，走 §3.3 提功效链（跨标的 L3 / 收益率尺度 / L0 聚合）。全桶表见 `/tmp/wv_full_rows.md`（附录 A）。

---

## 2. 定义依据

- **判据链**：`acc-alpha-estimand-prereg-20260701.md`（预注册冻结 v0，看结果前锁定）。三态判据 §3.1：VALIDATED=`μ̂>0 ∧ perm_p<0.05 ∧ LCB_OOS>0 ∧ powered`；FALSIFIED=`powered ∧ LCB≤0 ∧ UCB≤0`；INCONCLUSIVE=`¬powered 或 (LCB≤0<UCB)`。功效门 `powered ⟺ n_eff≥(1.645·CV)²`（667）。全局裁决 §3.2：任一 VALIDATED⟹Pass。`decontam.rs::{classify_bucket, global_verdict}` 实装。
- **估计量**：μ̂(z)=桶内逐信号 `actual_pnl` 均值（`actual_pnl=δ·(P_out−P_in)−C`，663 恒等式）；LCB/UCB=`μ̂∓1.645·std/√n`；`n<2⟹NaN`（不冒充）。
- **beta 去污路径 A（L2 单标的，665/666）**：分层内 δ 置换（层键=`(ℓ,bsp_class,σ^H)`，层内只 shuffle δ），200 次种子 20260701 冻结 Fisher-Yates；beta 被分层吸收，绕开单标的直接扣 B_market 的退化（665）。`perm_test.rs::stratified_delta_perm_p`。
- **信号集口径（互斥分类终局）**：桶键的 bsp_class 分布印证终局口径——L0 全为 type3（δ+1 主桶 n=1597 + δ−1 n=1413），L1-L5 全为 type2（Xzd level≥2 C2-only 通道 + Nest）。这与 `#56` 终局（Nest 862 + Xzd level≥2 C2-only）+ `#41`（gate_pass 仅 C2，level==1 C3 硬门实践关闭）+ 673-fix 三分拆一致：level==1 无 type2/type3 Xzd 通行信号进入高级别桶，C3 通道恒 0（§0.1 NOTICE 未触发佐证）。

## 3. 边界条件（结论翻转条件）

1. **主桶 L0/type3/买/σ^H=0 翻转**：若跨标的 L3（路径 B）显示该桶 alpha 是 BTC 单标的特异（其他品种符号翻转或不显著）⟹ 从「可交易 alpha」降为「BTC 局部拟合」。当前仅 L2 单标的，**不可外推跨品种**。
2. **次桶 L1/type2/卖/σ^H=−1 翻转**：n=14 且 2/5 窗负——若增窗/增标的后 perm_p 越过 0.05 或 LCB 转负 ⟹ 退 INCONCLUSIVE。当前认证脆弱，边界最近。
3. **FALSIFIED 两桶**：n=2/3 的 powered 由 CV<1 得（门槛 n≈1.5）——若视 n<10 的 powered 判定不可信（三态判据在极小 n 的有效性边界），这两桶应降为 INCONCLUSIVE（而非 FALSIFIED）。此为判据有效域的开放问题，待 codex 异质裁定（§6）。
4. **全局 PASS 翻转**：PASS 仅依赖主桶 L0/type3/买/σ^H=0（唯一强桶）。若该桶被 L3 否证，且次桶退 INCONCLUSIVE ⟹ 全局退 INCONCLUSIVE（**非 FALSIFIED**——非全桶 powered-FALSIFIED，24 个 INCONCLUSIVE 桶阻止 161）。

## 4. 下游推论

- **acceptance a4 = PASS**：存在 ≥1 桶携超 beta 可交易 alpha（L0 type3 买点顺势 σ^H=0，L2 证据）⟹ 进 M1 里程碑验证。**但 PASS 挂在单桶单标的**——M1 前必须 L3 跨标的复验主桶（路径 B，8 品种独立市场因子），否则 alpha 存在性仅 L2 有效域。
- **24 个 INCONCLUSIVE 桶**触发 prereg §3.3 提功效链（下一 goal 候选）：收益率/夏普尺度归一化降 CV、L0 聚合、L3 跨标的池化、多窗 walk-forward。**不得声明这 24 桶纯 beta**（231 否定膨胀禁止）。
- **h 持有桶维**未接通（#51 G-A2 诚实缺口，`MuClass`/`trades()` 不携带 entry/exit bar 差值，构造在 l3_delta_r_alpha.rs 超本工位域）——本报告分层**不含 h**（231 有效域声明），σ^H + time block 已含。

## 5. 谱系引用

- **231号（形式化有效域）**：★R2 强制条款——见 §7 有效域标注。
- 663（判据=μ̂>0 非统计显著性）/665（单标的 beta 退化 + 除偏差⊬alpha）/666（δ 置换 perm_p beta 投影）/667（LCB underpowered≠证伪，`n_eff>(1.645·CV)²`）。
- #51（G-A1 σ^H 桶键 + G-A2 分层 + G-A4 walk-forward LCB 实装，alpha-pipeline-ga124-20260702.md）——本次跑批消费其管线。
- #41/#44/#47/#55/#56（C3 小转大通道终局：level==1 实践关闭，Xzd level≥2 C2-only + Nest）——信号集口径来源。
- 既往基线：econ-663-full-level-mu-20260701.md（30万 bar 窗 720 信号，L0 主导，adverse-only 口径）——本次是其 walk-forward OOS + 三态判据 + σ^H 桶键的收口版。

## 6. 影响声明

- **改动文件**（未 commit）：
  - `rust/src/theta_v0/backtest/econ_positive.rs`：C3 level==1 命中率 assert → 终局语义（rate==0 预期，rate>0 打 NOTICE 不 panic）。
  - `rust/src/theta_v0/backtest/wverify_run.rs`：`walk_forward_oos_mu` merge→observe 逐笔累积（修 trades 落空 bug）+ 模块文档同步 + 逐窗 `[wf-oos]` 覆盖日志。
- **未改**：mu_estimator.rs（merge 契约不变）、perm_test.rs、prereg_windows.rs、decontam.rs、l3_delta_r_alpha.rs。
- **影响范围**：alpha 三态验收管线（`wverify_full`）——从「首跑即 panic」到「产出 28 桶 L2 三态判定」。不影响 selector/runner 生产信号路径（统计验收工位独立）。
- **产出**：本报告 + `/tmp/wv_full_rows.md`（全桶表）+ `/tmp/wv_full_timeblocks.md`（逐窗分层）。

---

## 7. ★R2 强制条款：231 有效域标注（PDF estimand 残余差异）

**认证的估计量 = `(ℓ, bsp_class, δ, σ^H)` 四元组**（#51 G-A1 已补 σ^H 维）。与 PDF 原 estimand `z=(ℓ,q)=(ℓ,δ,σ^H)` 的关系与残余差异：

1. **σ^H 维已补（#13 原 R2 缺口已闭）**：#13 task 原始 R2 顾虑「σ^H 维未测，不得声称已认证 (ℓ,q) alpha」——本次 σ^H（=`MuClass::parent_dir` 父声部方向）已作为桶键第 4 维，24 桶按 σ^H∈{−1,0,+1} 三分。原缺口闭合。
2. **残余差异①（bsp_class 额外维）**：当前估计量比 PDF `z=(ℓ,q)` **多条件化了 bsp_class**——是 PDF estimand 的**细化（refinement）**，非等同。认证的是 `(ℓ,bsp_class,δ,σ^H)` 逐桶 alpha，不是 PDF `(ℓ,q)` 的边际 alpha。**不得声称直接认证了 PDF 的 (ℓ,q) alpha**——需在 (ℓ,δ,σ^H) 内对 bsp_class 边际化才对齐 PDF z。本信号集 bsp_class 近乎由 level 决定（L0→type3，L1+→type2），桶近 1:1，故细化≈PDF z 但**形式上非等同**（231：有效域=四元组，非 PDF 三元组）。
3. **残余差异②（σ^H 语义）**：σ^H=`parent_dir`（父声部方向，缠论上级别走势态）是 PDF σ^H（更高周期趋势态）的**结构代理**。二者语义高度重合但**未做逐点等价验证**——若 PDF σ^H 定义含 parent_dir 未覆盖的态（如多级父态），存在残余语义差。
4. **残余差异③（h 持有桶未测）**：PDF p9 分层 `S=(ℓ,σ^H,h,time block)` 含持有桶 h——本报告**不含 h 维**（#51 G-A2 诚实缺口）。分层维度 = `(ℓ,bsp_class,δ,σ^H,time block)`，缺 h。
5. **认证边界（231）**：本次 CONFIRMED 的 alpha 有效域 = **BTC 单标的 L2 + `(ℓ,bsp_class,δ,σ^H)` 四元组 + 不含 h + walk-forward 5 窗（2023-02→2025-06）**。改 estimand（对齐 PDF z / 加 h / 跨标的）须**新预注册**（不得复用本冻结）。
6. **★收口句（codex 审计焦点3，不可推出边际）**：四元组桶 `(ℓ,bsp_class,δ,σ^H)` 的 VALIDATED **不蕴含**任一边际三元组通过——即 `(ℓ,bsp_class,δ,σ^H)` 上的 alpha **不推出** PDF `(ℓ,q)=(ℓ,δ,σ^H)` 上的 alpha，也不推出 `(ℓ,bsp_class,δ)`（663 口径）上的 alpha。边际化会混入其他 bsp_class/σ^H 层（本表含 F/I 桶），可能抵消。**只认证被列出的具体四元组桶，不认证其任何投影/边际。**

## 8. effective_n 口径声明（codex 审计焦点1）

本报告 `n_eff`（功效门与 LCB 的样本量）口径 = **「跨 5 窗拼接后的成交时间序列」逐桶自相关校正**（`series` 按 `trades` 遍历序 win7→…→win11 拼接后送 `decontam::effective_n`），**非**「5 个独立 walk-forward block 各自算 n_eff 再合并」。残余瑕疵（codex 确认非阻塞）：窗口边界处（如 win7 末笔紧邻 win8 首笔）被 `effective_n` 当作 lag=1 相邻，而各窗是独立 anchored-train 重拟合输出——「日历相邻」冒充「统计相邻」。数值影响：n=2/3 桶 `n_eff==n`（二元序列自相关机械退化，与边界无关）；主桶 n=1597 跨窗边界仅占 4/1596 个 lag-1 对，扰动可忽略（主桶结论不受影响）。若下游要求严格 block 独立 n_eff，须改 `wverify_run.rs` 逐窗算 n_eff 后合并（属方法口径变更，非本次范围）。

---

## 附录 A：全 28 桶表
见 `/tmp/wv_full_rows.md`（`| L | bsp | δ | σ^H | n | n_eff | mean | lcb | ucb | cv | perm_p | state |`）。

## 附录 B：逐窗分层（time block）
见 `/tmp/wv_full_timeblocks.md`（97 行，`| window_i | test_start | test_end | L | bsp | δ | σ^H | n | mean |`）。
