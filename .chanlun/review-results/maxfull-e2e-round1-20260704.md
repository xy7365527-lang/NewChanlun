# Π_max-full 端到端首轮 OOS 报告

> **工位**：swarm/ws-d3report（goal m8-endtoend-oos d3）｜分支 gap3-rework-codex9-fix｜基线 31635a3ce4
> **合成性质**：纯文档合成，零新数据、零代码改动。四层数值全部转引 e2e-d2 实测跑批（29.56s，2026-07-04），不改一位。
> **认识论等级**（231/formalization-validity-domain 强制）：signal 层转引 = L2/L3；execution/treasury/total 层 = **L2**（真实 BTC OOS 三窗，可否证）。
> **口径首次成立声明**：M0-M8 主线全闭合，Π_tested = Π_max-full **首次成立**——本报告**可合法使用完整策略口径**（不再受 §5.3「signal-full 不外推 max-full」约束，因检验对象本身已是 max-full）。但 confirmed alpha 只在 `LCB_OOS(R)>0` 时声明；本轮 LCB<0 ⟹ **判定 = INCONCLUSIVE**（完整策略在当前数据/口径下无 confirmed alpha；**INCONCLUSIVE ≠ 证明无 alpha**，§5.6）。

---

## 1. 一行结论

**Π_max-full 端到端首轮 OOS = 无 confirmed 完整策略 alpha（INCONCLUSIVE）。** 三窗 `R(Π_max-full)` 全负（−1.51e7 / −2.08e7 / −2.37e7），`LCB_OOS(R)` 全负 ⟹ 未过完整策略判据 `LCB_OOS(R_{Π_max-full})>0`。这是 signal 层无方向 alpha 沿「signal → execution → treasury → total」一条因果链传导到完整策略层的必然结果，不是四个独立否定。措辞纪律：**INCONCLUSIVE ≠ 无 alpha**（当前数据只支持 ∀z∈Z_tested LCB≤0，不支持 ∀z∈Z_full μ≤0）。fail 条件未触发，不上浮。

---

## 2. 四层验收全表（路线.pdf §13.1-13.4 / M8:163-168）

四层实测（BTC OOS，三系统同开，e2e-d2 §4 转引）：

| 窗 | n_orders | ΣN_tΔP_t | Comm+Slip | Funding | Borrow | LiqLoss | net_r(execR) | MaxDD | 声部(A/S/F) | 终Stage | Q_T=Q_0 | W_T | η_T/η_* | R(含浮盈) | LCB_OOS(R) | 三态 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| p3fold | 35692 | +3321924 | 18416544 | 198440 | 10 | 0 | −15293070 | 0.9383 | 320/197/204 | I(降成本) | 16543670 | 0 | 1448271/16543670 | −15094994 | −20173942 | 无(R≤0) |
| wf7 | 30917 | +165050 | 21014204 | 201071 | 10 | 0 | −21050235 | 0.8991 | 326/218/217 | I(降成本) | 23471260 | 0 | 2617747/23471260 | −20849370 | −23891638 | 无(R≤0) |
| wf8 | 28373 | +1043561 | 24748524 | 269776 | 8 | 0 | −23974747 | 0.8696 | 364/173/223 | I(降成本) | 28721770 | 0 | 4918847/28721770 | −23706385 | −27186600 | 无(R≤0) |

### 2.1 逐层判据 / 实测值 / fail·pass

| 层 | 判据（PDF §13.x / M8） | 实测值 | 判定 | 认识论等级 |
|---|---|---|---|---|
| **(1) signal** | `LCB_OOS(μ)>0 ∧ LCB_OOS(μ_R)>0` | 25 桶双门无一同时 VALIDATED；唯一 raw Validated 桶（L0 bsp3 σ+1）经四重链坐实 beta 漂移；μ_R 侧零 Validated；L3 池化 V=0 | **INCONCLUSIVE**（fail 双门） | L2/L3（转引 signal-full 首轮） |
| **(2) execution** | `E[R(Π_exec)]>0` | 三窗 net_r ∈ [−2.40e7, −1.53e7]，与 M6 基线三窗 bit-exact | **FALSE** | L2 |
| **(3) treasury** | `Reach(StageIII)>0 ∧ Q_T>Q_0 ∧ W_T≥I_0 ∧ η_T≥η_*` | 三窗终 Stage 全 = CostReduction(I)；Q_T=Q_0（未 BuyCore）；W_T=0<I_0；η_T<η_* | **FALSE** | L2 |
| **(4) 完整策略** | `R(Π_max-full)>0 ∧ LCB_OOS(R)>0` | 三窗 R 全负；LCB_OOS(R) 全负 | **FALSE ⟹ INCONCLUSIVE** | L2 |

