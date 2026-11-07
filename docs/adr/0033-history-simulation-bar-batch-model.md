# ADR 0033：完整历史模拟执行模型——已知前沿分段的 bar 批模型

**日期**：2026-10-13
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核采纳并补充同核口径），走 [[grilling] 完整历史模拟执行模型 #1353](https://github.com/xy7365527-lang/NewChanlun/issues/1353)（RD-17 / C10.3，SPEC #1340）
**状态**：已采纳。本 ADR 只定模型组织、适用域与版本合同字段；实现、取证与 H1–H8 oracle 归 TB-07 叶票（#1381 等），不以本 ADR 关票声称实现完成。
**继承**：[ADR 0018](0018-limit-order-fill-model.md) 五条、[ADR 0019](0019-limit-order-state-flow-interface.md) 全部，原样有效；本 ADR 只补多重并发维度。

## 裁定

### 一、模型 ＝ 已知前沿分段的 bar 批（第一问 A）

模型 id `history-sim/bar-batch-known-frontier/1`。第 t 根 bar 适用集合 `S_{t−1}`（t−1 收盘时已输出并经 RD-11 准入）；bar t 的外效构成批 `B_t`；批内各重订单各自独立对 OHLC 判定（严格穿越、开盘取优），互不排序、互不占用对方资金，`intra_batch_order: Unknown`；bar t 收盘才知道的新挂/撤/改从 t+1 起作用。市价动作在 `exec_index` bar 收盘价的具名槽成交；限价判定作用于全区间，逻辑上先于市价槽（收盘是 bar 最后价格的定义，不是推路径）。输入域外的请求 ⟹ `ModelOrderUnspecified`/`Unsupported`，不回落旧遍历。v1 确定性，`seed: none`。

**教义依据**：`040-第40课.md:20`【正文】「每一重都对应着一定的资金与筹码」、`040-第40课.md:22`【正文】「每一层次的操作都是独立又在一个整体的操作中」；经 [ADR 0013](0013-partition-machine-bidirectional-form.md) 裁定一、[ADR 0014](0014-intra-chong-unidirectional-inter-chong-no-arbitration.md) 裁定二落地 ⟹ ADR 0018 裁定三的排序理由（`rust/src/theta_v0/backtest/fill.rs:115` 的现金约束）在重之间不存在；C05.2 重内一次跃迁只一个动作。ADR 0018 裁定四「撤单不得用预算到期 bar」（锚 `049-第49课.md:62`【正文】「其后，中枢已经完成就无所谓中枢震荡了」）在多重上的形式就是前沿规则。

> 2026-10-26 用户批准 #1598：本节将合法选择落为具名版本字段；未签数值与未标定能力不因方法批准自动生效。

`model_id` 与 `seed`。`model_id` 可选值 = {`history-sim/bar-batch-known-frontier/1`（默认，确定性模型，seed: none；出处：ADR 0033 裁定一，批内各重订单各自独立对 OHLC 判定、互不排序、`intra_batch_order: Unknown`），`history-sim/stochastic-microevent/<rev>`（待标定，前置未满足）}。`stochastic-microevent` 标「待标定」理由：bar 内微事件概率分布与延迟需逐笔成交数据或盘口深度（#917 查实本仓三条数据通道全是聚合 Bar，无逐笔/盘口，与 ADR 0018 裁定一的 k 标定缺口同理），选中时容量记 Unknown ⟹ AwaitingPolicy 不伪造默认值。代价：bar 内路径是造出来的（实盘 bar 内顺序不可知，ADR 0018 裁定三），报告须单列随机模拟计数并标 L1 假设（#1418 报告分层规则），不得作为真实路径证据。`seed` 字段：v1 确定性模型取 `none`，随机模型版本取整数（非负 i64）。ADR 0038:29 双跑五元组固定 seed：同 (input bar 序列, policy 版本, model_id/revision, seed, launcher scheduler) 五元组逐位复现（#1411 双跑验收规则消费本字段）。前置：#917 数据接入查实逐笔/盘口数据源可得性（当前不可得）。

### 二、部分成交只来自具证容量合同（第二问 A）

`capacity_source ∈ {scenario_contract, bar_volume_derived}`，默认 `scenario_contract`；在默认 scenario_contract 下，原合同 `none/input(ref)/scenario(ref)` 的证据形态保留，语义如下（#1598）；新字段不反写旧合同版本：

- `none`：在 [ADR 0016](0016-chong-quantity-layer-capacity-frequency-shift.md) 容量门已准入的规模内，继承「穿越即全量成交」，显式标为 L0 假设。这类成交可以进入 TB-07 全史双跑，但报告须单列 L0 计数，不得作为容量证据（根补充）。
- `input/scenario`：批内同向可成量 q 按 [ADR 0032](0032-fill-allocation-policy.md) 裁定一/二的冻结比例分到各重，剩余量按 P7 进 E。

默认 scenario_contract 来自 #1353 resolution 第二问，场景合同声明或输入原件是其证据来源。`bar_volume_derived` 标「待标定」理由：缺从 bar.volume 推导可成交量的公式（需标定市场微观结构参数，当前无），选中时容量记 Unknown ⟹ AwaitingPolicy。前置：ADR 0016 裁定三容量门已准入（当前 L0 已准入）。代价：报告须单列 `capacity_source=bar_volume_derived` 的成交计数并标 L0 假设（#1418 报告分层规则），不得作为容量证据（ADR 0033:20 根补充「L0 单列要求」）。无容量证据时在已准入规模内继承「穿越即全量成交」（ADR 0033:20），显式标为 L0 假设、不伪造容量数值。批内同向可成量 q 按 ADR 0032 裁定一/二（#1350 投影清单）的冻结比例分到各重，剩余量按 P7 进 E。

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

## #1598 批复与受影响清单

本次修订沿用的 ADR 旧行号以获批草稿固定基线 `13ed4d45c8ff85175819ca987690328ab2a247a4` 为准；当前条款按标题定位。

用户 2026-10-26 批复 [resolution](https://github.com/xy7365527-lang/NewChanlun/issues/1598#issuecomment-6403277308)。本次字段修订为 `[新缠论:选择]`，ROUNDTRIP 沿用已裁推论。受影响文件、固定基线与候选承接见 [清单](../../.chanlun/review-results/issue1598-policy-20261026/affected-code.md)；政策批准不代表下游已接单或实现。


## #1381 实施发现的选择补充：容量 epoch 的累计与已知前沿交接（2026-11-07）

`[新缠论:选择]`。根采纳的 R3 已经非作者政策独审通过，见 [#1381 裁定](https://github.com/xy7365527-lang/NewChanlun/issues/1381#issuecomment-6699969429)。实施及运行验收尚未完成；政策 PASS 不代替工程独审或关票证据。下文补足本 ADR 的具证容量边界，不修改 ADR0032 的 E 外效投影合同。

## 对象和累计

capacity_manifest是历史模型具证容量分配清单；projection_manifest是E命令投影/外效归属清单，二者不是同一实体，不互换身份或累计。容量层复用ADR0032配额函数、并列规则和必要字段验证原语，不直接用tb03c_allocation外效清单对象冒充容量清单。

容量清单绑定capacity_epoch、标的/方向/限价或市价槽、完整可成参与者的稳定family身份与冻结requested_q、tie order、内容身份、产生它的bar事实与已准入命令前沿。每个容量输出另绑定实际获准command/family/revision及其projection_manifest；容量身份不授执行权，不替换E/S核验。

C是本容量epoch已分配累计；f_i是该epoch分到腿i的累计；F_family是原family实际累计。更新用A(C_new)-A(C_old)，不得每bar按余量重新归一。额度L-F_family按命令责任合同核，不拿新epoch的零当family零。历史更正保持原分量和原政策版本，不能自动增加新发送额度。

## 形成和切换

1. bar t只消费t-1已知且获准的命令状态；限价按当根完整OHLC独立判严格穿越，市价只在具名exec_index槽。分别形成各标的/方向/槽的完整可成集合，不按遍历次序分配。
2. 可成集合、获准额度约束和容量合同语义未变时，继续同epoch冻结权重与累计C。单纯bar编号、capacity ref或容量数值变化不授重置资格；正常成交减少余量本身也不触发重置。单个成员因正常成交达到零剩余额度时，必须保留在原冻结向量中，累计分配不再给其新增份额。零剩余额度成员不参与后续价格/槽资格变化的边界比较，其完成、退出可成活动集或后续OHLC变化均不单独触发交接。资格交接比较只考虑合法剩余额度为正的成员；价格/槽资格同时包括限价严格穿越和市价exec_index槽资格。发生其他真实交接时，新清单仅纳入正剩余额度且符合该槽资格的成员；所有成员完成时关闭epoch。
3. 获准新增/撤销/额度变化仅在其合法下一bar生效。按第2条排除零剩余额度成员后，完整bar价格/槽事实使可成成员改变时，也形成容量边界。纯改价若未改变可成集合/获准额度则只更新命令revision映射，不重置权重；若改变可成集合按本条交接。
4. 真实边界将旧epoch的C/f_i及其不可变原件前沿封存，新非空集合以每个family当时合法剩余额度L-F_family冻结requested_q，新成员取其获准量；新epoch C=0。旧epoch尚未满足的份额不成为新成员排队的前件。全部取消/完成或该槽无人可成时关闭epoch，不要求空successor；以后重新可成按合法剩余额度形成新epoch，不重复取得历史已成交份额。
5. 切换记录绑定旧epoch及累计证据（或首次/空态）、完整bar事实、完整已知命令集合及变更结束依据、新清单内容与实际命令映射、生效bar/槽。身份稳定、同身份异内容拒绝，重复消费幂等。不能以当前已见子集替代完整集合。旧bar原件先按旧epoch结算；收盘获知的新命令不追改本bar价格判定。由本barOHLC导致的资格交接发生在本bar容量分配前，不提前授予收盘新命令资格。
6. 切换、容量分配累计与该批原件同一X持久边界原子记录，恢复只见旧状态或完整新状态。同close消息到达序置换不能改变结果。实际authority来源按E/S既有合同，fixture只证明本片协议。

## 外效归属不随容量交接漂移

per_leg原件按实际来源command/family/revision归属；netted原件仍按原E冻结projection_manifest分配。容量集合新增C不能把A/B旧物理命令迟到回报改挂C，不能创建新物理命令。原fill/fee/bust引用原容量证据、原投影及原政策版本，因果更正追加不删原件。外效投影自身的替换仍归E合同。

## 决定性验收

- 无集合变化A1/B2两bar各容量1，累计(0,2)。
- A1/B2首bar给B；次bar仅A穿越，合法A取得容量；B再可成只能取得合法剩余额度。
- 显式新增C后新冻结为A1/B1/C1；撤B后A合法剩余可继续；末单撤销和全不穿越可关闭，无空投影要求。
- 冻结(1,2,3)前四份为C/B/C/B，B完成后下一份仍给C；只改变已完成B的价格/槽资格也不重置或改变结果。
- 纯bar/ref变化不重置；纯改价无资格变化不重置；到达序置换同结果。
- 缺失/伪造旧累计、半集合、身份内容冲突拒绝；切换前后崩溃恢复/重复消费不双应用。
- 跨epoch迟到fee/bust和旧netted回报维持原归属；不把更正当新增发令许可。

射程仅补历史模型input/scenario容量边界。不修改共同资金政策、E外效投影合同或paper/live权限，不以本候选关闭1381/1353/父TB。

### 方向容量与槽顺序

按本 ADR 裁定一的既定先后，限价全区间槽先消费同一方向的总容量，exec_index 收盘市价槽只消费剩余容量；不能每槽复制一份方向额度。同槽各重仍按冻结配额函数，不增加遍历优先级。买卖方向预算独立；none 仍按已准入规模的原全量合同。

### 实际受影响代码

- `rust/src/history_simulation/capacity.rs`：独立容量清单、C/f_i、交接和完整集合核验。
- `rust/src/history_simulation/model.rs`：family累计、槽预算、close变化与实际命令/投影映射。
- `rust/src/history_simulation/store.rs`：容量证据与原件/批进度同事务、旧fee/bust归属。
- `rust/src/history_simulation/runtime.rs`：完整获准命令核验、规范close批、QueryBatch证据入口。
- `rust/src/economic_session/mod.rs`：只转接现有配额累计差量函数，不复制算法或外效清单。
- `rust/src/history_simulation/tests.rs`、`rust/tests/history_simulation_entry.{rs,py}`：跨bar、空态、改撤/新增、零余量、恢复和真实X入口。
- `rust/src/history_simulation/README.md`：模拟wire的容量与投影身份边界。

Lean 定义与 Python 业务模型不在本次变更面；Python文件仅是受控进程验证夹具。E projection_manifest、其净额分配与旧回报归属继续由原合同拥有。
