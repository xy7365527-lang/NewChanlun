# #1349 受影响代码清单（共同资金争用、cap 与公用资金进出 policy）

> 票：[#1349](https://github.com/xy7365527-lang/NewChanlun/issues/1349)；裁定正本：[ADR 0029](../../../docs/adr/0029-common-cap-contention-policy.md)。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**（标 main）；#1394 财务线 **FIN `0d8a6017231348e2c4a005f68d4e2e7a457a59fe`**（`codex/1394-transport-revision-20261012`，未合入 main，标 FIN）。行号于各自基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/economic_session/finance.rs:204-213` | FIN `0d8a6017` | 政策字段加 version / authorizer / authorized_at / profile_id | #1394 或 #1408 |
| Rust | `rust/src/economic_session/finance.rs:225` | FIN `0d8a6017` | `allocation_policy_id` 由 `#1349/unresolved-common-cap-allocation` 改为 `#1349/atomic-slot-wait/v1` | #1394 |
| Rust | `rust/src/economic_session/finance.rs:231-232` | FIN `0d8a6017` | 现状：cap / valid_through_ns 已校验为数（不改） | — |
| Rust | `rust/src/economic_session/finance.rs:420-425` | FIN `0d8a6017` | 现状：锁额 = 数量×限价+费用尾（毛额，不改） | — |
| Rust | `rust/src/economic_session/finance.rs:833` | FIN `0d8a6017` | 现状：valid_through_ns 过期检查；补 `PolicyExpired` 原因写入等待 slot | #1408 |
| Rust | `rust/src/economic_session/finance.rs:905-919` | FIN `0d8a6017` | `:917` 文案改引具名政策；补 slot 失效（申请者重新 Prepare/撤回）与重组 | #1408 |
| Rust | `rust/src/economic_session/finance.rs:985-993` | FIN `0d8a6017` | 现状：私有锁须等于原计划全额（与裁定一一致，不改） | — |
| Rust | `rust/src/economic_session/finance.rs:1015-1024` | FIN `0d8a6017` | all_fit 判定（`:1023`）保留；触发源扩展到政策版本事件与责任释放事件；grant JSON（`:1024`）加 `policy_version` | #1408 |
| Rust | `rust/src/economic_session/finance.rs:1754` | FIN `0d8a6017` | 现状：协调入口遇 `AwaitingAllocationPolicy` 返回，不进 Commit（不改） | — |
| Rust | `rust/src/economic_session/finance_tests.rs:126` | FIN `0d8a6017` | 夹具政策 id 改具名；加版本字段 | #1394 |
| Rust | `rust/src/economic_session/finance_tests.rs:342-349` | FIN `0d8a6017` | `insufficient_cap_waits_for_exact_policy_without_selecting_winner` 断言的 id 改具名；语义不变 | #1394 |
| Rust | `rust/src/economic_session/book.rs:175` | main `465174240b` | 现状：fixed_q 必填正数；本票不动 Q | — |
| Rust | `rust/src/theta_v0/strategy/risk.rs:536/554/598` | main `465174240b` | 不改：旧比值帽，非 R2 owner | — |
| Rust | `rust/src/theta_v0/config.rs:211/243` | main `465174240b` | 不改：`enforce_gross_cap` 默认 false，旧实现 | — |
| Lean | `formal/Origin/LeverageCapital.lean` | main `465174240b` | 不改：净 ≤ 毛桥已锚；「不越过」是算术推论，若要形式化归 TB-04-I 自定 | — |
| JS | `s_session/browser/tb03a-client.js:181` | FIN `0d8a6017` | `need(p.allocation_policy_id===…unresolved…)` 改具名 | #1394 |
| JS | `s_session/browser/tb03a-client.js:406` | FIN `0d8a6017` | `waiting_for` 校验改具名；补 `PolicyExpired` 原因呈现 | #1394/#1408 |
| Python | `s_session/tests/tb03b_oracle.py:48` | FIN `0d8a6017` | 断言 id 改具名 | #1394 |
| Python | `s_session/tests/tb03b_oracle.py:65-66` | FIN `0d8a6017` | 等待 slot 断言不变，id 改具名 | #1394 |
| JS | `s_session/tests/test_tb03a_voice_client.cjs:447/500` | FIN `0d8a6017` | id 改具名 | #1394 |

## 本轮复核订正（相对工位草稿，基线 f95a7f3e → 465174240b）

- 工位草稿引的 `finance.rs:912-919` 与 `:1022-1024` 在 FIN 上分别对应 `:917`（等待文案）与 `:1023-1024`（all_fit 与 grant JSON），已按实际行号收窄。

基线迁移说明：工位草稿以 main `f95a7f3e` 为基线；`git diff f95a7f3e 465174240b` 在本清单涉及的全部路径上为空，main 行号因此不漂。
