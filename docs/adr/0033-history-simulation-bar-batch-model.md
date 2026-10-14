# ADR 0033：完整历史模拟执行模型——已知前沿分段的 bar 批模型

**日期**：2026-10-13
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核采纳并补充同核口径），走 [[grilling] 完整历史模拟执行模型 #1353](https://github.com/xy7365527-lang/NewChanlun/issues/1353)（RD-17 / C10.3，SPEC #1340）
**状态**：已采纳。本 ADR 只定模型组织、适用域与版本合同字段；实现、取证与 H1–H8 oracle 归 TB-07 叶票（#1381 等），不以本 ADR 关票声称实现完成。
**继承**：[ADR 0018](0018-limit-order-fill-model.md) 五条、[ADR 0019](0019-limit-order-state-flow-interface.md) 全部，原样有效；本 ADR 只补多重并发维度。

## 裁定

### 一、模型 ＝ 已知前沿分段的 bar 批（第一问 A）

模型 id `history-sim/bar-batch-known-frontier/1`。第 t 根 bar 适用集合 `S_{t−1}`（t−1 收盘时已输出并经 RD-11 准入）；bar t 的外效构成批 `B_t`；批内各重订单各自独立对 OHLC 判定（严格穿越、开盘取优），互不排序、互不占用对方资金，`intra_batch_order: Unknown`；bar t 收盘才知道的新挂/撤/改从 t+1 起作用。市价动作在 `exec_index` bar 收盘价的具名槽成交；限价判定作用于全区间，逻辑上先于市价槽（收盘是 bar 最后价格的定义，不是推路径）。输入域外的请求 ⟹ `ModelOrderUnspecified`/`Unsupported`，不回落旧遍历。v1 确定性，`seed: none`。

**教义依据**：`040-第40课.md:20`【正文】「每一重都对应着一定的资金与筹码」、`040-第40课.md:22`【正文】「每一层次的操作都是独立又在一个整体的操作中」；经 [ADR 0013](0013-partition-machine-bidirectional-form.md) 裁定一、[ADR 0014](0014-intra-chong-unidirectional-inter-chong-no-arbitration.md) 裁定二落地 ⟹ ADR 0018 裁定三的排序理由（`rust/src/theta_v0/backtest/fill.rs:115` 的现金约束）在重之间不存在；C05.2 重内一次跃迁只一个动作。ADR 0018 裁定四「撤单不得用预算到期 bar」（锚 `049-第49课.md:62`【正文】「其后，中枢已经完成就无所谓中枢震荡了」）在多重上的形式就是前沿规则。

### 二、部分成交只来自具证容量合同（第二问 A）

`capacity_source ∈ {none, input(ref), scenario(ref)}`。

- `none`：在 [ADR 0016](0016-chong-quantity-layer-capacity-frequency-shift.md) 容量门已准入的规模内，继承「穿越即全量成交」，显式标为 L0 假设。这类成交可以进入 TB-07 全史双跑，但报告须单列 L0 计数，不得作为容量证据（根补充）。
- `input/scenario`：批内同向可成量 q 按 [ADR 0032](0032-fill-allocation-policy.md) 裁定一/二的冻结比例分到各重，剩余量按 P7 进 E。

不从 bar volume 推可用量；不设模型容量函数。

### 三、撤改在下一边界生效（第三问 A）

旧单在当前 bar 继续担责并可能成交；bar 收盘即其终态累计 F 完整；改价新版本从 t+1 起，额度 L−F（C07.8/9）。禁止用收盘信息取消 bar 内可能已发生的成交（ADR 0018 裁定四）。

### 四、同 slot 多重同批发生，投影方式由清单冻结（第四问 A）

`projection_kind` 是冻结 manifest 的参数，与 ADR 0032 裁定一共用一份定义，无运行时回落（根补充）。

- 完整历史 v1 默认 `projection_kind = per_leg`（ADR 0024 裁定六档 B）；`netted` 只作测试 ADR 0032 的声明 profile。
- **同核口径**：凡「回放对拍 paper/live」的比较（Destination ⑤），回放必须使用被比较的 paper/live profile 冻结的同一 `projection_kind`。[ADR 0028](0028-first-capability-verification-profile.md) 的 `P1-hl-testnet` v1 为 `netted`，与它对拍的回放即取 `netted`。

重间不借钱（ADR 0016 裁定五）⟹ 平开资金依赖在本仓形态下不出现；重内反向穿零已禁（C05.2）。市限混合由裁定一解释；新挂单最早 t+1。

### 五、独立不可变原件 + 因果更正（第五问 A）

fill、fee、bust 各为独立原件，fee 带 `cause: fill(exec_id)`，经同一 P4 路径接纳；模拟 bust 只在 `scenario` 合同声明时产生。费率与角色来源由 `fee_profile` 引用，不填数；`rust/src/theta_v0/venue_fee.rs:112` 的 Taker 常量不证明角色。

## 版本合同字段（v1）

`model_id/revision/content_hash`；`input_domain`；`clock`（bar 索引、`known_frontier_cut`）；`price_rules`（带 ADR 0018/0019 条款 id）；`capacity_source`；`projection_kind`；`fee_profile`；`seed`；`unsupported_domains`；RD-11/RD-12 政策 id；调度轨迹。

## 数值口径

价格为整数 tick、数量为 quantum 整数、金额为 `i64`；模型不引入浮点。

## 旧臂

`fill.rs:4767`（`pi_theta_fill_loop_overlay`）、`fill.rs:5044`（`pending` 遍历）、`strategy/exec.rs:145`（`ConflictKey`）、`strategy/coverage/sizing.rs:533`（单净单 `schedule_order`）与 `fill.rs:200-255`（净额臂 entry_cost）列入 C11 收编/退役清单；本 ADR 不代裁它们的终态（根裁定第 10 条）。

## 受影响代码清单

见 [`.chanlun/review-results/issue1323-grilling-20261013/1353-AFFECTED-CODE.md`](../../.chanlun/review-results/issue1323-grilling-20261013/1353-AFFECTED-CODE.md)。Lean 结构定义不受影响；`formal/Origin/TotalWealth.lean:140`、`formal/Origin/LedgerReconciliation.lean:467` 不决定撮合。Python 没有获准的历史 adapter，TB-07 补实际清单。

## 下游票

[#1381](https://github.com/xy7365527-lang/NewChanlun/issues/1381)/[#1399](https://github.com/xy7365527-lang/NewChanlun/issues/1399)/[#1411](https://github.com/xy7365527-lang/NewChanlun/issues/1411)/[#1418](https://github.com/xy7365527-lang/NewChanlun/issues/1418)（TB-07）、[#1382](https://github.com/xy7365527-lang/NewChanlun/issues/1382)（TB-08）、[#1397](https://github.com/xy7365527-lang/NewChanlun/issues/1397)/[#1398](https://github.com/xy7365527-lang/NewChanlun/issues/1398)（TB-06 场景容量）、[#1350](https://github.com/xy7365527-lang/NewChanlun/issues/1350)（共享容量分配）、C11 清单（旧臂收编/退役）。

## 溯源标签

裁定一的「重间无次序」为 `[新缠论:推论]`（由 `040:20/:22` 经 ADR 0013/0014 推出）；其余为 `[新缠论:选择]`。
