# 跨标的 L3 alpha 池化重测 — 结果包（task #86，codex #85 裁 C 落地）

- **工位**：swarm/ws-l3 | 预注册冻结 `prereg-l3-cross-symbol-20260702.md`（commit 4cceee3fd6，看结果前冻结）
- **认识论等级**：**L3**（7 品种跨标的真去污，per-symbol B̂ 独立剥离，可否证）。否定性结果（V=0）缩小有效域边界（231）。
- **代码**：`wverify_run.rs::wverify_cross_symbol`（`#[ignore]`，确定性 46.38s）。护航 `cargo test --release --lib` = **1402 passed / 0 failed**（无退化）。

---

## 1. 结论（一句话）

**跨标的 L3 池化（7 品种残差拼接）全局裁定 = INCONCLUSIVE（V=0 / F=0 / I=29，14598 残差 / 29 桶）。裁定 C 的功效目标机制生效——主桶 L0/type3/买/σ0 的 n_eff 从 BTC 单标的 166 增至池化 700（过功效门 290）——但主桶未翻转为 VALIDATED，且 per-symbol 分解揭示 BTC 单标的的正点估计（+178.66）不跨品种复现：7 品种中仅 BTC 主桶为正，其余 6 品种全为负（ES/GC/QQQ 直接 Falsified）。池化的正 mean（+39.67）是 BTC 大美元尺度在原始-$ 拼接中压过其他品种的尺度伪影，非真跨品种 alpha。**

---

## 2. 池化逐桶判定（L3，7 品种拼接，全表见 /tmp/wv_xsym_pooled.md）

主桶及关键桶：

| 桶 (ℓ,bsp,δ,σ^H) | n | n_eff | mean(Y) | LCB | UCB | CV | perm_p | state |
|---|---|---|---|---|---|---|---|---|
| **L0,3,买,σ0（主桶）** | 6865 | **699.74** | +39.67 | +21.88 | +57.45 | **22.582** | 0.040 | **Inconclusive** |
| L0,3,卖,σ0 | 6060 | 1549.58 | −10.04 | −23.98 | +3.89 | 65.671 | 0.035 | Inconclusive |
| L0,3,买,σ+1 | 112 | 112.00 | +79.75 | −185.28 | +344.78 | 21.379 | 0.925 | Inconclusive |
| L1,2,卖,σ−1 | 72 | 26.99 | +140.88 | +18.18 | +263.58 | 4.493 | 0.965 | Inconclusive |

**全局 INCONCLUSIVE**（`decontam::global_verdict`：无 VALIDATED ∧ 存在 INCONCLUSIVE）。**V=0 / F=0 / I=29**。

**★主桶为何未翻转（powered 但仍 Inconclusive）**：VALIDATED 四条件 `μ̂>0 ∧ perm_p<0.05 ∧ LCB>0 ∧ powered` 表面看 mean=+39.67>0、lcb=+21.88>0、perm_p=0.040<0.05——但 `powered ⟺ n_eff≥(1.645·CV)²`：池化 CV 从 BTC 单标的 10.357 涨到 **22.582**，功效门 `(1.645·22.582)²≈1380 > n_eff 699.74` ⟹ **¬powered** ⟹ Inconclusive。池化把 n 增了 4.3 倍，但跨品种尺度异质性把 CV 也抬高 2.2 倍，功效门（∝CV²）涨 4.75 倍，快于 n 增速 ⟹ 净欠功效。这正是 codex #85 边界条件 (d)「跨标的池化引入不可接受的 symbol 异质性」的坐实。

---

## 3. ★per-symbol 分解：主桶 alpha 是 BTC 独有（全表见 /tmp/wv_xsym_persymbol.md）

主桶 L0/type3/买/σ0 逐品种：

