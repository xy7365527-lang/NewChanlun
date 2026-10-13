# #1353 受影响代码清单（完整历史模拟执行模型）

> 票：[#1353](https://github.com/xy7365527-lang/NewChanlun/issues/1353)；裁定正本：[ADR 0033](../../../docs/adr/0033-history-simulation-bar-batch-model.md)。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**（标 main）；#1394 财务线 **FIN `0d8a6017231348e2c4a005f68d4e2e7a457a59fe`**（`codex/1394-transport-revision-20261012`，未合入 main，标 FIN）。行号于各自基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/theta_v0/backtest/fill.rs:115` | main `465174240b` | 现金约束注释：ADR 0018 裁定三排序理由的出处；重间不适用（本模型依据），不改 | Q1 |
| Rust | `rust/src/theta_v0/backtest/fill.rs:4767` | main `465174240b` | `pi_theta_fill_loop_overlay` 旧模拟臂：C11 收编/退役清单，不作正式模型 | 根裁定第 10 条 |
| Rust | `rust/src/theta_v0/backtest/fill.rs:4806` | main `465174240b` | 单标量 `fee_rate`：新 profile 以 `fee_profile` 引用来源，不代填 | Q5 |
| Rust | `rust/src/theta_v0/backtest/fill.rs:5024` | main `465174240b` | 市价按 bar close：口径继承，由新 profile 的具名市价槽承载 | Q1,Q4 |
| Rust | `rust/src/theta_v0/backtest/fill.rs:5044` | main `465174240b` | `pending` Vec 遍历：遍历不是模型，C11 清单 | 根裁定第 10 条 |
| Rust | `rust/src/theta_v0/backtest/fill.rs:200-255` | main `465174240b` | 净额臂 entry_cost：C11 清单 | 根裁定第 10 条 |
| Rust | `rust/src/theta_v0/strategy/exec.rs:145` | main `465174240b` | `ConflictKey` 旧规划序，不挪为撮合或资金政策；C11 清单 | 根裁定第 10 条 |
| Rust | `rust/src/theta_v0/strategy/mod.rs:316/386` | main `465174240b` | `plan_orders` 旧 NT 排序落点，对照导航 | Q4 |
| Rust | `rust/src/theta_v0/strategy/coverage/sizing.rs:533` | main `465174240b` | 单净单 `schedule_order`：不能据此推 N 重不可达；C11 清单 | 根裁定第 10 条 |
| Rust | `rust/src/theta_v0/venue_fee.rs:112` | main `465174240b` | 生产 Taker 常量不证明角色来源 | Q5 |
| Rust | `rust/src/theta_v0/types.rs:363` | main `465174240b` | 旧 `Order` 只有 action/qty/exec_index：迁移导航，不装成 R2 族身份 | Q1 |
| Rust | `rust/src/economic_session/book.rs:55/58` | main `465174240b` | decoder：新增 simulated profile 标签，不改 TestOnly 标签冒充 | Q5 |
| Rust | `rust/src/economic_session/service.rs:169/179` | main `465174240b` | 模型原件经同一 P4 Recorded+Pending 路径 | Q5 |
| Rust | `rust/src/economic_session/service.rs:221` | main `465174240b` | ApplyAllocation 只作 RD-12 接口；matcher 不写 B 账 | Q4 |
| Lean | `formal/Origin/TotalWealth.lean:140` | main `465174240b` | 不决定撮合，不改 | 共同前提 |
| Lean | `formal/Origin/LedgerReconciliation.lean:467` | main `465174240b` | 不决定撮合，不改；批模型累计/潜量性质若要形式化另立文件 | 共同前提 |
| Python | `s_session/（完整历史 adapter）` | main `465174240b` | 本轮未定位获准的历史 adapter；TB-07 从正式 launcher 补实际清单；Python 不新判成交或费用归属 | TB-07 |

## 本轮复核订正（相对工位草稿，基线 f95a7f3e → 465174240b）

- 根裁定第 9 条补入 `projection_kind` 同核口径与 `capacity_source=none` 的 L0 单列；它们是 TB-07（#1381 等）与 TB-08-A（#1382）的实施义务，不对应现有代码行。

基线迁移说明：工位草稿以 main `f95a7f3e` 为基线；`git diff f95a7f3e 465174240b` 在本清单涉及的全部路径上为空，main 行号因此不漂。
