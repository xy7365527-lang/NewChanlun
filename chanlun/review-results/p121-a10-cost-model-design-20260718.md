# p121 施工图：A10 成本模型（保证金/强平/资金费）——M6 机制盘点 → A10 验收就绪的接口/标定/守恒/裁定设计

日期：2026-07-18 ｜ 性质：**施工图（只设计不实装）**——零代码改动、零数据重放、零 git mutation、主仓零写入、零 cargo 调用（p116 后台重放不受干扰）
工位：主线阶段 2 **关⑦（A10 成本模型：保证金/强平/资金费——设计/裁定）**
输入在案：`TARGET_STRATEGY.md:84,92-101,117-126`（693 三分冻结 + A10 硬裁决 + 后续 goal #2）；`TARGET_STRATEGY_MAXFULL.md:138-145,148-159,259-270`（M6 验收式/M7/与 693 关系）；`.chanlun/genealogy/pending/693-strategy-object-triple-freeze-a11-a10-hard-ruling.md`（L2 缺口声明）；`.chanlun/review-results/m6margin-c2-20260704.md`（M6 实装报告）；`.chanlun/review-results/margin-model-design-20260703.md`（D2 缺口清单）；`docs/three_stages_accounting_design.md:281-355`（教义域外声明 + 阶段×保证金耦合）
教义引用格式 `0XX:段号` = 主仓 `docs/chanlun/text/blog/0XX-第X课.md` 行号，全部直读原文逐条核对（§1.4/§6 自检）
代码坐标格式 `文件:行号` = worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`）2026-07-18 现行状态直读核对

---

## 0. 结论速览

| 必答 | 一句话结论 |
|---|---|
| 1 与 M6 六项的关系 | **机制层六项已全覆盖、无需重造**：Commission/Slippage 由 `ExecConfig` fee_rate 经 `apply_fill` 进 PnL（既有）；Funding/Borrow/LiquidationLoss 由 `CostModel`（`risk.rs:770-828`）进 π loop（`runner.rs:1117-1127,1185-1201`）；margin 五态+强平 RiskExit 由 `MarginModel`+`k_theta_risk_gate`（`risk.rs:717-745`、`runner.rs:766-865`）承担；守恒断言 `RDecomposition`（`risk.rs:840-873`）在 `runner.rs:1824-1833` 落 debug_assert，BTC OOS 三窗残差 ~1e-6（m6margin-c2 §1/§6）。**A10 验收就绪的剩余项不是机制，是四件**：费率标定（L2 缺口）、ADL 缺口、有向 funding 口径、TW 账本不见持盾成本的 η 高估桥（G1，本设计新surface）。 |
| 2 模型设计要点 | 保证金 = 版本化 datum 快照簿（`MarginScheduleBook`，`risk.rs:662-695`，零前视 `as_of`）；强平 = 五态机 M0-M4 + GlobalRiskClose={M0,M1}（`risk.rs:288-376`）+ P1 RiskExit 屏蔽（coverage，既有）；资金费 = 周期边界计提 `|N|×rate`（`risk.rs:809-815`），**v0 无向保守口径**（有向 datum 留裁定 C2）。三项成本从 f64 cash 扣、独立累计、进 R 分解——守恒恒等式构造性成立（m6margin-c2 §2）。 |
| 3 κ 政策参数接口 | **现状已足，缺的是生产注入面**：`RiskPolicy`（`ledger.rs:253-410`）有理定点 κ=num/den（0.5 诚实承载，codex `.kappa-ruling-20260704`）、三构造闸拒负、`eta_star` ceil/i128、`enter_ready` 五合取、`buy_core_legal` i128 均已实装并被 `transition.rs:319-361` 消费。当前注入面只有 env 诊断 knob（`runner.rs:906-920,1054`，未设 ⟹ κ=0 bit-exact）。设计：升级 `ThetaConfig.risk_policy: Option<RiskPolicy>`（None ⟹ baseline，bit-exact），取值永走构造闸；κ 非价格可导（不可识别性定理2，`ledger.rs:241-242`），生产正 κ 是编排者选择类（693 注记），敏感性网格 {0,1/2,1,2} 只作敏感性陈述不作择优。 |
| 4 费率数据源 | **必须用户供（外部 datum）**：真实 funding 历史（venue+symbol、带符号、版本化）、借贷/借券利率曲线（若股票短卖）、强平清算费率表、ADL 规则与触发史。**可参数化带声明政策**：v0 三个常费率（funding/borrow/liq_penalty）——机制闭合用，一切数值报告强制口径标签 `[L1机制/费率未标定]`，禁作 alpha 论据（m6margin-c2 §4 先例）。混合推荐：常费率保底 + `FundingScheduleBook` 接口预冻结（与 margin book 同构），datum 到位不改签名。 |
| 5 冲突面+单测 | 冲突面四处：ThetaConfig 新字段（Option+Default None 安全，字面量构造点须盘点）、`CostModel` 既有 5+3 测试锁死（改签名破测试 ⟹ 走 additive）、runner 三区共享（成本区 1065-1127 / TW ②'②'' 1134-1168 / 门 ctx 1184-1278，新 shadow 只读桥接）、`enter_ready` 语义若改（C5-b，影响 transition+ledger 测试族 ⟹ 列裁定不直设）。单测 5 件既有锁定不动 + 新增 10 项（§5：datum book 族、有向 funding、funding→M1 链路、TW 桥、OQ-5(d) 守卫、κ 注入、守恒补强、口径标签、关②联动重跑批）。 |

---

## 1. 现状证据（全部直读核对，文件:行号）

### 1.1 对象定义层：A10 的三态归属（693 冻结表）

- `TARGET_STRATEGY.md:84`：A10 保证金/强平/资金费/ADL 对 Π_signal-full = **OUT_OF_SCOPE**（定义外非豁免），对 Π_exec-full = **MUST**（PDF §7 优先级 #2），对 Π_treasury-full = **MUST**（若带杠杆）；`:120` 排入后续 goal #2。
- `TARGET_STRATEGY_MAXFULL.md:138-145`（M6 关）：验收式 `R = Σ_t N_tΔP_t − Commission_t − Slippage_t − Funding_t − Borrow_t − LiquidationLoss_t`；必须含 margin/funding/borrow/liquidation/ADL 或等价强平/RiskExit 优先级。`:266-267`：M5-M6 ↔ Π_exec-full、M7 ↔ Π_treasury-full。
- 693 谱系（pending，待编排者 /ritual 追认）：`693:41-46` 硬裁决表；`693:103-113` M0-M8 实装兑现注记——A10 的 exec-full MUST 已由 M6 CostModel 兑现（机制面），**「长期 waiver 态正式解除」**；`693:140-142` 有效域声明：「兑现证据 = BTC/L2 单标的，**A10 成本机制的费率标定仍是 L2 缺口**（M6 自陈『费率待外部标定』），不外推跨标的」。

### 1.2 机制层五件（已实装，commit `47e21c5e7f` 已确认为本分支祖先）

1. **真保证金模型**（D2，task #113）：`MarginSchedule`（BinanceTiered/CmeSimple，`risk.rs:597-657`，fail-loud 构造 `:607-643`）+ `MarginScheduleBook` 分段快照簿（`risk.rs:662-695`，`as_of` 零前视 `:688-694`）+ `RiskCushions`（`risk.rs:699-715`）+ `MarginModel`（`risk.rs:717-722`）+ `margin_inputs` 派生 `RiskModeInput`（`risk.rs:731-745`，`liq_flag = equity ≤ MM`）。存在论在案：MM/liq 规则是**版本化 datum 非 Θ 参数**（`risk.rs:577-582`），B1/B2 是 Θ_risk 缓冲。
2. **五态风险模式 + 强平**：`RiskMode` M0-M4（`risk.rs:288-299`）、`risk_mode` 短路穷尽互斥（`risk.rs:337-349`，对齐 Lean `risk_mode_complete_unique`）、`global_risk_close = {Insolvent, Liquidation}`（`risk.rs:374-376`）。生产接线 `k_theta_risk_gate`（`runner.rs:766-865`）：margin book `as_of` 命中 ⟹ 真 MM；M0/M1 ⟹ `force_flat`、M2/M3 ⟹ `no_increase_cap`（`runner.rs:784-803,846-864`）；RiskExit P1 屏蔽 P2..P10（coverage.rs:2598，m6margin-c2 §1 在案）。
3. **M6 成本模型**：`CostModel`（`risk.rs:770-775`）四字段 + fail-loud `new`（`:780-804`）；`funding_accrual`（`:809-815`，周期边界 bar、`|N|×rate`、空仓 0）；`borrow_accrual`（`:819-822`，借入 = `max(0,|N|−E)`）；`liquidation_penalty`（`:825-827`，`|N|×罚金率`）。π loop 计提：`runner.rs:1117-1127`（funding/borrow 从 cash 扣，`px>0 ∧ units≠0` 才计）；强平罚金边沿触发 + `liq_active` 去抖（`runner.rs:1185-1201`）。
4. **R 分解 + 守恒断言**：`RDecomposition` 六项独立累计 + `net_r` + `ledger_delta` + `conservation_residual`（`risk.rs:840-873`）；runner 组装 + debug_assert 容差 `max(1e-6, 1e-9·(|nav0|+|price_pnl|))`（`runner.rs:1819-1833`）。价格 PnL 毛额逐 bar `units·Δpx`（`runner.rs:1088-1092`）；fee 由 `apply_fill` 独立测得（非反推，`runner.rs:1101-1103`）。BTC OOS 三窗守恒残差 −1.9e-6/−2.4e-6/−2.9e-6（m6margin-c2 §6）。
5. **配置面 + bit-exact 锁**：`ThetaConfig.margin: Option<MarginModel>`（`config.rs:303`）、`ThetaConfig.cost_model: Option<CostModel>`（`config.rs:309`）——`None` ⟹ 退化口径逐位相同（测试 `m6_cost_model_none_bit_exact_and_conserves`，`runner.rs:3633`）。另两件机制测试：`m6_liquidation_penalty_triggered_on_reachable_m1`（`runner.rs:3653`）、`m6_cost_model_some_funding_borrow_accrue_and_conserve`（`runner.rs:3679`）；BTC 跑批 `m6_btc_oos_r_decomposition`（#[ignore]，`wverify_run.rs`）。

### 1.3 κ 政策层（RiskPolicy，GAP3/M7 既有链）

- `RiskPolicy`（`ledger.rs:253-270`）：κ = `kappa_num/kappa_den` 有理定点，字段模块私有，唯一构造闸 `baseline()`/`try_new`/`try_new_ratio`（`:276-296`）⟹ κ≥0 是 constructor-only 类型不变量（对齐 Lean `kappa_nonneg`，`:309-313`）。有理化根由在案：M7 网格含 0.5，i64 不能诚实表达、禁静默改 {0,1,2}（codex `.kappa-ruling-20260704`，`:259-264` 注释）。
- barrier：`eta_star(s) = ⌈L^wc + κ·Q⌉`（`ledger.rs:327-332`，i128 中间域精确，κ=0 ⟹ 与旧 i64 同值 bit-exact）；`L^wc = (notional_in − withdrawn)⁺`（`ledger.rs:205-207`，在险本金语义）；`eta_bucket` 四桶（`:349-359`）。
- 消费链：`enter_ready` 五合取（`ledger.rs:373-379`：S=II ∧ W≥I0 ∧ legs=0 ∧ RiskNormal ∧ η≥η⋆）由 `stage_progression`（`transition.rs:319-361`）消费，`risk_normal = matches!(risk_mode, Normal)`（`transition.rs:320`）；`buy_core_legal`（`ledger.rs:395-409`，i128 移项零截断）。
- **A10×M7 联动已接线**：π loop 的 margin 感知 `risk_mode_i`（`runner.rs:1184`）投影进 `TwStepCtx`（`runner.rs:1262-1278`）⟹ margin=Some 且风险模式非 Normal 时 `enter_ready` 的 RiskNormal 合取失败——强平/去杠杆态直接阻断三阶段推进。这是 A10 对 treasury-full MUST 的**已兑现耦合面**。
- κ 注入面现状 = env 诊断 knob `kappa_policy_from_env`（`runner.rs:906-920`，`KAPPA_BARRIER_NUM/_DEN`，非法 panic 不静默降级），调用点 `runner.rs:1054`；env 未设 ⟹ `baseline()` κ=0，生产/测试逐字节不变。codex 在案：**正 κ 生产选择是 M8 L3 的事**（`runner.rs:1051-1053` 注释）；693 注记：终态判据待 κ 外部风险政策裁定（选择类）。
- ★同名防线：`RiskConfig.kappa`（`config.rs:174`，默认 2.0，sizing 成本倍数，`config.rs:202`）与 `RiskPolicy` barrier κ **同名不同义**（`ledger.rs:249-250` 注释在案）——接口设计必须物理隔离两 κ（§3.3）。

### 1.4 教义域外声明（硬约束，贯穿本设计）

直读主仓原文逐字核对（主仓 `docs/chanlun/text/blog/`）：

- 缠师**三次**否定杠杆/借贷：026:78「你手中的钱，一定是能长期稳定地留在股市的，不能有任何的借贷之类的情况」；028:20「基金，不过是所谓合法地借贷了很多钱而已……性质一样」；074:38「借贷炒股还是不少见。这是绝对不允许的，把资本市场当赌场的，永远也入不了资本市场的门」。
- ⟹ **margin/强平/资金费整体在缠师原文论域之外**（three_stages §4.1，`three_stages_accounting_design.md:281-292`）：缠师机制 = 零杠杆现货 + 恒定股数；A10 全部是设计层延伸建模，标注 `[L0域外]`，**不可宣称为缠师原意**。
- **「负成本免强平」禁编码进强平判据**（three_stages §4.4，`:319-331`）：OQ-5(d) **已结算为定理**（`:331`）——禁止把 `cost_basis<0` 编码进期货逐仓强平豁免；强平判据永远读 basis/权益域（>0），负成本「无风险」是现货命题，期货越界。第33课留白：「期货是可以随时开仓的，和股票交易凭证数量的基本稳定不同……这在以后再说」（033:50）——恒仓本体是股票特性，期货打破前提。
- 阶段×保证金耦合已有编排者裁决（three_stages §4.6，`:339-355`）：①②逐仓、③全仓（账户级 NAV≤0，1x 现货口径；杠杆期货应为 NAV≤maintenance，`:355` 有效域标注）。**theta_v0 生产 gate 当前 stage-agnostic**（`k_theta_risk_gate` 不读 `tw.stage`，`runner.rs:766-865`）——阶段三全仓切换未接线，列裁定 C4。
- 三阶段资金语义锚：049:60「（成本为0后是筹码增加，当然，对于小级别的操作，不会出现成本为0的情况）」；031:36「成本为0前用机动资金做短差……成本为0后……仓位是增加的」、031:169「成本为0后，抛出后，跌回来，就把抛出的钱，全补进去」。

---

## 2. 输入假设（哪些依赖关①/关②最终验收结果）

| # | 假设 | 依赖关系 | 若翻转 |
|---|---|---|---|
| H1 | **机制层设计与关①/关②解耦**：margin/cost/守恒/κ 接口的驱动源是持仓、权益、保证金表、TW 账本——不消费证书存在性（关① p116）或 BSP 口径（关② p117）的任何结论。 | 无依赖。本文 §3 全部接口/断言设计在关①关②任何验收结果下成立。 | — |
| H2 | **R 分解数值基线依赖关②验收**：关②（BSP 侧口径修复 S0/S1a/S2S4/037:20）改 BSP 集合 ⟹ π 订单流/fill 序列变 ⟹ `cum_fee/cum_funding/cum_liq` 数值变。m6margin-c2 的 BTC OOS 三窗 R 表（p3fold/wf7/wf8）在关②落地后**仅作机制证据存档**，不作 A10 验收数值基线。 | **依赖关②最终验收**。A10 验收跑批（T-N10）以关②（及关①，若翻转证书锚）验收后的订单流为重跑前提。 | 旧 R 表按口径标签规则冻结作废，重跑重冻结（m6margin-c2 §4「口径变更须重跑」先例）。 |
| H3 | **693 过渡状态合法**：693 谱系仍 pending（待编排者 /ritual 追认），按其自述「追认前 TARGET_STRATEGY.md 即为工作口径」为合法输入（693:60-63）。 | 依赖编排者不翻转 A10 归属格。 | 本文按 /ritual 基底刷新流程退回生成态重裁。 |
| H4 | **κ 生产取值未裁**：κ>0 的一切结果只作敏感性陈述（v3：风险政策是声明不是统计择优；codex 已裁正 κ 生产选择属 M8 L3）。 | 依赖编排者 κ 政策裁定（选择类，693 注记）。 | κ 冻结值变更 ⟹ T-N6 期望值与 treasury-full 终态判据重锚。 |
| H5 | **教义域外状态不变**：A10 全件标 `[L0域外]`；任何「A10 = 缠师原意」的表述即作废本文。 | 无验收依赖（纪律常量）。 | — |

---

## 3. 方案设计

### 3.1 A10 范围 verdict 与 M6 六项关系矩阵

A10 = M6 验收式六项的**机制面已兑现**（693:112 在案）+ 四个剩余闭合项。逐项矩阵：

| M6 项 | 机制现状（锚点） | A10 验收剩余 |
|---|---|---|
| ΣN_tΔP_t | `runner.rs:1088-1092` 逐 bar MtM | 无 |
| Commission/Slippage | `ExecConfig` fee_rate（`treasury.rs:26-28` 同口径 3e-4）经 `apply_fill` 独立测 fee | 无 |
| Funding | `CostModel::funding_accrual`（`risk.rs:809-815`） | **费率标定（L2 缺口）+ 有向口径（C2）** |
| Borrow | `CostModel::borrow_accrual`（`risk.rs:819-822`） | **perp 语义裁定（C3）+ 利率标定（若适用）** |
| LiquidationLoss | `CostModel::liquidation_penalty` + 边沿去抖（`runner.rs:1185-1201`） | **清算费率标定；强平成交价恶化参数（可选）** |
| margin/强平/RiskExit | `MarginModel`+五态+`force_flat`/`no_increase_cap`+P1 屏蔽（§1.2-1/2） | **保证金表 datum 版本维护；阶段三全仓切换（C4）** |
| ADL | **未建模**（margin-model-design §1.3：`liq_flag` 仅价格触发子集，`risk.rs:730` 注释在案） | **OUT_OF_SCOPE 声明或 datum 实装（C1）** |
| 守恒 | `RDecomposition`+debug_assert（§1.2-4） | **TW 口径桥 G1（C5）+ 事件级断言补强（§3.4）** |

### 3.2 费率标定数据来源选项

**A. 必须用户供（外部 datum，机制无法内造，缺口即声明）**：

1. **真实 funding 历史**（venue+symbol 维度、结算周期对齐、**带符号**）：版本化快照，与 `MarginScheduleBook` 同构的 `FundingScheduleBook` 设计——分段 `(effective_from, effective_to, signed_rate)`，`as_of` 零前视，fail-loud 构造（空/重叠/乱序/非有限 ⟹ Err）。用户从 venue 导出（Binance perp 8h funding 序列等），datum 版本哈希进报告标签。
2. **借贷/借券利率曲线**：现货杠杆借贷利率、股票短卖借券费+替代股息（margin-model-design §1.3：当前 BTC perp 标的下不 binding，接股票时必须补）。
3. **强平清算费率表**：venue 公布的清算手续费率（版本化 datum，同 margin 表先例）；强平成交价相对 mark 的恶化**分布**不收（v3：不引入概率分布作决策基础）——恶化以保守常数参数并入 `liq_penalty_rate` 上界声明。
4. **ADL 规则与触发史**（若 C1 裁实装）：venue ADL 队列规则 + 历史触发记录——纯外部。

**B. 可参数化带声明政策（机制闭合用，现 CostModel 三费率即此类）**：

- `funding_rate_per_period` / `borrow_rate_per_bar` / `liq_penalty_rate` 常数（`risk.rs:770-775`）。**声明政策骨架**：
  1. 一切带成本 R 数值报告强制口径标签 `[L1机制/费率未标定]`；datum 注入后升 `[L2费率标定: datum 版本哈希]`（有效域 231 号规则，m6margin-c2 §5 先例）。
  2. 常费率数值只用于机制验证与敏感性扫描，**禁作 alpha 论据、禁作策略择优输入**（v3：历史数据只验证代码正确性）。
  3. borrow 对 perp 的语义错位在 C3 裁定前保留参数但报告须声明「非市场借贷利率」。

**C. 混合（推荐）**：常费率保底（现状，bit-exact）+ `FundingScheduleBook` 接口预冻结（additive 新类型，不改 `funding_accrual` 签名）——datum 到位即插，签名不变，既有测试全绿。

### 3.3 κ 政策参数接口设计

**现状满足度**：`RiskPolicy` 本体（构造闸/有理定点/eta_star/enter_ready/buy_core_legal）**已完整**（§1.3），不需要新代数。缺口仅在**生产注入面**（env 诊断 knob 不是生产接口）与**同名混淆防线**。

**设计（接口冻结）**：

1. `ThetaConfig.risk_policy: Option<RiskPolicy>`（新字段）——`None` ⟹ `baseline()` κ=0，与现路径逐字节相同（bit-exact）；`Some` ⟹ π loop `tw_policy` 取之（替换 `runner.rs:1054` 单点）。取值**永走** `RiskPolicy` 三构造闸（禁裸 i64/f64 κ 字段出现在任何 config——类型边界闭合先例 `ledger.rs:264`）。
2. 优先序写死防双源漂移：`env KAPPA_BARRIER_*`（诊断覆写）> `config.risk_policy` > `baseline`。env 保留为 L2 敏感性网格 knob（{0,1/2,1,2} = 0/1,1/2,1/1,2/1），非法值 panic（现状语义不变）。单源纪律对齐 χ G2 先例（`runner.rs:1244-1246` 注释）。
3. **同名 κ 防线**：新字段命名 `risk_policy`（不带 kappa 字样）；`RiskConfig.kappa`（sizing 成本倍数 2.0，`config.rs:174`）不动；文档并列两 κ 声明（`ledger.rs:249-250` 已有注释，config 侧补镜像注释）。
4. κ 语义声明随值走：任何 κ>0 的报告标注「κ = operator 声明式风险政策（不可识别性定理2，`ledger.rs:241-242`），非价格导出、非回测择优」。

### 3.4 守恒断言方案

**现有（保留不动）**：R 域六项独立累计 + `net_r − ledger_delta` 残差 debug_assert（`runner.rs:1824-1833`）；TW 域守恒定理 `tw_step_preserves_tw`（七构造子）+ 漂移不变量 `tw_step_realize_drift_equals_dpi`（Realize 唯一漂移源）（`ledger.rs:184-190` 注释在案）。

**缺口 G1（本设计 surface，必须裁定）**：cost_model=Some 时 funding/borrow/liq 只扣 f64 `cash`（`runner.rs:1123,1196`），**TW 账本（i64，#124 裁定4 单一生产真值源）不经任何构造子见到持盾成本**——TW 漂移只认 `Realize(⌊realized_cum⌋)`（平仓 fill 费后 PnL，`runner.rs:1161-1168`）。后果：η = `tw()` **高估**真实在险权益 ⟹ `enter_ready` 的 η≥η⋆ **易过 = 激进侧（不安全）**——m6margin-c2 实测 funding 单窗 ~2e5 量级，非噪声。treasury-full 带成本运行前必须闭合。

- **选项 a**：新 `TwEvent` 构造子（如 `HoldingCost(d)`）——触碰 GAP3 裁定 A' 构造子冻结（八构造子 + Realize 资金源硬边界，`ledger.rs:430-445`），须补 drift 引理族 + codex/编排者裁定。
- **选项 b（推荐，零账本侵入）**：π loop 增 `cum_holding_cost` i64 shadow（funding+borrow+liq 累计量化，tw_seen_basis 同款模式）；enter_ready 消费侧把 η 修正为 `tw() − cum_holding_cost`（策略层修正项，TwEvent 代数不动），并加桥接对账断言 `tw() − η_corrected == ⌊cum_holding_cost⌋`（取整界内）。改 `enter_ready` 语义 ⟹ 列裁定 C5-b，影响面见 §4。
- **选项 c**：声明不管（A10 对 treasury-full 冻结为「成本进 R 不进 TW barrier」）——与「η 高估 = 激进侧」直接冲突，**本设计否此选项**。

**事件级断言补强（新增，不涉裁定）**：

1. funding 周期边界 bar：`cash` 减少恰 == `funding_accrual` 返回值（单点恒等，debug_assert）。
2. liq 边沿：`liq_active` 状态机断言——持续 {M0,M1} 跨 bar 恰罚一次，离开复位。
3. force_flat 成交的 fee 仍进 `cum_fee`（强平 bar 守恒不破）。
4. 两域桥（C5-b 落地后）：TW 修正 η 与 f64 cash 域 equity 的差恰 = 未入账桥项，对账断言入 R 分解报告。

**守恒范围声明**：R 守恒覆盖 f64 cash 域；TW 守恒覆盖 i64 TW 域；两账本不同构（#90/674），桥接断言是两域间**对账不变量**而非统一账本——不声称跨账本单一 Realize（`ledger.rs:442-445` 裁定清单⑨）。

### 3.5 裁定点清单（本设计不自决，登记编排者/codex）

| # | 裁定点 | 选项 | 本设计倾向 |
|---|---|---|---|
| C1 | ADL 建模 | (i) OUT_OF_SCOPE 声明（venue 规则+对手方假设全外部）；(ii) datum 实装 | (i)——`liq_flag` 独立字段已预留扩展点（`risk.rs:324`，margin-design `:166`），声明期间禁称「强平模型完备」 |
| C2 | funding 有向性 | (i) 无向保守口径（现：`|N|×rate` 恒为成本）；(ii) 有向 datum（long/short 互付，signed） | (i) 保底 + (ii) 接口预冻结（§3.2-C，additive 新类型不破既有签名） |
| C3 | borrow 对 perp 语义 | (i) 置 0 + 声明（perp 无借贷市场，funding 已承载持有成本）；(ii) 保留作资金机会成本参数 | 编排者定；裁定前报告强制声明「非市场借贷利率」 |
| C4 | 阶段三全仓切换（three_stages §4.6 裁决的 theta_v0 接线） | (i) v0 保持 stage-agnostic（保守超集：全程 ①②逐仓口径）；(ii) 按 `tw.stage` 显式切换到账户级 NAV≤MM（杠杆域修正版，`:355`） | (i) v0；切换属 treasury-full 实装关。**OQ-5(d) 定理贯穿两选项**：任何形态禁把 cost_basis<0 编码进强平豁免（T-N5 守卫测试锁死） |
| C5 | TW 口径桥 G1 | (a) 新 TwEvent 构造子；(b) η 修正 + 桥接对账（推荐）；(c) 声明不管（已否） | (b)，但改 `enter_ready` 语义 ⟹ 须裁定后实装 |

---

## 4. bit-exact 冲突面

| 面 | 内容 | 处置 |
|---|---|---|
| F1 | `ThetaConfig` 新字段（`risk_policy` / 可选 `funding_book`）：Default None ⟹ 全 `Default` 构造路径 bit-exact；**struct 字面量构造点须盘点**（config 默认值断言族 `config.rs:335-341` 须补新字段断言）。 | Option+None 模式（margin/cost_model 先例 `config.rs:301-309`）；盘点后逐点更新。 |
| F2 | `CostModel` 既有测试锁死：risk.rs 5 测试（构造/funding 边界/borrow/liq/fail-loud）+ runner 3 M6 测试（`runner.rs:3633/3653/3679`）。改字段/签名（如 `funding_accrual` 加方向参）即破。 | **additive**：新类型 `FundingScheduleBook` + 新方法 `funding_accrual_signed`，旧方法保留逐字。 |
| F3 | runner.rs 三区共享（m6margin-c2 §6 文件域协调在案）：成本累计区 `:1065-1127`、TW ②'②'' `:1134-1168`、门+ctx `:1184-1278`。G1 的 `cum_holding_cost` shadow 落成本区，对 TW 区**只读**桥接，不改 ②'②'' 语义（Realize 资金源硬边界不动）。 | shadow 模式复刻 `tw_seen_basis`（`runner.rs:1055-1057`）；TW 构造子零新增（C5-a 若采则另行裁定 diff）。 |
| F4 | `enter_ready` 语义（C5-b）：消费点 `transition.rs:354` + ledger.rs 测试族（eta_star/enter_ready/buy_core 系列 `:1018` 起）+ `TwStepCtx`（`runner.rs:1273-1278`）+ η_bucket 第 15 维（`runner.rs:1215`，a5-etabucket 终裁：γ_t = η 与 η⋆ 比较的离散化，左操作数须与 enter_ready 同一个量——修正 η 时**两处须同步**，否则 z 维与判据裂口）。 | 列裁定不直设；若采 b，η_bucket 与 enter_ready 同源修正为一笔改动的强制约束。 |
| F5 | margin book / funding book 的 `as_of` 零前视（`risk.rs:688-694`）：新 book 复刻同模式（边界 from含/to不含、无覆盖段 None、fail-loud 构造族）。 | 测试族复刻（T-N1）。 |
| F6 | `kappa_policy_from_env`（`runner.rs:906-920`）保留；config 注入后优先序（env>config>baseline）写死，防双源静默漂移。 | T-N6 锁优先序。 |

---

## 5. 单测计划

**既有锁定（期望值不得改）**：T-keep-1 risk.rs CostModel 5 测试；T-keep-2 `m6_cost_model_none_bit_exact_and_conserves`（`runner.rs:3633`）；T-keep-3 `m6_liquidation_penalty_triggered_on_reachable_m1`（`runner.rs:3653`）；T-keep-4 `m6_cost_model_some_funding_borrow_accrue_and_conserve`（`runner.rs:3679`）；T-keep-5 ledger.rs κ 族（baseline/try_new/try_new_ratio 约分拒负/kappa_nonneg/eta_star ceil/buy_core i128，`ledger.rs:1018` 起）。

**新增（按裁定落地分批；括号 = 前置裁定）**：

- T-N1 `FundingScheduleBook`：`as_of` 边界（from 含/to 不含）、未来快照不可见（零前视）、空/重叠/乱序/非有限 ⟹ Err（复刻 margin book 测试族，`risk.rs:669-686` 镜像）。（C2-ii 接口预冻结可先行）
- T-N2 有向 funding（C2-ii）：long×rate>0 付费、short×rate>0 收（符号表按 venue 文档 golden 对照钉死）、空仓恒 0；无向保守口径回归（`funding_accrual` 旧签名逐字不变）。
- T-N3 funding→equity→M1 链路：合成高 funding 序列使 E 跌破 MM ⟹ 次 bar `risk_mode=M1` ⟹ `force_flat` + penalty **恰一次**（`liq_active` 边沿）；持续 M1 不重复罚；离开复位后再进再罚。
- T-N4 TW 桥对账（C5-b）：cost_model=Some 下 `η_corrected = tw() − cum_holding_cost`；`tw() − η_corrected == ⌊cum_holding_cost⌋`（取整界）；**回归**：κ=0 ∧ cost=None ⟹ 与现 `enter_ready` 判据同值 bit-exact；η_bucket 与 enter_ready 左操作数同源（F4 约束）。
- T-N5 **OQ-5(d) 守卫**：构造 stage=EarningShares + 负成本域账本态，断言 `margin_inputs`/`risk_mode` 判据读 equity/MM **不变**（cost_basis 不进强平判据）——任何未来把 cost_basis 喂进强平链的改动被此测试击杀（three_stages `:331` 已结算定理的代码锁）。
- T-N6 κ config 注入：`risk_policy=Some(try_new_ratio(1,2))` ⟹ `eta_star` 用 ⌈L^wc+Q/2⌉；None ⟹ 与现路径逐字节一致；env 覆写优先序锁（env>config>baseline）；非法 env panic 语义不变。
- T-N7 borrow 边界补全：`|N|≤E` ⟹ 0；`E≤0` ⟹ 借入=|N|（破产态全借入，`risk.rs:818` 注释口径）；空仓 ⟹ 0。
- T-N8 守恒补强：funding 边界 bar 事件级恒等（cash 减少 == 计提输出）；funding+liq 同 bar 叠加下 R 残差 ≤ tol。
- T-N9 口径标签：R 分解报告落盘强制费率标定标签字段（`[L1机制/费率未标定]` vs `[L2: datum 哈希]`）——测试标签存在性（措辞纪律配套，防数值冒用）。
- T-N10 关②联动（H2）：关①/关②验收后重跑 `m6_btc_oos_r_decomposition`（#[ignore] 跑批），新旧 R 表对照入档——数值预期变（订单流变），机制断言（守恒/边沿/bit-exact None 路径）全绿。

---

## 6. 风险与边界

1. **费率未标定 ⟹ 数值不作论据**：一切带成本 R 数值仅机制证据（`[L1机制/费率未标定]` 标签强制，T-N9）；措辞纪律：不得称「杠杆版完整策略已回测」（`TARGET_STRATEGY.md:99` 推论），A10 结论有效域 = BTC/L2 单标的（693:140-142），不外推跨标的。
2. **η 高估激进侧（G1）未闭合前**，treasury-full 带成本运行禁止——EnterReady 提前触发是真资金风险（§3.4）。
3. **borrow 语义错位**（C3 未裁）：perp 无借贷市场，参数保留期间报告强制声明。
4. **ADL 缺口期间**禁称「强平模型完备」（C1）。
5. **教义边界**：A10 全件 `[L0域外]`（026:78/028:20/074:38 三次否定借贷；three_stages §4.1）——不宣称缠师原意；期货有效域留白（033:50）；阶段三全仓的 NAV≤0 是 1x 现货口径，杠杆域须 NAV≤maintenance（three_stages `:355`）。
6. **v3 硬禁令合规自查**：费率标定数据只作成本真实化（代码正确性验证），不作信号/策略择优；κ 网格只作敏感性陈述；五态机/计提/守恒全是决定论规则，零概率推断进决策链；无任何新第三方依赖引入（本设计纯 crate 内类型）——辩证位置：成本是资本运动的现实摩擦（对象否定对象：利润被费用否定），不是外生噪声建模。
7. **关①关②数值联动（H2）**：验收前一切 R 数值基线标「关②前口径」；验收后 T-N10 重跑重冻结。
8. **p116 后台重放纪律**：本轮零 cargo 调用（已守）；实装关开跑前须确认重放收口。

---

## 7. 涉及文件清单

**只读证据（本轮零改动）**：

- `rust/src/theta_v0/strategy/risk.rs`（CostModel `:770-828`、RDecomposition `:840-873`、MarginModel/Schedule/Book `:584-745`、RiskMode/global_risk_close `:288-376`）
- `rust/src/theta_v0/config.rs`（margin `:303`、cost_model `:309`、risk.kappa `:174,:202`、默认断言族 `:335-341`）
- `rust/src/theta_v0/backtest/runner.rs`（κ knob `:906-920`、tw_policy `:1054`、成本累计 `:1065-1127`、TW ②'②'' `:1134-1168`、gate 调用+liq 边沿 `:1184-1201`、ZExt/TwStepCtx `:1212-1278`、R 组装+守恒断言 `:1819-1846`、`k_theta_risk_gate` `:766-865`、M6 测试 `:3630-3700`）
- `rust/src/theta_v0/strategy/ledger.rs`（TStage `:84`、TwState `:145-213`、l_wc `:205-207`、EtaBucket `:228-236`、RiskPolicy `:253-410`、TwEvent/Tw 守恒注释 `:184-190,:412-459`、κ 测试族 `:1018` 起）
- `rust/src/theta_v0/closed_loop/transition.rs`（stage_progression `:319-361`、hybrid_step+κ=0 便捷入口 `:536-561`）
- `rust/src/theta_v0/backtest/treasury.rs`（fee_rate `:26-28`、settle/settle_all `:62-87`）
- `rust/src/theta_v0/backtest/wverify_run.rs`（m6 BTC OOS 跑批 #[ignore]）
- 文档/谱系：`TARGET_STRATEGY.md`、`TARGET_STRATEGY_MAXFULL.md`、`.chanlun/genealogy/pending/693-…md`、`.chanlun/review-results/m6margin-c2-20260704.md`、`.chanlun/review-results/margin-model-design-20260703.md`、`docs/three_stages_accounting_design.md:281-355`
- 教义直读：主仓 `docs/chanlun/text/blog/026:78`、`028:20`、`074:38`、`033:50`、`049:60`、`031:36,169`

**未来实装触及面（裁定后，按 §4 冲突面执行）**：`config.rs`（risk_policy 字段+默认断言）、`risk.rs`（FundingScheduleBook 新类型+新方法，additive）、`runner.rs`（cum_holding_cost shadow+桥接断言+κ 注入点换源）、`transition.rs`+`ledger.rs`（C5-b 若采：η 修正+η_bucket 同源）、`wverify_run.rs`（T-N10 重跑批）。

---

## 8. 自检清单

- [x] 教义五条引文直读主仓原文逐字核对（026:78/028:20/074:38/033:50/049:60/031:36,169），行号即段号。
- [x] 全部代码坐标直读 worktree 现行状态核对（§1 各锚点逐一 grep/read 验证）。
- [x] 零代码改动、零 cargo、零 git mutation、主仓零写入（唯一写入 = 本文档）。
- [x] 输入假设节（§2）声明关①关②依赖边界；090：机制已实装/费率未标定/ADL 未建模/κ 未裁 四状态分别如实标注，无模糊地带。
- [x] 裁定点 C1-C5 不自决（选择类留编排者）；OQ-5(d) 已结算定理贯穿（T-N5 守卫）。