| symbol | n | n_eff | mean(Y) | LCB | UCB | perm_p | state |
|---|---|---|---|---|---|---|---|
| **BTC** | 1597 | 166.42 | **+178.66** | +102.49 | +254.82 | 0.005 | Inconclusive（欠功效） |
| ES | 1120 | 164.94 | **−6.73** | −9.15 | −4.30 | 0.995 | **Falsified** |
| CL | 1034 | 217.53 | −0.05 | −0.12 | +0.02 | 0.605 | Inconclusive |
| GC | 1099 | 220.65 | **−3.33** | −4.45 | −2.21 | 0.995 | **Falsified** |
| BRN | 790 | 602.34 | −0.08 | −0.27 | +0.11 | 0.250 | Inconclusive |
| DX | 632 | 128.12 | −0.04 | −0.07 | −0.01 | 0.995 | Inconclusive |
| QQQ | 593 | 143.81 | **−2.81** | −3.22 | −2.40 | 1.000 | **Falsified** |

**读出**：主桶正 alpha **仅 BTC**（+178.66）。其余 6 品种主桶全负，其中 ES/GC/QQQ 是 powered-FALSIFIED（LCB<0 ∧ UCB<0，有功效地判 μ≤0）。池化 mean +39.67 = BTC 残差在原始-$ 单位下（价 ~$30-100k，逐笔涨跌数百美元）压过 ES 点/CL 美元/QQQ 美元（数量级差 1-2 位）的**尺度伪影**——不是 7 品种共有的 alpha。若各品种残差不通约（本冻结用原始-$，未归一化），拼接均值被最大尺度品种支配。

---

## 4. 基线差分（vs BTC 单标的 s3）

| 口径 | 主桶 n | n_eff | mean | LCB | perm_p | 全局 | 主桶 state |
|---|---|---|---|---|---|---|---|
| BTC 单标的（s3，4 元组残差） | 1597 | 166.42 | +178.66 | +102.49 | 0.005 | INCONCLUSIVE V=0/F=3/I=25 | Inconclusive（欠功效） |
| **7 品种池化（本工位）** | 6865 | **699.74** | +39.67 | +21.88 | 0.040 | INCONCLUSIVE **V=0/F=0/I=29** | **Inconclusive（CV 膨胀反超功效门）** |

**差分读出**：池化增 n（166→700 过 290 门）机制生效，但 (1) CV 从 10.36→22.58 使功效门（∝CV²）反超 n 增益 ⟹ 主桶仍欠功效；(2) per-symbol 揭示正号 BTC 独有，池化正 mean 是尺度伪影。**裁定 C 的功效补救在原始-$ 拼接口径下不足以把主桶推向 VALIDATED**。

---

## 5. 边界条件（结论翻转）

- **主桶 INCONCLUSIVE→VALIDATED**：须残差**跨品种归一化到通约单位**（收益率/夏普/固定 horizon 归一，BTC 冻结 §3.3 提功效链第 1 项）使 CV 降到 `n_eff≥(1.645·CV)²`——但这是**新 estimand（换 judged 量），须新预注册**（不得看本结果后 post-hoc 改口径 = 数据挖掘，预注册 §5 / no-workaround）。本工位**不 post-hoc 改口径**，如实报告原始-$ 拼接的否定性结果。
- **主桶→跨品种 FALSIFIED**：若归一化后各品种主桶仍分散（BTC 正、其余负），则「主桶 alpha 是 BTC 独有、非缠论买卖点普适 edge」被坐实——per-symbol 表已强烈暗示此向。
- **codex #85 边界 (d) 触发**：跨品种 symbol 异质性（原始-$ 尺度不可通约）确实使 (C) 的原始口径退化——按裁定 (d)「(C) 应暂停，退回 (A) 或另寻路径」，下一步 = 归一化新预注册（新 estimand），非本冻结可推进。

---

## 6. 认识论等级 / L3 诚实声明（231）

