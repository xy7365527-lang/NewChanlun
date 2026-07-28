# #570 typed ledger 撞键修复前置探针

**日期**：2026-07-28  
**工位**：`/private/tmp/kimi-nest-mainline`  
**分支 / HEAD**：`kimi-nest-mainline-20260717` / `558104a0ff`  
**数据源**：`/tmp/issue71-chi-gamma-validation-fix4`（BTC wf8，既有四臂产物）  
**边界**：只读生产代码与既有 dump；曾用一次性隔离 bin 重放因果分类塔，把 Gamma 候选映射到
carrier `ElementId`，输出留在 `/tmp/issue570-ledger-probe.out`，取证后已删除。未改生产路径，未做
git 写操作，未碰 `chanlun/agent-roster*` 及并行线文件。

## 结论

1. `trades.jsonl` 的 `write_trade` 域包含五类 typed 生命周期结算：反向关闭、静默剪枝、
   RiskExit、CloseOverlay、窗口终点 Hold。账户级 `trade_pnls_with_forced` 的虚拟强制兑现不调用
   `write_trade`；但仍留在 `open_trades` 的 typed 项会在末可交易 bar 逐项写 Hold。因此“未结算
   默认不写行”不是当前 typed/OPSEM 域定义。
2. 病型 2 五条全部找到后续同 carrier 开仓。原 `LedgerOpen` 均被后续
   `open_trades.insert(leg.id, ...)` 覆盖；覆盖链末端随后正常结算并写出了 trade。五条统一归档为：
   **真丢账（覆盖造成；链末端已结算）**，不是“未结算不写行”，也不是“覆盖链末端未结算”。
3. 纸面两案中，`(ElementId, dir)` 只能挡住异向覆盖，挡不住同 carrier 同方向重开；
   `(ElementId, generation)` 能挡住本批全部插入覆盖，且是现有 `PositionNodeId` 四元组的自然投影。
   建议后者，**不作裁定**。只换 HashMap key 不足：四类 remove、止损、TW、PanDiv、窗末排序都要
   同步迁移，并先裁定 carrier-only 关闭事件如何消费多个 generation。

## 1. `write_trade` 域与强制结算口径

### 1.1 唯一 writer 与调用点

`OpsemDump::write_trade` 定义于 `rust/src/theta_v0/backtest/opsem_dump.rs:235-240`。它只把已构造的
`TypedTrade + LedgerOpen` 外化成一行 JSONL；它不自行发现开仓或离场。生产调用点只有下列五类：

| 生命周期事件 | `fill.rs` 符号锚 | `exit_type` | 写行条件 |
|---|---|---|---|
| `step_trace.closed` 反向关闭 | `:1808-1833` | `reverse_exit_type(entry_v, trigger)` | `remove` 命中旧 `LedgerOpen` |
| `step_trace.silent_drops` 静默剪枝 | `:1837-1871` | Ambient→`CloseRoot`；否则 `CloseShortDiff` | `remove` 命中 |
| `step_trace.risk_exits` 强平清活动腿 | `:1874-1900` | `RiskExit` | `remove` 命中 |
| `step_trace.overlay_closes` P2 关闭重叠腿 | `:1903-1930` | `CloseShortDiff` | `remove` 命中 |
| 窗口终点 censored | `:2135-2165` | `Hold` | 末可交易 bar 存在；`open_trades.drain()` 逐项结算 |

共同前提是 OPSEM dump 已启用；否则 typed ledger 仍 push，但不产 `trades.jsonl`。五处都用
`let _ = dump.write_trade(...)` 忽略 I/O 返回值，这是独立的可观测性风险；本批不是该风险，因为同
carrier 的覆盖链末端均在同一文件正常写行。

### 1.2 `trade_pnls_with_forced` 与 typed Hold 是两条域

- `fill.rs:2080-2133` 的 `trade_pnls_with_forced` 是账户/声部执行层的窗口终点虚拟兑现：
  净额路径 push `forced_pnl`，声部执行路径调用 `settle_forced_virtual`。这段不调用
  `write_trade`，也不直接生成 typed 行。
- `fill.rs:2135-2165` 紧接着独立处理 typed 在飞表：找末可交易 bar，`drain` 全部
  `LedgerOpen`，构造 `ExitType::Hold`，调用 `write_trade`，再 push `typed_ledger`。
- 所以“账户强制 PnL 有一笔”不能证明 typed 行存在；反过来，**只要某 typed entry 仍在
  `open_trades` 且窗口有末可交易 bar，它就会写 Hold**。本批缺行必须发生在 `drain` 之前。

## 2. 病型 2 五条逐条命运

