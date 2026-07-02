# W-VERIFY acc-alpha 最终数字：Geyer 配对 IPS 校正后全窗重跑（task #17，ws-geyer）

- 工位 ws-geyer | 修复对象 `rust/src/theta_v0/backtest/decontam.rs::effective_n`（codex-r2-audit Q1 处方）
- 处方来源 `.chanlun/review-results/codex-r2-audit-20260702.md`（AUDIT_DOWNGRADE 复审，Q1 geyer_ips_fidelity=reject）
- 前基线 `.chanlun/review-results/wverify-alpha-r2-20260702.md`（单 lag IPS，L0/3/+1 VALIDATED，n_eff=158.71）

## 收口态：verdict = **Inconclusive**（V=0 F=0 I=11）

```
WV_FULL trades=3602 buckets=11 verdict=Inconclusive V=0 F=0 I=11 levels=[0,1,2,3,4,5]
```

配对 IPS 校正把**唯一的 VALIDATED 桶（L0/3/+1）翻为 Inconclusive**——校正方向与 codex Q1 预测一致
（不利于 VALIDATED），且幅度超过 Q2 量化的容限（ΔΣρk=+0.86 > 0.48），恰好把 n_eff 推下功效门。

## 修复内容（单 lag → 标准 Geyer 配对 IPS）

| 项 | 修复前（单 lag） | 修复后（Geyer 配对） |
|----|----------------|--------------------|
| 截断准则 | `if ρk≤0 { break }`（单 lag 首非正） | `Γ_m=ρ_{2m}+ρ_{2m+1}`，首个 `Γ_m≤0` 截断 |
| 病理 | 中段单个噪声负 ρk 提前掐断正尾 ⟹ 低估 Σρk ⟹ **高估 n_eff** | 配对存活正尾 ⟹ 捕获更多正相关 ⟹ 更保守 n_eff |
| lag 上限 | 无（`1..n`，远 lag 少样本对噪声） | `⌊n/2⌋`（每个 ρk ≥⌊n/2⌋ 样本对） |
| 入口校验 | 仅 `c0==0` 挡常数 | 追加非有限值 ⟹ NaN 上浮（下游 powered→false→Inconclusive） |
| clamp | 靠单 lag 截断隐式保 Σρk≥0 | 显式 `τ=max(1+2Σρk, 1)` 保 n_eff≤n |

单测：`geyer_paired_ge_single_lag_on_noise_dip`（保真度方向，配对 Σρk≥单 lag，噪声 dip 构造），
`effective_n_autocorr_correction` 追加 NaN 上浮断言。decontam 7/7 绿。

## 逐桶三态表（全 OOS 窗 2023-01-01…2025-06-30，3602 笔，11 桶）

| ℓ | bsp | δ | n_raw | n_eff | μ̂ | LCB | UCB | CV | perm_p | 功效门(1.645·CV)² | state |
|---|-----|---|-------|-------|-----|-----|-----|-----|--------|------------------|-------|
| L0 | 3 | −1 | 1503 | 355.55 | −73.49 | −129.21 | −17.77 | 17.869 | 1.000 | 864.04 | Inconclusive |
| **L0** | **3** | **+1** | **1714** | **136.81** | **+259.64** | **+183.95** | **+335.34** | **7.337** | **0.000** | **145.67** | **Inconclusive** |
| L1 | 2 | −1 | 132 | 116.09 | +75.07 | −96.19 | +246.33 | 15.933 | 0.190 | 686.95 | Inconclusive |
| L1 | 2 | +1 | 132 | 86.88 | −67.47 | −258.59 | +123.65 | 19.784 | 0.810 | 1059.16 | Inconclusive |
| L2 | 2 | −1 | 46 | 28.44 | +62.08 | −192.57 | +316.74 | 16.912 | 0.645 | 773.97 | Inconclusive |
| L2 | 2 | +1 | 48 | 48.00 | +190.58 | −111.85 | +493.01 | 6.683 | 0.355 | 120.86 | Inconclusive |
| L3 | 2 | −1 | 9 | 9.00 | −90.39 | −355.83 | +175.05 | 5.355 | 0.525 | 77.60 | Inconclusive |
| L3 | 2 | +1 | 11 | 10.86 | −69.36 | −621.08 | +482.36 | 16.038 | 0.475 | 696.04 | Inconclusive |
| L4 | 2 | −1 | 4 | 4.00 | −149.60 | −308.44 | +9.24 | 1.291 | 0.770 | 4.51 | Inconclusive |
| L4 | 2 | +1 | 2 | 2.00 | +589.19 | −1017.26 | +2195.63 | 2.344 | 0.270 | 14.87 | Inconclusive |
| L5 | 2 | −1 | 1 | 1.00 | −9.93 | NaN | NaN | NaN | 1.000 | — | Inconclusive |

