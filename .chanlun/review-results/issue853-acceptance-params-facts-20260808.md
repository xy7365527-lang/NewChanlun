# #853 新验收判据参数 —— 裁定前事实面勘察（只读，不裁）

- issue: #853（`wayfinder:grilling`，map #787，由 #833 毕业，blocked-by #833 已关，blocking #847）
- 日期：2026-08-08
- 性质：**只摆事实，不回答那五问。** 凡遇取舍处一律标「⚖ 此处是取舍，归 #853 裁」。
- 纪律：每条断言附 `file:line`，均已逐条打开确认。凡「唯一 / 全部 / N 处」附检索式与覆盖目录，举不出的写「已查到的有 N 处」。查不到的写「查不到」，不写「不受影响」（090）。
- 基线：worktree HEAD 已 `git reset --hard main` → `d77de6bc43`。

---

## 0. 开工前搜索（先搜再开，结果照录）

| 检索 | 结果 |
|---|---|
| `gh issue view 853` | 已读全文（含 2026-08-05 #907 移交条目），本件 §E 引其原文 |
| `gh issue list --search "阈值 OR 置信下界 OR walkforward OR LCB OR 成本轨迹" --state all --limit 30` | 30 条命中中与本票直接相关的只有 #853 本身、#833（已关，产出 ADR 0015）、#907（已关，探针）、#915（已关，探针）、#847（SPEC，S8）、#424（walkforward 补口径声明，已关）、#392（七处统计实现清点，已关）、#410（`filter_gamma` 第三处手搓 `chi_t(mu_lcb(...))`，**仍 OPEN**） |
| `ls .chanlun/review-results/ \| grep -iE "0015\|cost\|lcb\|walkforward\|907\|853"` | 唯一命中 `issue907-cost-gate-level-20260805.md` |

**已在册、本票不必重推的部分**：

1. **逐级「波幅 ÷ 往返成本」的全部读数**已由 #907 测完并落盘（`.chanlun/review-results/issue907-cost-gate-level-20260805.md`），本件 §E 全文转录，**不要重跑探针**。
2. **逐级信号频率**已由 #915 测完（`.chanlun/review-results/issue915-capacity-frequency-20260805.md`），跨 8 标的，本件 §C 转录。
3. **闸门是否触达**已有生产代码自陈 + BTC 全史单测断言（§D），**不需要新写探针**——本次因此**未新增任何 bin**，`rust/Cargo.toml` 未改。
4. **统计口径标注规范**已是仓内正本（`docs/agents/stat-provenance.md`），本票只需引，不需另立。

---

## A. ADR 0015 那把尺子在代码里是什么

正本：`docs/adr/0015-cost-trajectory-acceptance-metric.md`（185 行，本次全文读毕）。

### A.1 「成本轨迹」的实装现状 —— 账本有，**速率没有**

- **成本基（cost basis）账本：实装。**
  - `rust/src/recursive_t/t_engine.rs:245` `core_cost_basis: f64`；注释 `:242-244` 自陈「核心仓有效持仓成本 per share（缠师"成本"）……穿 0 触发退本金」。
  - 实际递减一行：`rust/src/recursive_t/t_engine.rs:381` `self.core_cost_basis -= realized / rem;`（`rem` = reduce 后剩余多头 units，`:379`）。
  - 空头腿不入成本：`rust/src/recursive_t/t_engine.rs:406-408`「短差腿单独核算（点5）：不入 cost_basis」。
  - 镜像实现 `rust/src/recursive_t/rec_engine.rs:1400`。
  - **生产链**（θ_v0）契约：`rust/src/theta_v0/strategy/ledger.rs:88-99` `enum TStage { CostReduction, CapitalRecovered, EarningShares }`，迁移律 `:76`「单向不可逆迁移（OQ-9）：CostReduction(0) → CapitalRecovered(1) → EarningShares(2)」。
  - 注：ADR 0015 自己在 `:157` 已登记过一次「主生产链没有成本账本」的**误判并当场订正**，指向 `theta_v0/strategy/ledger.rs:149` `TwState`。本件复核该订正成立。
- **「成本下降速率」这个标量：没有实装。**
  - 全仓唯一沾边的标量是**进度峰值**，不是速率：`rust/src/recursive_t/t_engine.rs:384-388`
    ```rust
    let drop = ((self.campaign_entry_cost - self.core_cost_basis)
        / self.campaign_entry_cost * 1000.0).max(0.0) as u64;
    self.res.max_core_gain_x1000 = self.res.max_core_gain_x1000.max(drop);
    ```
    注释 `:382` 自陈是「诊断：降成本进度（(entry−cost)/entry，=1 ⇒ 成本归0）」。**无时间/bar 分母，且取历史 max**，因此它不是 d(cost)/dt，连「每期降幅」都不是。
  - 其消费点已查到的有 4 处，全是打印/字段声明：`rust/src/recursive_t/rec_stream.rs:1650`、`rust/src/recursive_t/t_engine_run.rs:300`、`rust/src/recursive_t/rec_engine.rs:1138`、`rust/src/fugue_v3/layer.rs:113`。
  - 标识符 `cost_trajectory` / `cost_down` / `reduce_cost` / `退本金`（作标识符）：**查不到**（`退本金` 仅出现于中文注释，如 `t_engine.rs:391`、`:416`）。检索式 `grep -rn` 覆盖 `rust/src` `rust/tests` `analysis` `scripts`。
  - `grep "降成本速率|下降速率"` 于 `rust/src`：**零命中**。

**⟹ 对问 1 的事实面结论：成本下降速率是从零定义，不是改现有实现。** 现有的只有「成本基本身」（有）和「相对入场成本的累计降幅峰值」（有，但无时间维、取 max）。

### A.2 ADR 0015 自己定了什么 / 明确留白什么（逐条，不概括）

**已裁定的七条**：

