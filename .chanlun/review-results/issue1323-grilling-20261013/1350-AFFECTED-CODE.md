# #1350 受影响代码清单（净额外效与部分成交费用的分配归属）

> 票：[#1350](https://github.com/xy7365527-lang/NewChanlun/issues/1350)；裁定正本：[ADR 0032](../../../docs/adr/0032-fill-allocation-policy.md)。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**（标 main）；#1394 财务线 **FIN `0d8a6017231348e2c4a005f68d4e2e7a457a59fe`**（`codex/1394-transport-revision-20261012`，未合入 main，标 FIN）。行号于各自基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/economic_session/book.rs:45` | main `465174240b` | fact_key 不变；allocation 记录增 (fill_allocation_policy_id, manifest_hash, policy_revision) | Q1,Q5 → #1406 |
| Rust | `rust/src/economic_session/book.rs:55/70` | main `465174240b` | 原件由单目标 allocation 扩为多目标列表，每条带政策版本与 manifest_hash | Q1 |
| Rust | `rust/src/economic_session/book.rs:225` | main `465174240b` | `du=0` 费用合法（与 #1348 同） | Q4 |
| Rust | `rust/src/economic_session/book.rs:270` | main `465174240b` | 具名利润的费用范围核对沿原 fill 分配 | Q4 |
| Rust | `rust/src/economic_session/book.rs:321/334/421` | main `465174240b` | apply 接受按本重分配量与费用分量；Unmatched 只留 family 不符与 per_leg 缺归属；Applied/Unmatched 记录带政策版本 | Q1,Q4,Q5 |
| Rust | `rust/src/economic_session/service.rs:130/169/179` | main `465174240b` | E 同 fact_key 核内容不变；Pending 原因增 AwaitingDependency（fee-before-fill）与前件指针；原件落盘保存 manifest 引用 | Q1,Q4 |
| Rust | `rust/src/economic_session/service.rs:221/244/257/286` | main `465174240b` | ApplyAllocation 接受多重 Resolved、逐重 Applied；每重一收据，同内容回原收据；同事务写多重 applications | Q1 |
| Rust | `rust/src/economic_session/voice.rs:149/166` | main `465174240b` | 跨代迟到回原 Voice 的不可变映射 | Q4,Q5 → #1421 |
| Rust | `rust/src/economic_session/store.rs:31/499` | main `465174240b` | commits 图像含 manifest 与政策版本 | Q1,Q5 |
| Rust | `rust/src/theta_v0/backtest/fill.rs:207/250` | main `465174240b` | 旧净额臂费用按名义累积（`fill.rs:200-255` 段）：列入 C11 收编/退役清单，不作分腿政策 | 根裁定第 10 条 |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:716/742` | main `465174240b` | 归属确定后才应用，不变 | Q1 |
| Rust | `rust/src/economic_session/finance.rs:205/225` | FIN `0d8a6017` | 引用：`allocation_policy_id` 是 RD-11 准入政策 id（字段表 `:205`，取值校验 `:225`）；外效分配 id 另命名 `fill_allocation_policy_id`，不改 | Q1 |
| Rust | `rust/src/theta_v0/strategy/exec.rs:145` | main `465174240b` | 不受影响：`ConflictKey` 是旧 NT 规划排序键，不是分配政策 | — |
| Lean | `formal/Origin/LedgerReconciliation.lean:456/467/483` | main `465174240b` | 不改；新增义务另立文件：Σ A_i(F)=F、单调、A_i(Q)=q_i、逆规则差量恒等 | Q1,Q3,Q5 |
| Python | `s_session/economic_query.py:72` | main `465174240b` | 透传多重 allocation 与政策版本 | Q1 |
| JS | `s_session/browser/tb03a-client.js:124` | main `465174240b` | 图像键集若扩 manifest/applications 须同步 | Q1 |

## 本轮复核订正（相对工位草稿，基线 f95a7f3e → 465174240b）

- 工位草稿与根裁定第 8 条都写 FIN `finance.rs:226` 为 `allocation_policy_id`；FIN 上 `:226` 是 `model` 校验，字段名在 `:205`、取值校验在 `:225`，已订正。结论（两个 id 空间分开）不变。

基线迁移说明：工位草稿以 main `f95a7f3e` 为基线；`git diff f95a7f3e 465174240b` 在本清单涉及的全部路径上为空，main 行号因此不漂。
