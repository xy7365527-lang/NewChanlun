# 跨品种残差归一化 L3 v2 alpha 重测 — 结果包（task #102，主桶功效收口）

- **工位**：swarm/ws-l3norm | 预注册冻结 `prereg-l3norm-20260703.md`（commit 1bbfdc23cb，看结果前冻结）
- **代码基线**：HEAD=0b2d8ccfcc（team-lead 发；ws-bottomup #101 NO-SHIP，生产 descend 与 PDF bottom-up bit-exact，信号集未变）。harness `wverify_run.rs::wverify_cross_symbol`（`#[ignore]`，确定性 47.82s）。护航 `cargo test --release --lib` = **1406 passed / 0 failed**（新增 σ̂ 自检，既有零退化）。
- **认识论等级**：**L3**（7 品种跨标的真去污 + σ̂ 尺度通约，per-symbol B̂/σ̂ 独立，可否证）。否定性结果（主桶 powered-Falsified）缩小有效域边界（231）。

---

## 1. 结论（一句话）

**σ̂-归一化消除了 v1 的 BTC 尺度伪影后，主桶 L0/type3/买/σ0 的功效目标达成（CV 22.58→16.48，功效门 1380→734，n_eff 700→754 过门 ⟹ powered），但收口方向是 FALSIFIED 而非 VALIDATED：池化主桶 mean 从 v1 的 +39.67($) 翻转为 −2.99(σ)，LCB=−3.97<0 ∧ UCB=−2.02<0 ∧ perm_p=1.000 ⟹ powered-FALSIFIED。per-symbol 逐品种在通约单位下仍仅 BTC 主桶为正(+6.19σ)、其余 6 品种全负——坐实 v1 暗示的『主桶正 alpha 是 BTC-idiosyncratic，非缠论买卖点普适 edge』。全局 INCONCLUSIVE（V=0/F=5/I=24），主桶从 v1 的『欠功效未定』被推进到『有功效否证』。**

---

## 2. σ̂ 表（尺度伪影量化——v1 BTC 支配的根源）

| symbol | σ̂_symbol（$/bar close-to-close std，pre-OOS） | pre-OOS bars | 相对 BTC |
|---|---|---|---|
| **BTC** | **29.813677** | 2,794,068 | 1.0× |
| ES | 1.043811 | 2,449,435 | 1/28.6 |
| GC | 0.414779 | 2,437,488 | 1/71.9 |
| QQQ | 0.153967 | 888,485 | 1/193.6 |
| BRN | 0.069015 | 1,302,533 | 1/432 |
| CL | 0.046061 | 2,457,529 | 1/647 |
| DX | 0.013515 | 1,114,407 | 1/2206 |

**读出**：BTC 的逐笔 $ 波动是 DX 的 2206 倍、ES 的 29 倍。v1 原始-$ 拼接下 BTC 残差（±数百 $）在数量级上压过 DX（±0.01 $）、CL（±0.05 $），池化均值被 BTC 单品种支配——这正是 v1 主桶正 mean(+39.67) 的来源。σ̂ 除法把每品种残差归一到自身 $ 波动尺度，7 品种同量纲（σ 单位）后公平合并。pre-OOS bars 均为百万级 ⟹ SIGMA_MIN_BARS=1000 守卫无触发，7 品种全入池。

---

## 3. ★v1↔v2 主桶差分（本报告核心，L0/type3/买/σ0）

| 口径 | n | n_eff | mean | CV | LCB | UCB | perm_p | powered? | state |
|---|---|---|---|---|---|---|---|---|---|
| v1 原始-$ 拼接 | 6865 | 699.74 | **+39.67**$ | 22.582 | +21.88 | +57.45 | 0.040 | **否**（门≈1380） | Inconclusive |
| **v2 σ̂-归一化** | 6905 | 754.35 | **−2.99**σ | **16.479** | **−3.97** | **−2.02** | 1.000 | **是**（门≈734） | **Falsified** |

**功效收口判定（编排者目标）达成**：σ̂-归一化把池化主桶 CV 从 22.582 降到 16.479 ⟹ 功效门 `(1.645·CV)²` 从 1380 降到 734.8 ⟹ n_eff=754.35 > 734.8 ⟹ **powered**（v1 主桶 n_eff 700<1380 欠功效，v2 收口）。归一化除去品种间尺度异质性对 std 的膨胀是 CV 下降的机制。

**但方向 = FALSIFIED**：收口后主桶 mean=−2.99σ（v1 +39.67$ 翻转符号）、LCB 与 UCB 均<0、perm_p=1.000 ⟹ `powered ∧ LCB≤0 ∧ UCB≤0` ⟹ FALSIFIED。**编排者"功效收口"目标达成，但收口到否证——通约单位下缠论主桶 alpha 有功效地判为 μ≤0**（预注册 §0.4 / §6.4 预期方向 (b)）。

（n 微异 6865→6905 源于基线 HEAD 演进：v1 数字取自 v1 结果包旧 HEAD，v2 在 0b2d8ccfcc；同口径残差管线，符号翻转 +39.67→−2.99 与 powered 状态变化幅度远大于 n 的 <1% 差异，结论方向不受影响。）