| # | 裁定内容 | file:line |
|---|---|---|
| 一 | 验收尺子换掉：净值型 `E[ΔR]>0` / `Sharpe>0` 退场（否决对象即 `买卖点alpha2.pdf` 答复A §12 = p9–10 的 L3 判据），改用原文 (成本↓, 筹码↑)；依据三处原文明文 `048:22`／`068:136`／`103:164` | `docs/adr/0015-cost-trajectory-acceptance-metric.md:17-31` |
| 二 | 不是二维同时优化，是**分段一维**：闸门前判据＝**成本下降速率**；闸门后＝**净头寸价值**；全程配套＝**操作频率**。并明写「成本只在已实现盈亏落袋时变动；价格自身涨跌不改变成本」⟹ 成本轨迹天然剔除 beta | `:33-64`（三段判据表 `:37-41`；剔 beta 论证 `:47-51`；频率引 `049:74` 在 `:57`） |
| 三 | **不单独测 alpha**——成本能稳定下降本身即检验。alpha 定义改写为「成本下降速率中不能由手续费与随机性解释的那部分」。且明写**统计机器整套保留**（样本量、置信下界、walkforward、置换检验），**只换喂进去的被估量**（每期净值收益率 → 每期成本降幅） | `:66-81`（改写定义 `:74-75`；统计机器保留 `:77`） |
| 四 | 「成本降到 0」的 0 **扣手续费与资金费率**。理由：裁定三论证以此为地基；永续资金费率持续计提。代价明写「使成本降到 0 明显更难更慢」 | `:83-94` |
| 五 | 旧 INCONCLUSIVE 按 A/B 两类重新界定（A＝ΔN≡0 的操作，旧结论**不携带信息**；B＝真改变持仓量，旧结论**仍有效**）。决定性证据 `ΔN_t = 0 ⟹ ΔSharpe = 0` | `:96-119`（定理 `:100`；分类表 `:112-115`） |
| 六 | **参数必须在重算任何旧实验之前钉死**，理由是「换尺子重算」与「改判据改到数据好看」外形完全一致、事后不可区分 | `:121-127` |
| 七 | 跨标的筹码不需加总（ADR 0010 + 0013 裁定一的推论） | `:129-135` |

**ADR 0015 明确留白 / 未裁的**（`:163-169`「本 ADR 未裁」节逐条）：

1. **新判据的具体参数**——「成本下降速率的通过阈值、最小样本量、置信下界算法、频率判据的合成方式」→ 毕业成 #853。`:165`
2. **账本加持仓量分量 + 重证 TW 守恒/OQ-9**（丙-5 硬前置）→ SPEC #847 S8。`:166`
3. **甲段 7 条推导的有效域声明订正 + 丙段其余 5 条重证** → #847 S8。`:167`
4. **旧实验逐条归 A/B 类** → 不开票，「用到再判」。`:168`
5. **`chatgpt.com-推导完全分类-fpscreenshot (3).pdf` 的 OCR** → #787 Not-yet-specified。`:169`

**ADR 0015 自陈的举证缺口**（`:146-151`，本票裁定时应知）：

- `:148`「**『成本下降速率』这把尺子在 44 份 PDF 里一次都没出现**……本 ADR 的尺子来源只能是缠师原文（第 31/49/75 课），不得写成『形式化链已推出』。」
- `:149` 46 页截图 PDF 未 OCR，冲击面在该文件上是**空白**，非「已确认无影响」。
- `:150` 142 个 Lean 文件精读 8 个，其余靠 grep；**未跑 `lake build`**。
- `:151` 子代理未能核到 `049-第49课.md`，裁定二引的频率句由主控自查核实。

**A.3 与本票直接相关的一条硬前置**：ADR 0015 `:144` 记「裁定二那个公式的**分母（持仓量）在 Origin canonical base 里不存在**」——`TWState` 七字段（`formal/Origin/TotalWealth.lean:94-103`）无股数分量。即 `basis ← basis − d·π/units` 的 `units` 在形式化侧目前无处安放。**这归 #847 S8，不归本票**，但它意味着本票定的速率公式在 Lean 侧暂无对应载体。

---

## B. 现有的 walkforward LCB 那套

检索覆盖：`rust/src` `rust/tests` `analysis` `scripts` `docs`，检索式 `grep -rn "lcb\|LCB\|lower_bound"`、`grep -rni "walkforward\|walk_forward"`、`grep -rn "Wilson\|Clopper\|binomial\|二项"`、`grep -rn "1\.96\|1\.645\|2\.326"`。
**LCB 实现已查到的有 3 处**，全在 `rust/src/theta_v0/backtest/`。`rust/tests/` 零命中；`analysis/`+`scripts/` 唯一命中 `scripts/optimal_morse.py:299 betti_lower_bound`（Morse 理论，与统计无关）。

### B.1 实装在哪

- **walkforward 本体**：`rust/src/theta_v0/backtest/econ_positive.rs:4179` `fn acc_walkforward_trainonly()`；滚动多窗循环 `:4255-4295`（`k_windows` 默认 4，`wf_train_frac = 0.6`，train 段挑赢家类、OOS 段算 LCB，`:4292` `if lcb > 0.0 { lcb_pos_count += 1; }`）。
- **另一条 walk-forward 路**：`rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:336 build_walk_forward_mu`，由 `rust/src/theta_v0/backtest/wverify_run.rs:237 walk_forward_oos_residuals` 消费。

### B.2 LCB 界的是什么量

**是均值（μ̂ / mean return），不是胜率比例。**

- walkforward 路，`rust/src/theta_v0/backtest/econ_positive.rs:4579-4586`：
  ```rust
  let mu = pnls.iter().sum::<f64>() / n as f64;
  let var = pnls.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n as f64;
  let se = if neff > 1.0 { (var / neff).sqrt() } else { f64::INFINITY };
  (mu, se, mu - 1.645 * se)
  ```
  被估量是逐笔 `actual_pnl` 的样本均值。
- 选择器准入路，`rust/src/theta_v0/backtest/mu_estimator.rs:518-522`：
  ```rust
  pub fn mu_lcb(&self, class: &MuClass, z_alpha: f64) -> Option<f64> {
      let w = self.buckets.get(class)?;
      let std = w.std_sample()?; // n<2 ⟹ None
      Some(w.mean - z_alpha * std / (w.n as f64).sqrt())
  }
  ```
  文档行 `mu_estimator.rs:508`：`LCB(μ(z)) = mean − z_α·(std/√n) 单边置信下界（§12 实操选择器 χ_t 的准入量）`。

### B.3 用什么方法

**正态近似 + 样本标准差型标准误。没有 Wilson、没有 Clopper-Pearson、没有 Student-t。bootstrap 存在但只出 p 值、不产 LCB。**

- 分位由调用方传入，`rust/src/theta_v0/backtest/mu_estimator.rs:510`：`z_alpha 是单边正态分位（如 95%→1.645，99%→2.326）`。
- **硬编码 1.645 已查到的有 3 处**：`econ_positive.rs:4586`（注释 `:4573`「单侧 5% LCB = μ − 1.645·se，se=σ/√neff（PDF §7.3，neff 而非 nraw）」）、`highlow_mu.rs:113` `let (za, pa) = (1.645_f64, 0.05_f64);`、`decontam.rs:168`（测试常量）。模块头 `decontam.rs:13` 声明「§4 冻结（`z_α=1.645`、`perm_α=0.05`、功效门 `n_eff≥(1.645·CV)²`）」。
- **生产默认 `z_alpha = 0.0`**：`rust/src/theta_v0/config.rs:189-191`，注释明写「**default 0.0**（LCB=mean ⟹ n≥2 类退化回裸 μ 门…）」。⟹ **生产上 LCB 目前是关的**，退化成点估计。这与 #907 移交条目里「N-6 用点估计出事」的判例同构。⚖ 此处是取舍，归 #853 裁。
- bootstrap 位置：`econ_positive.rs:4598 block_bootstrap_pvalue`（只出 p 值）；`metrics.rs:646 percentile_ci` 是**双侧** 0.025/0.975 百分位区间（`metrics.rs:401`），与 LCB 分离。

