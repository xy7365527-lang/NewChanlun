# W2 C1 实装报告：hedge-mode 双仓接入 m8 路径（dual_ledger × m8 三窗跑批）

日期：2026-07-19 ｜ worktree：`/tmp/kimi-nest-mainline`（分支 kimi-nest-mainline-20260717）｜ HEAD=`640609071d`（跑批时；工作树多工位并发在途改动 62 项，provenance 见 §6）
性质：实装报告（含 diff、测试输出、三窗双仓口径输出、守恒断言状态、F-01 前视声明处置登记）

---

## 1. 任务与授权面

任务：p120 dual_ledger 引擎（关⑤，已实装：`dual_ledger.rs:42-49` DualLedger / `:130` apply_fill_dual / `:233` compatible_leg_orders + `plan_and_fill_mtm_dual` + `nautilus/strategy.rs` 接线）接入 m8 路径——m8_e2e 原臂走 `run_theta_v0_pi_overlay` 净额路径，本工位新增**双仓臂** `m8_e2e_dual_oos`（#[ignore] 同款），跑 p3fold/wf7/wf8 三窗（与 m8 同清单同切片），产双仓口径四层报告。

授权文件处置（授权 ≠ 义务，最小改动原则）：

| 文件 | 处置 | 理由 |
|---|---|---|
| `rust/src/theta_v0/backtest/wverify_run.rs` | **本工位净增 +368 行**（1709→2077，全部新增区：辅助结构/函数 + 测试本体；零删改既有行）。说明：`git diff` vs HEAD 显示 +385/−8，其中含开工前兄弟工位在途未提交改动（开工时本文件已 `M`），非本工位手笔 | 新增 `m8_e2e_dual_oos` 测试 + `LegTrajectory`/`leg_trajectory` 辅助 |
| `rust/src/theta_v0/backtest/dual_ledger.rs` | **零改**（diff vs HEAD = 0，核验在案） | 引擎语义完备（D1-D8 内建断言在案），本工位只消费其公开 API（DualLedger 字段经 RunResult 轨迹间接观测，见 §3） |
| `rust/src/theta_v0/strategy/ledger.rs` | **本工位零触碰**（diff vs HEAD = +126/−0 系开工前兄弟工位在途改动，开工时 git status 已 `M`，本工位无任何 Edit） | 双仓臂 v1 路径 TW 未接线（runner.rs:3988 诚实 None），无 TW 口径变更需求 |
| `rust/src/theta_v0/backtest/runner.rs` | **零改（禁改遵守）** | F-01 前视声明处置留待下一波（§7 登记） |
| `rust/Cargo.toml` | 零改（禁令遵守） | — |

`wverify_run.rs` 的 `m8_e2e_all_systems_oos` 本体**零改**（净额臂保留作对照基线，任务③）。

## 2. 实装内容（wverify_run.rs 新增区）

- **`LegTrajectory` + `leg_trajectory`**（wverify_run.rs:1363-1426）：从 `RunResult.trades` 前缀和重建双腿持仓轨迹 q⁺(t)/q⁻(t)。依据：dual 路径 `track_position_transition` 按腿喂有符号转移（runner.rs:3745-3750/:3819-3824，多腿 +q⁺/空腿 −q⁻），`TradeRecord.long` 逐笔分腿（metrics.rs:214）；部分减仓逐 fill 产记录（runner.rs:4048-4055 F-06）⟹ 望远镜求和精确。
- **`m8_e2e_dual_oos`**（wverify_run.rs:1469-1745，#[test] #[ignore]）：三臂同窗同数据——
  - 臂① 双仓 `run_theta_v0_dual`（recognize_nested + plan_and_fill_mtm_dual，runner.rs:372/:3682）；
  - 臂② 净额同引擎对照 `run_theta_v0`（runner.rs:275）——隔离账本/嵌套效应（E4 兼容嵌入对拍 runner.rs:8320 的真实数据版，p120 §4.4）；
  - 臂③ m8 净额基线 `run_theta_v0_pi_overlay`（三系统同开，m8 本体 config 口径重跑作对照列）。
  - 四层报告落 `/tmp/m8_e2e_dual_oos.md`：层1 signal 转引不重算；层2 execution（R/MaxDD/n_orders/腿级分解）；层3 treasury（臂①② TW 未接线诚实 None；臂③终Stage）；层4 完整策略（LCB_OOS(R) block bootstrap 2.5 分位 + 三态，同 m8 M8:168 判据）。

## 3. 设计要点与诚实声明（090）