### 2.2 因果链解读（四层 fail 是同一因果链的四个观测点）

四层逐层 fail **不是四个独立否定**，是 signal 层无方向 alpha 沿单一因果链传导的四个观测点：

```
signal 无方向 alpha（双门不过，唯一正桶=beta 漂移）
   ↓ signal 无方向 ⟹ 逐笔已实现 PnL 无正累积
execution net_r 负（−1.5e7~−2.4e7；高频成交费 Comm+Slip 主导拖累）
   ↓ 已实现 PnL 无正累积 ⟹ RecoverCapital 门（holding≥Q ∧ free≥退本目标）永不满足
treasury 停 Stage I（CostReduction；W_T=0 未退本金，η_T<η_*）
   ↓ 净额亏损 + 三阶段不推进
total R<0 ⟹ LCB_OOS(R)<0 ⟹ INCONCLUSIVE
```

- **execution net_r 极负非机制缺陷**：三窗 net_r 与 M6 基线（m6margin-c2）逐位一致（p3fold −15293070 / wf7 −21050235 / wf8 −23974747，bit-exact ⟹ overlay 臂消费 cost_model 与 M6 独立跑数无相互污染）。ΣN_tΔP_t 三窗全为正（价格 MtM 贡献 +3.3M/+0.17M/+1.0M），被高频成交费（3万+单 / 26万 bar）碾成负 net_r——**成本真实化**的诚实账目，不是 M6 成本机制缺陷。
- **treasury 停 Stage I 非架构不可达**：合成 witness `pi_loop_realized_profit_reaches_earning_shares`（L1）已证机制在足额已实现利润下三阶段可达；本轮真实 BTC 该口径**不产足额已实现利润**（净亏损），故三阶段自然停在降成本期。这与 memory `project_gap3_l2_unreachable_architecture`（"不可达=数据层非架构"）的订正一致。

---

## 3. M0-M8 全线终态总表（整条主线谱系总账）

| 关 | 目标对象（693 三分表） | 完成日期 | 锚点 commit / 跑批 HEAD | 验收判据 | 兑现情况 | 认识论 |
|---|---|---|---|---|---|---|
| **M0** 命名冻结 | Π_max-full 定义 | 2026-07-04 | TARGET_STRATEGY_MAXFULL.md（M0 产出物本身） | 十四元组 + 最终订单策略公式；禁用「终局」 | ✅ 十四元组 §1、公式 §1.1、措辞纪律 §5 承载 | 文档（L0） |
| **M1** 结构塔 bit-exact | Π_signal-full | 已闭 | final-alpha F1（type1>0 全历史） | `T^inc=T^full ∀t`；parity=0 | ✅ 中枢延伸/走势分解/局部趋势门 parity 全绿；type1>0 全级别 | L2 |
| **M2** 区间套+Cand | Π_signal-full | 已闭 | classifier/nest.rs, descend.rs; final-alpha F3 | `∀γ N=1, J_e⊆…⊆J_ℓ`；depth 分布 | ✅ max_depth=3；90.91% 小转大有锚 750；95%+ 退化 base-case = 真实几何非未调用 | L2 |
| **M3** 分类状态 | Π_signal-full | 2026-07-04 | m3lock-b2 Task#10；classifier/bsp.rs | `𝒳=⊔_z C_z, 运行时 Σ_z 1_{C_z}=1` | ✅ 运行时不变量锁定零违例；ForceState 真入主桶；ExitType 不进桶键（裁定甲）；d/μ_R 落地 | L2 |
| **M4** 统计准入 | Π_signal-full | 2026-07-04 | signalfull-oos-round1 HEAD bd01afc1b8（冻结基 26ebe90b29） | 三态 VALIDATED/FALSIFIED/INCONCLUSIVE + 双主判据 | ✅ 六子项全跑；三态输出 = **INCONCLUSIVE**（无 confirmed 方向 alpha） | L2/L3 |
| **M5** 声部执行账本 | Π_exec-full（A11 MUST） | 2026-07-04 | m5overlay-c1 Task#13（基线 7f621fa570） | `ΔN=Net(P^sep_{t+1})−Net(P^sep_t)` + 逐声部 pnl_v | ✅ OverlayState hedge-mode P^sep 簿；对账残差 9.24e-7 PASS；by-role 归因（A/S/F）真实入账 | L1（管线正确性） |
| **M6** 保证金强平 | Π_exec-full（A10 MUST） | 2026-07-04 | m6margin-c2 Task#14（基线 7f621fa570） | `R=ΣN_tΔP_t−Comm−Slip−Funding−Borrow−LiqLoss` | ✅ 六项分解 + 守恒断言（残差~1e-6 无泄漏）；三窗 net_r 极负=成本真实化 | L1（机制，费率待外部标定=L2 缺口） |
| **M7** 三阶段资金 | Π_treasury-full（GAP3） | 2026-07-04 | m7treasury-c3 Task#15 + kappa-d1 Task#16（基线 31635a3ce4） | `Reach(III)>0 ∧ Q_T>Q_0 ∧ W_T≥I_0 ∧ η_T≥η_*` | ✅ 机制装（L1 可达已证）；L2 witness 照实否定（真实 BTC 净亏损⟹停 Stage I）；κ=0 冻结（有理定点承载 κ=0.5，网格 {0,½,1,2} 全 Stage I，binding=RecoverCapital 门非 barrier） | L2（否定性结果合格） |
| **M8** 端到端 OOS | Π_max-full（三对象合成） | 2026-07-04 | e2e-d2 Task#17（基线 31635a3ce4） | `LCB_OOS(R_{Π_max-full})>0` | ✅ 四层报告产出；三窗 LCB_OOS(R) 全负 ⟹ **INCONCLUSIVE** | L2 |