---

## 4. per-symbol 主桶逐品种（★BTC 独占正号坐实，L0/type3/买/σ0，σ 单位）

| symbol | n | n_eff | mean(Ỹ) | LCB | UCB | perm_p | state |
|---|---|---|---|---|---|---|---|
| **BTC** | 1613 | 167.55 | **+6.19** | +3.65 | +8.73 | 0.020 | Inconclusive（欠功效，门≈272>168） |
| ES | 1125 | 162.42 | **−6.75** | −9.07 | −4.42 | 1.000 | **Falsified** |
| CL | 1043 | 222.02 | −1.13 | −2.66 | +0.40 | 0.640 | Inconclusive |
| GC | 1102 | 227.30 | **−7.67** | −10.38 | −4.96 | 1.000 | **Falsified** |
| BRN | 792 | 602.55 | −1.21 | −3.96 | +1.54 | 0.255 | Inconclusive |
| DX | 634 | 127.79 | −3.17 | −5.46 | −0.88 | 0.995 | Inconclusive（欠功效，门≈330>128） |
| QQQ | 596 | 133.97 | **−17.54** | −20.27 | −14.82 | 1.000 | **Falsified** |

**读出**：通约单位（σ）下逐品种主桶——**仅 BTC 为正（+6.19σ，perm_p=0.020 显著但欠功效）**，其余 6 品种全负，其中 ES/GC/QQQ 是 powered-FALSIFIED。这与 v1 原始-$ 的 per-symbol 结论**方向一致**，但现在是**公平尺度对比**（v1 的负值可能被误读为小尺度品种的噪声；归一化后负号在 σ 单位下是实打实的负 edge）。**主桶正 alpha 是 BTC 单品种特有，不跨品种复现**——对照 `oddeven_mu_identity`（BTC beta 漂移伪结构参照），BTC 主桶正号需进一步判别是否 idiosyncratic beta 残留。

---

## 5. 全局裁决 + Falsified 桶（池化，7 品种拼接归一化）

**全局 = INCONCLUSIVE（V=0 / F=5 / I=24，14678 残差 / 29 桶）**。v1 为 V=0/F=0/I=29 ⟹ 归一化后新增 5 个 powered-FALSIFIED：

| 桶 (ℓ,bsp,δ,σ^H) | n | n_eff | mean | LCB | UCB | CV | perm_p | 备注 |
|---|---|---|---|---|---|---|---|---|
| **L0,3,买,σ0（主桶）** | 6905 | 754.35 | −2.99 | −3.97 | −2.02 | 16.479 | 1.000 | ℓ0 干净，主结论 |
| L1,2,买,σ−1 | 45 | 45.00 | −9.27 | −16.75 | −1.80 | 3.287 | 0.920 | ℓ1 干净 |
| L3,2,卖,σ0 | 6 | 6.00 | −26.41 | −51.00 | −1.82 | 1.386 | 1.000 | ℓ≥2 小样本 |
| L3,2,买,σ−1 | 8 | 6.69 | −37.37 | −57.51 | −17.24 | 0.926 | 1.000 | ℓ≥2 小样本 |
| L4,2,卖,σ0 | 6 | 5.96 | −5.18 | −8.48 | −1.89 | 0.947 | 1.000 | ℓ≥2 小样本 |

主结论落在 **L0 主桶（ℓ0 干净）**，不依赖 3 个 ℓ≥2 小样本桶（n=6–8，powered 仅因 CV 极低致门≈5–6；代码现口径 frontier 污染标注已解除 c546b5633c，但小样本高级别桶本身样本饥饿，不作独立强否证）。L0/L1 两个 Falsified 桶（主桶 + L1 二买）在干净有效域内，是有功效的否证。

---

## 6. 边界条件（结论翻转）

- **主桶 FALSIFIED→VALIDATED**：须某品种/口径下主桶 μ̃>0 ∧ LCB(Ỹ)>0 ∧ powered。当前 7 品种仅 BTC 正且欠功效——若 BTC 单标的补功效（更长窗/更多 BTC 数据使 n_eff>272）且正号稳健 ⟹ **BTC 单标的**主桶可能 VALIDATED，但那是单品种 alpha 非跨品种系统性。
- **BTC 正号是否 idiosyncratic beta 残留**：BTC 主桶 +6.19σ perm_p=0.020，但 per-symbol B̂ 剥离后仍正 ⟹ 需对照 `oddeven_mu_identity` 反事实置换判别是否几何真结构 vs beta 漂移伪结构（下一诊断候选）。
- **σ̂ regime 稳定性**：本冻结用 pre-OOS 固定标量 σ̂。若某品种 vol regime 在 pre-2023/OOS 间剧变致误缩放成翻转因素 ⟹ 升级逐笔因果扩张窗 σ̂_i（prereg §1 升级路径）。当前 7 品种 pre-OOS 百万级 bar，标量估计稳健。
- **归一化度量选择**：σ̂=逐 bar close-to-close $ std（未按 √h 调持有期）。若持有期分布跨品种差异成为翻转因素 ⟹ 改 σ̂·√h Sharpe-per-trade 归一（prereg 未采，活动部件最少优先）。

