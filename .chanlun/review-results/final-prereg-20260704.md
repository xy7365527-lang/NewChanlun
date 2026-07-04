# 终局 alpha 重跑预注册冻结（Task #181，A 类全清零后总冻结）

- **工位**：swarm/ws-finprereg | **task #181** | **解锁** #182 终局跑数（a5 验收判定件）
- **冻结时刻**：2026-07-04（本文件独立 commit = 冻结点；#182 任何产生判定数字的跑数在本 commit 之后的 HEAD 上执行——**跑数先于本冻结 commit ⟹ 终局 alpha 直接 fail**，#135 纪律）
- **冻结基 HEAD**：`663fdfe375`（A 类 12 条全闭合；`cargo test --release --lib` 1484 passed / 0 failed / 111 ignored）。#182 跑数所在 HEAD 必须包含本 commit 及其父 663fdfe375。
- **认识论**：本文件 **L0**（判据/口径规格冻结，零 L2 信息增量）。#182 跑数产出 = **L2**（BTC 单标的全历史 walk-forward OOS）/ **L3**（7 品种池化）。逐桶三态照实标注，否定性结果（INCONCLUSIVE/FALSIFIED）是合法诚实产出（161号）。
- **认识论预承诺（090/161，防翻盘幻觉）**：全战役至今**无 confirmed 正 alpha 在案**——p3/f3/L3/q4 四轮全 INCONCLUSIVE；A2 三口径一类桶（G1/G2/G3）36-37 桶中除 L0/bsp3/δ+1/σ+1 单桶 Validated 外全 Inconclusive。本终局重跑**不预期翻转**该结论。若 π^full 在补齐全部 A 类实装后仍无正 alpha，INCONCLUSIVE/FALSIFIED 就是终局诚实产出，残差口径 `Y_i = δ(H − B̂) − C` 不动摇。

---

## 0. 继承结构（本冻结 = 三份既冻 prereg 之上的终局叠加）

本冻结**全文继承不改**以下三份，仅在其上叠加 A 类闭合后的新语义标记与 B30 四项处置：

1. **`prereg-q4-fullpi-20260703.md`（388a9ebc16）七项硬要求 ①-⑦**——gross_cap=true / μ 新口径功效 / 置换桶键三处同批 / CME-simple margin / 两类 TW bit-exact 风险 / 窗口·品种·种子·判据·三态规则 / 重跑清单 R1-R6。本文件 §5 逐项标注哪些不变、哪些因 A 类闭合而更新。
2. **`prereg-rerun5-q4-20260703.md`（继承 q4 七硬要求 + 七硬要求全文）**——canonical 中枢链版本标记 + 一类信号集 614 基线 + 与 q4 旧结果双列差分 + dx 真封①开放问题（#147 已修，见 §1④）。
3. **`acc-alpha-estimand-prereg-20260701.md`** 的三态判据链（VALIDATED/FALSIFIED/INCONCLUSIVE + 功效门 `n_eff ≥ (1.645·CV)²` + LCB≤0 不外推 μ≤0）——本文件 §3 判据全部沿用，零改动。

**冻结后不改本文件**（q4 fail 条件 2 同款）：任何偏离照实入 #182 结果包，不回改冻结文本。

---

## 1. 语义版本标记（★A 类闭合后全口径产物有效域重置——冻结令①）

A 类闭合切换了信号生产链与 z 桶键的语义。**所有 A 类闭合前的 alpha 产物（q4/rerun5/p3/f3 数字）有效域重置**——#182 是其新口径重估，旧结论作差分档案不作基线。以下逐项冻结新口径的版本锚：

### 1.1 K_i 宇宙切换（A12 #160，commit `a87540bdb6`）
- 子声部激活宇宙从结构树 T_i 切换为 carrier 森林 **K_i**（endpoint-complete，host^op）。切换后**全口径产物有效域重置**——A12 前的 μ̂/W-VERIFY/L3 数字不作 #182 基线。
- 双视图冻结：结构树 T_i（划分语义）vs carrier 森林 K_i（endpoint-complete 承载语义）并存，voice_eat 子声部激活走 K_i。#182 信号生产链在 K_i 宇宙上运行。

### 1.2 canonical 中枢链（#142 延伸 + #148 升级）
- **中枢延伸**（#142，§5，commit `c84c39e908`）：修复序第 1 步，canonical 中枢链的延伸语义已实装。
- **中枢升级**（#148，中心定理二，commit `f9b3e41636`）：延伸 ≥9 段升高级别中枢。升级语义出处订正归第 33 课（33 课多义性约定覆盖 20 课，#171 谱系）。
- 信号生产链完整序：#142（§5 延伸）→ #143（§6 走势分解 + Q8 CurrentMove）→ #144（局部趋势门 + Type1Cand 五条件）→ #145（ac4 A/C 类型化 + Q7 方向）→ #148（升级）。#182 跑数所在 HEAD 必须包含全序 commit。