**总账收口**：M0-M8 九关全部闭合。M1-M7 各关验收判据（机制层/管线层）**全部兑现**（PASS）；M4 signal 三态与 M8 total 判据的**结论**为 INCONCLUSIVE（无 confirmed alpha）——判据兑现（跑通）与结论方向（无 alpha）是两件事，前者 PASS、后者否定，二者不矛盾。否定性结论是 M0-M8 主线的**合法终点**（§5.6，161号照实）。

---

## 4. 与 signal-full 首轮报告的差分（结论如何从「信号层 INCONCLUSIVE」升级为「完整策略 INCONCLUSIVE」）

signal-full 首轮（signalfull-oos-round1）检验对象 = **Π_signal-full（M1-M4）**，受 §5.3 约束**明令不得外推 max-full**，其 §4.2 标 M5-M8「⬜ 未进入」。本报告是三对象合成后的首次完整策略检验，新增以下三层，使检验对象从 Π_signal-full 升级为 Π_max-full：

| 维度 | signal-full 首轮 | 本报告新增 | 差分性质 |
|---|---|---|---|
| **执行层真实化** | 仅诊断层代数骨架（overlay_net_delta 只读净目标差）；多空双开/短差**不影响订单** | M5 OverlayState：P^sep→N→Order 真实驱动净额账户 + 逐声部 pnl_v 归因（A11 MUST 兑现）；execution net_r 层落地 | 诊断 → 交易策略 |
| **成本真实化** | 无 funding/borrow/liquidation 成本记账 | M6 CostModel：R = ΣN_tΔP_t − Comm − Slip − Funding − Borrow − LiqLoss 六项分解 + 守恒断言 | 价格 PnL → 费后净收益 R |
| **资金层真实化** | 三阶段 TW 账本未接端到端 | M7 三阶段 TW 账本入端到端（overlay 臂透传 tw_final）；treasury effect 层（Stage/Q_T/W_T/η_T）落地 | 缺口标注 → L2 可达性实证 |

