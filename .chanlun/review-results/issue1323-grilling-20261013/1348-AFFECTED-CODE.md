# #1348 受影响代码清单（本金与真实回收的边界会计 policy）

> 票：[#1348](https://github.com/xy7365527-lang/NewChanlun/issues/1348)；裁定正本：[ADR 0031](../../../docs/adr/0031-principal-batches-phase-predicate-corporate-actions.md)；教义正本：[`sanjieduan.md`](../../definitions/sanjieduan.md)。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**（标 main）；#1394 财务线 **FIN `0d8a6017231348e2c4a005f68d4e2e7a457a59fe`**（`codex/1394-transport-revision-20261012`，未合入 main，标 FIN）。行号于各自基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/economic_session/book.rs:151` | main `465174240b` | 分量白名单增 principal_contribution / fee_only / dividend_cash / withholding_tax / split | Q1,Q2,Q5 → #1406 |
| Rust | `rust/src/economic_session/book.rs:225` | main `465174240b` | `du==0` 二分：无费用分量拒；有已声明费用分量则接纳、只改 K | Q2 → #1406 |
| Rust | `rust/src/economic_session/book.rs:347` | main `465174240b` | `principal_baseline` → Known 批次清单；`capital_phase` → phase_events + growth_eligible；`unit_cost_policy` → 整数对/NotComputable | Q1,Q3,Q4 → #1406 |
| Rust | `rust/src/economic_session/book.rs:356-357` | main `465174240b` | 非负存量域不变，q=0 合法 | Q2 |
| Rust | `rust/src/economic_session/book.rs:370` | main `465174240b` | Π/A/W/R 算子不变；A 不进 P | Q1 |
| Rust | `rust/src/economic_session/book.rs:398-413` | main `465174240b` | K 有符号累加（处置现金、回收、分红进入后可为负），不 clamp | Q3,Q4,Q5 → #1406 |
| Rust | `rust/src/economic_session/store.rs:636` | main `465174240b` | 审计重放覆盖 fee_only / principal_contribution / 公司行动事件 | Q1,Q2,Q5 → #1406 |
| Rust | `rust/src/economic_session/voice.rs:149` | main `465174240b` | 新 Voice 开局依 Applied 原件；不接旧尾账 | Q2 → #1421 |
| Rust | `rust/src/economic_session/voice.rs:165` | main `465174240b` | 开局量 > 0 不变 | Q2 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:94` | main `465174240b` | `TStage` 只作阶段事件史投影；分工注释须带 #1348 | Q3 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:121` | main `465174240b` | `advance_to` 不改 | Q3 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:175` | main `465174240b` | `TwState`：notional_in 不是本金批次；stage 不作准入输入 | Q1,Q3 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:716/742` | main `465174240b` | `LedgerEvent` / `ledger_step_components` 不变 | Q1 |
| Rust | `rust/src/theta_v0/backtest/fill.rs:200-255` | main `465174240b` | 旧净额臂（全平重置 entry_cost，`:224`）与本裁定不同口径，不作本金定义；列入 C11 收编/退役清单，本轮不代裁终态 | 根裁定第 10 条 |
| Rust | `rust/src/session_protocol.rs:39` | main `465174240b` | 引用：`parse_canonical_i64` 规范十进制整数文本，不改 | 共同前提 |
| Rust | `rust/src/economic_session/finance.rs:143-145` | FIN `0d8a6017` | 引用：`checked_add` 整数约定，不改 | 共同前提 |
| Rust | `rust/src/economic_session/finance.rs:208-210/227` | FIN `0d8a6017` | 引用：money_unit / quantity_unit / quantum 政策字段与 `quantum == "1"` 校验，不改 | 共同前提 |
| Rust | `rust/src/economic_session/finance.rs:234-239` | FIN `0d8a6017` | 引用：`baseline.amount` 是 E 共同责任界，与本金 P 禁止互代，不改 | Q1 |
| Lean | `formal/Origin/TotalWealth.lean:97` | main `465174240b` | `TWState.notional_in` 登记为非本金批次名分；不改定义 | Q1 |
| Lean | `formal/Origin/TotalWealth.lean:140` | main `465174240b` | `advanceTo` 不改 | Q3 |
| Lean | `formal/Origin/TotalWealth.lean:194` | main `465174240b` | `stage_rank_monotone` 不改；新增对拍义务：全域单向不变式 `growth_eligible ⇒ latch = EarningShares`（#1406 OQ-2 订正：原「无费用事件时二者一致」被回本后无费用亏损往返反证），Rust 侧 growth_eligible 另有同输入同输出锁 | Q3 → #1406 |
| Lean | `formal/Origin/TotalWealth.lean:208` | main `465174240b` | EarningShares 不可回退（`earning_no_regress`）不改 | Q3 |
| Lean | `formal/Origin/LedgerReconciliation.lean:467` | main `465174240b` | `hAlloc` 前件不改；不从中推本金 | Q1 |
| Python | `s_session/economic_query.py:72` | main `465174240b` | 只透传 principal / capital_phase / unit_cost 的 typed 结构 | Q1-Q4 |
| Python | `s_session/economic_http.py:36/62` | main `465174240b` | GET/POST 入口只透传，不新判 | Q1-Q4 |
| JS | `s_session/browser/economic.html:67` | main `465174240b` | 「本金与阶段」卡片显示 NotComputable 原因与 K/q 整数对，不计算比率 | Q2,Q4 |
| JS | `s_session/browser/tb03a-client.js:124` | main `465174240b` | 图像校验键集若扩 principal/phase_events 须同步 | Q1,Q3 |

## 本轮复核订正（相对工位草稿，基线 f95a7f3e → 465174240b）

- 工位草稿引 FIN `finance.rs:226` 为「money_unit/quantum 政策字段形态」，FIN 上 `:226` 是 `model` 校验；字段在 `:208-210`、`quantum` 校验在 `:227`，已订正。
- 工位草稿引 FIN `finance.rs:235` 为 `baseline.amount`；`baseline` 校验块在 `:234-239`，已订正。
- 工位 ADR 草稿把规范十进制解析写作 `session_protocol.rs:39`，实际路径为 `rust/src/session_protocol.rs:39`（main），已订正。
- `s_session/economic_http.py`、`s_session/browser/economic.html` 本轮补定位到 `:36/:62` 与 `:67`。

基线迁移说明：工位草稿以 main `f95a7f3e` 为基线；`git diff f95a7f3e 465174240b` 在本清单涉及的全部路径上为空，main 行号因此不漂。