### B.4 现有样本量门槛

**无统一 `n_min` 常量；已查到的有 3 个不同的门。**

1. **方差门 `n<2 ⟹ None`**：`rust/src/theta_v0/backtest/mu_estimator.rs:443-450`（`样本标准差 √(m2/(n−1))；n<2 ⟹ None`），语义 `:513-514`「n=1：标准差未定义 ⟹ None（**不**冒充 LCB=mean——单样本无方差信息）」。
2. **功效门 `n_eff ≥ (z_α·CV)²`**：`rust/src/theta_v0/backtest/decontam.rs:119-126`
   ```rust
   pub fn powered(n_eff: f64, cv: f64, z_alpha: f64) -> bool {
       let threshold = (z_alpha * cv).powi(2);
       n_eff >= threshold
   }
   ```
   派生说明 `:121-122`：「样本不足以把 z_α 倍相对噪声压到均值量级下 ⟹ ¬powered ⟹ LCB≤0 是无检出力的必然结果，不判纯 beta」。CV=10 时门槛 ≈ 270.6（`decontam.rs:175`）。**这是唯一一个写清了推导的门。**
3. **Le Cam 硬墙 `n<30`**（报告判定门，非 LCB 计算门）：`econ_positive.rs:5021`，说明 `:4802`「level3+ 全历史 n < 30 ⟹ 结构性稀疏（高级别本身低频，Le Cam 硬墙；有效域 < 定义域）」，`:4993`「n<30 ⟹ 结构性稀疏；n≥50 ⟹ 可 OOS 验」。

**`n_eff` 本身有两套互不相同的实现**：`econ_positive.rs:4548 neff_autocorr`（`neff = nraw/(1+2Σρ_k)`，只累加 ρ>0，lag_max=20）与 `decontam.rs:100-116 effective_n`（Geyer 式**成对**截断 `(2m, 2m+1)`，首个非正配对处停）。⚖ 若本票沿用现有那套，得先裁用哪个 `n_eff`——**归 #853**。

### B.5 ★ 关键：现有那套的假设里有没有依赖「双边分布」

**有，且不止一处。** 三个 LCB 实现全部建立在正态近似 + 二阶矩标准误上；它**不是** proportion 方法（无 Wilson / Clopper-Pearson / 二项）。逐条：

1. **正态分位假设写在文档里**：`mu_estimator.rs:510`「z_alpha 是单边正态分位（如 95%→1.645，99%→2.326）」。1.645 = 标准正态单侧 5%，依赖 μ̂ 抽样分布正态（CLT）；小 n 未换 t 分位。
2. **std-dev 型 SE 假定对称展布**：`mu_estimator.rs:521`、`econ_positive.rs:4581-4586`。二阶矩标准误对左右偏度不作区分——**右偏厚尾时实际左尾覆盖率会偏离名义 5%**。
3. **明确的对称双侧结构**：`mu_estimator.rs:526` 注释「与 `mu_lcb` **严格对称**——同 z_alpha、同标准误 std/√n，符号相反」；`mu_ucb` 在 `:536-540`。
4. **双侧判据实际在用**：`decontam.rs:142-148` 三态裁决同时消费 lcb 与 ucb：
   ```rust
   if powered && mu_hat > 0.0 && perm_p < perm_alpha && lcb > 0.0 { Validated }
   else if powered && lcb <= 0.0 && ucb <= 0.0 { Falsified }
   ```
   而 lcb/ucb 由 `highlow_mu.rs:122` 同一个 `za=1.645` 生成：`let (lcb, ucb) = (mean - za * se, mean + za * se);` ——**两端各用单侧 5% 的 z 拼成名义 90% 对称区间**。其 `Falsified` 分支的正确性依赖上下两侧覆盖率同时成立，**比纯 LCB 更强地依赖对称性/正态性**。
5. **一处口径不一致**：`highlow_mu.rs:121` 的 SE 用**原始 n**（`std / (n as f64).sqrt()`），`n_eff`（`highlow_mu.rs:130`）只喂功效门未进 SE；而 `econ_positive.rs:4582` 的 SE 用 `neff`。**同仓两条 LCB 对自相关的处理不同。**
6. 抗尾部手段全是**旁路的、非参的**，不改 LCB 本身：`drop_top_winners`（`econ_positive.rs:4590`）、`block_bootstrap_pvalue`（`:4598`）、`perm_test::drop_top_k_mean`（`highlow_mu.rs:131`）。

**⟹ 对问 3 的事实面结论**：#853 票面说「现有那套是为净值收益率设计的」——**属实，且更具体地说它是为『可正可负、近似对称、CLT 可用』的量设计的**。成本降幅在闸门前是单边（只降不升，除非亏损）⟹ 分布在 0 处有质点/截断，正态 SE 的左尾覆盖会失真。**这一点在代码里没有任何补偿**。⚖ 沿用还是另起，归 #853 裁。

### B.6 仓内统计口径正本对这套的现成判词

`docs/agents/stat-provenance.md`（70 行，全文读毕）**不规定算法，只规定标注与引用**：

- `:3`「管的是：一份归档判决的统计结论，在**被引用的那一刻**，能不能被人一眼判定『过没过异质审查』。**不规定必须用哪套统计算法**……本规范防的是『没标来源的结论获得与冻结口径同等的权威』。」
- `:41`「没标统计口径的归档，引用方**一律当「未审」降级使用，不得当决策依据**。」
- 两档判据 `:17-19`：轻档＝落盘进 `.chanlun/review-results/` ⟹ 须标口径一行字；重档＝**拿某结论当决策依据** ⟹ 引用方先查过没过异质审查，没过则当场补或标「未审」降级。
- **边界样本表直接点名本次对象**：
  - `:65`「| econ_positive::acc_walkforward_trainonly（报告落 :3371） | 触发 → **未标口径** | 触发 → **异质审查零覆盖**（#392 复核） | **两档皆越线** → 交 #395 |」
  - `:66` `acc_multilevel_highlevel_mu（:3770）` 同上
  - `:67`「| selector::chi_open_gate_lcb | 不触发（死代码，不产结论） |」
  - `:68`「| metrics::percentile_ci | 不触发（指标展示，非归档判决） |」