**结论升级路径（措辞纪律合法）**：
- signal-full 首轮合法表述（§1 钦定）：「当前已修复信号层没有 confirmed direction alpha；完整执行层和资金层尚未闭合。」——后半句「尚未闭合」在本报告**已消解**（M5-M8 全闭）。
- 本报告合法表述：「**完整策略（Π_max-full）在当前 BTC OOS 数据/口径下没有 confirmed alpha（INCONCLUSIVE）；执行层与资金层已闭合，四层判据全跑通，total LCB_OOS(R)<0。**」
- **升级的实质**：signal-full 的 INCONCLUSIVE 是「M1-M4 信号层」的局部结论，受禁止外推约束；本报告的 INCONCLUSIVE 是「Π_max-full 完整策略」的合法结论——检验对象本身已是 max-full，无需外推，§5.3 不再适用。**这是 Π_tested=Π_max-full 首次成立带来的口径合法性变化，不是把 signal 结论强行外推。**

---

## 5. 编排者问题的正面回答：「为什么恒 CostReduction」

三窗终 Stage 全 = CostReduction(I)、κ 网格 {0,½,1,2} 四档也全 Stage I——**为什么资金层三阶段从不推进？**

### 5.1 L1 机制可达 vs L2 燃料缺失的区分

- **L1（机制层）可达**：合成 witness `pi_loop_realized_profit_reaches_earning_shares` 已证——在**足额已实现利润**注入下，三阶段 CostReduction→CapitalRecovered→EarningShares 完整推进，BuyCore 派发。机制没有死锁，架构可达（memory `project_gap3_earning_shares_reachable` / `gap3_l2_unreachable_architecture` 订正）。
- **L2（数据层）燃料缺失**：真实 BTC 该窗口 + frozen Θ 的 **Σ已实现PnL = −562943（净亏损，M7 witness 10万 bar）**。三阶段推进的燃料 = 正的已实现利润；signal 层无方向 alpha ⟹ 已实现 PnL 无正累积 ⟹ 燃料为负 ⟹ 三阶段无法启动。**恒 CostReduction 是燃料缺失（L2 数据），不是机制不可达（L1 架构）。**

### 5.2 binding 约束 = RecoverCapital 门（不是 barrier η_*）

κ 网格（kappa-d1，BTC 100000 bar）坐实 binding 约束的精确位置：

| κ | 终 TStage | n_orders | Reach_II | Reach_III | η_T−η_*（barrier 距离） |
|---|---|---|---|---|---|
| 0 | CostReduction | 14472 | false | false | −562943 |
| 1/2 | CostReduction | 14472 | false | false | −1062943 |
| 1 | CostReduction | 14472 | false | false | −1562943 |
| 2 | CostReduction | 14472 | false | false | −2562943 |

- 全部 κ 卡在 **Stage I**，binding 约束是 **RecoverCapital 门**（进 Stage II 前置：holding≥Q ∧ free≥退本目标；witness holding=175732 vs Q=1e6，free=261325 vs 1e6），**不是** barrier η_*。κ 只门控 II→III 转换，该转换从未到达。
- `n_orders` 跨 κ 恒 14472 ⟹ κ 不改交易轨迹（只在 II→III 谓词出现，未触发）。
- barrier 距离随 κ 线性下降（每 0.5 步减 0.5·Q=500000）——坐实有理定点算术精确。
- **codex 单调性推论兑现**：κ=0 是最小可达性见证，κ=0 都不达 Stage II ⟹ 正 κ 对三阶段可达性**无必要**（正 κ 是稳健性/部署政策，非机制可达性证明）。κ 生产选择推迟 M8 后 L3。

### 5.3 多窗覆盖（含牛市窗，已证否）

编排者问「真实 BTC 上是否有任何窗口进 Stage II/III（含正 PnL 概率最高的 2020-2021 牛市段）」——e2e-d2 §4.1 已跑**全部 12 个 anchored walk-forward 窗口**（2019-2025，κ=0，nav=窗首价×1000，生产 π 路径 tw_final 单读，115.57s，commit 68f8333429）：

| 窗 | 期间 | 终Stage | Σ已实现PnL | holding−Q(门距离) | Reach≥II |
|---|---|---|---|---|---|
| wf1 | 2020-02..2020-08（牛市启动） | I | −9551320 | −9554071 | false |
| **wf2** | **2020-08..2021-02（牛市主升）** | I | **−2625432**（全窗最小亏损） | **−2836238**（全窗最近门距离） | false |
| wf3 | 2021-02..2021-08（牛市高点） | I | −48811031 | −49269780 | false |
| （其余 9 窗 wf0/wf4-wf11） | 2019-2025 | I | 全负 | 全负 | false |

