## 问题

**CI 从不执行 `cargo test`** ⟹ 仓内所有 Rust 侧的测试锁**只编译、从不运行**，「违规即测试红」在 CI 里是空话。

`.github/workflows/ci.yml` 实查三个 job：

| job | 实际跑什么 |
|---|---|
| `test` | pytest（`-m "not slow"`）+ 一条 grep gate（`:36` G1） |
| `rust-check` | `cargo check --all-targets` / `cargo check --features backtest_bin` / `cargo fmt --check` —— **无 `cargo test`** |
| `fixture-drift` | `lake build` + `scripts/check_fixture_drift.py`（Lean → fixture 一致性） |

**已知受害面**（非穷举）：

- `rust/tests/nest_isolation_guard.rs` —— 文件头自陈「违规即测试红」，实际从不执行；
- **34 个 Lean↔Rust parity 断言**（[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 清点）—— 同上；
- `fixture-drift` 只证 Lean→fixture 这半环，**fixture→Rust 半环断开**。

## 为什么现在做

这条是三张裁定票共同的前置：

- [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十一立原则：**验收判据必须真的被 CI 执行才算数，只编译不执行的锁按「无锁」计**；并明写「**下游实施链的第一条动作 = 让 CI 跑 `cargo test`**，否则本票裁的任何判据落地即空转」。
- [#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 裁定一把总缝规则的机械锁定为**对拍测试锁**（属「行为不变式」类，grep 挡不住同义代码重写）—— 同样落地即空转。
- [#806](https://github.com/xy7365527-lang/NewChanlun/issues/806) 查出「一二档的锁都已打好、没接进任何 gate」，本票即其成因的直接修复。

**不依赖任何还没裁的口径**，与 [map #787](https://github.com/xy7365527-lang/NewChanlun/issues/787) 的教义收敛主线**并行**，不占其路径。

## 要做的

1. `rust-check` job（或新 job）加 `cargo test`，**跑到绿**；
2. **先跑一遍看现状**——本票的第一个产出是「当前有多少测试是红的 / 多少是慢的」这个读数。**红的不许直接删或 `#[ignore]` 掉**：每一条要么修，要么单独挂票并在本票留指针说明为什么留着。
3. 慢测试的处置（是否分层、是否只在 push to main 跑）按实测耗时定，本票内决定并写明。
4. `#[ignore]` 的既有测试（如 `theta_v0_fixture_drift`）维持现状，不在本票范围。

## 验收

- CI 上 `cargo test` 真的执行且为绿（贴 run 链接）；
- 报一份数：跑了多少条、耗时多少、有没有因此发现真 bug（有则单独挂票）；
- 关票走 `docs/agents/delivery-discipline.md` 关票门全子句。

## 来源

[map #787](https://github.com/xy7365527-lang/NewChanlun/issues/787) 走图期间浮出（[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 净发现①）。**本票不属该图子票**——它交付的是成果不是决策，按本仓 wayfinder 正本走 triage，不进图。
