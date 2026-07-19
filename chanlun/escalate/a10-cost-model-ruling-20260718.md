# 裁定：关⑦ A10 成本模型 C1–C5 裁定点定稿（ADL＝OUT_OF_SCOPE 登记 venue 适配项 / funding 有向 datum＋additive 接口预冻结 / v0 perp 口径 borrow≡0 / 阶段三全仓 v0 不切＋OQ-5(d) 定理 T-N5 锁死 / TW 桥 G1 取 (b) η 修正＋桥接对账；附 κ 接口与费率标定数据源两则）

**状态：代理裁定生效（2026-07-18）**

**授权**：编排者 2026-07-18 指令——把关⑦（A10 成本模型）C1–C5 裁定点定稿，「直接采纳设计推荐，落为正式裁定」；κ 政策接口与费率标定数据源照设计落为条款；约束：零 cargo、零 git mutation、主仓只读。裁定权链条同 `nest-predicate-caliber-ruling-20260717.md` 文头（编排者 2026-07-17 下放）。本文落锤 C1–C5 及附则 A/B；编排者醒后复议通道开放，见文末声明。

**依据**（引用格式：`设计:NN`＝`chanlun/review-results/p121-a10-cost-model-design-20260718.md` 行号；`TS:NN`＝`TARGET_STRATEGY.md` 行号；`MF:NN`＝`TARGET_STRATEGY_MAXFULL.md` 行号；`693:NN`＝`.chanlun/genealogy/pending/693-strategy-object-triple-freeze-a11-a10-hard-ruling.md` 行号；`TS3:NN`＝`docs/three_stages_accounting_design.md` 行号（主仓只读核对）；代码坐标＝worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`），runner.rs 行号漂移注记见文末 090 节）：

- 设计全文：裁定点表 设计:137-145；费率数据源 设计:89-105；κ 接口 设计:107-116；守恒与 G1 三选项 设计:118-135；冲突面 设计:149-158；单测计划 设计:164-177；风险边界 设计:181-190。
- 693 工作口径：TS:84（A10 三态归属格）、TS:92-101（§3.1 硬裁决＋措辞约束）、TS:117-126（§5 后续 goal #2）；MF:138-145（M6 关六项验收式）、MF:147-159（M7 η⋆/EnterReady/κ 冻结）、MF:259-270（M 段↔693 对象映射与边界互斥）；693:41-46（硬裁决表）、693:103-113（M6 实装兑现注记，「长期 waiver 态正式解除」）、693:140-142（有效域声明：费率标定仍是 L2 缺口，不外推跨标的）。
- 教义域外声明：TS3:281-292（§4.1 零杠杆论域，缠师三次否定借贷 026:78/028:20/074:38，A10 全件 `[L0域外]`）、TS3:319-331（§4.4 负成本免强平禁编码，OQ-5(d) 已结算为定理）、TS3:339-355（§4.6 阶段×保证金耦合编排者裁决 2026-06-20；杠杆期货 NAV≤maintenance 有效域标注）。

## 总览

| # | 裁定点 | 裁定 | 核心锚 |
|---|---|---|---|
| C1 | ADL 建模 | **OUT_OF_SCOPE 声明**：v0 无 ADL 规则史 datum，不建模；登记为部署环境适配项（venue adapter），datum 到位另立新裁定；声明期间禁称「强平模型完备」 | 设计:86,96,141,186；MF:140；TS:99 |
| C2 | funding 有向性 | **有向 datum＋additive 接口预冻结**：费率带符号、方向由持仓符号决定；`FundingScheduleBook`＋`funding_accrual_signed` 预冻结；常费率无向保守口径保底，报告强制 `[L1机制/费率未标定]` | 设计:82,93,105,142,154；MF:143 |
| C3 | borrow 对 perp 语义 | **v0 perp 口径 borrow≡0**（选项 (i)：永续无借券，funding 已承载持有成本）；borrow 项仅适用现货杠杆/借币场景；参数保留＋报告强制声明「非市场借贷利率」；(ii) 机会成本解释否决 | 设计:83,94,103,143,185；TS3:283-286 |
| C4 | 阶段三全仓切换 | **v0 保持 stage-agnostic**（选项 (i)：全程①②逐仓口径，保守超集）；切换属 treasury-full 实装关（口径预锚 NAV≤maintenance）；**OQ-5(d) 定理贯穿**，T-N5 守卫测试升强制 | 设计:55,144,172；TS3:331,343-355 |
| C5 | TW 口径桥 G1 | **取 (b)：η 修正＋桥接对账**（零账本侵入，GAP3 A' 冻结不动）；(a) 新构造子否决（碰冻结面）、(c) 声明不管否决（η 高估＝激进侧不安全）；F4 同源修正强制；G1 闭合前 treasury-full 带成本运行禁止 | 设计:122-126,135,145,156,171,184；MF:150,153 |
| 附则A | κ 政策接口 | `ThetaConfig.risk_policy: Option<RiskPolicy>`（None⟹baseline bit-exact）；优先序 env>config>baseline 写死；同名 κ 物理隔离；κ=0 基线冻结；正 κ 生产取值留编排者选择类 | 设计:107-116,151,158,173；MF:149-150 |
| 附则B | 费率标定数据源 | 必须用户供四类 datum；v0 三常费率保底＋强制标签（禁作 alpha 论据）；`FundingScheduleBook` additive 预冻结；强平恶化不收分布 | 设计:89-105,176,183,188；693:140-142 |

---

## C1 ADL（自动减仓）＝ OUT_OF_SCOPE 声明＋venue adapter 适配项登记

### 背景
A10 对象清单含 ADL（TS:84；MF:140「margin；funding；borrow；liquidation；ADL 或等价强平；RiskExit 优先级」）。机制现状：强平面已由五态机 M0-M4＋GlobalRiskClose={M0,M1}＋RiskExit P1 屏蔽实装（设计:34），但 `liq_flag` 仅覆盖价格触发子集（`risk.rs:730` 注释「v0 逐仓价格触发；ADL/funding 为缺口，诚实标注」，直读核对），**ADL 未建模**（设计:86）。ADL 的规则与触发史是纯外部 datum（venue 队列规则＋对手方假设，设计:96），v0 运行环境无此数据，机制无法内造。设计倾向 (i) OUT_OF_SCOPE 声明（设计:141）。

### 依据
- 设计:86（§3.1 矩阵 ADL 行：未建模，剩余项＝OUT_OF_SCOPE 声明或 datum 实装）；设计:96（§3.2-A.4：ADL 规则与触发史纯外部）；设计:141（§3.5 C1 行：倾向 (i)，`liq_flag` 独立字段已预留扩展点 `risk.rs:324`）；设计:186（§6-4：ADL 缺口期间禁称「强平模型完备」）。
- 693 行锚：MF:140（M6 必须含「ADL **或等价强平**」）；693:103-113（A10 exec-full MUST 已由 M6 CostModel 机制面兑现，「长期 waiver 态正式解除」）；TS:84（A10 对 exec/treasury-full＝MUST）。
- 教义域外声明锚：TS3:290（margin/强平/资金费整体在缠师原文论域之外，`[L0域外]`；ADL 尤甚——venue 微观结构规则在教义中无任何对应物）。
- 措辞纪律锚：TS:99（禁以缺失构件支撑「完整策略」表述的 090 推论）。

### 裁决
1. **ADL 对 v0＝OUT_OF_SCOPE 声明**：v0 环境无 venue ADL 规则与触发史 datum；ADL 不建模、不实装、不以任何参数近似冒充。
2. **登记为部署环境适配项（venue adapter）**：ADL 缺口登记为部署环境适配任务——venue 部署目标确定且 datum（队列规则＋历史触发记录）到位后，经 venue adapter 层实装；扩展点已预留（`liq_flag` 独立字段，`risk.rs:324` 直读核对在案）。实装启动＝新任务＋新裁定，不在本裁定范围。
3. **M6 验收式语义不受损**：MF:140 要求「ADL 或等价强平」——等价强平/RiskExit 优先级面已由五态机＋GlobalRiskClose＋RiskExit P1 屏蔽兑现（693:103-113 在案）；本 OUT_OF_SCOPE 声明不撼动 A10 机制面的 MUST 兑现状态，仅是缺口如实标注。
4. **措辞纪律（090）**：声明期间任何报告/文档**禁称「强平模型完备」「A10 机制闭合」**（设计:186）；不得以完整策略名义引用缺失构件；引用强平机制时须随文注记「ADL 未建模（venue 适配项）」。

### 验收线
- 文档闸：本裁定后新增带强平/清算语义的报告无「ADL 未建模（venue 适配项）」注记＝文档缺陷；出现「强平模型完备」表述＝090 违例。
- 零代码改动：本裁定前后生产源码零改动；`liq_flag` 语义不变（`risk.rs:730` 注释口径维持）。
- datum 触发条件：venue 部署目标确定且 ADL datum 可获取 ⟹ 立 venue adapter 适配项任务（新裁定），验收线由该新裁定确立。

### 不回滚条款
- 不得以 ADL 缺失为由宣布 A10 机制面 MUST 失效或回滚 M6 既有实装（693:103-113）。
- 不得在 OUT_OF_SCOPE 声明下静默实装任何 ADL 逻辑（含以调高 `liq_penalty_rate` 冒充 ADL 模拟——090 声明与能力一致纪律）。
- 声明的解除只能经 venue adapter 适配项新裁定；声明期间措辞纪律不得放宽。

---

## C2 funding 有向性＝有向 datum＋additive 接口预冻结（常费率保底带标签）

### 背景
v0 现状为**无向保守口径**：`funding_accrual` 按周期边界计提 `|N|×rate`、恒为成本（`risk.rs:809-815` 直读核对：`net_notional_usd.abs() * rate`；设计:16）。真实 funding 是带符号的（long/short 互付），真实历史是外部 datum（venue+symbol 维度、结算周期对齐、版本化，设计:93）。设计倾向 (i) 无向保底＋(ii) 有向接口预冻结（设计:142），混合方案 §3.2-C（设计:105）。

### 依据
- 设计:82（§3.1 矩阵 Funding 行：剩余＝费率标定 L2 缺口＋有向口径 C2）；设计:93（§3.2-A.1 `FundingScheduleBook` 设计：分段 signed_rate、`as_of` 零前视、fail-loud 构造）；设计:105（§3.2-C 混合推荐：additive 新类型不破既有签名）；设计:142（§3.5 C2 行）；设计:154（F2：CostModel 5+3 测试锁死，有向化必须 additive）。
- 693 行锚：MF:143（M6 验收式 Funding_t 项）；693:140-142（费率标定 L2 缺口声明，有效域 BTC/L2 单标的）。
- 教义域外声明锚：TS3:290（funding 属 margin 论域延伸建模，`[L0域外]`）。

### 裁决
1. **有向 funding 口径确立**：funding 费率为带符号 datum（signed rate），成本方向由持仓符号决定——long×rate>0 付费、short×rate>0 收费（符号表以 venue 文档 golden 对照钉死，T-N2）；空仓恒 0。模型支持有向费率。
2. **additive 接口预冻结**：新类型 `FundingScheduleBook`（分段 `(effective_from, effective_to, signed_rate)`、`as_of` 零前视、fail-loud 构造：空/重叠/乱序/非有限 ⟹ Err，与 `MarginScheduleBook` 同构）＋新方法 `funding_accrual_signed`——**不改 `funding_accrual` 既有签名与字段**（F2，设计:154）；datum 到位即插，既有 5+3 测试全绿（设计:105）。
3. **常费率保底**：v0 无向保守口径 `|N|×rate` 保留为保底路径，bit-exact 不动；datum 未注入期间，一切带 funding 成本的 R 数值报告强制标注 `[L1机制/费率未标定]`（与附则 B 联动）。

### 验收线
- T-N1：`FundingScheduleBook` 测试族复刻 margin book（`as_of` 边界 from 含/to 不含、未来快照不可见、构造拒错族；设计:168）。
- T-N2：有向 funding——long×rate>0 付费、short×rate>0 收（venue 文档 golden 对照钉死）、空仓恒 0；无向保守口径回归（`funding_accrual` 旧签名逐字不变）（设计:169）。
- T-keep 族（CostModel 5＋M6 3＋κ 族）期望值零改动全绿（设计:164）。

### 不回滚条款
- 不得改 `funding_accrual` 旧签名/字段（F2 测试锁死）；有向化只走 additive 新类型＋新方法。
- 常费率保底路径与 `[L1机制/费率未标定]` 标签在 datum 注入前不得移除或弱化。
- funding 符号方向不得自行定义——只能锚 venue 文档 golden（T-N2）；datum 版本哈希随报告标签走（附则 B）。

---

## C3 borrow 对 perp 语义＝v0 perp 口径 borrow≡0＋语义边界声明（选项 (i)）

### 背景
M6 验收式含 Borrow_t 项（MF:143），机制已实装 `borrow_accrual`（借入＝`max(0,|N|−E)`，`risk.rs:819-822`，设计:35）。但**永续合约无借券/借贷市场**，持有成本已由 funding 承载——borrow 参数对 perp 是语义错位（设计:143,185）。设计 §3.5 C3 行未给倾向（选项 (i) 置 0＋声明 / (ii) 保留作资金机会成本参数，留编排者定，设计:143）；编排者指令落 **(i)**。

### 依据
- 设计:83（§3.1 矩阵 Borrow 行：perp 语义裁定 C3）；设计:94（§3.2-A.2：借贷/借券利率曲线——BTC perp 标的下不 binding，接股票时必须补）；设计:103（§3.2-B.3：报告强制声明「非市场借贷利率」）；设计:143（§3.5 C3 行）；设计:174（T-N7 borrow 边界）；设计:185（§6-3）。
- 693 行锚：MF:143（验收式 Borrow_t 项——borrow=0 是合法取值，六项机制面齐备即满足）。
- 教义域外声明锚：TS3:283-286（缠师三次否定借贷——borrow 概念本身在原文论域之外，延伸建模属性最显）。

### 裁决
1. **v0 perp 口径 borrow≡0**：永续无借券，funding 已承载持有成本；perp 标的一切生产/验收运行 borrow 项恒 0。
2. **语义边界声明**：borrow 成本项仅适用于现货杠杆借贷、股票短卖借券、期货借币场景；该类标的接入时走 datum 实装（利率曲线＝外部 datum，附则 B 裁决第 1 条②），另立任务。
3. **参数保留＋强制声明**：`borrow_rate_per_bar` 字段与 `borrow_accrual` 机制保留（F2 测试锁死不动）；v0 perp 口径一切报告引用 borrow 项时强制声明「非市场借贷利率」（设计:103,185）。
4. **选项 (ii) 否决**：perp 口径下不得把 borrow 参数解释为「资金机会成本」用于任何数值论据——语义错位参数的机会成本化是数值冒用通道（090）。

### 验收线
- T-N7（机制边界钉死，参数保留期间）：|N|≤E ⟹ 0；E≤0 ⟹ 借入=|N|；空仓 ⟹ 0（设计:174）。
- 报告闸：perp 口径带成本 R 报告的 borrow 行＝0 且带「非市场借贷利率」声明；缺声明＝文档缺陷。
- 接股票/借币标的时：borrow datum（借券费＋替代股息/借币利率）到位＋新任务裁定，双前置缺一不得启用。

### 不回滚条款
- perp 口径下不得以任何名义（含机会成本）给 borrow 赋非零值进验收证据。
- 「非市场借贷利率」声明不得移除（datum 实装后由新裁定替换为 L2 标定标签）。
- `borrow_accrual` 机制与字段不得因本裁定删改（F2）；语义边界是口径声明，不是机制删除。

---

## C4 阶段三全仓切换＝v0 保持 stage-agnostic（选项 (i)）＋OQ-5(d) 定理 T-N5 锁死

### 背景
three_stages §4.6 已有编排者裁决（2026-06-20）：阶段①②逐仓、③全仓（账户级 NAV≤0，1x 现货口径；杠杆期货应 NAV≤maintenance，TS3:355 有效域标注）。但 theta_v0 生产 gate 当前 **stage-agnostic**——`k_theta_risk_gate` 不读 `tw.stage`（`runner.rs:839` 起，直读核对；设计:55），阶段三全仓切换未接线。设计倾向 (i) v0 保持 stage-agnostic（保守超集：全程①②逐仓口径），切换属 treasury-full 实装关（设计:144）。**OQ-5(d) 已结算为定理**（TS3:331）：禁止把 `cost_basis<0` 编码进期货逐仓强平豁免——贯穿两选项。

### 依据
- 设计:55（§1.4：阶段×保证金耦合已有编排者裁决，v0 gate stage-agnostic，列裁定 C4）；设计:85（§3.1 矩阵 margin 行：阶段三全仓切换 C4）；设计:144（§3.5 C4 行：倾向 (i) v0，OQ-5(d) 贯穿）；设计:172（T-N5 守卫测试）。
- 693 行锚：MF:267（M7↔treasury-full——全仓切换是 treasury-full 语义）；MF:150（η 负成本缓冲是 M7 对象，不是强平判据输入）。
- 教义域外声明锚：TS3:319-331（§4.4：负成本「无风险」是现货命题；强平判据永远读 basis（>0）；第33课期货留白；OQ-5(d) 已结算为定理，不待裁决）；TS3:339-355（§4.6 编排者裁决：①②逐仓③全仓表；:349-351 与 §4.4 一致性论证——显式切换≠静默豁免；:353 1x 几何塔 NAV≥0 自洽；:355 杠杆期货 NAV≤maintenance）。

### 裁决
1. **v0 保持 stage-agnostic（选项 (i)）**：`k_theta_risk_gate` 不读 `tw.stage`，全程按 ①②逐仓口径运行（保守超集）；**阶段三全仓切换 v0 不接线**，列为 treasury-full 实装关任务。
2. **切换口径预锚**：未来切换落地时按 TS3 §4.6 裁决执行，且杠杆期货域必须用修正版 **`NAV ≤ maintenance`**（TS3:355），不得直接搬 1x 现货口径 `NAV ≤ 0`；落地＝新任务＋新裁定。
3. **OQ-5(d) 定理贯穿**：任何形态（v0 逐仓或未来全仓）下**禁止把 `cost_basis<0` 编码进强平判据/豁免**——强平判据永远读 basis/权益域（>0）（TS3:326,331）；负成本不免强平，与教义域外声明一致。
4. **T-N5 守卫测试升为强制**：构造 stage=EarningShares＋负成本域账本态，断言 `margin_inputs`/`risk_mode` 判据读 equity/MM **不变**（cost_basis 不进强平判据）——任何未来把 cost_basis 喂进强平链的改动被此测试击杀（设计:172）；T-N5 从计划项升为本裁定强制验收件。

### 验收线
- T-N5 落地并常驻测试族（OQ-5(d) 的代码锁）。
- v0 行为零变化：本裁定不改代码；`k_theta_risk_gate` stage-agnostic 现状维持，既有 gate/M6 测试族全绿。
- 报告闸：引用阶段×保证金耦合时带 `[L0域外]` 标注＋「v0 全程逐仓口径（保守超集）」注记；缺失＝文档缺陷。

### 不回滚条款
- OQ-5(d) 定理不可重开（TS3:331 自动结算、不待裁决）；T-N5 不得删除、弱化或改期望值。
- 不得静默接线阶段三全仓切换（含以参数/配置形式引入账户级判据）——切换只能经 treasury-full 实装关新裁定，口径预锚 NAV≤maintenance（杠杆域）。
- `[L0域外]` 标注不得移除（TS3:290）；不得宣称逐仓/全仓阶段映射为缠师原意。

---

## C5 TW 口径桥 G1＝取 (b) η 修正＋桥接对账（(a)/(c) 否决）

### 背景
**G1 缺口**（设计:122）：cost_model=Some 时 funding/borrow/liq 只扣 f64 `cash`，**TW 账本（i64，#124 裁定4 单一生产真值源）不经任何构造子见到持盾成本**——TW 漂移只认 `Realize(⌊realized_cum⌋)`。后果：η＝`tw()` **高估**真实在险权益 ⟹ `enter_ready` 的 η≥η⋆（MF:153 五合取第五项）**易过＝激进侧（不安全）**——m6margin-c2 实测 funding 单窗 ~2e5 量级，非噪声。treasury-full 带成本运行前必须闭合（设计:184）。三选项：(a) 新 TwEvent 构造子（碰 GAP3 A' 冻结）；(b) η 修正＋桥接对账（零账本侵入，推荐）；(c) 声明不管（设计已否）（设计:124-126）。

### 依据
- 设计:122（G1 缺口与后果定量）；设计:124-126（三选项与倾向 (b)）；设计:135（守恒范围声明：两账本不同构 #90/674，桥接断言＝对账不变量非统一账本）；设计:145（§3.5 C5 行）；设计:156（F4：η_bucket 与 enter_ready 左操作数同源修正强制约束，a5-etabucket 终裁）；设计:171（T-N4）；设计:184（§6-2：G1 未闭合前 treasury-full 带成本运行禁止）。
- 693 行锚：MF:150,153（M7 EnterReady 五合取 η_t≥L^wc＋κQ——η 取数口径直接决定判据松紧）；MF:267（M7↔treasury-full）。
- 教义域外声明锚：TS3:290（TW/η 属三阶段会计设计层延伸，`[L0域外]` 同步适用）。
- GAP3 A' 冻结锚：设计:124（八构造子＋Realize 资金源硬边界，`ledger.rs:430-445` 注释在案；`pub enum TwEvent` 直读核对 `ledger.rs:447`）。
- 直读核对：`tw_seen_basis` shadow 模式（`runner.rs:1130` 及 :1133 注释「对累计值量化再派」）；η_bucket 生产点（`runner.rs:1288`，:1283 注释「η_t=tw.tw() 即 enter_ready 判据左操作数」——F4 同源约束的代码在案形态）。

### 裁决
1. **取 (b)：η 修正＋桥接对账（零账本侵入）**。π loop 增 `cum_holding_cost` i64 shadow（funding+borrow+liq 累计量化，`tw_seen_basis` 同款模式）；`enter_ready` 消费侧把 η 修正为 **`tw() − cum_holding_cost`**（策略层修正项，TwEvent 代数不动）；加桥接对账断言 `tw() − η_corrected == ⌊cum_holding_cost⌋`（取整界内）（设计:125）。
2. **(a) 新 TwEvent 构造子否决**：触碰 GAP3 裁定 A' 构造子冻结（八构造子＋Realize 资金源硬边界，`ledger.rs:430-445`）；本裁定不授权任何构造子新增——若未来复活 (a)，须补 drift 引理族＋codex/编排者新裁定双前置（设计:124）。
3. **(c) 声明不管否决**：η 高估＝激进侧（EnterReady 提前触发是真资金风险，~2e5/窗实测非噪声），与「不闭合」直接冲突（设计:122,126）；**G1 闭合前 treasury-full 带成本运行禁止**（设计:184）升为本裁定硬条款。
4. **F4 同源强制约束**：η_bucket（ZExt 第 15 维）与 `enter_ready` 左操作数必须**同源修正为一笔改动**——两处不同步则 z 维与判据裂口（设计:156，a5-etabucket 终裁：γ_t 是 η 与 η⋆ 比较的离散化，左操作数须与 enter_ready 同一个量）。
5. **守恒范围声明**：R 守恒覆盖 f64 cash 域、TW 守恒覆盖 i64 TW 域，两账本不同构（#90/674）；桥接断言＝两域间**对账不变量**，不声称跨账本单一 Realize/统一账本（设计:135，`ledger.rs:442-445` 裁定清单⑨）。

### 验收线
- T-N4：η_corrected＝tw()−cum_holding_cost；桥接断言取整界成立；**回归锁**：κ=0 ∧ cost=None ⟹ 与现 `enter_ready` 判据同值 bit-exact；η_bucket 与 enter_ready 左操作数同源（F4）（设计:171）。
- T-N8 事件级守恒补强：funding 边界 bar cash 减少恰＝＝计提输出；funding+liq 同 bar 叠加 R 残差 ≤ tol（设计:175）。
- 运行闸：G1 闭合（T-N4 全绿）前，任何 treasury-full 带成本跑批产出不得作验收证据，只能作机制诊断并标注「G1 未闭合」。

### 不回滚条款
- GAP3 A' 冻结不动：TW 构造子零新增；Realize 资金源硬边界不动（`ledger.rs:430-445`）。
- (c) 不得以任何形式复活（含「η 高估量级小可忽略」的重新主张——~2e5/窗实测已证非噪声）。
- η 修正不得只修 `enter_ready` 或只修 η_bucket 单侧（F4 一笔改动强制）。
- bit-exact 回归锁（κ=0 ∧ cost=None 同值）不得移除；桥接断言不得降格为告警（断言即断言）。

---

## 附则A κ 政策接口条款（接口冻结＋优先序写死＋κ=0 基线＋正 κ 留编排者选择类）

### 背景
`RiskPolicy` 本体（有理定点 κ=num/den、三构造闸拒负、eta_star ceil/i128、enter_ready 五合取、buy_core_legal）已完整实装（`ledger.rs:253-410` 直读核对；设计:41-45）；缺口仅在**生产注入面**（现状＝env 诊断 knob `kappa_policy_from_env`，`runner.rs:984` 直读核对）与同名混淆防线（`RiskConfig.kappa` sizing 成本倍数 2.0，`config.rs:174,202` 直读核对——同名不同义）。MF:149 要求「κ 风险政策冻结」；正 κ 生产选择 codex 在案属 M8 L3、693 注记为选择类（设计:45）。

### 依据
- 设计:107-116（§3.3 接口设计四条）；设计:46（同名防线）；设计:151（F1 struct 字面量构造点盘点）；设计:158（F6 优先序）；设计:173（T-N6）。
- 693 行锚：MF:149-150（κ 风险政策冻结；κ 价格不可导出——不可识别性定理2，`ledger.rs:241-242`）。
- 教义域外声明锚：TS3:290（三阶段资金框架整体 `[L0域外]`，κ 为其参数）。

### 裁决
1. **接口冻结**：`ThetaConfig.risk_policy: Option<RiskPolicy>`——`None` ⟹ `baseline()` κ=0，与现路径**逐字节相同（bit-exact）**；`Some` ⟹ π loop `tw_policy` 取之（替换 `runner.rs:1127` 单点）。取值**永走** `RiskPolicy` 三构造闸（`baseline`/`try_new`/`try_new_ratio`）；**禁裸 i64/f64 κ 字段出现在任何 config**（类型边界闭合，设计:113）。
2. **优先序写死**：env `KAPPA_BARRIER_*`（诊断覆写）> config.risk_policy > baseline；env 非法值 panic 语义不变；单源纪律防双源静默漂移（设计:114,158）。
3. **同名 κ 防线**：新字段命名 `risk_policy`（不带 kappa 字样）；`RiskConfig.kappa`（sizing 成本倍数 2.0）不动；config 侧补镜像注释（设计:46,115）。
4. **κ=0 基线冻结；正 κ 留编排者选择类**：本裁定口径 κ=0 基线不变；**正 κ 生产取值本裁定不裁**（选择类，693 注记；codex 在案属 M8 L3，设计:45）；敏感性网格 {0,1/2,1,2} 只作敏感性陈述不作择优；任何 κ>0 报告强制标注「κ＝operator 声明式风险政策（不可识别性定理2），非价格导出、非回测择优」（设计:116）。

### 验收线
- T-N6：`risk_policy=Some(try_new_ratio(1,2))` ⟹ `eta_star` 用 ⌈L^wc+Q/2⌉；None ⟹ 与现路径逐字节一致；env>config>baseline 优先序锁；非法 env panic 语义不变（设计:173）。
- F1：struct 字面量构造点盘点＋config 默认值断言族补新字段断言（设计:151）。
- 报告闸：κ>0 的报告无「operator 声明式风险政策」标注＝文档缺陷。

### 不回滚条款
- 不得引入绕开构造闸的 κ 注入路径（裸数值字段、静默降级）；κ≥0 类型不变量（对齐 Lean `kappa_nonneg`）不得破。
- 优先序 env>config>baseline 不得倒换、不得增第三源。
- κ=0 基线 bit-exact 回归锁（T-keep-2 族）不得移除；正 κ 不得以「回测择优」名义落地（v3＋不可识别性定理2）。

---

## 附则B 费率标定数据源条款（datum 四类用户供＋常费率带声明保底＋additive 预冻结）

### 背景
693 有效域声明在案：A10 成本机制的**费率标定仍是 L2 缺口**（M6 自陈「费率待外部标定」），不外推跨标的（693:140-142）。设计 §3.2 把数据源分两类：A 必须用户供（外部 datum，机制无法内造，缺口即声明）／B 可参数化带声明政策（机制闭合用）；推荐混合方案 C：常费率保底＋`FundingScheduleBook` 接口预冻结（设计:91-105）。

### 依据
- 设计:91-96（A 类四项）；设计:98-103（B 类声明政策骨架三条）；设计:105（C 混合推荐）；设计:95（强平恶化不收分布）；设计:176（T-N9 标签测试）；设计:183,188（§6-1/6 措辞与 v3 合规）。
- 693 行锚：693:140-142（费率标定 L2 缺口、有效域 BTC/L2 单标的）；MF:143（验收式成本四项——费率值是 datum 不是机制）。
- 教义域外声明锚：TS3:290（费率属延伸建模层，`[L0域外]`）。

### 裁决
1. **必须用户供（外部 datum）四类**：① 真实 funding 历史（venue+symbol 维度、结算周期对齐、**带符号**、版本化快照，datum 版本哈希进报告标签）；② 借贷/借券利率曲线（现货杠杆/股票短卖接标时必须补）；③ 强平清算费率表（venue 公布、版本化 datum，同 margin 表先例）；④ ADL 规则与触发史（C1 OUT_OF_SCOPE 期间不启动；venue adapter 适配项 reopen 时随 datum 到位）。
2. **可参数化带声明政策**：v0 三常费率（`funding_rate_per_period`/`borrow_rate_per_bar`/`liq_penalty_rate`，`risk.rs:770-775` 直读核对）保底作机制闭合用——**一切带成本 R 数值报告强制口径标签 `[L1机制/费率未标定]`**；datum 注入后升 `[L2费率标定: datum 版本哈希]`（有效域 231 号规则，m6margin-c2 §5 先例，设计:101）；常费率数值**禁作 alpha 论据、禁作策略择优输入**（v3：历史数据只验证代码正确性，设计:102）。
3. **`FundingScheduleBook` additive 接口预冻结**：与 margin book 同构（分段/`as_of` 零前视/fail-loud 构造）；datum 到位即插、签名不变、既有测试全绿（与 C2 裁决第 2 条同一件）。
4. **强平成交价恶化不收分布**（v3：不引入概率分布作决策基础）——恶化以保守常数参数并入 `liq_penalty_rate` 上界声明（设计:95）。

### 验收线
- T-N9：R 分解报告落盘强制费率标定标签字段，测试标签存在性（措辞纪律配套，防数值冒用）（设计:176）。
- T-N1：`FundingScheduleBook` 测试族（设计:168，与 C2 共享）。
- 有效域闸：一切带成本 R 数值有效域声明＝BTC/L2 单标的（693:140-142），不外推跨标的；datum 注入后标签升级须带 datum 版本哈希，不得跳级。

### 不回滚条款
- 口径标签不得移除/弱化；常费率数值不得作 alpha 论据或策略择优输入（v3 硬禁令，设计:188）。
- 费率 datum 只作成本真实化（代码正确性验证），不得进信号/策略择优链。
- `FundingScheduleBook` 只走 additive；不得借 datum 实装改 `funding_accrual` 旧签名（F2）。
- 强平恶化不得引入概率分布建模（v3）——只准保守常数上界声明。

---

## 代理裁定性质声明

1. **授权来源**：编排者 2026-07-18 指令（把关⑦ C1–C5 定稿，「直接采纳设计推荐，落为正式裁定」；κ 接口与费率源照设计落条款），裁定权链条同 `nest-predicate-caliber-ruling-20260717.md` 文头（编排者 2026-07-17 下放）。本文 C1–C5＋附则 A/B 由代理工位签署，自落盘起为工作口径生效；编排者醒后复议通道开放。
2. **醒后复议通道**：编排者归来后可对任何单项发起复议；推翻/修订以新 escalate 文档显式 SUPERSEDE 本条为准，推翻前本文有效。各项复议触发条件——C1：venue 部署目标确定且 ADL datum 可获取；C2：funding datum 到位（标签升级不重裁），或有向语义与 venue golden 冲突；C3：接入现货杠杆/股票短卖/借币标的；C4：treasury-full 实装关开启阶段三全仓切换；C5：(b) 实装暴露 F4 同源修正无法一笔落地或桥接断言实测裂口，或 (a) 复活提案带 drift 引理族成熟；附则A：编排者行使正 κ 选择类裁定（M8 L3）；附则B：venue datum 口径与本裁定条款冲突。
3. **090 纪律自检**：本文全部结论可追溯至设计文档（行锚逐条）、693 工作口径三件（TS/MF/693 谱系行锚，逐段直读核对）、教义域外声明（TS3 行锚；026:78/028:20/074:38/033:50 经设计 §1.4 直读主仓核对在案）；四状态如实标注——机制已实装（M6 六项）、费率未标定（L2 缺口）、ADL 未建模（venue 适配项）、κ 正取值未裁（选择类），无模糊地带（对齐设计:217 自检）；OUT_OF_SCOPE 项（C1）措辞纪律：不以完整策略名义引用缺失构件。**直读核对注记**：本裁定落笔前抽查直读承重代码坐标——`risk.rs:770-775`（CostModel 四字段）、`:809-815`（funding_accrual 无向实现）、`:324`（liq_flag 字段）、`:730`（ADL/funding 缺口注释）、`config.rs:174,202,338`（RiskConfig.kappa 2.0 及默认断言）、`ledger.rs:253-410`（RiskPolicy 构造闸/eta_star/enter_ready）、`transition.rs:319,354`（stage_progression 消费点）、`runner.rs:984,1127,1130,1283,1288`（κ knob/tw_policy/tw_seen_basis/η_bucket）全部语义一致在案；**`runner.rs` 行号相对设计文档引用整体漂移约 +73 行**（tw_policy 设计:1054→现 :1127、k_theta_risk_gate 设计:766-865→现 :839 起；ledger/transition/config/risk 四文件坐标未漂），本裁定引用 runner.rs 坐标以今日直读值为准、设计文档引用以其行锚为准，语义结论不受漂移影响。**本裁定是设计推荐＋编排者指令下的代理拍板，不是人裁终审**。
4. 零 cargo 调用；零 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入（three_stages 只读核对）；生产源码零改动（本裁定纯文档，实装面全部留后续任务）；新增文件仅本文（worktree `/tmp/kimi-nest-mainline` 内）。

## 签字位

- [x] C1 ADL＝OUT_OF_SCOPE 声明＋venue adapter 适配项登记（声明期间禁称「强平模型完备」）——代理签署生效
- [x] C2 funding 有向 datum＋additive 接口预冻结（FundingScheduleBook＋funding_accrual_signed），常费率保底带 `[L1机制/费率未标定]` 标签——代理签署生效
- [x] C3 v0 perp 口径 borrow≡0＋语义边界声明（选项 (i)；机会成本解释 (ii) 否决）——代理签署生效
- [x] C4 v0 保持 stage-agnostic 全程逐仓（保守超集）；OQ-5(d) 定理贯穿；T-N5 守卫测试升强制——代理签署生效
- [x] C5 G1 取 (b) η 修正＋桥接对账（(a)/(c) 否决；GAP3 A' 冻结不动；F4 同源修正强制；G1 闭合前 treasury-full 带成本运行禁止）——代理签署生效
- [x] 附则A κ 接口（risk_policy Option＋None bit-exact；env>config>baseline 写死；同名 κ 物理隔离；κ=0 基线；正 κ 留编排者选择类）——代理签署生效
- [x] 附则B 费率标定数据源（datum 四类用户供；常费率保底强制标签禁作 alpha 论据；FundingScheduleBook additive 预冻结；恶化不收分布）——代理签署生效
- [ ] 编排者复议（空位，醒后填）
