# Codex GAP3 返工二轮复审 — verdict fail（收敛到 chokepoint cash-sound gate）

commit 7000c2ec2c 复审。1致命(未resolved:pub transition_adapter 绕 cash 约束) + 2重要(不可达域过宽/stale注释) + κ小注释 + i128 resolved。

verdict: **fail**

**问题：public `transition_adapter` 仍可绕过 cash 约束，`free >= 0` 不是全路径不变量**  
位置：[transition.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/transition.rs:295)、[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:321)、[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:339)  
严重性：致命  
修复建议：`schedule_adapter` 路径已修住，但 `OrderOut`/`transition_adapter` 是 public；外部可传 `TwEvent::ShortDiff(-1)` 或 `RecoverCapital(1)`，OQ-9 `assert!` 会放行，因为这两类事件当前恒合法，随后 `tw_step` 可把 `free=0` 打成负数。要么收窄 `OrderOut`/`transition_adapter` 可见性，要么在 `transition_adapter` 增加 cash-sound gate：`ShortDiff(d<0)` 要求 `free >= -d`，`ShortDiff(d>0)` 要求 `holding >= d`，`RecoverCapital(w)` 要求 `0 <= w <= free`，并断言结果 `free/holding/withdrawn >= 0`。

**问题：不可达证明方向是对的，但“任何 TW 事件序列”表述过宽**  
位置：[runner.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1877)、[runner.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1906)、[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:327)  
严重性：重要  
修复建议：这不是上一轮那种硬凑可达见证；对 `funded_campaign -> hybrid_step/policy_output/stage_progression` 的 sound 生产路径，TW=Q 下 `holding>=Q` 与 `free>0` 互斥，EarningShares 不可达是诚实结构结论。但原文说“任何 TW 事件序列”不成立：raw/legal `RecoverCapital(1)` 可从 `free=Q, holding=0` 直接推进 `CapitalRecovered`，只是 unsound。把证明域改成“stage_progression 派生的 sound closed-loop 路径”，或增强 `TwEvent::is_legal_from` 的 cash legality。

**问题：残留旧注释仍宣称结构可达/引用已删除测试**  
位置：[runner.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1845)、[state.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/state.rs:210)、[transition.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/transition.rs:291)  
严重性：重要  
修复建议：删除“EarningShares 结构可达”“能真推进”“见 `stage_progression_reaches_earning_shares_with_funded_recovery`”这类旧口径，改成“仅在有外部 sound 利润/TW 注入前提时机制可推进；当前 funded campaign L0/TW 守恒路径结构不可达”。

**问题：κ 可见性注释仍不精确**  
位置：[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:190)  
严重性：建议  
修复建议：字段不是 `crate-private`，而是模块私有且子模块测试可访问。改成“public API / crate 外部不可构造负 κ；本模块测试可构造反向见证”。

六问总结：

1. 致命1：**未resolved/部分resolved**。`schedule_adapter` 生产路径修住；public `transition_adapter` 漏网。  
2. 致命2：**部分resolved**。删硬凑见证是对的；不可达结论需收窄到 sound closed-loop 路径。  
3. stale 注释：**未resolved**。仍有明确残留。  
4. release 门：**部分resolved**。OQ-9 `debug_assert` 已改 `assert!`，但 cash-sound 非法转移仍 release 放行。  
5. κ 口径：**基本resolved，仍有小注释错误**。  
6. i128 边界：**resolved**。测试与失败安全说明到位。

未运行测试：当前环境只读，无法可靠执行 cargo 测试。
verdict: **fail**

**问题：public `transition_adapter` 仍可绕过 cash 约束，`free >= 0` 不是全路径不变量**  
位置：[transition.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/transition.rs:295)、[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:321)、[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:339)  
严重性：致命  
修复建议：`schedule_adapter` 路径已修住，但 `OrderOut`/`transition_adapter` 是 public；外部可传 `TwEvent::ShortDiff(-1)` 或 `RecoverCapital(1)`，OQ-9 `assert!` 会放行，因为这两类事件当前恒合法，随后 `tw_step` 可把 `free=0` 打成负数。要么收窄 `OrderOut`/`transition_adapter` 可见性，要么在 `transition_adapter` 增加 cash-sound gate：`ShortDiff(d<0)` 要求 `free >= -d`，`ShortDiff(d>0)` 要求 `holding >= d`，`RecoverCapital(w)` 要求 `0 <= w <= free`，并断言结果 `free/holding/withdrawn >= 0`。

**问题：不可达证明方向是对的，但“任何 TW 事件序列”表述过宽**  
位置：[runner.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1877)、[runner.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1906)、[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:327)  
严重性：重要  
修复建议：这不是上一轮那种硬凑可达见证；对 `funded_campaign -> hybrid_step/policy_output/stage_progression` 的 sound 生产路径，TW=Q 下 `holding>=Q` 与 `free>0` 互斥，EarningShares 不可达是诚实结构结论。但原文说“任何 TW 事件序列”不成立：raw/legal `RecoverCapital(1)` 可从 `free=Q, holding=0` 直接推进 `CapitalRecovered`，只是 unsound。把证明域改成“stage_progression 派生的 sound closed-loop 路径”，或增强 `TwEvent::is_legal_from` 的 cash legality。

**问题：残留旧注释仍宣称结构可达/引用已删除测试**  
位置：[runner.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/backtest/runner.rs:1845)、[state.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/state.rs:210)、[transition.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/closed_loop/transition.rs:291)  
严重性：重要  
修复建议：删除“EarningShares 结构可达”“能真推进”“见 `stage_progression_reaches_earning_shares_with_funded_recovery`”这类旧口径，改成“仅在有外部 sound 利润/TW 注入前提时机制可推进；当前 funded campaign L0/TW 守恒路径结构不可达”。

**问题：κ 可见性注释仍不精确**  
位置：[ledger.rs](/Users/silencehan/Projects/NewChanlun/rust/src/theta_v0/strategy/ledger.rs:190)  
严重性：建议  
修复建议：字段不是 `crate-private`，而是模块私有且子模块测试可访问。改成“public API / crate 外部不可构造负 κ；本模块测试可构造反向见证”。

六问总结：

1. 致命1：**未resolved/部分resolved**。`schedule_adapter` 生产路径修住；public `transition_adapter` 漏网。  
2. 致命2：**部分resolved**。删硬凑见证是对的；不可达结论需收窄到 sound closed-loop 路径。  
3. stale 注释：**未resolved**。仍有明确残留。  
4. release 门：**部分resolved**。OQ-9 `debug_assert` 已改 `assert!`，但 cash-sound 非法转移仍 release 放行。  
