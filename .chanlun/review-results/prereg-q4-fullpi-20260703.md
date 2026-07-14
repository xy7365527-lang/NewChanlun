# 预注册：q4 π^full 全接通回测（goal g-full-spec-pi 最后验收项）

> 工位 swarm/ws-q4pi | task #135 | **冻结先于跑数**（本文件独立 commit 后方可跑任何产出判定的数字；跑数先于冻结 = q4 直接 fail，可证伪）
> 回测对象：π^full = 《完整的策略.pdf》§16 十环链在 HEAD 全接通状态——G1(#115 force 热路由)/G2(#132 σ_higher 第9维)/G3(#138 z 13维)/G4(#134 typed exit ledger)/G5(#124 P1..P10 统一解释器)/G6(#125 证明整合)/G7(#133 gross cap)/GAP3(#139/#140 TwEvent::Realize)/hl13(#123 高级别一三类)/margin(#113 真 MM 机器)。
> 冻结基 HEAD = a043abba51 的后继（本 prereg commit 的父）。跑数全部在本 commit 之后的 HEAD 上。
> 认识论预承诺：全战役至今**无 confirmed 正 alpha 在案**（p3/f3/L3 全 INCONCLUSIVE）。本回测不预期翻转该结论——若 π^full 同样无正 alpha，INCONCLUSIVE/FALSIFIED 就是诚实产出。残差口径 `Y_i=δ(H−B̂)−C` 不动摇。

---

## ① enforce_gross_cap=true（G7 硬要求）

所有 π^full 臂（Arm1/Arm2，见 §⑥）显式 `cfg.risk.enforce_gross_cap = true`。不开则 PDF §11「毛/净同时约束」一致性声明不成立（g7-impl §3 诚实声明：default false = frozen 口径无毛 cap，#135 必须显式开启）。

例外（显式标注）：Arm0（旧口径复现/漂移归因臂）与 Arm3（隔离臂）保持 `false`——它们的存在目的是隔离归因，不是 π^full 声明的载体。任何以 Arm0/Arm3 数字冒充 π^full 结论 = 声明膨胀（090）。

## ② μ 观测新口径功效声明（G4 #134 语义后果）

- **口径**：μ 观测对象 = treatment-on-the-treated——生产 π fill loop 真正开腿的信号（`typed_ledger_from_bars` → `build_mu_from_bars`，τ^reverse 已删除）。record 桶/slot 冲突/AncOK 被剪候选不入 μ。
- **样本量声明**：BTC 16K 半窗实测 **18 腿 / 8 μ 类**（g4-impl §1 L2 冒烟）。较旧 τ^reverse 口径（3468 残差 @全 OOS）下降约两个数量级。全 OOS walk-forward（5 个 test 窗，每窗 ≈26 万 bar）线性外推 ≈ 10²–10³ 腿总量——外推不确定，以跑数为准，照实报告实测 n。
- **功效门**：逐桶 `n_eff ≥ (1.645·CV)²`（decontam.rs，667号）**公式不变**（本身样本量自适应），但**功效预期重设计**：新样本量下预期**绝大多数（可能全部）桶 ¬powered → INCONCLUSIVE**。这是生产 π 语义的忠实镜像，不是失败——欠功效桶照实报 INCONCLUSIVE，不放宽门、不换判据、不借旧口径桶粒度预期。
- **桶粒度**：主判据桶键 = 现行 4 元组聚合 `(level, bsp_class压缩, parent_dir | δ)`（s3/f3 同款，δ-free 聚合基）。**不可复用**旧口径的"28 桶/52 桶"粒度预期——新口径类数以实测为准（冒烟 8 类）。不再细分。
- **χ treat_empty_as_pass 语义（显式裁定）**：新口径 μ 类数骤减 ⟹ 生产 χ 查询大量命中 μ=None ⟹ 该参数语义权重大增（false≈几乎全拒 / true≈几乎全放）。冻结：**主臂 Arm1 = false**（§13/p25「无正边际收益证据不交易」的 spec 一致语义）；**敏感臂 Arm2 = true**（全放语义对照）。两臂差分本身就是该参数语义权重的度量，照实报告。n=1 类例外（mu_lcb=None 走同参数）沿用 selector.rs 现语义。

## ③ 置换检验桶键裁定

- **σ_higher（MuClass 第9维）与 force_state（第8维）本轮均不进置换分层/聚合基**。依据：(a) i_class×δ 共线自毁教训（memory：方向性/准方向性维进桶键即毁 δ 置换）；(b) G2 边界条件明文——σ_higher 若进分层，`stratified_delta_perm_p_fullz` base 键 / 重构闭包 `sigma_higher: None` / `wverify_fullz` 判定键投影**三处必须同批改**，任改其一即重现全表 miss / 代表值泄漏。本轮三处均不动 = 一致地不进。
- **方向性维不进桶键**：完整 i_class（buy/sell 分裂）及任何 δ-派生量不进聚合基（p3 §3.3 退化机制）。聚合基保持 δ-free `(level, bsp_class(), parent_dir)`；分层键保持 `(ℓ, h_bucket, time_block, σ^H)`。
- **事后验证锚**：主桶 perm_p 若退化到 1.000 ⟹ 桶键共线检查证伪，照实上浮（f3 先例：实测 0.020 未退化）。

## ④ margin 输入源显式选择

- **诚实前提**：真实交易所历史分段 MM 快照（逐段规则表时间序列）**不可得**。
- **冻结口径**：单段 CME-simple 快照近似全历史——`MarginSchedule::cme_simple(pct_maint=0.37, retail_mult=1.10)`（risk.rs L1 golden 同值，CME BTC 期货维持保证金历史量级），快照区间 = 数据全域单段。`RiskCushions{buffer1 = 0.02·nav₀, buffer2 = 0.05·nav₀}`（美元绝对值，nav₀=该臂首可交易价×1000 与 p3 nav 同源；满足 0<B1<B2）。buffer1/2 归 Θ_risk（codex-margin 裁定(a)），敏感性网格属 margin 工位（P0-C）范围，非本轮——本轮单点冻结并声明。
- **有效域声明（231/090）**：本口径 = **「CME-simple 单段近似」**——非 SPAN、非交易所逐段历史快照、非实盘保证金。margin 对结论的作用路径 = RiskMode M1/M2/M3 → 订单流约束（GlobalRiskClose 强平 / no_increase_cap）。一切含 margin 的结论有效域**限于此口径**，报告中一律带「CME-simple」标签，不得声称经验校准。
- Arm0/Arm3 显式 `margin=None`（MM=0 退化口径，p3/f3 基线同源）。

## ⑤ 两类 TW bit-exact 风险声明（#139/#140 后）

TW（三阶段资金账本）→ 订单流的唯一反馈通道是解释器 P2/P3/P4 触发（TW 是 shadow，不进 base_units/sizing）。#140 落地 TwEvent::Realize 后 TW 真值源 = 已实现利润入账，两类订单流分叉风险：

1. **P2 CloseOverlay 直接产订单**——P2 触发 bar 直接产生关腿订单事件；
2. **P3/P4 priority masking 间接改单 bar 订单**——P3/P4 无订单事件但消耗当步裁决 ⟹ 同 bar 普通开仓 qty 从 >0 变 0（coverage 测试已断言 Order 本身不同）。

**冻结处置**：全历史重跑中若任一 bar 触发 P2/P3/P4，该 bar 起订单流相对旧档案（p3/f3）分叉——**照实记录触发计数与首触发 bar**，不回避、不修补、不以"应当 bit-exact"预设结论。P3 触发门（已实现利润 ≥ ~1.5×NAV 且再投资足额）预期在真实数据难触发（#140：现有非忽略场景不触发），但全历史 L2 是否触发**以跑数为准**（#140 明文：单测不触发 ≠ 全历史 bit-exact）。

## ⑥ 窗口 / 品种 / 种子 / 判据 / 三态规则

- **品种**：BTC = 主判据标的；CL = policy 臂次要对照（p3 同口径）；L3 重跑 = 7 品种池化（σ̂ 归一化，OKLO 剔除）。
- **窗口**：
  - μ/W-VERIFY/L3：`PREREG_WINDOWS` BTC anchored walk-forward，`test_start ≥ OOS_START(2023-01-01)` 的窗（i7–i11，i11 clipped 沿用现引擎行为），逐窗独立 `build_mu_from_bars`（wverify_run 现机制，零改动）。
  - π policy 臂：**walk-forward 逐窗**——对每个 OOS 窗 i∈{i7..i11}：train = 该窗 test_start 前推 6 个月（有界 train，p3 成本先例），test = 该窗 test 段；逐窗报告 + 聚合。另跑 **p3 可比单折**（train 2022-07-01..2022-12-31 → OOS 2023-01-01..2023-06-30）用于与 p3/f3 档案直接差分。
- **种子**：全链确定性（置换检验引擎内固定种子，跨进程复现由既有 L1 测试 `fullz_uclass_reproducible_and_refines` 等保证；无其他 RNG）。
- **主判据（§12 口径，非 p<0.05）**：**LCB_OOS(μ(z,a)) > 0**，z_α=1.645。χ 配置：`chi_theta=Some(0.0)`，`chi_z_alpha=1.645`（p3 同）。
- **三态判定规则**（decontam 现引擎，零改动）：
  - VALIDATED ⟺ powered（n_eff≥(1.645·CV)²）∧ LCB>0 ∧ perm_p<0.05（perm 为抗置换假阳性护栏，非主判据）；
  - FALSIFIED ⟺ powered ∧ UCB<0；
  - 否则 INCONCLUSIVE（欠功效/未达显著）。
  - 全局：任一桶 VALIDATED ⟹ 该桶报可交易 alpha；全桶无 VALIDATED ⟹ 维持「无 confirmed alpha」。
- **臂位设计**（policy π，BTC+CL）：

| 臂 | χ | treat_empty_as_pass | enforce_gross_cap | margin | 用途 |
|----|---|---------------------|-------------------|--------|------|
| Arm0 | 无 χ | — | false | None | 旧口径复现/漂移归因（对 p3 档案无 χ 基线；预期因 hl13 2364 新信号分叉，照实归因） |
| Arm1（主） | LCB 门 | **false** | **true** | **Some(CME-simple)** | π^full 本体 |
| Arm2（敏感） | LCB 门 | **true** | true | Some(CME-simple) | χ 空类语义对照 |
| Arm3（隔离） | LCB 门 | false | false | None | 隔离 G7+margin 订单流效应（Arm1−Arm3） |

- **差分表（冻结结构）**：(a) Arm1 vs p3 档案 χ 门数字（fullz-policy §4）；(b) Arm1−Arm3（G7+margin 增量）；(c) Arm1−Arm2（χ 空类语义权重）；(d) Arm0 vs p3 档案无 χ 数字（HEAD 漂移归因，预期 hl13/#115/#124 致分叉，逐项归因）；(e) μ 三态表 vs f3 档案（V/F/I 计数+主桶）。度量：Σpnl(含浮盈)/max_dd/n_orders/n_trades/strat_return（p3 同口径）+ P2/P3/P4 触发计数 + gross_cap 触发计数（可得则报）。

## ⑦ 重跑清单（全项，跑数阶段逐项兑现）

| # | 项 | 入口 | 口径变化源 |
|---|-----|------|-----------|
| R1 | μ（typed exit 新口径，BTC 全 OOS walk-forward） | `wverify_full` | G4 |
| R2 | χ / π^full policy 四臂 | 新增 `#[ignore]` 入口（本 prereg 后实装，臂位=§⑥ 表） | G2/G3/G4/G7/margin/hl13/GAP3 |
| R3 | W-VERIFY（4 元组主判据 + full-z/UClass 并列） | `wverify_full` + `wverify_fullz` | G4 + G2（fullz 键 σ_higher→None 投影不变） |
| R4 | L3 跨标的池化（7 品种 σ̂ 归一化） | `wverify_cross_symbol` | G4 |
| R5 | hl13 +2364 三类候选下游 | 上述 R1–R4 在 HEAD 自动含其效应（assemble_gamma/χ/digest/GOLDEN/closed_loop 级别路由已在 HEAD 落地并全绿）；差分归因段单独读出 level≥1 桶的出现/计数 | #123 |
| R6 | XZD C3 高级别 Type3 消费重新评估 | #137 死门探针口径（默认 300K 窗基线 routed=75/sub_bsp_type3_total=3150，旧 C3 全级恒 0）在更长窗（成本允许的最长窗）复核 C3 命中计数；仍恒 0 ⟹ 维持「旧 C3 死门」并记录；非 0 ⟹ 行为变化上浮 | #123/#137 |

**受影响旧结论清单** = g4-impl-20260703.md §4（W-VERIFY PASS 旧口径 / L3 跨标的 / 663/667 μ̂ 表 / perm_p 数字 / econ-663 报告）——本轮 R1–R4 即其新口径重估。

## 可证伪 fail 条件

1. 跑数先于本 prereg 冻结 commit ⟹ q4 直接 fail。
2. 冻结后修改本文件 ⟹ 谱系断裂（偏离照实入结果包，不改冻结文本）。
3. 主桶 perm_p 退化 1.000 ⟹ §③ 桶键检查证伪，上浮。
4. R6 若 C3 命中非 0 ⟹ 「旧 C3 死门」结论翻转，上浮。

## 认识论等级（231）

- R1–R4/R6 = **L2**（BTC 真实全历史 walk-forward OOS；CL 对照）；R4 = **L3**（7 品种池化交叉验证）。判据/三态逻辑本身 L0。新增 harness 入口的管线自检 = L1（零信息增量，只证管线）。
- 否定性结果照实：INCONCLUSIVE/FALSIFIED 均合法产出，价值在缩小有效域边界。

## 谱系引用

- codex-q1-spec-rulings-20260703（G2/G4/G7 三终裁）+ g2/g4/g7-impl 三结果包（本 prereg ①②③ 的实装依据）。
- codex-gap3-ledger-20260703 裁定 A' + gap3-realize-impl（§⑤ 两类 TW 风险的出处，清单⑧）。
- codex-margin-ruling-20260703（(a) buffer 归 Θ_risk /(c) CME-simple 强制口径标签——§④ 的裁定源）。
- p3（fullz-policy-backtest，冻结 9a19bb5775）+ f3（f3-fugue-backtest，冻结 9c97ff7580）：差分档案。
- 663/665/667（三态/去污/功效门）；675（残差走 z_of_candidate 生产路径）；231（有效域）；090（no-patch/声明膨胀禁止）；i_class×δ 共线（memory，§③ 依据）。
- hl13-impl-20260703 §6（R5 下游清单）；dx-deadgate-20260703 §1.5（R6 基线数据）。

## 影响声明

本预注册冻结 q4 的臂位/桶键/判据/margin 口径/TW 风险处置/重跑清单七项。预期代码改动：wverify_run.rs 新增一个 `#[ignore]` policy 四臂入口（复用 run_theta_v0_pi/run_theta_v0_pi_chi/build_mu_from_bars，零生产逻辑改动）+ 可能的触发计数读出（只读）。冻结后不改本文件。跑数产出落 `q4-fullpi-results-20260703.md`（结果包六要素+三态判定+差分表+否定性结果照实）。
