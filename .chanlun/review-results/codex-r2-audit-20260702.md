# codex 异质复审 r2：n_eff 校正落实 + 全窗 VALIDATED 存活 + FWER 临界（task #16）

- 工位 ws-codex-r2 | 审计对象 `wverify-full-domain-20260702.md`（全 OOS 窗定版，锚 d906648041）+ `wverify-alpha-r2-20260702.md`（ws-neff task#15 r2，锚 bb21146d9）
- 前审基底：`codex-alpha-audit-20260702.md`（AUDIT_DOWNGRADE，三缺口：①FWER 未注、②32k 前缀非全域、③n_eff 用 raw n——致命）
- codex 交互原始记录：`.chanlun/review-results/codex-review-20260702-0529.md`（CLI 自动持久化，review 模式）

## Verdict：AUDIT_DOWNGRADE（复审确认，非撤回）

n_eff 自相关校正**已真实落实**（前审缺口③已修复），32k 截窗**已消除**（前审缺口②已修复）。但复审发现 VALIDATED 判定仍不稳健，三处新/延续问题：n_eff 实装与文档声称的 Geyer IPS 方法有偏离且偏离方向不利于结论稳健性、功效门临界余量过窄、FWER 在 R=200 分辨率下无法确证。**当前不应维持全局 Pass，应改为条件性/待定表述。**

## 三问逐一裁决

### Q1：n_eff 实装质量 —— `needs_work`（Geyer IPS 保真度：`reject`）

`decontam.rs::effective_n` 用**单 lag 首次非正即截断**（`if rho <= 0.0 { break; }`），这不是标准 Geyer 初始正序列（IPS）法——标准 IPS 对**配对**自相关 `Γ_m = ρ_{2m}+ρ_{2m+1}` 截断，目的是防止中间一个噪声导致的负相关提前掐断仍显著为正的尾部。当前实装：若中段出现单个噪声负 ρk，会提前截断，**低估 Σρk ⟹ 高估 n_eff**——这正是对本次 VALIDATED 结论**不利的方向**（偏向让边际样本"看起来更 powered"）。这是代码 docstring 声称的方法（"Geyer 初始正序列估计"）与实际实装之间的忠实度缺口，非误读——docstring 与实装不匹配本身就是问题。

次要：无 lag 上限（`for k in 1..n`），对 n=1714 尚未致命但缺乏 bandwidth 保护，远 lag 的 `ck` 基于极少样本对估计，噪声可能污染 `Σρk`；`c0==0.0` 只挡严格常数序列，未做 NaN/Inf 入口校验。均为"重要/建议"级，非致命但应修。

### Q2：158.71 vs 145.67 临界余量 —— `needs_work`

功效门槛 `(1.645×7.337)²≈145.68`，n_eff=158.71，余量仅 **8.86%**。换算到 Σρk 空间：当前点估计 Σρk≈4.900，只要多估出 **≈0.48**（相对当前估计约 10%）即跌破门槛、VALIDATED 翻转为 Inconclusive。`effective_n` 是**无置信区间的点估计**，Q1 指出的方法偏离恰好可能贡献这个量级的低估。LCB>0 与 perm_p 本身不受 n_eff 影响，但"是否有检出力"（powered 判定）完全悬在这个窄余量上——这是 r2 报告未量化的脆弱性，复审新增。

`neff/nraw=0.093`（比 665 号先例 0.21 更强聚集）应明确写成"强 regime 依赖"，不能只用 raw n 从 44→1714 的增长叙事支撑稳健性——约 10.8 笔原始交易才抵 1 个独立观测。

### Q3：FWER 临界性 —— `reject`（VALIDATED 的 FWER-corrected 声明不成立）

- `0/200` 置换加一平滑 `p≈0.004975`，Bonferroni 阈值（11 桶）`α/11≈0.004545`——名义上临界失守（差 0.00043）。
- **新增关键量化**：R=200、0 次超越的单侧 95% Clopper-Pearson 置信上界 **≈0.0149**（rule-of-three 量级，`1-(0.05)^{1/200}`），是 Bonferroni 阈值的 **3.3 倍**。这意味着在 R=200 分辨率下，**无法以统计置信度断言真实 p 值确实低于 Bonferroni 阈值**——不是"临界失守"这么简单，而是"当前分辨率下 FWER 判定本身不可确证"。
- BH-FDR 在"11 桶中仅 1 桶名义显著"场景下第一阶阈值与 Bonferroni 相同（`0.004545`），不构成逃逸路径。
- R=200 是预注册 §4 冻结常量，不可更改（改 R 违反预注册纪律）。故**唯一诚实结论**：当前数据在 R=200 下无法给出确定性的 FWER-corrected VALIDATED 判定。

