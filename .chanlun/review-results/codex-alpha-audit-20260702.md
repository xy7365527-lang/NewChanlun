# codex 异质审计：W-VERIFY acc-alpha L2 BTC 定版跑批（task #4）

- 工位 ws-codex-alpha | 审计对象 `.chanlun/review-results/wverify-alpha-20260702.md`（commit 7682aa4024，锚 d906648041）
- codex 交互原始记录：`.chanlun/review-results/codex-review-20260702-0458.md`（CLI 自动持久化）

## Verdict：AUDIT_DOWNGRADE

全局裁决应从 `Pass` 降级为 `Inconclusive`，且需在补齐 n_eff 校正前不得再声明 VALIDATED。

## 四面向审计结果

### (1) 单桶 Pass 的多重检验稳健性 —— 部分成立，非致命
5 桶同测，唯一 VALIDATED 桶 perm_p 报告为 `0.000`（应更精确写作 `0/200`，加一平滑后 ≈0.00498，不是真 0）。按 Bonferroni（5 桶，α/5=0.01）该桶仍过关。但"先验桶键排除选择偏差"（预注册命题①）解决的是"桶怎么定义不挑收益"，不解决"5 桶同时做显著性判断的 family-wise error"——两者是独立问题，预注册未显式给出 family 校正口径。**非致命，但预注册口径有缺口，需补注**。

### (2) O(n²) 截窗有效域 —— 成立，声明须限定范围
32000 bar 是全 OOS（461 万 bar）的 0.7%，且是**连续前缀**（OOS 窗最早一段），不是随机/分层抽样。结论最多能写"2023-01-01 起前 32000 bar 前缀内观察到候选正效应"，不能代表整个 2023-2025 OOS 窗——连续前缀天然绑定早期 regime。**当前工位产出文件已用"截 32000 bar/非全窗结论"标注，方向正确，但"有效域内超 beta alpha"的措辞仍需明确限定为该前缀时段**。

### (3) n_eff 替代问题 —— 致命，核心发现
预注册 §1.3 明确要求 `powered` 门槛的 `n_eff` 是**事件聚集/自相关校正后**的有效样本数（"Σρk 自相关校正，665 neff/nraw"）。生产代码 `rust/src/theta_v0/backtest/wverify_run.rs`（`decontam::classify_bucket(mean, lcb, ucb, perm_p, n as usize, cv, za, pa)`）直接把**原始逐笔成交笔数 n**喂给 `n_eff` 形参，代码库内这条链路没有任何自相关校正实现。

同代码库内有直接先例：665 号谱系对同一 BTC/bsp 信号族实测 `neff/nraw=0.21`（Σρk=1.88，同方向三类买卖点信号时间聚集）。唯一 VALIDATED 桶（L0, bsp_class=3, δ=+1, n=44, CV=3.189）的 powered 门槛 = `(1.645×3.189)²≈27.52`：
- 若 `neff/nraw≈0.21` 适用 → `n_eff≈9.2 < 27.52` → **不 powered**
- 即使打五折 `n_eff=22 < 27.52` → 仍 **不 powered**

按预注册 §3.1 三态规则，不 powered ⟹ 直接掉入 INCONCLUSIVE，不可能是 VALIDATED。**这是预注册规格与生产实装之间的真实缺口，不是数据本身否证——必须先补 n_eff 校正实装，再重新跑批分类，当前 VALIDATED 判定不能成立。**

### (4) "既有否证未被推翻"声明一致性 —— 措辞不够，需加强
工位自我标注"非稳健可交易声明……既有否证未被推翻"。这个谦抑措辞本身没错，但不够——问题不是"Pass 但不稳健"，而是**支撑 Pass 的唯一 VALIDATED 桶的 powered 判定本身没有按预注册实装**，因此当前 `Global verdict: Pass` 不应该成立。既有否证（665/666/667）确实未被推翻，当前结果只能作为"待 n_eff 校正补齐后复核的候选"。

## 结果包六要素

1. **结论**：verdict=AUDIT_DOWNGRADE。`wverify-alpha-20260702.md` 的全局裁决应从 Pass 改为 Inconclusive：`当前仅为 BTC OOS 前 32000-bar 连续前缀上的候选正效应；唯一 VALIDATED 桶未使用预注册要求的自相关/事件聚集校正 n_eff，故不得声明 VALIDATED 或 Pass。须补实现 n_eff 校正、明确多重检验 family，并在全窗或分层窗口复核后再判。`
2. **定义依据**：预注册 §1.3 对 `n_eff` 的定义（"事件聚集校正后有效样本数，Σρk 自相关校正，665 neff/nraw"）与 §3.1 powered 门槛定义（`n_eff≥(1.645·CV)²`）。生产代码 `decontam.rs::powered`/`classify_bucket` 的 `n_eff` 形参在 `wverify_run.rs` 调用点被喂原始 `n`，未满足定义。
3. **边界条件（结论翻转）**：若补实现自相关校正后，L0 δ+1 桶的实测 `n_eff` 仍 ≥27.52（即该信号族的事件聚集程度显著弱于 665 号 L0 卖桶的 Σρk=1.88），则 powered 判定可能恢复成立，VALIDATED 可能重新站住——但这需要**真实校正实装 + 重新跑批**，不能靠折算估计代替。
4. **下游推论**：acc-alpha acceptance 工位当前不应以 Pass 结案；task #3/工位声明的"L0 三买 VALIDATED"论断需撤回或降级为"待 n_eff 校正复核"。这也是一个可复用的代码层缺口——凡是复用 `decontam::classify_bucket` 的其他跑批（若未来有）都要检查调用点是否同样把 raw n 当 n_eff 喂入。
5. **谱系引用**：665（neff/nraw=0.21 先例，同 BTC/bsp 信号族）、667（powered/LCB≤0 三态判据来源）、231（有效域≠定义域，声明膨胀禁止——本次正是"生产代码未落实预注册定义"导致的声明膨胀）。
6. **影响声明**：新增本文件 `.chanlun/review-results/codex-alpha-audit-20260702.md`；不改动代码/定义。建议后续工位：在 `wverify_run.rs`/`decontam.rs` 调用链路实装自相关校正 `n_eff`（`f64` 类型，非 `usize`——codex 附带建议），补跑后重新产出裁决。