### 1.3 z 15 维 canonical 形态（MuClass，真值化冻结）
`mu_estimator.rs::MuClass` 冻结 15 维（桶键投影见 §3；本表是完整载体，非全部进桶键）：

| 维 | 字段 | 语义 | 真值化 commit / 任务 |
|----|------|------|---------|
| 1 | `level` | 执行级 e（§6，G3 澄清：信号执行级） | #138 |
| 2 | `delta` | 交易方向 δ∈{+1,−1} | 基线 |
| 3 | `i_class` | 6-bit 买卖点掩码（buy1..sell3） | 基线 |
| 4 | `parent_dir` | 持仓树父声部方向 σ_p | 基线 |
| 5 | `short_swing` | 短差标志 | 基线 |
| 6 | `position` | PositionState | 基线 |
| 7 | `horizontal` | H(g) 水平关系（Some=真候选/None=裸证书，231 诚实） | 基线 |
| 8 | `force_state` | β^div 力度支配态 ForceStateA5（§9.1，A6 #159 透传真值化） | `a3fb547375` |
| 9 | `sigma_higher` | 上级方向态（§6，G2 #132；0=计算结果非未知） | #132 |
| 10 | `cand_channel` | Cand 门通道 NestTrigger（Type1/Type2·3/Xzd，G3 #138） | #138 |
| 11 | `nest_depth` | 区间套下沉深度 Ndepth（Some(0)=基例；BTC 实测 95.36% 基例、4.64% d=1） | #138 |
| 12 | `origin_level` | 起始级 ℓ（§6；`origin_level = level + nest_depth` 恒等，Jchain 无独立自由度） | #138 |
| 13 | `risk_mode` | RiskMode M0-M4 + MarginState 双覆盖（§6，G3 #138） | #138 |
| 14 | `t_stage` | TStage 三阶段（第 31 课降成本/退本金/增股数，OQ-9 不可逆，真值化） | `16ba38d5d5` / #149 |
| 15 | `eta_bucket` | ηBucket γ_t 四桶（§10 Deficit/Zero/PositiveUnsafe/PositiveSafe，真值化） | `fe582c3847` / #175 |

- **真值化冻结**（TStage/eta_bucket/force_state）：三维已从占位/派生升为 π fill loop 账本真值（`Some`=账本态真值，`None`=裸口径诚实缺失，231 不伪造）。commit 锚 `16ba38d5d5`（TStage）/ `fe582c3847`（eta）/ `a3fb547375`（force）。
- **origin_level 恒等式**：`origin_level = level + nest_depth`（Nest 通道，debug_assert 守护）。Jchain（§6 区间套包含链）由 N^δ 定义强制逐级相邻，链签名 ≅ (ℓ,e)，无独立自由度，由 `origin_level + level` 完整携带（对照表论证，非缺口）。

### 1.4 一类 614 基线（裁定C 后，rerun5 实证冻结）
- 一类信号集（裁定C 后新基线，BTC 全历史 4613599 bar，`level_signal_census_btc` 实测）：**L0 575（b244/s331）/ L1 18（b7/s11）/ L2 14（b5/s9）/ L3 7（b3/s4）/ L4 0，一类总计 614**。
- 旧 prereg 时代=1057（裁定C 前基线，其 L≥1 的 77-95% 为 fallback 锚伪信号）。#182 所有一类桶差分按「裁定C 前/后」双列，旧结论有效域=裁定C 前语义。
- **dx 真封①开放问题（#147 已修）**：rerun5 prereg 时 sig_post 手写门探针与生产 collect_signals 有 14 信号差（675 探针分叉）；#147 已修复。#182 走生产 `collect_signals` 侧，不消费手写探针；若发现生产侧漏/多信号须重估（继承 rerun5 fail 条件 5，现判据：#147 修复已落地故该条不触发，除非 #182 跑数暴露新差）。

### 1.5 XZD lvl≥2 C2-only 维持裁定（#179，commit `39e48b3636`）
- XZD（小转大）消费口径终裁：**C2-only 维持**（vs 全 level C3 硬门）。#148 后 C3 脱 0 新基线复审（#170）确认 C3 高级别 Type3 消费仍恒 0（死门维持）。
- #182 的 R6（XZD C3 高级别 Type3 消费重评估）沿用此裁定：死门探针复核窗**冻结写死 = BTC 全历史 4613599 bar**（codex FAIL-1 修订：原「成本允许最长窗」是可调自由度，删除——全历史是唯一确定口径，与 §1.4 一类信号 census 同窗）；C3 命中仍恒 0 ⟹ 维持「旧 C3 死门」记录；非 0 ⟹ 行为变化上浮（继承 q4 fail 条件 4）。

---

## 2. exit-μ-BUCKETING-FROZEN 条款（★#180 裁定，冻结令②，commit `29cdc2adbb` 载体）

直接继承 `exitmu-bucketing-20260704.md` §1 冻结条款（codex 裁定第四形态 d，L0 因果可测性 + 样本量代数）：