1. **Q⁺/Q⁻ 可观测面**：`FillOutputDual.ledger`（终态 DualLedger）与 `leg_log` 是 runner.rs 模块私有（runner.rs:3616/:3628 非 pub），授权文件不含 runner.rs ⟹ 双腿轨迹经公开 `RunResult.trades` 前缀和重建——与 leg_log 同源的公开可观测面，**非第二查法**。**加仓不入记录**（v0 单标量近似，runner.rs:4010-4011）⟹ 腿级价格 PnL 列在含加仓时为近似口径，报告标 †、不作守恒断言对象。
2. **守恒断言（报告层对账；dual_ledger 内建断言 D1-D8 + runner E 组为引擎层）**：
   - **A（现金-权益守恒，精确恒等）**：`nav0·equity_终 − nav0 − Σpnls(含强平) − fee·px_终·Σ强平qty ≡ 0`（容差同 m8）。推导：权益曲线终点 = 强平**前**净值（强平在权益曲线后应用，runner.rs:3898-3900）；强平双腿现金 = net·px − fee·gross·px ⟹ 费项显式入账后恒等。
   - **B（分腿结构）**：q⁺(t) ≥ 0 ∧ q⁻(t) ≥ 0 ∀t（M14 `R≥0` 坐标）；终点强平后双腿零残留。
   - **C（净额对照结构锁）**：臂② 共存 bar 恒 0（净额账户「先平后开」物理不可共存，dual_ledger.rs:9-10 对照面）。
3. **可行性前置测量**：`plan_and_fill_mtm_dual` 每 bar 全窗重识别（runner.rs:3778）——临时探针实测 p3fold 窗（260560 bar，BSP 944）：recognize_nested 单帧 1.29ms ⟹ 全窗 ~6min/窗，三窗可行（探针已删，非交付面）。
4. **config 口径**：臂①② = default + env preset（margin/cost_model 对 v1 路径惰性——v1 不消费，如实声明）；臂③ = m8 同口径（margin=CME-simple + cost_model 参数化三项）⟹ 臂③与臂①②非 like-for-like，**like-for-like 对照是臂②**。κ env 残留检查：跑批前 `unset KAPPA_BARRIER_NUM KAPPA_BARRIER_DEN THETA_DIR_PRESET M7_WITNESS_A10`（κ=0 + Neutral 基线，p127 §5 同款）。

## 4. 三窗输出（/tmp/m8_e2e_dual_oos.md 原样，2026-07-19 跑批）

| 窗 | 臂 | n_orders | R(含浮盈) | MaxDD | LCB_OOS(R) | 三态 | 守恒残差 | Q⁺笔 | Q⁻笔 | 共存bar | maxGross |
|---|---|---|---|---|---|---|---|---|---|---|---|
| p3fold | ①双仓 | 37 | -8075576 | 0.5202 | -350390 | 无(R≤0) | -5.92e-9 | 10 | 9 | 1 | 1152 |
| p3fold | ②净额 | 630 | +2931230 | 0.0642 | -1550267 | INCONCLUSIVE | -4.46e-8 | 161 | 152 | 0 | 936 |
| p3fold | ③π基线 | 32195 | -15232144 | 0.9406 | -18771457 | 无(R≤0) | -1.77e-6 | — | — | — | — |
| wf7 | ①双仓 | 7 | +2285068 | 0.1341 | -57424 | INCONCLUSIVE | 5.17e-9 | 2 | 2 | 1 | 1142 |
| wf7 | ②净额 | 615 | +3012462 | 0.1094 | -3214685 | INCONCLUSIVE | -1.11e-8 | 157 | 150 | 0 | 1290 |
| wf7 | ③π基线 | 29190 | -20835598 | 0.8966 | -23871127 | 无(R≤0) | -2.31e-6 | — | — | — | — |
| wf8 | ①双仓 | 15 | +19027856 | 0.1558 | -271893 | INCONCLUSIVE | -4.25e-9 | 3 | 5 | 1 | 1374 |
| wf8 | ②净额 | 595 | +6735213 | 0.0798 | -1095316 | INCONCLUSIVE | -3.07e-8 | 150 | 148 | 0 | 1324 |
| wf8 | ③π基线 | 14244 | -16844347 | 0.6490 | -21097485 | 无(R≤0) | -1.86e-6 | — | — | — | — |

（完整 16 列含 Q⁺/Q⁻ 价格 PnL†、隐含费†、终Stage，见 /tmp/m8_e2e_dual_oos.md:19-27；† = 加仓单标量近似列。）

**守恒断言状态（任务②）**：三窗 × 四臂断言**全过**——臂① A 残差 ∈ [−5.92e-9, 5.17e-9]（容差 ~2.5e-2~4.8e-2）；臂① B min_leg=0.00e0、终点残留=0.00e0；臂② A 残差 ∈ [−4.46e-8, −1.11e-8]、C 共存=0；臂③ R 残差 ∈ [−2.31e-6, −1.77e-6]（m8 口径容差内）。硬断言任一不过则测试红、报告不落盘——报告存在即全过。