## L0/3/+1 桶的翻转机制（唯一曾 VALIDATED 桶）

| 量 | 单 lag（r2） | Geyer 配对（终跑） | Δ |
|----|-----------|------------------|---|
| n_eff | 158.71 | **136.81** | −13.8% |
| τ=n_raw/n_eff | 10.80 | 12.53 | +1.73 |
| Σρk=(τ−1)/2 | 4.900 | **5.764** | +0.864 |
| 功效门 (1.645·7.337)² | 145.67 | 145.67 | 0 |
| powered (n_eff≥门) | ✅ 余量 8.86% | ❌ **短 8.86**（136.81<145.67） | 翻转 |
| state | Validated | **Inconclusive** | ¬powered |

μ̂=+259.64、LCB=+183.95>0、perm_p=0/200<0.05 三项仍成立——去污后点估计名义为正且去污显著；
唯一失守的是**功效门**（校正 n_eff 落到门下）。故收口态用 codex 建议措辞：

- **NOMINAL_POSITIVE**：L0/3/+1 μ̂>0 ∧ perm_p<perm_α ∧ LCB>0（名义正效应，去污显著）。
- **UNDERPOWERED**（非 r2 的 POWER_BORDERLINE）：校正 n_eff=136.81 < 门 145.67 ⟹ ¬powered。
  r2 的 8.86% 正余量在配对 IPS 下反号为 8.86% 负缺口——脆弱性兑现（codex Q2 预测）。
  依 667/231：¬powered 的 `LCB>0` 不足以断言可交易 alpha，判 Inconclusive，不冒充 VALIDATED。

## FWER@R=200 注记（R 冻结不可改，codex Q3）

无 VALIDATED 桶 ⟹ 无 FWER-corrected 声明需要维护，Q3 对本轮结论**不再构成活跃约束**。存档记录：
即便功效门通过，R=200、0/200 的单侧 95% Clopper-Pearson 上界 ≈0.0149，是 Bonferroni 阈（α/11≈0.004545）
的 3.3 倍——R=200 分辨率下 FWER 判定本身不可确证。R 为预注册 §4 冻结常量，本工位不改。

## 结果包六要素

1. **结论**：acc-alpha 全 OOS 窗、Geyer 配对 IPS 校正后 verdict=**Inconclusive**（0 VALIDATED / 0 FALSIFIED / 11 INCONCLUSIVE）。无超 beta 可交易 alpha 被认证；最强桶 L0/3/+1 为 NOMINAL_POSITIVE 但 UNDERPOWERED。**不 Pass、不 161**——走预注册 §3.3 提功效重估路径。
2. **定义依据**：预注册 §1.3 `n_eff=n/(1+2Σρk)`、§3.1 `powered ⟺ n_eff≥(z_α·CV)²`、§3 三态；`effective_n` 现实装标准 Geyer 配对 IPS，docstring 与实装对齐（消除 codex Q1 忠实度缺口）。
3. **边界条件（结论翻转）**：若 L0/3/+1 桶原始笔数增至使校正 n_eff≥145.67（当前需 Σρk≤4.90，即事件聚集减弱或样本增加），且 perm_p 仍<0.05、LCB 仍>0 ⟹ 该桶重回 VALIDATED、verdict 翻 Pass（届时须单独处理 Q3 FWER）。反之维持 Inconclusive。
4. **下游推论**：acc-alpha acceptance 工位本轮结案为 **Inconclusive（非 Pass）**。当前策略在校正 n_eff 下无法以功效证据支撑可交易 alpha 声明。若排期后续：(a) 提功效（更长窗/更多标的降 neff/nraw=0.093 强聚集），(b) FWER 正式接入 `classify_bucket`（当前仅报告层）——但仅在功效门先通过时才有意义。
5. **谱系引用**：667（powered/LCB 判据，¬powered 的 LCB>0 不断言 alpha）、665（neff/nraw=0.080 强 regime 聚集，比 r2 的 0.093 更强——12.5 笔原始交易抵 1 独立观测）、231（有效域≠定义域，实装须匹配声称方法学）、090（严格性：docstring 与实装一致）、161（照实收口，VALIDATED 翻转如实报告）。
6. **影响声明**：改动 `rust/src/theta_v0/backtest/decontam.rs`（`effective_n` 配对 IPS + lag 上限 + NaN 校验 + 显式 clamp；新增私有 `geyer_paired_sum`；更新 docstring；追加 2 项单测断言）。不改预注册常量、不改 R、不改判据结构。新增本报告。数据 `analysis/data_cache/btc_1m_full.json` 全 OOS 窗，未截断。