> **exit-μ-BUCKETING-FROZEN**：μ 的生产估计键保持 `Z = z_entry`（`build_mu_from_bars` 现口径 `est.observe(MuObservation{class: t.entry_z, x_gamma})` 不变，单键）。`typed exit` / `exit_type` / `exit_z` / 账本态迁移向量 `(exit_z − entry_z)`：
> - **允许**：只通过 `X^full = δ(P_{τ^typed} − P_t) − C` 进入样本收益值 `x_gamma`（§9 计价时刻）；事后诊断 / 异质性报告（W-VERIFY 按 exit_type 拆解占比）。
> - **禁止**：进入 μ 桶键（MuClass key）；作为 `χ_t(z,a) = 1[LCB_t(μ(z,a)) > θ ∧ …]` 门控的可查询协变量。
> 依据：出场信息在入场决策点 t 不可观测（post-treatment），进桶键 = 未来信息泄漏 + 样本碎裂（657）。

- **样本量代数支撑**（#180 §3，n≈2209 typed-exit 口径）：(a) 三元组 ≈2.0/桶数值不可行；(b) exit_type 5 层 ≈29.5/桶边际不可行；(c) 账本三元交互 ≈2.3-5.5/桶不可作确认层。即使无因果问题，n≈2209 在 15 维稀疏下不支持再乘 5×-75× 桶数。
- **下游冻结**：TYPED_TRADE_SCHEMA 的 `exit_z` 增列**不触发 μ 样本 schema 重冻结**（`MuObservation = {class, x_gamma}` 不变）。W-VERIFY 的 exit_type 占比（CloseRoot/ReduceCore/CloseShortDiff/RiskExit/Hold 分布）作诊断输出——判「no alpha」来自真稀有还是出场路径语义，诊断层不进门控。
- **翻转边界**（#180 §4）：仅当样本量升数量级（跨标的池化 n≈数万 + 657 归一化新 prereg）且 exit_type 替换为**入场时可预测的 ex-ante exit regime 代理**（届时是新入场维须新 prereg），或 §12 的 Z 语义从入场点 t 移到出场点 τ（改 χ_t 存在论，属定义冲突须 escalate）——两者均不属本终局重跑域。

---

## 3. 判据 / 桶键 / 三态规则（继承 acc-estimand + q4，冻结令，不改）

### 3.1 桶键三层分离（裁决聚合基 / 报告桶 / 诊断切片——657/共线免疫 + post-treatment 隔离）
桶键按**用途**严格三分（codex 审计 FAIL-2 消除：δ 双口径自洽 + h post-treatment 隔离）：

1. **主判据裁决聚合基**（δ-free，唯一 accept/reject 依据）：`(level, bsp_class压缩, parent_dir)`（s3/f3/q4 同款）。三态判定（§3.2）**只在此聚合基上做置换检验**。δ **不进裁决聚合基**（657 置换免疫：δ 进聚合基即毁 δ 置换，perm_p 退化 1.000）。
2. **W-VERIFY full 报告桶**（描述性，非裁决）：`(ℓ, bsp_class, δ, σ^H)`（三口径 OOS 同口径）。此口径**逐桶展示 μ̂/LCB/UCB 供异质性阅读**，δ/σ^H 是**描述维**——`δ` 在此出现是描述性拆分，不与「δ 不进裁决聚合基」冲突（用途分离：报告桶展示，聚合基裁决）。W-VERIFY 报告桶的单桶 state 不单独触发 acceptance；全局裁决走聚合基（§3.2）。
3. **诊断切片**（纯报告，绝不进 1/2）：`h_bucket`（持有期）、`time_block`、`Θ_SCORE bin`、`exit_type` 占比。**h 与 exit_type 是 post-treatment（依赖 exit_bar，出场后才知）**——只作事后异质性诊断，**绝不进裁决聚合基、不进 W-VERIFY 裁决桶、不进 χ_t 门控**（§2 exit-μ-BUCKETING-FROZEN 铁律；#180 §5「分层 ⊥ 桶键」的严格边界：分层是不改 accept/reject 的切片）。h 分层若影响 acceptance = post-treatment 进判据层 = §2 违规（codex FAIL-2）。

- **方向性维不进裁决聚合基**（657 + i_class×δ 共线自毁铁律）：完整 i_class（buy/sell 分裂）及任何 δ-派生量、force_state（第 8 维）、σ_higher（第 9 维）**均不进置换聚合基**。σ_higher 若进（报告桶除外），`stratified_delta_perm_p_fullz` base 键 / 重构闭包 `sigma_higher:None` / `wverify_fullz` 判定键投影**三处必须同批改**，任改其一即重现全表 miss/代表值泄漏——本轮三处均不动 = 一致地不进裁决聚合基。
- **事后验证锚**：主桶 perm_p 若退化到 1.000 ⟹ 桶键共线检查证伪，该桶结论作废并上浮（继承 q4 fail 条件 3；「上浮」= 该结论不成立、须停下复核，非软警告）。