- **任一窗 Reach≥Stage II = FALSE**。全 12 窗（含牛市三窗 wf1/wf2/wf3）终 Stage I。
- 最接近的牛市主升段 wf2 门距离 −2836238、已实现 PnL −2625432——**全窗中亏损最小但仍为负**。即使 BTC 正 PnL 概率最高的窗口，frozen Θ 也不产足额已实现利润使 holding 累积过名义基线 Q。
- **「只是没碰上牛市」解释已被排除**：牛市窗已覆盖且证否。treasury 不可达是数据层结果（signal 无方向 alpha ⟹ 已实现 PnL 无正累积），非窗口选择偏差、非架构不可达（M7 c3 合成 witness 已证机制在足额利润下可达 Stage III）。有信息量的照实否定（161号）。

---

## 6. 下一步方向候选（不裁决，列给编排者）

现有 Θ 冻结口径下 Π_max-full 完整检验已闭合（M0-M8 全线），signal 层无 confirmed alpha 且完整策略 INCONCLUSIVE。后续方向候选（互斥性由编排者判断，四分法 = 选择类，不自决）：

1. **跨标的扩容（L3）**：现结论有效域 = BTC 单标的 L2。扩到 CL/ES/金油等多标的池化，检验 signal 层无 alpha 是否 BTC 独有（signal-full L3 池化已显示唯一 raw Validated 桶为 BTC-独有 beta，memory `l3_cross_symbol_btc_idiosyncratic`）。AncOK ceiling waiver 有效域限 BTC/L2，跨标的触发翻转条件（§4.2）→ 须先处理 ceiling。
2. **A股 K4 选股 M2**：切换支柱——从缠论引擎（Π_max-full 已闭合）转向 K4 选股正则化里程碑（ROADMAP.md M2）。与当前 alpha 检验正交，不受 BTC 结论约束。
3. **新信号假设**：现有 Θ 冻结口径已完整检验闭合（无 confirmed alpha）。若要在缠论支柱继续，须提新信号假设（新桶键维度 / 新 estimand / 新 prereg），不是重跑现有口径——重跑现口径信息增量为零（L0→L1 同义反复，231号）。

---

## 7. 结果包六要素

1. **结论**：Π_max-full 端到端首轮 OOS = **无 confirmed 完整策略 alpha（INCONCLUSIVE）**。三窗 R(Π_max-full) 全负、LCB_OOS(R) 全负，未过 `LCB_OOS(R_{Π_max-full})>0`。四层逐层 fail 是「signal→execution→treasury→total」单一因果链的四个观测点。Π_tested=Π_max-full 首次成立 ⟹ 可合法用完整策略口径，但 INCONCLUSIVE≠无 alpha。

2. **定义依据**：
   - 完整策略 confirmed alpha 判据 = `R(Π_max-full)>0 ∧ LCB_OOS(R)>0`（PDF p21/p17，TARGET_STRATEGY_MAXFULL.md M8:163-168）。
   - 四层判据 = signal `LCB(μ)>0∧LCB(μ_R)>0` / execution `E[R(Π_exec)]>0` / treasury `Reach(III)>0∧Q_T>Q_0∧W_T≥I_0∧η_T≥η_*` / total `LCB_OOS(R)>0`（PDF §13.1-13.4）。
   - R 净收益 = `ΣN_tΔP_t−Comm−Slip−Funding−Borrow−LiqLoss`（M6:143）。
   - 三阶段 TStage∈{CostReduction,CapitalRecovered,EarningShares}，η_*=L^wc+κQ（M7:150）。
   - **输入特征满足定义的条件**：BTC OOS 三窗（p3fold/wf7/wf8）经生产 π fill loop（三系统同开 run_theta_v0_pi_overlay）产 R/net_r/tw_final 真值，逐窗 R<0 满足 `R>0` 判据的否定条件；LCB_OOS(R) 用 block bootstrap 在已实现口径上算下界（e2e-d2 §3.1：已实现口径 LCB≤0 ⟹ 净收益 R 的 LCB≤0 更强，否定判据保守正确）满足 `LCB_OOS(R)>0` 的否定条件。