**⚠ 一处张力，照实登记**：`stat-provenance.md:67` 把 `chi_open_gate_lcb` 判为「死代码」，但 `rust/src/theta_v0/backtest/pi_bsp_timing.rs:486` 确有调用 `if !chi_open_gate_lcb(est, &z, theta, z_alpha, true, true, true)`。二者时序谁先谁后**本次未核**（涉 #394 / #410，#410 仍 OPEN）。**不据此下结论。**

---

## C. 频率判据

### C.1 判据（门）：**没有实装**

- rust/src 内 `frequency` 的命中只在探针 bin（见 C.2）；`signals_per` / `trades_per` / `rate_per_bar` 作**生产判据**：**查不到**。检索覆盖 `rust/src` `analysis` `scripts`。
- `rust/src/recursive_t/rec_stream.rs:103` 的「触发频率」是 556 解冻诊断打印，**不是仓位门**。
- 即：**生产代码里没有任何以频率为条件决定开/不开仓的分支。**

### C.2 测量探针：有

- `rust/src/bin/p915_capacity_frequency.rs`，口径定义 `:83-87`：`per_year_cal = n_bsp / (日历跨度天数 / 365.25)`；`per_1e6_bars = n_bsp / n × 1e6`（结构时间口径，「两个都报，报告里以后者为主」）。头部 `:5-6` 自陈是 ADR 0016 裁定三/四的落地前置；`:89-90`「只读纪律：不动判据、不写主路径状态、零生产行为变更」。
- 姊妹探针 `rust/src/bin/p907_cost_gate_level.rs`，`:10`「只测不裁——阈值归 #817」。

### C.3 ADR 0016 裁定四原文（逐字）

`docs/adr/0016-chong-quantity-layer-capacity-frequency-shift.md:104-112`：

> ### 四、重开不开 ＝ 该级别的频率
>
> > **信号频率太低的级别，根本不开重。**
>
> `[新缠论:推论]`。这一条是走票尾声由编排者的追问（「那装不下不得不挪怎么办？」）逼出来的前置，成因是主控前半程**漏掉了原文点名的第二个因子**：
>
> > `049-第49课.md:74`【正文】：「这里所说的利润率，是指**每次操作的平均利润／需要占用资金的平均时间**，但，真正能产生总体利润的，**还与操作的频率有关**」
>
> **容量与活跃度是两个量。** 一个级别可能装得下很多钱，但一年只发三次信号。

配套：`:121` 判据表行「| **频率** | 这个重**开不开** | `049:74` | 每级的信号频率 |」；`:129`「**闲置的代价由裁定四的频率门在开重那一刻拦掉，不在运行中拦。**」；`:291` 自陈「本票**未测任何数**。容量与频率两个量全部未测，裁定三/四落地前必须先有探针读数。」

**注意口径差**：ADR 0016 裁定四把频率用作「**这个重开不开**」的准入门（级别筛选）；ADR 0015 裁定二把操作频率列为「**全程配套**」判据（`0015:41`）。**两处是不是同一个量、同一道门，本次查不到有任何文件把它们对齐。** ⚖ 归 #853 裁。

### C.4 #915 已测的逐级频率读数（转录，勿重跑）

`.chanlun/review-results/issue915-capacity-frequency-20260805.md`。主口径 `:167` = **每百万根 K 的买卖点数，买／卖分列**。表 `:169-178`（单位：个/百万 bar）：

| 标的 | L0 | L1 | L2 | L3 | L4 | L5 |
|---|---|---|---|---|---|---|
| BTC（引 #907） | 3813 | 1673 | 813 | 448 | 152 | **0** |
| ES | 3574 | 1403 | 603 | 211 | 31 ⚠ | **0** |
| CL | 3694 | 1413 | 642 | 255 | 107 | **0** |
| GC | 3692 | 1559 | 707 | 281 | 79 | **41（150买/0卖）** |
| BRN | 3480 | 1307 | 617 | 312 | 114 | **18（0买/44卖）** |
| DX | 3620 | 1385 | 672 | 200 | 52 | **0** |
| QQQ | 3431 | 1182 | 415 | 159 | **0** | — |
| OKLO | 3566 | 1404 | 623 | 140 | **0** | — |

- `:180` BTC 日历口径对照：**2002 / 878 / 427 / 235 / 80 / 0** 个每年。
- 三条读数 `:184-186`：L0 跨 8 标的近常数（3431–3813，极差 11.2%）「是引擎的结构常数，不是市场属性」；每上一级约除 2.3（L0→L1 除 2.28–2.90、L1→L2 除 2.06–2.85 紧；L2→L3 除 1.81–4.46、L3→L4 除 2.38–6.82 散）；「**塔顶必坏，8/8**……塔顶级别连一趟交易都构不成（零信号或单边）」。
- `:57`「⟹ 落地要求：逐级信号频率这条曲线必须**按买／卖分列**，只报总数会整类漏掉。」
- ADR 0016 `:114` 另引 `analysis/recursive_position_experiment.md` §5.2b/5.3 的 OKLO per-ladder 买点计数 L2/L3/L4/L5 = **1964/297/18/2**，且「51.6% 的 quota 绑定在从未发事件的那一级上」。

**⟹ 对问 4 的事实面结论**：频率的**测量**已有（探针 + 8 标的读数）；频率的**合成公式**（乘积？两道独立门？只报告不作门？）在代码、ADR 0015、ADR 0016、缠师原文四处**均查不到**。`049:74` 只说「两者不能偏废」。⚖ 全属留白，归 #853 裁。

---

## D. 闸门

### D.1 定义（逐字）

**ADR 0013**（`docs/adr/0013-partition-machine-bidirectional-form.md:74-79`）：

> ### 四、扩量的闸门 ＝ 本金收回
>
> - **闸门之前**：M = N（同股数），量锁死，赚的全部用于降成本／抬高开空均价。依据 `chan99/0033:11`／26:34「**在持股成本变成负数前**，仓位是一直不变的，最开始多少就是多少……绝对不加仓，一开始就买够」。
> - **闸门之后**：每段结束把已实现利润并进本金，下一段用更大的量。依据 `031-第31课.md:169`「成本为0前，只补进相同的数量，仓位不增加；**成本为0后……股票才会越来越多**」。
>
> **闸门在两侧的表现不同、判据一致**（即 #842 R-6「枢轴是本金回收、与方向无关」）：多头重＝成本降到 0；空头重＝累计已实现利润 ≥ 该重投入保证金。

溯源标签在 `docs/adr/0013-partition-machine-bidirectional-form.md:320`（`[旧缠论]` ＋ 空头侧 `[新缠论:推论]`）；闸门后两侧通道数在 `:85`。

**ADR 0015**（`docs/adr/0015-cost-trajectory-acceptance-metric.md:35-41`）：