### 3.2 三态判定（decontam 现引擎，零改动，z_α=1.645）
对每桶 `z`：`powered(z) := n_eff(z) ≥ (1.645·CV(z))²`（667 功效门，公式不变，样本量自适应）。

| 桶态 | 条件 | 含义 |
|------|------|------|
| **VALIDATED** | `μ̂>0` ∧ `powered` ∧ `LCB_OOS>0` ∧ `perm_p<0.05` | 携超 beta 可交易 alpha（perm 为抗假阳护栏，非主判据） |
| **FALSIFIED** | `powered` ∧ `UCB_OOS≤0` | 有功效地判定 μ≤0——真纯 beta，照实 161 |
| **INCONCLUSIVE** | `¬powered`（欠功效）**或**（`LCB≤0<UCB`） | 判据无检出力——`LCB≤0 ⊬ μ≤0`（667+231），不判纯 beta，不落 161 |

- **全局裁决（两条 co-primary 路径，Holm/α-split 校正）**：路径1（逐桶）任一桶 VALIDATED **或** 路径2（方向不对称 §4-B30③）β=μ_sell−μ_buy>0 校正后显著 ⟹ 报可交易 alpha（acceptance PASS）；两路径均无检出且**全部**桶 powered-FALSIFIED（含 UCB≤0）⟹ 纯 beta 照实 161；存在 INCONCLUSIVE 且无 VALIDATED 无 β 显著 ⟹ acceptance INCONCLUSIVE（不 PASS 不 161，走提功效链）。
- **主判据（§12 口径，非 p<0.05）**：`LCB_OOS(μ(z,a)) > 0`。χ 配置 `chi_theta=Some(0.0)`，`chi_z_alpha=1.645`。
- **功效教训（#180 §3 + memory oddeven）**：typed exit n≈2209，一类桶 n=1..11 结构性欠功效 ⟹ 预期**绝大多数（可能全部）桶 INCONCLUSIVE**。这是生产 π 语义忠实镜像，不是失败——欠功效桶照实报 INCONCLUSIVE，**不放宽门、不换判据、不借旧口径桶粒度预期**。桶粒度设计须匹配样本量：不复用旧「28 桶/52 桶」粒度，类数以实测为准。

---

## 4. B30 四项处置（★冻结令③——alpha 重启前置清单，逐项定形态）

`gap-master2-final` B30「alpha 重启 prereg 前置清单」四项在重启 alpha 检验时全部升 A。exit-μ-BUCKETING-FROZEN（§2）先于四项冻结（#180 §5）。逐项定本次 prereg 冻结形态：

| B30 项 | 原文要求 | 本次 prereg 形态 | 进/后置 + 理由 |
|--------|---------|-----------------|---------------|
| **①** B̂ᵢ 残差减法 + h/time-block 分层维 | alpha分离.pdf §4.1/§4.2：残差 `Y_i=δ(H−B̂)−C` 上检验；分层键 (ℓ, h bucket, time block, σ_higher) | 残差减法：**跨标的路径 B（L3）进本次**（`raw_i≠B_i` 减法非退化，7 品种池化）；单标的 BTC 走路径 A 分层内 δ 置换（beta 被分层吸收，665 单标的退化定理禁直接扣 B_market）。h/time-block：`h=exit_bar−entry_bar` 是 **post-treatment 量**（出场后才知），**只作 §3.1 第 3 层诊断切片**（事后异质性阅读），**绝不进裁决聚合基、不影响 accept/reject、不进 χ_t**（codex FAIL-2 修订：h 进判据层 = post-treatment 泄漏 = §2 违规）。time_block（入场时刻，非 post-treatment）作合法分层维 | **进本次**（路径 B 残差减法 L3 进裁决；h 诊断切片-only；time_block 合法分层）。**冻结写死，无待裁**（codex FAIL-1 修订：原「h 主判据 vs 切片」自由度删除——h 只能诊断切片，post-treatment 无判据资格） |
| **②** 生产 selector LCB 门控 | 严格alpha.pdf §12：`χ_t=1[LCB_t(μ)>θ]`；selector.rs 现为裸阈值无 LCB | selector LCB 门控只查询 `LCB_t(μ(entry_z, a))`（entry_z 单键，§2 exit 禁入门控）。检验层 LCB 已在 decontam/mu_estimator 装 | **后置**（附理由）：alpha 终局 INCONCLUSIVE 后生产侧 LCB 门控**无消费者**（μ̂ 门已降级为防灾难非 alpha，memory i_class×δ 共线）。#182 是检验层跑数不是生产化——生产 selector LCB 化待 #182 出现 VALIDATED 桶后触发。本次冻结检验层 LCB 口径（已装），不改生产 selector |
| **③** 方向不对称回归 μ_sell−μ_buy | alpha检验.pdf §7-§8：回归 `X_i=α+βD_i+ε_i`（D_i=1[level0 卖]），`β=μ_sell−μ_buy>0`，block bootstrap / cluster SE；§8 可加 regime 交互 `X_i=α+βD_i+γR_i+η(D_iR_i)`；§13 Step1 列 H2 为「更有希望」的预注册主假设 | 方向不对称量 `β=μ_sell(z)−μ_buy(z)` 逐结构桶（δ-free 基上按 δ 拆），回归 D_i 作 contrast + block bootstrap（时间块）或 cluster SE；可选 regime 交互项（R_i=市场窗涨跌）。**δ 是回归对象不进聚合桶键**（657 免疫：δ 作 contrast 不作 stratifier）。**§11 口径不混淆铁律**：block bootstrap = 评估**全信号**现策略（保留所有信号收益）；「每趋势段只取一个」= 定义**新去重策略**——本轮取前者（全信号 block bootstrap），不切换去重策略 | **进本次作 co-primary acceptance 判据**（§7 裁定1=A，冻死）：`β=μ_sell−μ_buy>0` 显著（block bootstrap 固定 **50 bar**，§7 裁定2=A）与逐桶 LCB>0 **并列**，任一成立即候选 PASS，**Holm/α-split 多重比较校正**（两独立路径防假阳）。block bootstrap 评估全信号策略（§11 口径，不切换去重策略）。理由：§13 列 H2 为更有希望主假设 |
| **④** shrinkage 层级收缩 | 过拟合.pdf：winner's curse 标准解 = shrinkage+LCB+OOS+block | shrinkage 只收缩 **entry-side 结构桶**（level 层级收缩，James-Stein 型向池化均值收缩），**不引入未来出场态**（§2 exit 禁入）。LCB 已装，shrinkage 显式估计 dlpdf-e 未单列（存疑留白） | **后置**（附理由）：shrinkage 是 winner's curse 缓解，其价值在**存在 VALIDATED 候选桶**时防单桶假阳。当前全 INCONCLUSIVE 无候选 ⟹ 无收缩对象。#182 若出现 VALIDATED 桶，shrinkage 作二次确认层触发。本次冻结「LCB 已装 + shrinkage 后置」的诚实状态，不造死估计器 |