**Q⁺/Q⁻ 同存观测（照实判读）**：三窗臂①双腿同存**各 1 bar**（≈0.00%）——hedge-mode 双开共存**可观测且非零**（通道有效），但真实数据上极稀薄：nested ShortDiff 角色门（四合取，strategy/mod.rs:712-）+ 活动集 𝒟_x 使臂①订单流远稀于臂②（37/7/15 vs 630/615/595），子腿生命周期短。臂①vs臂② R 分叉（p3fold −11.0M / wf7 −0.73M / wf8 +12.3M）= 嵌套 ShortDiff 新能力效应（p120 §4.4），非旧数字漂移。**三窗 LCB_OOS(R) 全 ≤0，无 CONFIRMED**——措辞§5.6：INCONCLUSIVE≠无 alpha；臂①②数值为 F-01 前视诊断口径，不进 alpha 论据。

## 5. 测试输出

- 双仓臂跑批（命令行全文）：`unset KAPPA_BARRIER_NUM KAPPA_BARRIER_DEN THETA_DIR_PRESET M7_WITNESS_A10 && cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_dual_oos -- --ignored --nocapture`
  - 结果：`test theta_v0::backtest::wverify_run::m8_e2e_dual_oos ... ok. 1 passed; 0 failed`；墙钟 1151.28s（臂① ~370-387s/窗 + 臂②③ + significance ×9）。
  - 逐窗行：`p3fold: ①R=-8075576 LCB=-350390 共存1 ②R=+2931230 LCB=-1550267 ③R=-15232144 LCB=-18771457`；`wf7: ①R=+2285068 …`；`wf8: ①R=+19027856 …`（现场日志与 /tmp/m8_e2e_dual_oos.md 一致）。
- **全套闸门**（任务④）：`cargo test --release --lib` 结果见 §8（本报告回填区）。

## 6. provenance 与并发声明

- HEAD=`640609071d`；跑批时工作树脏文件 62 项（多工位并行在途，含 classifier/strategy/runner 面）——**本跑批 BSP 集以跑批时工作树为准**；数据 sha256=`16ea13d5…4707b`（p127 登记值）。
- **并发漂移注记**：实装期间 runner.rs 被兄弟工位在途改动（7944→8142 行，+189 行段平移）。三窗跑批执行于 7944 行状态编译产物；跑批后本报告将 wverify_run.rs 注释/报告字符串内的 runner.rs 锚点校正至 8142 行状态（:3682/:3616/:3628/:3988/:3745-3750/:3819-3824/:3898-3900/:4010-4011/:4048-4055/:8320）。/tmp/m8_e2e_dual_oos.md 内锚点为跑批时状态（:3493/:3427/:3439/:3790 等），两处差异=同一函数的行号平移，语义同一。锚点校正是注释改动，行为零改，未重跑跑批。

## 7. F-01 前视声明处置登记（runner.rs:366-369，留待下一波）

- 现状：臂①②入口 `run_theta_v0_dual`/`run_theta_v0` 均 F-01 deprecated（结构确认前视：全窗分类决策回放历史），产出**禁 L2/L3 认识论声明**（runner.rs:363-371/:272-274）。本工位对臂①②的用途限定：诊断/管线冒烟/账本结构验收（守恒断言、Q⁺/Q⁻ 可观测性、嵌入分叉观测）。
- **处置**：本工位授权文件不含 runner.rs ⟹ **不处置、如实登记，留待下一波协调**。待办项：(a) 双仓臂的 per-bar 因果重放验收闭环（嵌套产量的因果口径，施工图 §2 依赖层）；(b) 若届时需 leg_log/终态 DualLedger 直接可观测（免 trades 重建），最小方案 = runner.rs 侧 pub 化 `FillOutputDual` 或加 sidecar 字段——属 runner.rs 授权面，本工位不越权。
- 生产因果接线在 `nautilus::strategy::ThetaCore::recognize_current`（per-bar 窗口，runner.rs:368 已锚）。

## 8. 闸门回填区（cargo test --release --lib）

- 命令行全文：`unset KAPPA_BARRIER_NUM KAPPA_BARRIER_DEN THETA_DIR_PRESET M7_WITNESS_A10 && cargo test --release --lib`（worktree `/tmp/kimi-nest-mainline/rust`，2026-07-19）。
- 结果：**`test result: ok. 1755 passed; 0 failed; 129 ignored; 0 measured; 0 filtered out; finished in 0.98s`**——全绿零变红。
- 计数对账：任务书基线 1749 passed；当前 1755 passed（+6 = 多工位并发在途新增的测试，非本工位——本工位新增 1 个 #[ignore] 测试计入 129 ignored 侧，不进 passed；本工位对既有测试零改动）。

## 9. 纪律自检

- 090：臂①② 前视诊断口径与 LCB≤0 照实落盘（无 CONFIRMED 不修饰）；加仓近似列标 † 不作断言对象；provenance/并发漂移如实登记；授权面零越权（dual_ledger.rs/strategy/ledger.rs/runner.rs/Cargo.toml 零改）。
- v3：零概率/统计推断作决策基础（LCB 为验收判据非策略输入）；零回测验证策略（三窗跑批只验账本结构与守恒）；零 EMH。
- 零 git mutation；主仓零写入；改动仅 wverify_run.rs + 本文档（chanlun/review-results/ 授权面）。