> **「(成本↓, 筹码↑) 二维向量没有全序」这个难题不存在**——ADR 0013 裁定四的闸门已经把它解掉了：
>
> | 阶段 | 在动的量 | 判据 |
> |---|---|---|
> | **闸门之前**（成本未归零，持仓量锁死 M＝N） | 只有成本 | **成本下降速率** |
> | **闸门之后**（多头成本＝0／空头本金已收回） | 只有筹码 | **净头寸价值**（持仓市值） |
> | 全程配套 | — | **操作频率** |

以及 `:53`：「净头寸价值（`units × price`）单独作主判据有硬伤：**它是价格的函数**……**但在闸门之后该硬伤消失**——此时成本已归零，持仓市值全部是白赚的量，它就是战果本身。故它是**后半段**的判据，非主判据。」

### D.2 代码实装：有

- 闸门判据字面就是 `cost_basis <= 0`：`rust/src/recursive_t/t_engine.rs:390-395`
  ```rust
  if self.core_cost_basis <= 0.0 && self.enable_three_stage {
      … self.stage = FlatTStage::CapitalRecovered;
      self.res.n_capital_recovered += 1;
      self.try_withdraw_capital(c);
  }
  ```
  注释 `:391`「退本金：cost_basis 穿 0（点2，纯状态切换，无额外交易）」。
- 单向状态机：`rust/src/recursive_t/t_engine.rs:138-152`（`EarningShares` 注释 `:138`「本金已全额退出。纯利润买更多 units」）；镜像 `rust/src/recursive_t/rec_engine.rs:609-617`；生产契约 `rust/src/theta_v0/strategy/ledger.rs:76`。
- **⚠ 同名不同物**：仓内另有一个「闸门」——命题 4 读法乙的**背驰段闸门**（`rust/src/recursive_t/rec_stream.rs:103`、`:326`、`rust/src/recursive_t/rec_engine.rs:893`），与 ADR 0013/0015 的**本金闸门无关**。引用时勿混。
- 标识符 `zero_cost` / `cost_zero` / `funded_campaign` / `barrier`：**查不到**。

### D.3 生产上触达过没有 —— **没有，已在案，无需新探针**

票面疑「本仓从未跑到过闸门」——**查实成立**，两条直接证据：

1. **源码自陈**：`rust/src/theta_v0/backtest/runner.rs:728`「**本闭环在生产策略下不触达 EarningShares**（stage 恒 CostReduction）」。
2. **真实 BTC 全史断言**：`rust/src/theta_v0/backtest/runner_tests.rs:8198` `fn l2_btc_earning_shares_unreachable_hwm_debearing()`；`:8232`
   ```rust
   assert_eq!(count, 0, "closed_loop 生产策略（PhaseI 恒 Buy 无平仓）L2 BTC 不触达 EarningShares（count=0）…");
   ```
   `:8236` 另断言 `stage == TStage::CostReduction`。数据源 `:8201` `load_by_symbol("BTC")` 全历史。文档注 `:8187-8195` 明写「C' 前 count=1（hwm_gain 棘轮，被裁）；C' 后 count=0」「**策略性不触达，非账本结构不可达**」。
3. **L0 侧对应件**：`rust/src/theta_v0/backtest/runner_tests.rs:7926-7935`——有界振荡价格流上 EarningShares 不触达，机制写死在 `:7933-7934`：「free 恒 0（诊断浮盈不入 free，无真实卖出现金回流）⟹ 永不足额退本金 ⟹ stage 恒 CostReduction」。
4. **审计口径**：`.chanlun/review-results/issue795-map-landing-audit-20260730.md:30`「机制在链上、**终态从未被生产路径实际达到**」（`:115` 复述）。
5. **反例（闸门在非生产路径/单测里到得了）**：`rust/src/theta_v0/closed_loop/state.rs:221` 提「生产见证 `pi_loop_realized_profit_reaches_earning_shares`」；`rust/src/recursive_t/t_engine.rs:1591` `assert_eq!(eng.stage(), FlatTStage::EarningShares, "已进增股数")`（合成上涨序列驱动，`:1470-1494`）。

**查不到的**：记忆里流传的「真实 BTC **461 万 bar**」这个具体 bar 数，仓内**查不到**落盘定值——`runner_tests.rs:8228` 只在运行时 `eprintln` 打 `bars={} (全量{})`。GAP3 原始报告（`acc-gap3-earningshares-reachable-20260701.md` 等）已归档出树，仅 `.chanlun/review-results/archive-sweep-20260728.md:58`、`:419-424` 留文件名索引。**引用该数字前须自行重跑或改引上述断言。**

**⟹ 本次因此未新增探针 bin**（要测的东西已有断言）；`rust/Cargo.toml` 未改，`required-features` 条款不适用。

### D.4 闸门后那段（净头寸价值）：**没有实装**

`净头寸价值` / `net_position_value` 在 `rust/`、`analysis/` 全库**零命中**；只在 ADR 0015 `:40`、`:47`、`:53` 作纯裁定文本出现。

**⟹ 对问 5 的事实面结论**：票面猜测「可能现在定不了」——**事实面支持这个猜测**：判据无实装、生产无样本（count=0，两条独立断言）。⚖ 是否写「暂缺，待首次触达闸门后再定」，归 #853 裁。

---

## E. #907 移交那条（中位 vs 下尾）

报告正本：`.chanlun/review-results/issue907-cost-gate-level-20260805.md`（324 行）。

### E.1 移交要求的原文（逐字）

**报告侧**，`:23`：

> **关键后果（交给裁定方）**：按中位数看，成本门在 BTC 1m 塔的**任何已测级别都不咬**（连塔底 L0 都有 2.6 倍余量）。**但按下尾看，L0…L3 的 p10 全部 < 1**（0.31/0.38/0.43/0.78）——即「成本门咬不咬」取决于裁定方用**中位数**还是**下尾/失败比例**立判据。这两种选法给出的停止级别不同。**这是本读数最重要的一条，不是脚注。**

**票面侧**（#853 issue body，2026-08-05 移交条目，逐字）：

> **本票裁「阈值/最小样本量/置信下界」时，须同时裁一件已经具体化的张力：同一条曲线用中位数读和用下尾读，答案不同。**
> …
> **⟹ 本票必须给出的一句**：这类「比值/比例 vs 阈值」判据**用中位数、用某个分位、还是用置信下界**？**不定这一句，曲线会被两种读法读出两个结论。**

票面同处给出的**同族判例两例**（都已订正）与**元教训**：