**B30 冻结顺序**（#180 §5，本文件落地）：exit-μ-BUCKETING-FROZEN（§2）→ ①残差减法+分层 → ②selector LCB → ③方向不对称 → ④shrinkage。①③进本次检验层，②④后置待 VALIDATED 触发（附理由：无消费者/无收缩对象，非回避——alpha 未确认前生产化/收缩是声明膨胀）。

---

## 5. metadata 增补（★冻结令④——Θ 口径 + 多特征扩展）

### 5.1 Θ_SCORE 分层键（A2 #163）
- Θ_SCORE 原语已落（`theta_score()`：`m=dif_peak`；`β_norm=(m_A−m_C)/(m_A+m_C)∈[−1,1]`；K=3 边界 {≤0 非背驰, (0,0.33) 弱, ≥0.33 强}）。
- **本次冻结（写死，非候选）：Θ_SCORE bin 只作 §3.1 第 3 层诊断切片，不作 D 判定口径、不进裁决聚合基/W-VERIFY 裁决桶、不进 χ_t 门控**（codex FAIL-1 修订：「分层键候选」的模糊表述删除——Θ_SCORE bin 的地位写死为诊断切片-only）。角色 = 事后稳健性异质性阅读（beta-bucket-design v2 §4.3 方案 B），与 h/time-block 诊断切片同类。Θ_SCORE 由 dif_peak 计（入场时可算，非 post-treatment），但仍不进判据层——分层切片不改 accept/reject（§2 门控泄漏禁止同源）。
- 理由：Θ_SCORE 是标准化多特征得分（连续），进桶键碎样本（657）；作分层切片检验各 bin 异质性合法。

### 5.2 Θ_LEX 入终局口径（A3 #164）
- 四口径 D 判定探针 **G1-G4 已备**并同批 OOS（`wverify_run::thetadom_three_gauge_oos` 扩四口径）：
  - **G1 MacdArea**（默认，bit-exact 冻结基线，`Area(C)<Area(A)`）
  - **G2 ThetaDom**（`ForceStateA5==Dominated`，𝒜₅ 全支配衰减）
  - **G3 Conjunction**（G1∧G2）
  - **G4 ThetaLex**（`weak_theta(Lex)`：DIF 主 ▷ 面积次两层词典序，第17课黄白线主；「结构」层由 selector 级别分桶承载）
