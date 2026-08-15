# 旧 barspec 1s-bar 参数化与 tick 链的关系：复用 / 吸收 / 并列（issue #971）

日期：2026-08-14　对象：map #967 雾 5（tick 链「逐笔=单位K线」与现役 1s-bar 参数化的关系）
执行器：只读研究子代理（本体直办，无 worktree，纯只读调查）
结论：**③ 并列**——tick 链在**数据加载/年化/时间戳/聚合**四层走另一条路径，不在 `bar_seconds` 上复用或吸收；但**结构判定链（Bar → 分型 → 包含 → 笔 → 线段 → 中枢）复用**，这是唯一且真正的复用面。

---

## 一、现役「1s-bar 参数化」到底参数化了什么

`bar_seconds` 是**时间粒度**（秒/bar），整套参数化在三个点（A/B/C）+ 一个消费点，全部假设 bar 是**等时长**的：

| 点 | 位置 | 内容 |
|---|---|---|
| A 年化基数 | `rust/src/theta_v0/backtest/data.rs:92-93` | `bars_per_year(bar_seconds) = 365.25*24*3600 / bar_seconds`，公式前提 = bar 等时长 |
| B 时间戳秒分辨率 | `data.rs:183-186`、`data.rs:200-207` | `date_to_timestamp` 取 `YYYYMMDDHHMMSS` 前 **14 位（秒）**；注释明写「1s bar 秒位区分同分钟内的 60 根（否则时间戳塌缩，违反 reference:16 单调性）」 |
| C 查表粒度列 | `data.rs:53-63`、`data.rs:306-327` | `SYMBOLS: [(&str,&str,u32);8]`，8 品种全填 60；`load_by_symbol` 查表取 `bar_seconds` 传 `load_symbol` |
| 消费点（年化） | `rust/src/bin/theta_backtest.rs:95-98` | `years = bars.len() / bars_per_year(dataset.bar_seconds)`——**bar 计数近似年数** |

配套守卫：`data.rs:490-503` `symbols_table_bit_exact_60s`（8 品种恒 60，1m 路径逐 bit 等价）；`data.rs:224-225` `bar_seconds == 0` 直接 Err（「粒度秒数，bars_per_year 分母」）。

commit 溯源：`5a94a72f3e`（C 点 + 守卫）、`e7cf6a679b`（1s 拉取脚本 dates 含秒 ISO 列），均已并入 main HEAD（`68f68fb020`，实测 `git merge-base --is-ancestor` 通过）。`roadmap.yaml:981` orthogonality_note 记「1s-bar 参数化基础设施……作为引擎能力保留，不随 falsified 撤销」——**保留的是『等时长时间粒度』这一档能力，不是逐笔能力**。

## 二、tick 链是什么（map #967 已裁）

- **D2 bar 规格**：「**不聚合**，逐笔 = 一根单位 K 线；A0 中枢构件 = 线段」——每笔成交 O=H=L=C=成交价，一根 bar。
- **F1**：「tick-A0 之上按『中枢=3 次级别走势重叠』递归生成新级别，与现役 1min 级别并存」。
- **F2**：对照臂起步 + 插件架构（#960 换装转生产）。

tick bar 与 1s bar 的关键差别：**逐笔 bar 不是等时长**——活跃时段一秒内可能多笔、隔夜/停牌可隔数小时无笔。「秒/bar」维度对逐笔 bar 无定义。

## 三、三选一判定

### ① 复用（tick 也走 `bar_seconds` 这套）——不成立

1. `bar_seconds` 语义是「秒/bar」（`data.rs:84-85`：1min=60，1s=1）。逐笔 bar 的粒度维度是「笔/bar」（计数），不是「秒/bar」——没有合法取值。
2. `bars_per_year` 公式 `365.25*24*3600/bar_seconds`（`data.rs:92-93`）对逐笔 bar 无有限值：`bar_seconds=0` 被拒（`data.rs:224-225`），任何 ≥1 都给出「等时长」年化，而逐笔 bar 的年 bar 数 = 该品种当年成交笔数，**数据依赖、非恒定**。
3. `date_to_timestamp` 秒分辨率（`data.rs:183-186`、`data.rs:206` `take(14)`）会把同一秒内的多笔成交塌缩成同一时间戳——代码自己声明这就是「违反 reference:16 单调性」的塌缩（`data.rs:185-186`）。逐笔需要亚秒（ms/ns）分辨率。
4. `nautilus/backtest_engine.rs:79-84` `aggregation_for(bar_seconds)` 只映射时间聚合（1→Second、60→Minute、86400→Day、其它→Second(step)），无 Tick/Count 分支；逐笔要的是 count/tick 聚合。

⟹ 把逐笔硬塞进 `bar_seconds` 只能靠「偷换语义」，不是复用。

### ② 吸收（tick 成为 `bar_seconds` 的一档）——不成立，且是假统一

「吸收」= 把 `bar_seconds: u32`（秒）扩成可表示 tick 的档。要成立必须：

- 把 `u32` 改成 tagged union（`Time(u32) | TickCount(u32)` 之类）——A/B/C 三点的消费者（`bars_per_year`、`date_to_timestamp`、`aggregation_for`）**仍须逐一分叉**，因为等时长与计数两条路径在这三处没有任何共同算式可共享；
- 撞改 `symbols_table_bit_exact_60s` 守卫（`data.rs:490-503`）的前提「全 8 品种恒 60」。