> 1. **N-6 退场条款**原写「rung 长度 95%+ 为 0 ⟹ 必须收敛」用**点估计**，实测 96.03% 形式过线；改用**单边 95% 下界**重算后**三窗两不过、合计不过** ⟹ 未触发。订正后四条门：①用下界不用点估计（**方法须写明**）②≥3 个互不重叠窗 ③每窗各自过线不看合计 ④合计覆盖 ≥50% 全史。
> 2. **同一条订正 comment 里主控交的表本身方法混用**（标 Wilson，实为两格 Clopper-Pearson、两格两法都不是），重算后数字全部更低。⟹ **「一表一法、并列要各自标名并写清 `z` 或精确法」这条是现场教训。**
>
> **⟹ 元教训（建议本票收进正本）**：凡「某比例超阈值即触发某动作」的判据，**必须同时写清四件事——点估计还是下界／几个独立窗／每窗还是合计／覆盖多少总体**。四项缺任一项，判据就会在**擦线处**失效，而擦线恰恰是它最常被引用的地方。

### E.2 逐级读数全表（转录）

**表 A —— VIP0 taker，往返 20.0 bp**（`issue907-cost-gate-level-20260805.md:179-185`）
分母 = `20.0 bp` + `2 × DELAYERR 中位数`（合成规则 `:175`「一张表只用一个费率档」）

| 级别 | 分母 (bp) | **中位数比值** | p25 比值 | **p10 比值** | 均值比值 |
|---|---|---|---|---|---|
| L0 | 26.4 | **2.59** | 0.93 | **0.31** | 5.50 |
| L1 | 25.8 | **4.51** | 1.29 | **0.38** | 10.05 |
| L2 | 25.6 | **7.89** | 1.54 | **0.43** | 22.93 |
| L3 | 25.0 | **20.26** | 3.38 | **0.78** | 55.90 |
| L4 | 24.0 | **148.86** | 35.10 | 1.64 | 405.34 |

**表 B —— VIP0_BNB25 taker，往返 15.0 bp**（`:189-195`）

| 级别 | 分母 (bp) | **中位数比值** | p25 比值 | **p10 比值** |
|---|---|---|---|---|
| L0 | 21.4 | **3.20** | 1.14 | **0.38** |
| L1 | 20.8 | **5.59** | 1.60 | **0.47** |
| L2 | 20.6 | **9.80** | 1.92 | **0.53** |
| L3 | 20.0 | **25.33** | 4.23 | **0.98** |
| L4 | 19.0 | **188.04** | 44.34 | 2.07 |

**表 C —— 负例，AMP_EXEC 口径同算（VIP0）**（`:199-205`）：L0 1.49 ↓ / L1 1.44 ↓ / L2 1.27 ↓ / L3 1.11 ↓ / **L4 0.81 跌破 1**。⟹ 口径选错会得出「高级别不划算、该往低走」的**反向**结论；`:207` 证明是簇内配对伪影。

**「跌破成本」样本占比（分位数夹逼，非精确计数）**（`:209-217`）：L0 **25%–50%**（贴近 25–30%）／L1 **10%–25%**／L2 **10%–25%**／L3 **10%–25%**／L4 **< 10%**。`:219`「**这是夹逼，不是精确比例，也还没有置信区间。**」

**样本量 `n_bsp`**（`:124-133`）：

| 级别 | FULL `n_bsp` | W1（26% 前缀）`n_bsp` | FULL ROUNDTRIP 样本 n |
|---|---|---|---|
| L0 | 17,593 | 4,817 | 18,247 |
| L1 | 7,717 | 2,177 | 8,913 |
| L2 | 3,750 | 1,212 | 4,387 |
| L3 | 2,067 | 708 | 2,311 |
| L4 | **703** | **192** | **676** |
| L5+ | **0** | **0** | — |

FULL 合计 `ids_total=31,830`、`events_total=36,159`。

**跨窗稳定性**（`:273`）：「**L0…L3 的波幅读数与单调性跨两窗稳定；L4 不稳且严重聚簇 ⟹ 表 A/B 的 L4 行（148.86 / 188.04）不可采信，请裁定方勿据以定阈值。**」机制：一趟 L4 往返中位持有 700,524 bar ≈ **486 天**，1.2M bar（≈2.3 年）窗装不下几趟。

**分布形状**（`:147`）：「**分布强右偏，均值全部约为中位数的 2–3 倍。**……用均值会把每一级的典型波幅高估 2 倍以上。」（L0 均值/中位 = 2.12，`:141`）

### E.3 #907 自陈的未完成项（本票若沿用其读数须知）

`:294-296`：

1. **比值曲线没有置信区间**——只有点估计与分位数夹逼。需补：(a)「跌破成本」比例的**精确计数** + 单一方法区间（**一张表只用一种方法，并标名与 `z`**；因样本聚簇还需**按月分块 block bootstrap** 作簇稳健版本——在案「逐例与逐簇数字差 4 倍」）；(b) 中位数的区间（bootstrap 或次序统计量精确法，**标名**）。
2. **逐窗报**——当前只 2 窗（FULL 与 26% 前缀）且**探针只支持前缀窗**（`P907_MAX_BARS`），需加窗口起点参数才能做互不重叠等长窗。
3. **L4 需要更长数据或多标的**才能给出可采信读数。

以及 §6 敏感性（`:320-322`）：ROUNDTRIP 假定持到同级第一个反向信号，**持有期资金成本/funding 没进分母** ⟹ 高级别列是**上界**；若生产改盘口成交或加真实滑点，**L0/L1 首先翻**；`:322`「真实误差若比这个下界大一个量级（例如 30 bp 而非 6 bp），分母翻倍以上，**L0 中位比值会从 2.59 掉到 ~1.4**，L0 的 p25 早已 < 1」。

`:282`「**真实交易误差（延迟 + 滑点 + 盘口价差 + 冲击）：查不到。**」`:288` 持有成本（funding / borrow / 强平）不在费率簿覆盖面 ⟹ 未计入分母，「L4 中位持有 486 天，**这一项在高级别可能远大于成交费**——本件**没测**，不许读成『不受影响』」。

---

## F. 手续费口径现状（问 2 的输入）

### F.1 在册费率簿

目录 `analysis/data_cache/`，2 份 datum + 1 份溯源件：

**`analysis/data_cache/venue_fee_binance_spot_20260726.json`**
- `:3` `"venue": "BINANCE_SPOT"`；`:4` `"source_url": "https://www.binance.com/en/fee/trading"`；`:5` `"retrieved_at_utc": "2026-07-26"`；`:6` note 声明 venue 口径 = spot（#303 裁定），且「perp 费率表（未核，报告 4.1）不混用」「resolve 失败即 fail-loud，禁静默退化」。
- BTC / `tier="VIP0"`：`:12` `"maker_bps": 10.0`、`:13` `"taker_bps": 10.0`（note `:14`「Regular User（30 日成交额 < $1M）0.1000%/0.1000%」）。
- BTC / `tier="VIP0_BNB25"`：`:20` `"maker_bps": 7.5`、`:21` `"taker_bps": 7.5`。