探针逐 bar 复用了生产同源的增量分类塔与“本 bar 新确认 BSP”切片，只在既有 Gamma 行出现的 bar
做 `(level, source_index, gamma_index) → carrier ElementId` 映射。随后用同臂
`trades.jsonl.voice_id` 对同 carrier 全量反查。

| 原 opened 裂口 | carrier | 后续同 carrier Gamma 事件 | 覆盖链末端 trade | 归档 |
|---|---|---|---|---|
| Arm0 bar3047 L1 Long src3017 | `L1#3` | 同 bar gamma2 又开 Short；bar3176 L1 Short src3063 再次 `opened=true` | entry3176→exit3176，Short，`CloseRoot` | **真丢账（覆盖；末端已结算）** |
| Arm0 bar3047 L1 Short src3017 | `L1#3` | bar3176 L1 Short src3063 `opened=true`；同 bar Long 候选 `opened=false` | entry3176→exit3176，Short，`CloseRoot` | **真丢账（覆盖；末端已结算）** |
| Arm0 bar78090 L0 Long src78063 | `L0#683`，父 `L1#166` | bar78101 L0 Long src78020 `opened=true`；同 bar另一 Long 候选 `opened=false` | entry78101→exit78101，Long，`CloseShortDiff` | **真丢账（覆盖；末端已结算）** |
| Arm0 bar209394 L0 Long src209363 | `L0#1817`，父 `L1#430` | bar209429 L0 Long src209276 `opened=true`；同 bar另一 Long 候选 `opened=false` | entry209429→exit209429，Long，`CloseShortDiff` | **真丢账（覆盖；末端已结算）** |
| Arm2 bar245306 L1 Long src245248 | `L1#509`，父 `L2#123` | bar245330 L1 Short src245302 `opened=true`；bar245409 同 carrier 两候选均未开 | entry245330→exit245330，Short，`CloseRoot` | **真丢账（覆盖；末端已结算）** |

### 2.1 代码路径追因

每个 `step_trace.opened` 都先获得独立 `generation`，并构造含
`(carrier, entry_certificate, side, generation)` 的 `PositionNodeId`
（`fill.rs:1680-1696`）。但 `open_trades` 的实际 key 仍只有 `ElementId`
（`:974-978`），随后 `insert(leg.id, LedgerOpen)`（`:1795-1806`）不检查返回的旧值、不结算旧值。

因此：

- bar3047 的插入顺序是 L1 Long 后 L1 Short：Short 立即覆盖 Long；
- bar3176 的同 carrier Short 再覆盖 bar3047 Short，随后当 bar remove，故文件只见 entry3176；
- bar78101、bar209429、bar245330 同理，分别覆盖三个较早 entry，再由当 bar 关闭路径写出链末端。

这也排除了“原 entry 仍在表中但窗末没写”的解释：若仍在表中，`:2141` 的 `drain` 必会取到；实际
同 carrier 行的 `entry_bar` 已换成后一次 open，直接证明旧 `LedgerOpen` 在此前被替换。

## 3. 两种键形的消费面

现状所有消费都建立在“一个 carrier 至多一个 `LedgerOpen`”上：

| 消费面 | 现状锚 | 改键后必须回答的问题 |
|---|---|---|
| 类型/构造 | `fill.rs:974-978` | key 的实例身份；是否另建 carrier→instances 索引 |
| 开仓 insert | `:1680-1696`, `:1795-1806` | 用 candidate `dir` 还是已分配的 `generation`；旧值不得静默丢弃 |
| ShortDiff/TW 投影 | `:1353-1362`, `:1933` 后的 `open_share_leg` | `shortdiff_leg_ids` 仍是 carrier 集还是实例集；open/close 计数必须对称 |
| PanDiv 父腿快照 | `:1389-1398` | 同 carrier 多实例时 `entry_v/units` 取谁，禁止任意 `get` 一条 |
| 风控止损 | `admission.rs:1379-1386`, `:1427-1449` | 必须枚举该 carrier 的实例，按实例 side/stop 聚合 long/short stop |
| 反向关闭 | `fill.rs:1808-1833` | carrier-only `ActiveLeg`/trigger 要关哪个实例，还是关该 carrier 全部实例 |
| 静默剪枝 | `:1837-1871` | 同上；每个被关实例各自产一行 |
| RiskExit | `:1874-1900` | 应清空 carrier 下全部实例 |
| CloseOverlay | `:1903-1930` | 只关 ShortDiff 实例还是 carrier 全部；TW 计数需对应 |
| 窗末 Hold | `:2135-2165` | drain 所有实例；确定序键须加入 side 或 generation |
| generation 高水位 | `:990-998`, `:1683-1687` | 仍按 carrier 维护即可；它已经提供稳定的实例序号 |

### 3.1 案 A：`(ElementId, dir)`

优点：

- 同 carrier 同时 Long/Short 不再互相覆盖；
- 止损侧可天然按方向拆开。