- **本次冻结：四口径全部进终局 OOS 对比**（a2 边界(f) 解除）。**默认口径 MacdArea bit-exact 不变**（ThetaDom/Conjunction/ThetaLex 仅 `ThetaConfig.divergence_gauge` 显式激活，非默认切换——判定口径变更影响信号集合，090 bit-exact 铁律）。
- 力度族 **𝒜₅ amended**（codex-a2 裁定 ISOMORPHIC-WITH-AMENDMENT）：`{macd_area, dif_peak, price_amplitude, price_speed, tv}` 5 proxy。第 6 成员 SubMovePower（Σ次级别同向段 m_{ℓ-1}）**维持诚实缺口**——A/C 抽取路径仍在 `&[Segment]` 坐标上，未携 LeveledMove.sub_moves 句柄（codex-a4 前置#1 未满足），升名 ForceStateA5→ForceState 不做（231 不造死字段）。#182 各口径统计报告标 A5 amended，不冒充原 A4。
- **既有探针结果**（`thetadom-three-gauge-oos-20260704.md`，A2/A3 已跑，作 #182 基线对照非终局结论）：G1 residuals=2256/type1=38/V=1·I=37；G2·G3 residuals=2233/type1=17/V=1·I=36。三口径唯一 Validated 桶均为 `L0 bsp3 δ+1 σ+1`（G4-ThetaLex 待 #182 补跑同口径）。

### 5.3 多特征扩展（待裁）
- 多特征扩展（Θ_SCORE 多 proxy 加权、force 6 成员补全 SubMovePower）**不占位**（§7 裁定3=B，冻死）：SubMovePower 缺口锁在 A/C 抽取路径 LeveledMove 化（独立 signal.rs 重构工位，非本终局域），数据源当前不可达 ⟹ 占位一个不可达形态 = 空自由度声明（231 不造死声明）。本次冻结现状 5-proxy 𝒜₅，多特征扩展待前置工位解锁后全新 prereg，不属本终局域。

---

## 6. q4 教训继承 + 样本量功效声明（★冻结令⑤⑥）

### 6.1 rerun5 七硬要求 + q4 四教训（继承不改）
1. **χ 语义空仓**（q4 §②）：新口径 μ 类数骤减 ⟹ χ 查询大量命中 μ=None。冻结：主臂 Arm1 `treat_empty_as_pass=false`（§13「无正边际收益不交易」spec 一致语义）；敏感臂 Arm2 `=true`（全放对照）。两臂差分 = 该参数语义权重度量，照实报告。q4 实证 χ spec 语义 9/12 窗空仓——本轮预期同类，照实。
2. **G7 margin 零 binding 声明**（q4 §④/§⑤）：q4 实证 G7 gross_cap 与 margin binding=0（无触发）。冻结：Arm1 显式 `enforce_gross_cap=true` + `margin=Some(CME-simple)`；若 #182 全历史 binding 仍 0，照实声明「G7/margin 在本数据零 binding」不回避。CME-simple 单段近似标签强制（231 有效域，不冒充经验校准）。
3. **置换分层 B22 三处同批**（q4 §③）：σ_higher 三处（base 键/重构闭包/判定键投影）同批不动 = 一致地不进桶键（§3.1）。方向性维（i_class 完整、force_state、σ_higher、任何 δ-派生）不进聚合基。
4. **657 共线自毁**（memory i_class×δ）：方向性/准方向性维进桶键即毁 δ 置换（perm_p 退化 1.000 + pooled 符号翻转 + 样本碎裂）。§3.1 事后锚监控主桶 perm_p。

### 6.2 样本量功效声明（typed exit n≈2209 量级）
- **口径**：μ 观测 = treatment-on-the-treated——生产 π fill loop 真开腿信号（`typed_ledger_from_bars`→`build_mu_from_bars`，τ^reverse 已删）。record 桶/slot 冲突/AncOK 被剪候选不入 μ。
- **样本量**：typed exit 口径 BTC 全 OOS **n≈2209 量级**（三口径 residuals 实测 2233-2256）。一类 typed trades 仅 17-38。较旧 τ^reverse 口径（3468 残差）下降约两个数量级（G4 typed exit 语义后果）。
- **功效教训（桶粒度须匹配样本量）**：一类桶 n=1..11 结构性欠功效（`¬powered`）⟹ INCONCLUSIVE 非证伪（667）。n≥门槛的桶集中在 L0（`n≈100-428`）。桶粒度设计约束：**不细分到样本量不支撑的粒度**——15 维载体全进桶键 ⟹ 每桶 <3 样本（#180 §3 代数），故主判据桶键投影到 δ-free 4-5 元组（§3.1），15 维余维作分层切片/诊断，不作确认层桶键。功效不足桶照实报 INCONCLUSIVE，走提功效链（收益率尺度 / L0 聚合 / L3 池化 / 多窗 walk-forward，acc-estimand §3.3），不放宽门。

---

## 7. codex decide 裁定（★已冻死，非待裁——codex FAIL-1 修订）

codex 审计 FAIL-1 指出「跑数前回填」= 数据挖掘后门（预注册不能带跑数前可调自由度）。故三项**在冻结时刻（本文件 commit 前）由 codex decide 裁定并写死**，不留到 #182：