**`analysis/data_cache/venue_fee_ibkr_pro_20260726.json`**
- `:3` `"venue": "IBKR_PRO_US_EQUITY"`；`:4` source_url；`:5` retrieved_at；`:6` note 指向 `chanlun/review-results/venue-fee-source-research-20260726.md` §2.3/§2.5，「只登记 OKLO——其余品种的 venue 假设未核，不入簿」。
- 单条 OKLO entry，**按股计费无 bp**：`:13` `commission_per_share_usd 0.0035`、`:14` `min_commission_usd 0.35`、`:15` `max_commission_frac_of_notional 0.01`、`:16` `clearing_per_share_usd 0.0002`、`:17` `cat_per_share_usd 0.000003`、`:18` `sell_sec_fee_frac 0.0000206`、`:19` `sell_taf_per_share_usd 0.000195`、`:20` `sell_taf_cap_usd 9.79`、`:21`/`:22` passthru。

**`analysis/data_cache/venue_fee_provenance.md`** 的三条有效域声明（`:46-52`，逐字要点）：
- `:46-47`「**L2 只覆盖佣金/监管/清算科目**。`slippage_bps` 仍是未标定常数（价差/冲击性质，另票）」
- `:50`「**maker 档存在但生产恒 Taker**：本引擎按 bar close 市价撮合，无挂单语义，取 maker 档是声明膨胀（090）」（对应常量 `rust/src/theta_v0/venue_fee.rs:112` `pub const PRODUCTION_LIQUIDITY_ROLE: LiquidityRole = LiquidityRole::Taker;`）
- `:51-52`「**注入通道未落**：`ExecConfig::default()` 恒 `None`，CLI 无 datum 参数 ⟹ 生产口径上本簿尚未被消费」

### F.2 `ExecConfig` 默认值

`rust/src/theta_v0/config.rs:251` `pub struct ExecConfig`；`impl Default` 在 `:315-325`：

```rust
impl Default for ExecConfig {          // :315
    fn default() -> Self {
        ExecConfig {
            entry_delay_bars: 1,       // :318
            commission_bps: 1.0,       // :319
            slippage_bps: 2.0,         // :320
            tax_bps: 0.0,              // :321
            fee_schedule: None,        // :322
        }
    }
}
```

测试锁同值：`config.rs:458`（commission 1.0）、`:459`（slippage 2.0）、`:477`（tax 0.0）。

### F.3 每个默认值有没有标定

| 字段 | default | 结论 | 承重 |
|---|---|---|---|
| `commission_bps` | 1.0 | **无来源的设计值，代码明文自认未标定** | `config.rs:254`「commission（bp/side）。default 1。[设计选择;L3经验待标定]」；`:256-257`「本字段…是 `fee_schedule = None` 时的未标定 fallback，口径标签 `[L1机制/费率未标定]`」 |
| `slippage_bps` | 2.0 | **未标定常数，两处明文** | `config.rs:259-262`「default 2。L3。★#360 不覆盖滑点…标定档下本字段**仍未标定**——venue 费率表只标定佣金/监管/清算科目」；`venue_fee_provenance.md:46` 同判 |
| `tax_bps` | 0.0 | 0 **不是标定值，是约束占位** | `config.rs:264` 标 L3；`:266-270`「标定档下必须为 0…唯一强制处是 `backtest::treasury::fee_quoter` 内的 `assert!`」（理由：datum 已逐项承载税费，再叠会重复计） |
| `fee_schedule` | `None` | **尚不是端到端可执行出口** | `config.rs:242-246`「★尚不是端到端可执行出口（#374 MED-B）：全仓 `Some(...)` 赋值点**只在测试内**——`ExecConfig::default()` 恒 `None`，CLI 无 datum 注入参数」 |

**落差已在册（不是隐藏 magic number）**：`rust/src/theta_v0/strategy/risk.rs:880-887` 逐字登记——commission 1bp + slippage 2bp = **3 bp/side**，而一手数字是 Binance 现货 VIP0 taker **10 bp/side**，「差 ≈3.3 倍……且方向与持有成本相反（交易成本一侧是**低估**，不保守）」，并写「此处**不改值**：这两个参数均未按 venue 标定」。`risk.rs:892-893`：「**default 仍是 `None`**（三常数 3bp/side）⟹ 上述落差与全部在册数值结论**逐位不变**；把 default 切成标定档是**另一次裁定**」。口径标签常量 `risk.rs:776` `pub const RATE_UNCALIBRATED_LABEL: &str = "[L1机制/费率未标定]";`

**⚠ 即：#907 表 A/B 用的 20.0 / 15.0 bp 往返，来自费率簿；而生产 `ExecConfig::default()` 用的是 3 bp/side = 6 bp 往返。两者不是同一个数。** ⚖ 本票定阈值时用哪一个，归 #853 裁。

### F.4 #907 的往返成本口径

- **探针本身不合成往返成本数值**，只打印在册档位：`rust/src/bin/p907_cost_gate_level.rs:224-235` 打 `P907_COST_CFG commission_bps_uncal=… slippage_bps_UNCALIBRATED=… tax_bps=… fee_schedule=…`；`:223` 注释「成本侧：只打印在册档位与未标定项，**不在此处替裁定方合成阈值**」。
- 探针算的是**分子侧** ROUNDTRIP 幅度。语义 `:337-341`「入场信号 → **同级最近的反向信号**（一趟完整往返）…成本是**按往返两次成交**收的（taker × 2）」；配对实现 `:345-354`；幅度 `:360` `rt_vals.push(((close(b1) - c0).abs() / c0) * 10_000.0);`（bp）。
- **数值合成在报告里**：`issue907-cost-gate-level-20260805.md:87`「**往返成本** = 2 × 单边：VIP0 = **20.0 bp**；BNB25 = **15.0 bp**」；`:86`「**生产恒 taker** ⟹ 一律取 taker。**maker 档不参与比值**」；`:106`「一趟往返有两次成交 ⟹ 误差项取 **2 × DELAYERR 中位数**」；`:95`「这个 2.0（slippage）不许当实测用，本件不拿它填分母」。

### F.5 执行场所可能变更（#931）

- **「三家候选 taker 费率差 20 倍」这句结论：仓内查不到。** 没有任何文件把三个候选 venue 的 taker 费率并列并给出 20 倍差。（`20 倍` 的全部命中都是别的语境：`docs/adr/0017-*.md:22` 是持仓量差 20 倍；`docs/persistence_theory.md:188` 是 MACD 面积；`.chanlun/review-results/issue917-*.md:559` 是容量差异。）**照实登记：该数字未在册，请裁定方勿据以推理。**
- **#931 确是在册 map**，挂件已核：`docs/adr/0020-backtest-cash-live-perp-filtered.md:4`（走 #941，map #931）、`:9`「执行场所是**加密永续**（24/7 有报价），回测数据源是**真实美股历史**」；`.chanlun/review-results/perp-vs-cash-price-deviation-20260807.md:3`（#934）；`.chanlun/review-results/hl-subaccount-abstraction-default-20260808.md:3`（#942）；另 `hl-margin-table-disabledex-20260808.md:3`、`hyperliquid-hip3-isolation-20260807.md:3`、`structure-input-ab-c-divergence-20260807.md:3`、`bsp-layer-mismatch-20260808.md:3`、`three-nt-impls-facts-20260808.md:1`。
- **在册的按 venue 费率数字**（全部已打开原行）：