- **L3 真去污**：per-symbol B̂_i 由各品种自身扩张窗漂移独立估（`build_mu_from_bars`），7 品种各剥各自 beta，减法非退化。跨品种置换 stratum 按 SYMBOL_STRIDE 隔离（无跨品种 δ shuffle 混 beta）。
- **否定性结果价值（231）**：全局 INCONCLUSIVE（V=0）+ per-symbol BTC 独占正号——缩小有效域边界：**缠论主桶 alpha 在原始-$ 跨标的池化口径下不成立，且强烈暗示 BTC 独有**。比确认性结果信息量高。
- **level≥2 桶 frontier 污染标注（预注册 §4.1，Lead 强制）**：所有 ℓ≥2 桶（L2-L5，18 桶）挂 `frontier-bt-consumed-20260702.md` 污染未排除标注（增量/全量 level≥2 真发散，#87 在审）——**不作独立否证/确认**。ℓ∈{0,1} 不受影响（50K bit-exact）。本结论（主桶=L0）落在 ℓ0，不受 frontier 污染。

---

## 7. 下游推论

- **acceptance 跨标的 L3 = INCONCLUSIVE**：原始-$ 拼接口径下无 confirmed 跨品种 alpha。裁定 C 的功效补救机制生效（增 n）但被跨品种尺度异质性抵消。M1 里程碑不得以本 estimand 的跨标的 alpha 存在性为通行凭证。
- **下一 goal 候选（新预注册）**：残差归一化（收益率/夏普尺度）后重跑跨标的池化——通约单位消除 BTC 尺度支配 + 降 CV。**须新 estimand 新冻结**（judged 量改变），不复用本冻结。
- **full-z 延后维持**（codex #85）：本轮跨标的池化未产 VALIDATED ⟹ full-z 加维仍无功效动机，延后不变。
- **BTC 独占正号的诊断**：per-symbol 表暗示主桶正 alpha 可能是 BTC-idiosyncratic（beta 漂移伪结构？对照 oddeven_mu_identity 谱系）——归一化重跑可判别。

---

## 8. 结果包六要素

1. **结论**：跨标的 L3 池化（7 品种原始-$ 拼接）= INCONCLUSIVE（V=0/F=0/I=29）；主桶 n_eff 166→700 过功效门但 CV 10.36→22.58 反超 ⟹ 未翻转；per-symbol 揭示主桶正号 BTC 独有（6/7 品种负，ES/GC/QQQ Falsified），池化正 mean 是 BTC 尺度伪影。
2. **定义依据**：可交易 alpha μ(z)>0（663）；三态判据 + 功效门 `n_eff≥(1.645·CV)²`（667）；桶键 4 元组先验给定（665）；跨品种 B̂_i 独立 ⟹ L3 真去污（预注册 §2）。
3. **边界条件**：残差跨品种归一化（通约单位）→ 降 CV → 可能翻转，但须**新预注册**（新 estimand，防 post-hoc 口径挖掘）；codex #85 边界 (d) 触发（symbol 异质性）。
4. **下游推论**：跨标的原始-$ 口径无 confirmed alpha；归一化重跑为下一新预注册 goal；full-z 延后不变；BTC 独占正号待归一化判别 idiosyncrasy。
5. **谱系引用**：codex #85 裁 C + 边界 (d)；663/665/667（判据/去污/功效门）；231（否定性结果缩边界 + level≥2 有效域）；预注册 §2/§3/§4.1/§5；s3 strict-alpha-retest（BTC 基线）；frontier-bt-consumed（ℓ≥2 污染）；oddeven_mu_identity（BTC beta 漂移伪结构参照）。
6. **影响声明**：新增本结果包 + `wverify_run.rs`（SYMBOL_STRIDE/L3_UNIVERSE 常量 + walk_forward_oos_residuals symbol_index 参数 + bucket_verdict helper + wverify_cross_symbol 测试）。BTC 单标的 wverify_full 传 index=0 保持 bit-exact。**未改**：perm_test.rs（裁定明令）、decontam、prereg_windows、l3_delta_r_alpha 残差逻辑、预注册冻结文档。护航 1402 测试基线不退化。