---

## 7. 认识论 L3 诚实声明（231 + formalization-validity-domain）

- **L3 真去污 + 通约**：per-symbol B̂_i 各剥各自扩张窗 beta（build_mu_from_bars），σ̂_symbol 各按自身 pre-OOS $ 波动归一（因果无前视，entry 全 ≥OOS_START）。跨品种置换按 SYMBOL_STRIDE 隔离 stratum，σ̂ 品种常数使层内 shuffle 同构（perm_test.rs 零改动）。
- **否定性结果价值（231）**：主桶从 v1『欠功效 Inconclusive』被归一化推进到『powered Falsified』——这是**信息增量为正的否证**：缩小有效域边界至「跨品种通约后缠论主桶（L0/type3/买）无正 alpha，正号 BTC 独有」。比 v1『尺度伪影正 mean』信息量高（v1 的 +39.67 是伪迹，v2 的 −2.99 是通约后的真判）。
- **有效域**：ℓ∈{0,1} 桶（含主桶）干净结论生效；ℓ≥2 小样本桶（3 个）样本饥饿，不作独立强否证。

---

## 8. 下游推论

- **归一化 L3 = 主桶功效收口达成但 alpha 否证**：编排者「主桶功效收口」目标机制生效（CV 降、n_eff 过门、powered），但收口到 FALSIFIED。**跨品种通约口径下缠论主桶无 confirmed alpha**。M1 里程碑不得以跨标的主桶 alpha 存在性为通行凭证。
- **BTC 单标的是唯一残余正号线索**：7 品种唯 BTC 主桶正（欠功效）。下一候选 = (a) BTC 单标的主桶补功效验证正号稳健性；(b) 对照 oddeven_mu_identity 判 BTC 正号是几何真结构 vs beta 漂移伪结构。
- **full-z 延后维持**：本轮跨标的归一化未产 VALIDATED ⟹ full-z 加维无功效动机，延后不变。
- **跨品种缠论买卖点普适 edge 假设被否证**：per-symbol 6/7 负 + 池化 powered-Falsified ⟹ 「缠论买卖点在多品种上系统性正 alpha」在本 estimand（L0/type3/买/σ0，σ̂ 归一化）下不成立。

---

## 9. 结果包六要素

1. **结论**：σ̂-归一化消除 BTC 尺度伪影后，主桶功效收口达成（CV 22.58→16.48，n_eff 754 过门 734 ⟹ powered），但方向 FALSIFIED（mean +39.67$→−2.99σ 翻转，LCB/UCB 全负，perm_p=1.000）；per-symbol 通约单位下仍仅 BTC 正(+6.19σ)其余 6 负，坐实主桶正 alpha BTC-idiosyncratic；全局 V=0/F=5/I=24。
2. **定义依据**：可交易 alpha μ(z)>0（663）；三态 + 功效门 `n_eff≥(1.645·CV)²`（667）；桶键 4 元组先验给定（665）；σ̂ 因果无前视（pre-OOS 估，entry≥OOS_START，F_entry-可测）；σ̂ 品种常数 + stratum 品种隔离 ⟹ perm_test 不改（预注册 §3）。
3. **边界条件**：BTC 单标的补功效且正号稳健 ⟹ BTC 单品种主桶可能 VALIDATED（非跨品种系统性）；BTC 正号需 oddeven_mu_identity 判 idiosyncratic beta；σ̂ regime 剧变 ⟹ 升级逐笔因果 σ̂_i。
4. **下游推论**：跨品种通约口径主桶无 confirmed alpha；BTC 单标的是唯一残余正号线索（下一诊断候选）；full-z 延后不变；缠论买卖点跨品种普适 edge 假设被否证。
5. **谱系引用**：prereg-l3norm-20260703（本冻结，σ̂ 口径 (A) 决断）；v1 prereg-l3-cross-symbol-20260702 §5（归一化列为新预注册候选）+ v1 结果包（BTC 尺度伪影诊断）；codex #85 裁 C + 边界 (d)（尺度不可通约，本报告坐实归一化后 Falsified）；663/665/667（判据/去污/功效门）；231（否定性结果缩边界，v2 −2.99 通约真判 > v1 +39.67 伪迹）；oddeven_mu_identity（BTC 正号 idiosyncratic beta 判别参照）。
6. **影响声明**：新增本结果包 + `wverify_run.rs`（sigma_pre_oos/stdev_consecutive_diffs helper + wverify_cross_symbol σ̂ 缩放循环 + σ̂ 表输出 + L1 自检 sigma_stdev_matches_hand_calc + 清理 v1 遗留「290」误导标注）。**未改**：build_mu_from_bars/ResidualTrade/perm_test.rs/decontam/prereg_windows.rs/bucket_verdict/wverify_full（全 bit-exact）。护航 1406 测试不退化。