1. **方向不对称 β=μ_sell−μ_buy>0 的地位** → **裁定 A：co-primary acceptance 判据**（与逐桶 LCB>0 并列，任一成立即候选 PASS），**预写 Holm/α-split 多重比较校正**（两条独立检验路径须校正，z_α 按 Holm 步降或 α-split 分配，防多重比较假阳）。理由：alpha检验.pdf §13 明列 H2 为「更有希望」的预注册主假设，理应进 co-primary；多路径由 Holm/α-split 校正吸收。冻结形态见 §4-B30③。
2. **block bootstrap block 长度** → **裁定 A：固定 50 bar**（写死，非自相关自适应）。理由：冻结期须可复现，宁保守牺牲功效不引数据依赖自由度。
3. **多特征扩展占位** → **裁定 B：不占位**（待前置工位 A/C 抽取路径 LeveledMove 化解锁后全新 prereg）。理由：SubMovePower 数据源当前不可达（诚实缺口），占位一个不可达形态 = 空自由度声明（231 不造死声明）。本次冻结 5-proxy 𝒜₅，多特征扩展不在本终局域。

三项均定理类/技术选型（codex decide 裁定，非价值选择），已冻死写入判据，本文件无残留待裁自由度。

---

## 8. 可证伪 fail 条件（继承 + 终局新增）

1. 跑数先于本 prereg 冻结 commit ⟹ 终局 alpha 直接 fail。
2. 冻结后修改本文件 ⟹ 谱系断裂（偏离照实入 #182 结果包，不改冻结文本）。
3. 主桶 perm_p 退化 1.000 ⟹ §3.1 桶键共线检查证伪，上浮。
4. R6 若 XZD C3 高级别 Type3 命中非 0 ⟹ 「旧 C3 死门」（§1.5）翻转，上浮。
5. #182 跑数所在 HEAD 缺任一 §1 版本锚 commit（a87540bdb6/c84c39e908/f9b3e41636/16ba38d5d5/fe582c3847/a3fb547375/29cdc2adbb/39e48b3636 或本 commit）⟹ 口径不一致，fail。
6. #182 走生产 collect_signals 侧若暴露与 §1.4 之 614 基线的新信号漏/多差（#147 未覆盖）⟹ 本轮 alpha 结果降级 INCONCLUSIVE 重跑。

**★可证伪性诚实边界（codex 审计 CONCERN-3，667/161）**：本轮**能**证伪的是**「存在可交易 alpha」**（fail 条件 = 无任一桶 VALIDATED 且 β 不显著 ⟹ acceptance 不 PASS）与**「特定桶纯 beta」**（powered-FALSIFIED，需 UCB≤0，L0 高 n 桶可达）。本轮**不能**在全局证伪**「无 alpha」**——因多数一类桶结构性欠功效（n=1..11 < 门槛），INCONCLUSIVE 是判据无检出力的诚实产出，`⊬ μ≤0`（667）。这是样本量的客观边界，**不是 fail 条件的空洞化**：把欠功效 INCONCLUSIVE 当「无 alpha」= 231 否定膨胀，把它当「有 alpha」= 声明膨胀，两者都禁。全局「无 alpha」的证伪须走提功效链（§6.2：收益率尺度/L0 聚合/L3 池化/多窗），是 #182 之后的独立里程碑，非本轮判据。fail 条件 3/4 的「上浮」= 该局部结论作废并停下复核（硬处置，非软警告）。

---

## 9. 结果包六要素