## 简化质询（codex 否定是否成立）

- **是否误读上下文**：否。Q1/Q2/Q3 均基于贴出的真实代码/数值做计算，数学可独立复核（Σρk=1714/158.71 反推、CP 上界公式均验证一致）。
- **是否已被其他机制覆盖**：否。现有 `effective_n_autocorr_correction` 单测只验证"正相关降低 n_eff / 反相关不降低"的定性方向，未针对 Geyer IPS 保真度或临界余量做任何断言，FWER 校正完全未接入 `classify_bucket`。三处均是真实空位。
- **严重性判定是否合理**：合理。geyer_ips_fidelity 与 fwer_validated_claim 判 `reject`（结构性缺口，非润色），n_eff margin 判 `needs_work`（脆弱但非当场证伪）——分级与前审 AUDIT_DOWNGRADE 一致的严重性梯度相符，未见夸大。

**结论：codex 三点否定全部成立，AUDIT_DOWNGRADE 复审确认。**

## 结果包六要素

1. **结论**：verdict=AUDIT_DOWNGRADE。全局裁决建议从 `Pass` 改为条件表述：`n_eff raw→校正 + 32k→全窗两项前审缺口已修复；但 (a) effective_n 实装偏离其声称的 Geyer IPS 方法且偏离方向不利于稳健性，(b) VALIDATED 桶功效门余量仅 8.86%（Σρk 误差容限≈0.48）对方法偏离敏感，(c) FWER 在 R=200 分辨率下 Clopper-Pearson 上界(0.0149)远超 Bonferroni 阈值(0.004545)，不可确证。故当前不应声明确定性 VALIDATED/Pass，应收口为"名义正效应，功效边际，FWER 未可确证"（codex 建议态：`NOMINAL_POSITIVE / POWER_BORDERLINE / FWER_UNRESOLVED_AT_R200`）。
2. **定义依据**：预注册 §1.3 n_eff 定义（Σρk 自相关校正）、§3.1 powered 门槛 `n_eff≥(z_α·CV)²`、§4 R=200/种子冻结。`decontam.rs::effective_n` 的实装与 docstring 声称的"Geyer 初始正序列估计"方法学定义不完全匹配（单 lag vs 配对 lag 截断）。
3. **边界条件（结论翻转）**：若改用标准配对 IPS（`Γ_m=ρ_{2m}+ρ_{2m+1}`）重算 Σρk 后 n_eff 仍 ≥145.68 ⟹ Q1/Q2 顾虑解除，VALIDATED 站住（仍需单独处理 Q3 FWER）；若预注册允许追加正交置换批次（不算改 R，而是独立验证批次的元分析）使加一平滑 p 显著低于阈值 ⟹ Q3 解除。三者任一不满足则维持 AUDIT_DOWNGRADE。
4. **下游推论**：acc-alpha acceptance 工位不应以本轮结果结案为 Pass；建议后续工位（若排期）：(a) 重实装配对 IPS 版 `effective_n` 并重跑 VALIDATED 桶，(b) 把 FWER（Holm/Bonferroni）正式接入 `classify_bucket` 而非仅报告层注记，(c) 若两项修复后仍 VALIDATED 存活，方可去掉"条件性"表述。
5. **谱系引用**：665（neff/nraw 先例，本轮 0.093 更强聚集）、667（powered/LCB underpowered 判据）、231（有效域≠定义域，声明膨胀禁止——本次是"实装与其声称的方法学定义不完全匹配"导致的潜在膨胀）、090（严格性语法规则，docstring 与实装须一致）。
6. **影响声明**：新增本文件；不改动代码/定义。原始 codex 交互见 `.chanlun/review-results/codex-review-20260702-0529.md`。建议下游修复落点：`rust/src/theta_v0/backtest/decontam.rs`（`effective_n` 改配对 IPS + lag 上限 + NaN 入口校验）、`rust/src/theta_v0/backtest/wverify_run.rs`/`decontam.rs::classify_bucket`（FWER 校正正式接入判据，而非仅报告层旁注）。