3. **边界条件（结论翻转）**：
   - (a) LCB 口径有效域（231号强制）：LCB 用 trade_pnls（已实现平仓口径，不含成本+浮盈）做 bootstrap；total R 含全部成本+浮盈。因成本拖累 net_r<Σtrade_pnls ⟹ `LCB(已实现)≤0 ⟹ LCB(R)≤0` 更强，**否定判据保守正确**（否定不因口径近似翻转为假 confirmed）。翻转 confirmed 须在 R 逐笔分解序列上 bootstrap（现 r_decomp 只给聚合值，逐笔 R 分解序列不存在——M8+ 精化项，本轮否定不依赖）。
   - (b) treasury 翻转须 signal 层 Θ 产足额正已实现利润使 RecoverCapital 门可过（holding≥Q ∧ free≥target）。窗口选择已排除为翻转路径——全 12 窗（含牛市三窗 wf1/wf2/wf3）已跑，最近门距离 −2836238（wf2 牛市主升）仍为负（§5.3）。翻转须换 signal 有 alpha 的 Θ，非换窗口。
   - (c) 若换 signal 层有 alpha 的 Θ ⟹ 已实现 PnL 正累积 ⟹ 四层因果链整体可能翻转。
   - (d) 跨标的（L3）：若 CL/ES/金油等在 signal 层出跨品种可复现 alpha ⟹ 完整策略结论可能翻转（本轮仅 BTC/L2）。
   - (e) κ>0 生产政策：本轮 κ=0 已不可达 Stage II ⟹ κ>0 更不可达（barrier 更高）；κ 生产选择须 M8 后 L3 裁定。

4. **下游推论**：
   - Π_max-full 完整检验闭合确认缠论引擎支柱（ROADMAP M1）在 BTC/L2 下**无 confirmed alpha**——signal 层 beta 漂移、execution 成本碾压、treasury 燃料缺失三重一致。
   - 生产化/收缩无触发（无 VALIDATED 候选 = 无消费者）。
   - 后续方向（§6）：跨标的 L3 / A股 K4 选股 M2 / 新信号假设——三者互斥性由编排者裁（选择类）。现有 Θ 冻结口径重跑信息增量为零。
   - overlay 臂 tw_final 透传后成为 exec+treasury 双层数据单一出口，M8+ 全窗跑批/L3 跨标的可直接复用。

5. **谱系引用**：
   - 权威源：TARGET_STRATEGY_MAXFULL.md（M0 十四元组 + M8 判据）；docs/formal-chain/路线.pdf（p9-21）。
   - 输入结果包：e2e-d2-20260704（四层实测）、kappa-d1-20260704（κ 网格）、m5overlay-c1 / m6margin-c2 / m7treasury-c3（M5-M7 三件）、signalfull-oos-round1-20260704（差分基准）。
   - 674号：R/TW/overlay 三会计范畴不同构，端到端同开不混淆。
   - memory：`project_type1_goal_closed_final_inconclusive`（signal 无方向 alpha=上游根因）、`project_gap3_l2_unreachable_architecture`（订正：不可达=数据层非架构）、`project_gap3_earning_shares_reachable`（L1 机制可达）、`l3_cross_symbol_btc_idiosyncratic`（正号 BTC 独有）、`oddeven_mu_identity`（唯一 raw Validated=beta 漂移）、`q4_fullpi_no_alpha`。
   - 规则：231/formalization-validity-domain（L2 标注 + 有效域声明 + LCB 口径）；090/161（bit-exact + 否定性照实合格）；135（冻结先于跑数）；措辞纪律 §5（INCONCLUSIVE≠无 alpha，Π_tested=Π_max-full 后 §5.3 不再适用）。
   - **发生史无新概念分离**：本报告是 M0-M8 既有产出的合成，未产生新谱系条目。

6. **影响声明**：
   - 新增文件 `.chanlun/review-results/maxfull-e2e-round1-20260704.md`（本报告）。
   - **纯合成，零新数据、零代码改动**——四层数值转引 e2e-d2 实测（29.56s 跑批，1507 passed / 0 failed 不劣化），不改一位。认识论等级 L2/L3 照实标注。
   - **不 commit**（任务约束，供 Lead 联合真封）。
   - 引用但未修改：e2e-d2 / kappa-d1 / m5overlay-c1 / m6margin-c2 / m7treasury-c3 / signalfull-oos-round1（六份源结果包）、TARGET_STRATEGY_MAXFULL.md、docs/formal-chain/路线.pdf（权威源）。