1. **结论**：冻结终局 alpha 重跑的完整口径——继承三份既冻 prereg（q4 七硬要求 / rerun5 614 基线 / acc-estimand 三态判据）+ 叠加 A 类闭合六项冻结令（①K_i/canonical/z15维/614/XZD 版本锚；②exit-μ-BUCKETING-FROZEN；③B30 四项逐项形态：①残差减法进裁决·h 诊断切片-only·③方向不对称升 co-primary·②④后置附理由；④Θ_SCORE 诊断切片·Θ_LEX 入四口径·多特征不占位；⑤q4 四教训；⑥typed exit n≈2209 功效声明）。桶键三层分离（裁决聚合基/报告桶/诊断切片，§3.1）。§7 三项 codex decide 已冻死写入判据，无残留待裁自由度。
2. **定义依据**：可交易判据 = μ(z,a)>0 非显著性（663）；桶键先验给定 δ-free（665 方向不对称免疫 + 657 共线免疫）；exit 信息进 X 计价不进桶键（§9 恒等式 + §12 因果可测性，#180 L0）；LCB≤0 不外推 μ≤0（667+231）；Θ_LEX/Θ_DOM = 关于背驰.pdf §9.2 三套预注册 Θ 的力度签名（p6/p10 Weak=力度度量非仅 MACD）。
3. **边界条件（结论翻转）**：(a) #182 任一桶 VALIDATED ⟹ acceptance PASS，触发 B30②selector LCB + ④shrinkage 生产化；(b) 全桶 powered-FALSIFIED(UCB≤0) ⟹ 纯 beta 161；(c) 主桶 perm_p→1.000 ⟹ 桶键共线证伪上浮；(d) SubMovePower 前置解锁（A/C 抽取 LeveledMove 化）⟹ 𝒜₅→𝒜₆ 支配序收窄须重冻结；(e) 默认口径切 ThetaLex/ThetaDom ⟹ 信号集变更全链重冻结。
4. **下游推论**：本冻结解锁 #182 终局跑数（a5 验收判定件）；#182 结果决定 M1 里程碑（有 alpha 进验证 / 无 alpha 照实 161 / INCONCLUSIVE 走提功效链）；B30②④ 的生产化/收缩以 #182 出现 VALIDATED 为触发前置。
5. **谱系引用**：135（冻结先于跑数）；180/exitmu-bucketing（exit 键裁定）；657（分类/方向维进桶键病理）+ memory i_class×δ 共线（δ 置换自毁）；663/665/667（判据=μ>0 / 单标的 beta 退化 / 功效门 inconclusive≠证伪）；231/formalization-validity-domain（有效域 ≠ 定义域，L0/L1/L2 分级 + CME-simple 标签）；090/161（bit-exact 铁律 + 否定性照实 + 声明膨胀禁止）；671（force=feature 非 veto，Weak 门在 judge 非 selector）；A 类各实装结果包（a2-thetadom / a3-weakforce / a7-exitmu / #138 zdims / #148 upgrade / #160 dualview / #175 etabucket）。
6. **影响声明**：新增 `.chanlun/review-results/final-prereg-20260704.md`（终局预注册冻结规格）。**不改任何代码、不改谱系、不产 L2 数字**——本文件是 #182 跑数前的口径冻结。冻结后不改本文件（fail 条件 2）。§7 三项 codex decide 已在冻结前裁定并写死，本文件无残留待裁自由度。

---

## 10. codex 审计记录（冻结前，异质审计闭环）

**审计源**：codex exec（gpt-5.5，`--skip-git-repo-check --sandbox read-only`），仅审计文档未改文件。参考原文 完整的策略.pdf §9/§12/§15/§16 + alpha检验.pdf §3/§5/§7-§8/§11。

**首轮审计判决**（草稿 v0）：
1. **冻结完备性 FAIL**：§7 三项「跑数前回填」= 数据挖掘后门；R6「成本允许最长窗」+ Θ_SCORE「分层键候选」未写死。
2. **口径泄漏 FAIL**：`h=exit_bar−entry_bar` 是 post-treatment，进检验分层/主判据违反 §2 exit-μ-BUCKETING-FROZEN；δ 在 W-VERIFY 桶键（90 行）与「方向性维不进桶键」（91 行）表述冲突。
3. **可证伪性 CONCERN**：perm_p=1/C3 非零/信号差只「上浮/降级」非硬 fail；预期大量欠功效 INCONCLUSIVE ⟹ 「无 alpha」难本轮证伪。

**修订（no-patch 严格性，逐条闭合）**：
- FAIL-1 → §7 三项在冻结前由 codex decide 裁定并写死（裁定1=A co-primary+Holm/α-split、裁定2=A 固定 50 bar、裁定3=B 不占位）；R6 窗口写死为 BTC 全历史 4613599 bar；Θ_SCORE bin 写死为诊断切片-only。
- FAIL-2 → §3.1 重写为**桶键三层分离**（裁决聚合基 δ-free / W-VERIFY 报告桶含 δ 作描述维·不裁决 / 诊断切片含 h·exit_type·Θ_SCORE·绝不进判据层）；h 从「主判据 vs 切片」自由度降为 post-treatment 诊断切片-only（进判据层 = §2 违规）；δ 双口径用途分离自洽（聚合基裁决用不含 δ，报告桶展示用含 δ）。
- CONCERN-3 → §8 增「可证伪性诚实边界」：本轮能证伪「存在 alpha」与「特定桶纯 beta」，不能全局证伪「无 alpha」（样本量客观边界，非 fail 空洞化）；欠功效 INCONCLUSIVE 两向膨胀均禁；「上浮」= 局部结论作废停下复核（硬处置）。

**复审判决（第二轮 codex，同口径）**：三维度全 **PASS**，无残留问题——(1) §7 三项已冻死（β co-primary+校正/block=50/不占位）+ R6 全历史窗 + Θ_SCORE 诊断-only；(2) §3.1 三层分离闭合（裁决聚合基 δ-free / 报告桶含 δ 仅描述 / h·exit_type·Θ_SCORE 只诊断）；(3) §8 可证伪性诚实（无 VALIDATED/β 不显著不伪装 PASS、欠功效不伪装无 alpha、上浮=硬停复核）。codex 交互全文内联本节摘录（首轮 FAIL/FAIL/CONCERN 三判决 + 三裁定 decide + 复审 PASS/PASS/PASS），未走 newchan.codex CLI 持久化路径（模块未安装）。
