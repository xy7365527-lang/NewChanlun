# Codex GAP3 返工复审 — 2026-07-01T21:26:47Z

- mode: review (异质复审)
- subject: GAP3 返工（codex#9 三致命修复）commit 5e92b99380
- verdict: **fail**（phantom_buy/phase_desync resolved；其余未 resolved 或不完整）

## Codex 原始输出

已对 `5e92b99380` 做复审。verdict: **fail**。

1. 问题：`negative_free` 只修了退本金路径，开仓 `ShortDiff(-Δ)` 仍可把 `free` 打成负数。
位置：[transition.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/transition.rs:143)、[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:337)、[state.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/state.rs:192)
严重性：致命
修复建议：`schedule_adapter`/`risk_adapter` 必须受可用 `free` 约束，或 `transition_adapter` 返回 `Result` 拒绝 `ShortDiff(-Δ)` 超过 free 的订单。加回归：`AssemblyState::initial` 经 `hybrid_step_baseline` 不得产生 `free < 0`，否则当前 `initial free=0, positions=0 -> Buy Δ=1 -> free=-1` 是漏网。

2. 问题：`funded_recovery` 见证态是手工拼的“已获利 campaign”，不是闭环/fill 可达态。
位置：[runner.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1891)
严重性：致命
修复建议：用真实 fill/closed-loop 序列生成 `free=Q ∧ holding=Q`，并同步 `positions`、`ledger_state`、`cum_net_cash` 的利润证据。现在它直接构造 `free=q, holding=q, notional_in=q`，但 `positions` 仍来自 `funded_campaign` 的 0，利润来源没有路径证据；只能证明局部 `stage_progression`，不能证明 GAP3 真实可达。

3. 问题：L0 同价 `EarningShares` 不触达本身是诚实的负结果，但当前改法把原“回测触达”验收改成“不触达”，属于未显式裁剪验收域。
位置：[runner.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1849)、[runner.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:809)
严重性：重要
修复建议：要么正式把 `acc-GAP3-earningshares-reachable` 改为“L0 不触达 + L2 利润路径触达”，要么补真实 L2 价格升值/卖高/fill 利润路径测试。另，`run_closed_loop` 注释仍说“每 bar ShortDiff(-1) 累 holding 到 Q 后 EarningShares 可达”，与新实现/测试矛盾，必须改。

4. 问题：`release_gate` 仍未解决；非法 `OrderOut.tw_event` 在 release 下可穿过 `transition_adapter`。
位置：[transition.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/transition.rs:75)、[transition.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/transition.rs:286)
严重性：重要
修复建议：`transition_adapter` 不应只 `debug_assert!`。要改成 `Result<AssemblyState, Error>`、普通 `assert!`，或收窄可见性，保证生产路径不能对非法 `OpenShareLeg/CloseShareLeg/EnterEarning` raw-step。

5. 问题：`kappa_nonneg` 外部 API 基本修了，但“类型层不可构造”表述过强，模块内测试仍直接构造负 κ。
位置：[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:185)、[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:673)
严重性：重要
修复建议：测试和模块内代码统一改用 `RiskPolicy::try_new`/`baseline`/`kappa()`，删除 `RiskPolicy { kappa: -1 }` 这种反向见证，或把表述降级为“外部 API 不可构造”。

6. 问题：`lean_parity` 的 i128 改动解决了 release wrap 主问题，但缺少边界回归/Lean 有界对应证明。
位置：[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:229)、[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:276)
严重性：建议
修复建议：补 `i64::MAX` 附近的 `eta_star`/`buy_core_legal` 回归，并明确 clamp 后不再是 Lean 无界 Int 的 bit-exact，只是 Rust 有界失败安全近似。

stance 总结：`phantom_buy` resolved；`phase_desync` resolved；`negative_free` 未resolved；`release_gate` 未resolved；`kappa_nonneg` 严格口径未resolved；`lean_parity` 代码层 resolved、证明/边界测试未resolved。

验证限制：尝试 `cargo test gap3 --lib --no-default-features`，但只读沙箱无法打开 `rust/target/debug/.cargo-build-lock`，未能实际跑测试。