| venue / 品种 | maker | taker | file:line |
|---|---|---|---|
| Binance 现货 BTC VIP0 | 10.0 bp | 10.0 bp | `analysis/data_cache/venue_fee_binance_spot_20260726.json:12-13` |
| Binance 现货 BTC VIP0_BNB25 | 7.5 bp | 7.5 bp | 同上 `:20-21` |
| Binance **TradFi Perps** | 全档 **0%** | 0.04%（普通）→ 0.0085%（VIP4–9） | `.chanlun/review-results/binance-subaccount-isolation-20260807.md:43-46`；分档 `:207` |
| Binance 普通加密永续（第三方摘要，非逐字核实） | ~0.02% | ~0.05% | 同上 `:211` / `:46` |
| Binance Futures / OKX / Bybit / dYdX v4（BTC 永续对照） | 0.02 / 0.02 / 0.01 / 0.01 % | 0.05 / 0.05 / 0.06 / 0.05 % | `docs/quant-platform-research-2026.md:490` |
| **Hyperliquid maker/taker bp** | — | — | **查不到**（`hyperliquid-hip3-isolation-20260807.md` 只记 deployer fee `:222` 与资金费率乘数 `:208`；`hl-subaccount-abstraction-default-20260808.md:73` 只提「手续费档位继承主账户、返佣不适用于子账户」，无具体 bp） |
| IBKR Pro 美股 OKLO | per-share 档（无 bp） | 见 F.1 | `venue_fee_ibkr_pro_20260726.json:13-22` |

- **未标实时性项**：`binance-subaccount-isolation-20260807.md:44-46` 自注「页面本身『No records found』未登录不可核实实时状态」；`:230` 列 U5 未核项。
- **venue 变更时代码的现有姿态**：`docs/adr/0018-*.md:161`「真有 maker 收益的场所（Binance 永续、VIP 高档）目前全是未核项，`VenueFeeBook::resolve` 对其返回 `Err`（fail-loud）」。⟹ **场所若变，现有费率簿一律 resolve 失败，不会静默沿用 Binance 现货档。**

---

## G. 五问的事实面齐备度

| 问 | 事实面 | 还缺什么 |
|---|---|---|
| **1** 速率的可算定义 | **齐**（A.1：从零定义，成本基有、速率无；A.2：ADR 0015 只说「成本下降速率」四字，未给公式；C.4 提供「每 bar」与「每次操作」两种采样单位各自的在册读数） | 无阻塞。⚖ 采样单位（次/bar）、分子形式（绝对/相对/年化）全属留白 |
| **2** 通过阈值 | **半齐**。手续费现状全在 F；ADR 0015 裁定三的论证在 `0015:70`（「无优势 ⟹ 扣费后期望为负 ⟹ 成本不降反升」） | **缺**：(a) 执行场所未定（#931 在裁），F.5 的候选费率不全（Hyperliquid bp 查不到）；(b) `ExecConfig` 3bp/side 与费率簿 10bp/side 差 3.3 倍且方向不保守（`risk.rs:880-887`）；(c) 资金费率（ADR 0015 裁定四要求计入）**在费率簿覆盖面之外**（`venue_fee_provenance.md:46`、`issue907:288`）⟹ 阈值数字要等场所定 |
| **3** 最小样本量 + 置信下界算法 | **齐**（B 全节）。现有 = 正态近似 + std-型 SE，`z=1.645` 硬编码 3 处，生产 `z_alpha` 默认 0.0（LCB 实际关着），无统一 n_min（3 个不同门），`n_eff` 两套实现，双侧依赖 4 处已逐条列出 | 无阻塞事实缺口。⚖ 沿用/另起、用哪个 n_eff、生产 z_alpha 要不要开，全归本票 |
| **4** 频率判据合成 | **半齐**。测量与读数齐（C.2/C.4，8 标的逐级）；ADR 0016 裁定四原文齐（C.3） | **缺公式来源**：合成方式在代码、ADR 0015、ADR 0016、`049:74` 四处**均查不到**。另 ADR 0015「全程配套」与 ADR 0016「开重准入门」是否同一道门，**无任何文件对齐** |
| **5** 闸门后判据 | **齐，且指向「现在定不了」**（D.3 两条独立断言 count=0；D.4 净头寸价值零实装） | 若要改判「能定」，需先让生产触达闸门——而 `runner.rs:728` 自陈是策略性不触达（PhaseI 恒 Buy 无平仓），属策略改动，不在本票范围 |

---

## H. 没查到 / 不确定（照实，不省略）

1. **「三家候选 venue taker 费率差 20 倍」查不到**（F.5）。仓内无并列表，Hyperliquid 的 maker/taker bp 无落盘。
2. **「真实 BTC 461 万 bar」这个具体数字查不到**（D.3）。只有运行时 eprintln，无落盘定值；GAP3 原始报告已归档出树。
3. **真实交易误差（延迟+滑点+盘口价差+冲击）查不到**（`issue907:282`）。#907 用 `entry_delay_bars=1` 的实测价位差作**下界**代理。
4. **持有成本（funding / borrow / 强平）未测**（`issue907:288`），且不在费率簿覆盖面。ADR 0015 裁定四要求计入资金费率，但**该量本仓无来源**。
5. **`chi_open_gate_lcb` 死代码判定与实际调用点的张力未核**：`stat-provenance.md:67` 判死代码，`pi_bsp_timing.rs:486` 有调用；时序谁先谁后本次未核（涉 #394 / #410，#410 仍 OPEN）。不据此下结论。
6. **`filter_gamma` 相关的第三处手搓 `chi_t(mu_lcb(...))`**（#410，OPEN）本次未展开——若本票改动 LCB 口径，该票是受影响面，但**本次未核其具体行号**。
7. **未跑任何构建/测试**。本件全部为静态阅读；`runner_tests.rs:8232` 的 count=0 断言是**读源码所得**，未实跑验证。
8. **ADR 0015 自陈的举证缺口**（A.2 末）原样继承：44 份 PDF 里这把尺子零出现、46 页截图 PDF 未 OCR、142 个 Lean 文件精读 8 个、未跑 `lake build`。
9. **本次未新增任何探针 bin**，`rust/Cargo.toml` 未改（D.3 说明理由）。