结果只是把「两个不同数据路径」藏进同一个字段名后面，分叉一处不少，还污染了现役干净的等时长粒度类型。这与仓内纪律「承重的是它在系统里干什么，不是它长什么样」（AGENTS.md #804 判别式）相悖。**吸收在名义上统一、实质上还是并列，且比并列更差**（守卫/测试/类型全部被搅动）。

### ③ 并列（tick 链走另一条路径）——成立，且正是 map #967 自己选的架构

判据对齐：

| 层 | 1s/1min 链（现役） | tick 链 | 关系 |
|---|---|---|---|
| 数据加载 schema | `RawData{opens,highs,lows,closes,volumes,dates}`（`data.rs:35-48`） | trades 逐笔成交（Databento Trades / Binance aggTrade，map #967 Notes） | **并列**（新 loader） |
| 粒度描述 | `bar_seconds: u32` 秒 | 需要新描述（tick/计数，或直接不设固定粒度） | **并列**（不复用 `bar_seconds`） |
| 时间戳 | 14 位秒（`data.rs:200-207`） | 需 ms/ns 亚秒 | **并列**（新编码） |
| 年化基数 | `bars_per_year(bar_seconds)`（`data.rs:92-93`） | 数据依赖（实际笔数/年）或从 ns 时间戳走历法年 | **并列**（新算法） |
| nautilus 聚合 | `aggregation_for` Second/Minute/Day（`backtest_engine.rs:79-84`） | BarAggregation::Tick/Count | **并列**（新分支） |
| 结构判定链 | `Bar` → 分型 → 包含 → 笔 → 线段 → 中枢 | 同左 | **复用** |

复用面是真实存在的，但它不在 `bar_seconds` 参数化里，而在更底层：

- `Bar` 结构（`types.rs:49-58`）：O/H/L/C/volume/untradable，逐笔 O=H=L=C=成交价可直接填入，无需改字段；
- `quantize`（`types.rs:24`）整数 tick 量化；
- 分型/包含/笔/线段/中枢判定链建在 `Bar` 上、粒度无关（此面由 sibling 票 #969 勘察改动面，本票不越界）。

**并列为「对照臂」不触 #799 收敛通则**：时间 bar 链与逐笔 bar 链是「不同数据粒度/不同对象」的平行臂，不是同一个判断的宽严两档（map #967 F2 已定「对照臂起步」）。

## 四、建议（落地划分）

1. **判 ③ 并列**：tick 链新建独立数据路径（tick loader + 计数粒度描述 + 亚秒时间戳 + 数据依赖年化或历法年 + nautilus Tick/Count 聚合），**不要**往 `SYMBOLS` 的 `bar_seconds` 列塞 tick、不要扩 `bar_seconds` 类型。
2. **复用 `Bar` + `quantize` + 结构判定链**：逐笔 bar 填 `Bar`（O=H=L=C），下游分型/笔/线段/中枢原样复用（#969 给 file:line 改动面）。
3. **诚实登记一个易混点**：research 正本（`tick-l0-three-paths-research-20260814.md` 结论四）写「落地成本：data.rs 加一行 `bar_seconds=1`（或 N-tick）条目」——**这句只对「聚合为 1s 等时长 bar」这条路线成立，对 map #967 D2「不聚合、逐笔=单位K线」不成立**。逐笔 bar 没有 `bar_seconds` 取值。若未来 bar 规格从「逐笔」改回「聚合为 1s 时间 bar」，那才是 `bar_seconds=1` 的纯复用；两条路线不可混用同一句「零改动」。

## 五、与 sibling 票的边界（不重复做）

- #968：tick 数据源落地（vendor schema/深度/成本/坏 tick 清洗）——本票只引用其结论「trades 逐笔成交 + MBP-1 + aggTrade」，不查 vendor。
- #969：`Bar`(1min)→逐笔单位 K 线的 parser 改动面——本票只声明「结构链复用」这一方向，file:line 清单归 #969。
- #970：tick 粒度背驰/力度判据——本票不涉。

## 附：承重断言索引

| 断言 | 证据 |
|---|---|
| bar_seconds 是「秒/bar」时间粒度 | `data.rs:84-85` |
| bars_per_year 公式等时长前提 | `data.rs:92-93` |
| 时间戳只到秒、且代码自述亚秒会塌缩 | `data.rs:183-186`、`data.rs:200-207` |
| bar_seconds=0 被拒 | `data.rs:224-225` |
| nautilus 聚合无 Tick/Count 分支 | `nautilus/backtest_engine.rs:79-84` |
| 年化消费 = bar 计数 / bars_per_year | `theta_backtest.rs:95-98` |
| 守卫锁 8 品种恒 60 | `data.rs:490-503` |
| Bar O/H/L/C/volume/untradable | `types.rs:49-58`；quantize `types.rs:24` |
| 两 commit 已并入 HEAD | `git merge-base --is-ancestor 5a94a72f3e/e7cf6a679b HEAD` 实测通过，HEAD `68f68fb020` |
| 1s-bar 参数化「引擎能力保留」 | `roadmap.yaml:981` |
| tick 链 = 不聚合、逐笔=单位K线 | map #967 D2（gh issue view 967） |