缺口：

- 同方向重开仍撞键；本探针已给出 `3047 Short→3176 Short`、
  `78090 Long→78101 Long`、`209394 Long→209429 Long` 三条实证；
- 不包含 entry certificate 与 generation，只是 `PositionNodeId` 四元组的不充分投影；
- `ActiveLeg` 关闭事件只带 carrier 与结构 `dir`，并不携开仓 candidate 的独立实例身份。同 carrier
  两个方向都在表中时，简单 `remove(&(leg.id, leg.dir))` 可能只结算一侧，另一侧变成孤儿 Hold；
- PanDiv、TW 与关闭扇出仍需实例策略，不能靠方向二元组自动解决。

### 3.2 案 B：`(ElementId, generation)`

优点：

- 每次 `step_trace.opened` 已在 insert 前分配单调 generation，异向、同向、同 bar、跨 bar 均不撞；
- 与现有 `PositionNodeId = (carrier, entry_certificate, side, generation)` 严格一致：
  `(carrier,generation)` 是在“generation 对 carrier 单调唯一”不变量下的无碰撞索引，完整四元组仍保存在
  `LedgerOpen` 内；
- 本批 12 条缺失从“存储能否保留每个 opened”这一层全部可救。

后遗症：

- 关闭事件仍只有 carrier，必须有 `carrier → [generation...]` 二级索引或按 carrier 扫描；
- 必须裁定关闭语义：同一结构 carrier 的一次关闭是结算全部 live generations，还是只结算某个
  generation；若新 open 语义上取代旧实例，则还需要一个可陈述的 supersede 结算类型/规则，现有
  五枚举没有 `Superseded`；
- 风控要逐实例读 frozen stop/side，TW 要逐实例做 open/close 对称计数，PanDiv 的 parent units
  需要明确聚合，不得继续任取一条；
- 窗末输出排序至少用 `(entry_bar, carrier.level, carrier.ordinal, generation)`。

## 4. 撞键 7 条的两案救回矩阵

下表只回答“能否阻止该条 entry 在 insert 时被覆盖”。任何“完整救回”仍以第 3 节关闭/TW/风控消费
同步迁移为前提。

| 缺失 entry | `(ElementId,dir)` | `(ElementId,generation)` | 说明 |
|---|---|---|---|
| bar2978 L1 Long src2922 | 可 | 可 | 与存活 Short 异向 |
| bar99685 L1 Long src99659 | 可 | 可 | 与存活 Short 异向 |
| bar149455 L3 Short src149353 | 可 | 可 | 与存活 Long 异向 |
| bar150372 L3 Short src150282 | 可 | 可 | 与存活 Long 异向 |
| bar152690 L3 Short src152640 | 可 | 可 | 与存活 Long 异向 |
| bar3047 L1 Long src3017 | 可 | 可 | 同 bar 与 Short 异向；后续没有 Long `opened=true` 覆盖证据 |
| bar3047 L1 Short src3017 | **不可完整救回** | 可 | 案 A 虽挡住同 bar Long，但仍被 bar3176 同 carrier Short 覆盖 |

作为旁证，病型 2 另外三条中：bar78090、bar209394 都是同方向重开，只能案 B 挡住；bar245306 是
Long→Short，案 A/B 都能挡住插入覆盖。故案 A 在本批五条病型 2 中只能保住 bar3047 Long 与
bar245306 Long，不能作为总修复。

## 5. 建议（不裁定）

建议选择 **案 B：`(ElementId, generation)`**；更彻底的类型安全形态可直接以
`PositionNodeId` 作主键，但按票体二选一，案 B 已具备同等无碰撞核心。配套建议：

1. 保留 `gen_hiwater: HashMap<ElementId,u32>`，在 insert 前分配 generation；
2. 主表按 `(carrier,generation)` 存实例，另维护确定序的 carrier→generation 集合，供
   carrier-only 生命周期事件消费；
3. 在实施前单独裁定“carrier 关闭时 drain 全部 live generation”与“新开即 supersede 旧 generation”
   二者。前者无需新增 exit enum，但声明多个 opened 确实同存续；后者更接近单 active carrier，却必须
   新增诚实的 supersede 结算语义，不能借 `CloseRoot/Hold` 冒充；
4. 裁定后一次性迁移四类 remove、风险 stop、TW 计数、PanDiv 父快照、窗末 drain/sort，并增加：
   同 bar 异向、跨 bar 同向重开、跨 bar 异向重开、RiskExit 全清、窗末多 generation Hold 五组守卫；
5. 不建议把 `(ElementId,dir)` 当终局修复：它对本次已经实证的同方向重开仍会静默丢账。

本报告只给修复前置事实与方案比较，不裁定多 generation 的生命周期语义，不改生产代码。
