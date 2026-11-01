# ADR 0057：P0 场所 profile v2 四字段签署（tick_size / slippage_bps / tax_bps / entry_delay_bars）

> 正本：本文件是 P0 场所 profile v2 中 `tick_size`、`slippage_bps`、`tax_bps`、`entry_delay_bars` 四字段的签署状态、来源与生效日期的正本。
> 最后裁定：#1774（2026-10-28，用户批 P 类）
> 受影响代码清单：见本文「受影响代码清单」节（实施归 #1640 owner 消费；本 ADR 不改代码）

**状态**：P 类裁定，用户已批，文书待 main 合入。本 ADR 只签字段状态与来源；`venue-profile-P0.json` v2 落盘、解析器与测试锁更新归下游实施，不以本 ADR 声称实现完成。Part of #1323。

**裁定依据**：用户 2026-10-28 批复原话「1774 照建议批，entry_delay 做成可调」，票面记录 [#1774 评论 6472081788](https://github.com/xy7365527-lang/NewChanlun/issues/1774#issuecomment-6472081788)。研究原件：`/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1775-P0-VENUE-DATA-RESEARCH.md`（下称 R29A）与 `1441-P0-PROFILE-RESEARCH28M.md`（下称 R28M），外部原件 `1775-receipts/`。

**行号基线**：main `3087582e45`（#1791 合入）。

## 背景

#1640 生产臂（A1 S86、A2、A4、双进程、重启、G0、C4 production-4）的真实前置是 P0 场所 profile 出 v2（R28M 结论 1）。现行 v1（`rust/src/theta_v0/policy_records/venue-profile-P0.json`，sha256 `cc2f6a37…6c`）只签了 `commission_bps = 10`（ADR 0044 P0 费率），其余四项均为 `"AwaitingPolicy"`。解析器缺 `tick_size` 记 `venue_profile` 槽，缺任一成本项记 `cost_inputs` 槽（`rust/src/theta_v0/run_records.rs:233-248`），非 test 构建 S 写者 open 恒取 P0（`rust/src/bin/s_session_v3/driver.rs:331`；同口径 `rust/src/bin/s_structure_session.rs:3316`）。

考古：#1602 明文把数值签署排除在外；#1559／ADR 0044 签的是成本停止 policy 四项输入，不含 `tick_size`、`tax_bps`、`entry_delay_bars`；#1441 是成本停止 policy 的读取方，不是测量票（R28M 结论 2、3）。tracker 与仓内推论层双路检索无其他 owner（R29A §5）。

研究查实三件事决定了裁定形状：

1. 仓内「P0 回放数据」有两条互不相同的来源（R29A §1）：回测/成本口径用 Binance 现货 BTCUSDT 1m 公共归档（`scripts/download_btc_binance.py:3`），浮点价在加载时经 `quantize`（`rust/src/theta_v0/types.rs:24-26`；`rust/src/theta_v0/backtest/data.rs:256`、`:277-280`）量化；#1640 S 写者口径输入已是整数 tick，profile `ohlc_integer_tb02a_v1`（`s_session/profiles/ohlc_integer_tb02a_v1.json:6` `tick_size: "1"`、`:11` `not_product_default: true`；`rust/src/bin/s_session_v2/tb02a.rs:6`），S86 源是合成序列。两者价位尺度无换算关系。
2. 回放撮合 `slippage_bps` 是 `ExecConfig` 的市价撮合常率（`rust/src/theta_v0/backtest/fill.rs:48-49`；`rust/src/theta_v0/venue_fee.rs:320`），成本停止的 slippage 是带 `Known|ProxyLowerBound|Missing` 状态的证据输入（ADR 0035:14、:19）。唯一把二者接在一起的是 v1 json 的 `slippage_bps.source` 说明文字，没有裁定明文（R29A §3）。
3. `entry_delay_bars` 唯一取值 1 来自设计默认（`docs/reference-theta-v0.md:52`「[设计选择,默认值]」），只在诊断夹具（`rust/src/theta_v0/policy_records/venue-profile-diagnostic-fixture.json:31-34`）；Binance 不发布此类量。

## 裁定

### 一、签署归属

本票（#1774）是 P0 场所 profile v2 四字段 `tick_size`、`slippage_bps`、`tax_bps`、`entry_delay_bars` 的签署归属。`commission_bps` 维持 ADR 0044 P0 已签值（10 bp/side），本 ADR 不动。

### 二、回放撮合滑点与成本停止滑点是两个量

venue profile 的 `slippage_bps`（回放撮合执行假设）与 ADR 0035／0044 成本停止 policy 的 `slippage` 输入（成本门分母的证据项）分开管理，不共用同一个值，也不互为来源。推论：

- 回放 `slippage_bps` 将来签值，不使成本门的 `slippage` 升为 `Known`；成本门继续按 ADR 0044:26、:56 为 `Missing` ⟹ `NotEvaluable`（ADR 0035:19）。
- 成本门 slippage 将来升级（TB-08／#1367 或其他测量），也不自动改写回放 `slippage_bps`。
- v1 json `slippage_bps.source` 中「ADR 0044 P0 slippage: Missing……ADR 0035:27」那段出处在 v2 改写为指向本 ADR；ADR 0035 被否方案现行行号为 `:44`（旧引 `:27` 已漂）。

标签 `[新缠论:选择]`。代价：回放读数将来会带一个与成本门证据不同源的撮合假设，输出须标注其来源，不得被读作真实交易误差。

### 三、tick_size 拆成两份带版本的 profile，不用一个值同时代表两边

| profile id | `tick_size` | 状态 | 来源与射程 |
|---|---|---|---|
| `replay_binance_spot_1m` | `0.01` | Signed（仅证快照日） | Binance 现货 `api.binance.com/api/v3/exchangeInfo?symbol=BTCUSDT` PRICE_FILTER `tickSize "0.01000000"`。收据见下节 R1/R2。只证明 2026-10-27 快照当日的场所规则，不证明 2017–2026 全历史 tick 恒为 0.01（R29A U-B；2018-12-04 有一根 0.001 分辨率 bar，R29A D4） |
| `production_synthetic_ohlc_integer_tb02a_v1` | `1` | Signed（输入约定，`not_product_default`） | `s_session/profiles/ohlc_integer_tb02a_v1.json:4`、`:6`、`:11`（`unit: tick`、`tick_size: "1"`、`not_product_default: true`），sha256 `48225025…d6`。它是 #1640 S 写者整数 tick 输入的量化约定，不是任何场所事实，不得被读作产品默认 |

裁定要点：

- 两份 profile 各自独立版本化，任一改值即该份 +1 版本（ADR 0041:23；`CONTEXT.md:388`）。
- 禁止把 `0.01` 签给 S 写者生产臂，也禁止把 `1` 当作 Binance 场所 tick。两份之间不设换算、不设回退：一次运行只绑定其中一份（ADR 0041 判别句，:13）。
- `production_synthetic_ohlc_integer_tb02a_v1` 继承 `not_product_default`：凭它跑出的生产臂读数只证 synthetic 整数输入路径，不证 Binance 现货价位尺度，也不构成真实场所准入证据。
- 「P0 数据 profile 指哪条数据」的长期归属仍在 #1536（R29A §5）；本 ADR 只在 venue profile 层把两条拆开，不裁 #1536 的数据集选择。

### 四、slippage_bps、tax_bps 记 AwaitingPolicy，不填默认值

| 字段 | 状态 | 缺的来源 | 退场条件（满足即出新版本签值） |
|---|---|---|---|
| `slippage_bps` | `AwaitingPolicy`（事实层 NotEvaluable） | 仓内无 P0 逐笔滑点观测（ADR 0044:26；`.chanlun/review-results/issue907-cost-gate-level-20260805.md:282`）；1m OHLCV 测不出盘口滑点（R29A §3） | 有带日期与哈希的实测来源（逐笔成交/盘口数据测得的滑点分布），或用户按 `[新缠论:选择]` 单独签一个回放撮合假设值并写明代价。二者任一成立前保持 `AwaitingPolicy` |
| `tax_bps` | `AwaitingPolicy`（事实层 NotEvaluable） | 无可回查的场所原件给出现货交易税；税务通常取决于账户主体所在辖区（R29A §2、U-C） | 有账户所在辖区的税务权威来源，或 Binance 官方明文页。另须遵守既有结构约束：有 `fee_schedule` 时 `tax_bps` 须为 0（`rust/src/theta_v0/config.rs:487`、`:498`，强制处 `backtest::treasury::fee_quoter`）——该约束是结构理由，不是签署来源，不得据此代填 0 |

两项都不得用诊断夹具值（slippage 2.0、tax 0.0）或未标定常数凑数（ADR 0035:44 被否「缺项代 0」「代理下界充 Known」的同向纪律；ADR 0041:21 签署门默认值 = `AwaitingPolicy`）。

### 五、entry_delay_bars 是模型参数，版本化可配置，本 ADR 不签具体值

`entry_delay_bars` 是撮合模型选择，不是场所事实（R29A §2）。按用户「有自由度的选择要可调可选，不写死」与 ADR 0041 通则：

- **配置载体**：所选 venue profile 记录（上节两份之一）里的 `fields.entry_delay_bars`，按 ADR 0041:19-23 四件写全——版本化记录字段、默认值、出处（可选值集合 = 非负整数根数，各值来源与代价）、改签即新版本。不另建 CLI 或环境变量配置面（ADR 0041:19）。研究工具就地改值时照现行机制标 `research-override`，不冒充签署版本（`rust/src/theta_v0/config.rs:581`）。
- **缺省行为**：默认值 = `AwaitingPolicy`（ADR 0041:21）。字段缺失、为 `null` 或 `"AwaitingPolicy"` ⟹ 解析记 `cost_inputs` 槽并显式拒绝，调用方停止、不产读数；**不得静默取 1**，也不得回退诊断夹具值。
- **夹具值 1 不作证据**：`docs/reference-theta-v0.md:52` 的「延迟 1 根」与诊断夹具 `:32` 的 `1` 只是设计默认；#907 DELAYERR 是在 `entry_delay_bars=1` 假设下测的误差下界，不能反过来证明延迟取 1（`.chanlun/review-results/issue907-cost-gate-level-20260805.md:280`）。
- **取值由谁签**：哪一份 profile 要跑，就由该 profile 的使用方提请用户按 `[新缠论:选择]` 签一个值进该份新版本；签值附代价说明（延迟越小越乐观）。本 ADR 不预签。
- 不同取值是同一判据的不同参数值，不是 #799 所禁的宽严两档；但不得出现「主值失败换另一个延迟重跑」的兜底（ADR 0041:15）。

### 六、来源与生效日期回执

| # | 签署值 | 原件 | sha256 | 取得/生效 |
|---|---|---|---|---|
| R1 | `replay_binance_spot_1m.tick_size = 0.01` | `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1775-receipts/binance_spot_exchangeInfo_BTCUSDT.json`（PRICE_FILTER `minPrice 0.01000000`、`tickSize 0.01000000`；serverTime `1793139712758`） | `f1929b007e6978e7d469bbc262705118433a2be3734a634323579d22d8507b54` | 抓取 2026-10-27T22:21:52Z；生效 2026-10-28（本裁定日）；射程 = 快照当日 |
| R2 | R1 抓取时刻 | 同目录 `retrieved_at.txt`（内容 `2026-10-27T22:21:52Z`） | `73ab034c4e96a2163cd55159d98458c3c39fc69a74ff5a30390ace0ebe9ce378` | 同上 |
| R3 | `production_synthetic_ohlc_integer_tb02a_v1.tick_size = 1` | `s_session/profiles/ohlc_integer_tb02a_v1.json`（main `3087582e45`） | `482250254133ba120afdd5cd428d1a32d4e5cb40cd7876c556b7007320e599d6` | 生效 2026-10-28；射程 = `s-ohlc/1` 整数输入，`not_product_default` |

外部收据在 `/Volumes/AgentStorage`，不入仓；落 v2 json 时须把 R1–R3 的 sha256 与时刻写入各字段 `source`，以便离线核对。有效期对齐已签正本惯例：签署日 + 365 天（ADR 0044:64 同批口径），即至 2027-10-28；到期或 Binance 规则变更即重抓、出新版本。该有效期是签署记录的有效期，不是行情适用日期区间，不延长 R1 仅证 2026-10-27 快照日的事实射程。

## 被否方案

- **一个 `tick_size` 同时代表回测与生产臂**：两条数据价位尺度无换算关系，签 0.01 给 S 写者只是放行开关、对价格无物理意义（R29A §1 判读）。
- **回放 slippage 与成本门 slippage 共用一值**：二者在代码与正本中是两个量；若等同，P0 生产臂将无期限等待一份 P0 数据测不出的滑点（R28M U1）。
- **`tax_bps` 据 `fee_schedule` 互斥约束代填 0**：约束是防重复计的结构条件，不是税务来源。
- **`entry_delay_bars` 签死 1 或缺省静默取 1**：违用户「可调不写死」原则与 ADR 0041:21。

## 受影响代码清单

供 #1640 owner 消费（行号按 main `3087582e45`，实施时重定位）：

| 位置 | 现状 | 本裁定要求 |
|---|---|---|
| `rust/src/theta_v0/policy_records/venue-profile-P0.json`（全文） | v1，四项 `AwaitingPolicy`，`slippage_bps.source` 引 ADR 0044/0035 | 按裁定三拆为两份带版本 profile 记录（或等价的 profile 选择），`tick_size` 分签 0.01／1；`slippage_bps`、`tax_bps`、`entry_delay_bars` 保持 `AwaitingPolicy`；source 指本 ADR 与 R1–R3 |
| `rust/src/theta_v0/run_records.rs:38`、`:48-49` | `PRODUCTION_VENUE_PROFILES` 只有 P0 一行 | 容纳两份 profile 且一次运行只绑定一份 |
| `rust/src/theta_v0/run_records.rs:233-248` | 缺 tick ⟹ `venue_profile`；缺成本项（含 `entry_delay_bars`）⟹ `cost_inputs` | 保持显式拒绝；不得给 `entry_delay_bars` 加缺省 1 |
| `rust/src/theta_v0/run_records.rs:516-534` | 测试锁：P0 必报四字段 `AwaitingPolicy` | 随 v2 更新：`tick_size` 不再缺，其余三项仍报 |
| `rust/src/theta_v0/config.rs:452-454`、`:473`、`:483`、`:491` | `ExecConfig` 四字段只从场所 profile 读 | 不变；注释中「成本门 slippage」与回放 slippage 的口径按裁定二区分 |
| `rust/src/theta_v0/config.rs:487`、`:498` | `tax_bps` 与 `fee_schedule` 互斥约束声明 | 不变；不作 `tax_bps` 签值来源 |
| `rust/src/theta_v0/config.rs:581` | `research-override` 标记机制 | `entry_delay_bars` 研究改值沿用 |
| `rust/src/theta_v0/config.rs:803-807` | `venue_same` 逐字段比对 | 两份 profile 下仍逐字段比对 |
| `rust/src/theta_v0/config.rs:847-853` | `admissible()` 要求全部 Signed | 不放宽；v2 三项仍 `AwaitingPolicy` 时 open 仍失败 |
| `rust/src/theta_v0/config.rs:965-980`、`:986-1009` | 测试锁：P0 必 `AwaitingPolicy`；签署样例（tick 0.01／slip 4／tax 0.5／delay 2）可读回 | 前者随 v2 更新；后者只是读回样例，不作签值依据 |
| `rust/src/bin/s_session_v3/driver.rs:321-332`；`rust/src/bin/s_structure_session.rs:3316` | 生产 open 恒取 `for_production_profile("P0")` | 改为绑定 `production_synthetic_ohlc_integer_tb02a_v1`（或其 P0 别名）；不绕过 launcher |
| `rust/src/bin/s_session_v3/driver.rs:533-536` | `require_admissible` 准入门 | 不放宽 |
| `rust/src/theta_v0/backtest/data.rs:256`、`:277-280`；`rust/src/theta_v0/types.rs:24-26` | 回测加载按 `tick_size` 量化 | Binance 回测的 venue profile 身份归 `replay_binance_spot_1m`，不与 synthetic profile 混用；本 ADR 仅签快照日的 `tick_size` 事实，不裁历史数据区间的量化规则，不据此把 0.01 统一用于整段历史。历史数据与模型 profile 的选择仍由 #1536 承接。 |
| `rust/src/theta_v0/backtest/fill.rs:48-49`；`rust/src/theta_v0/venue_fee.rs:320` | 回放费率 = commission + slippage + tax | 回放 `slippage_bps` 与成本门 slippage 分开，不互相取值 |
| `rust/src/theta_v0/strategy/exec.rs:43` | 按 `entry_delay_bars` 定成交 bar | 值只来自所绑定 profile 的签署版本 |
| `rust/src/theta_v0/policy_records/venue-profile-diagnostic-fixture.json:31-34` | 夹具 `entry_delay_bars = 1` | 只留诊断，不进生产、不作证据 |

## 后果照实

- v2 落盘后 `tick_size` 不再缺，但 `slippage_bps`、`tax_bps`、`entry_delay_bars` 三项仍为 `AwaitingPolicy` ⟹ 生产入口仍报 `AwaitingPolicy{cost_inputs}`，`admissible()` 仍为假。**#1640 生产臂要跑通，还须由用户为所选 profile 签 `entry_delay_bars`，并为 `slippage_bps`、`tax_bps` 满足裁定四的退场条件（或用户按 `[新缠论:选择]` 另签）。** 本 ADR 不替用户做这一步。
- 成本门不受影响：ADR 0044 P0 slippage 仍 `Missing`，成本门 `NotEvaluable`，L0 照常。

## 授权边界

本 ADR 不授权：真实资金或账户动作、放宽 `require_admissible` 或任何隔离、诊断夹具进入生产、绕过 `production_session` launcher、改写 ADR 0035／0044 现文、合入 main 或关票。#1640 的独立评审、exact-head CI 与 main 合入仍是各自独立的门。

## 溯源标签

- 裁定一、三的拆分、五：`[新缠论:选择]`（用户 2026-10-28 批）。
- 裁定二：`[新缠论:选择]`，依据 R29A §3 的代码与正本事实。
- 裁定三 R1 的 0.01：场所事实，射程限快照日；R3 的 1：本仓输入约定，非场所事实。

## 追加修订：synthetic profile 三项成本签值与使用域（#1774，2026-10-31，用户批 P 类）

> 本节只追加，不改写上文。上文「后果照实」所述「三项仍 `AwaitingPolicy`」自本节起只对 `replay_binance_spot_1m` 成立；synthetic profile 按本节另签。

**裁定依据**：用户 2026-10-31 批复「都批准」（[#1774 评论 6542854318](https://github.com/xy7365527-lang/NewChanlun/issues/1774#issuecomment-6542854318)），采用草稿 `/Volumes/AgentStorage/issue1774-synthetic-policy-29m/draft.md` 与增量独审 `/Volumes/AgentStorage/review1774-synthetic29n/review.md`（PASS）的推荐组合。走上文「后果照实」的「用户按 `[新缠论:选择]` 另签」路径，只针对 synthetic profile。

### 七、synthetic profile 版本 +1：三项成本签为具名模型假设

`production_synthetic_ohlc_integer_tb02a_v1`（记录 `venue-profile-synthetic-tb02a`，version 2）：

| 字段 | 值 | 标签 | 代价 |
|---|---|---|---|
| `tick_size` | 1 | 已签（裁定三 R3） | 不变 |
| `commission_bps` | 10 | 继承（裁定一：维持 ADR 0044 P0 已签值，本记录由 P0 v1 拆出） | 本次未另签；若要为 synthetic 单独签值，须新裁定 |
| `entry_delay_bars` | 0 根 | `[新缠论:选择]` 模型假设 | 最乐观：从信号 bar 起找首根可交易 bar（`strategy/exec.rs` `fill_bar_index`）；不得读作真实延迟；可改签任一非负整数即新版本 |
| `slippage_bps` | 0 bp/side | `[新缠论:选择]` 模型假设 | 最乐观：不模拟盘口摩擦；不使成本门 slippage 升 Known（裁定二）；可改签任一 ≥0 有限值即新版本 |
| `tax_bps` | 0 bp/side | `[新缠论:选择]` 模型假设 | 「本模型不计税」，不是税务事实（synthetic 输入无账户主体与辖区）；可改签任一 ≥0 有限值即新版本 |

三项数值不进入 S 结构计算（S 写者只把 `exec` 装入 runtime，结构计算不读）；签署改变的是准入、policy record 元数据与 `venue_same` 逐字段核对。零值不以「让测试变绿」为理由。一次运行只绑一个已签版本，就地改值照 `research-override` 机制失去准入。

### 八、使用域 B1+B2：边界放在选择层

synthetic 记录头 `symbol: null`、`usage: "s_structure_writer"`、继承 `not_product_default`：

- **B1**：symbol 路（`production_venue_for_symbol`；`theta_backtest`／`theta_replay`／`theta_accept`）只匹配非空 symbol，选不到 synthetic。
- **B2**：通用 profile 路（`production_venue_for_profile`；`ThetaConfig::for_production_profile`、Python `ThetaStream(venue_profile=…)`）遇 `usage: s_structure_writer` 返回 `InvalidPolicy{venue_profile}`，原因文本写明 usage 不符，不产配置。新增 S 专用选择器 `production_venue_for_s_writer` 与构造 `ThetaConfig::for_s_structure_writer`，只供 S 生产入口（`s_session_v3/driver.rs`、`s_structure_session.rs` `advance_core`）。对称地，S 专用入口不接 `usage` 缺省的记录（不得绑 replay）。
- **P0 别名归 replay**：`venue-profile-P0`（version 2，`tick_size = 0.01` 按裁定三 R1 签）即 `replay_binance_spot_1m`，`profile_id` 保持 `P0`。`for_symbol("BTC")`、`for_profile("P0")`、Python `venue_profile="P0"` 的调用方式与所选身份不变，仍因三项 `AwaitingPolicy` 拒启；缺口由 `venue_profile, cost_inputs` 收敛为 `cost_inputs`（tick 已签）。无 profile、未知 profile、生产表无记录的品种仍报两槽。
- **无 `usage` 头的旧已签记录**经同一选择函数按 symbol 或 profile 都可选中、原值读回、admissible；`usage` 不是旧记录的新前提。

**不覆盖面**：直接手写 `ExecConfig`、诊断夹具、测试代码或直接读 `PRODUCTION_VENUE_PROFILES` 文本表再自行解析，都不经此合同；不为此追加全库封装。

### 不变项（本节不改）

`require_admissible` 不放宽；replay 三项保持 `AwaitingPolicy`；未签拒启语义保留（synthetic v1 夹具 `rust/tests/fixtures/venue-profile-synthetic-tb02a-unsigned-v1.json` 经正式绑定步仍报 `cost_inputs`）；诊断开关负控保留；原八项生产验收（#1769）不因本节折算完成；真实资金、隔离放宽、launcher 绕过、合入与关票均不在本节授权内。

### 受影响代码清单（本节；行号按实施 PR head）

| 位置 | 要求 |
|---|---|
| `rust/src/theta_v0/policy_records/venue-profile-P0.json` | version 2：`tick_size = 0.01`（R1 sha256 与时刻写入 source）、三项成本 `AwaitingPolicy`、scope 标明 `replay_binance_spot_1m` 与 P0 别名 |
| `rust/src/theta_v0/policy_records/venue-profile-synthetic-tb02a.json`（新） | version 2：`symbol: null`、`usage`、tick 1（R3）、commission 10（继承）、三项 0 与 `[新缠论:选择]` source |
| `rust/src/theta_v0/run_records.rs` | `VenueUsage` 记录头枚举（缺省 General）；`PRODUCTION_VENUE_PROFILES` 含两份；`production_venue_from_records` 按入口 usage 核对；新增 `production_venue_for_s_writer` |
| `rust/src/theta_v0/config.rs` | 新增 `ThetaConfig::for_s_structure_writer`；`admissible()`／`venue_same` 不放宽 |
| `rust/src/bin/s_session_v3/driver.rs`、`rust/src/bin/s_structure_session.rs` | 生产入口由 `for_production_profile("P0")` 改为 `for_s_structure_writer()`；`require_admissible` 不变 |
| `rust/src/theta_v0/ffi.rs` | 构造不改；经 B2 拒 synthetic（测试更新） |
| `rust/tests/production_session_entry.rs`、`tests/test_theta_pyo3_bridge.py` | BTC/P0 断言收敛为 `cost_inputs`；B2 负控；正式路径以已签 synthetic 开门的替换锁。五项 TestOnly cfg 的去除归 #1769 owner |

### 溯源标签

- 裁定七三项零值：`[新缠论:选择]`（用户 2026-10-31 批）；`commission_bps` 继承：裁定一既有口径，非新签。
- 裁定八 B1+B2 与 P0 别名归属：`[新缠论:选择]`（用户 2026-10-31 批）。
