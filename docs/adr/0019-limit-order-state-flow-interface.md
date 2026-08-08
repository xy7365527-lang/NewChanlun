# ADR 0019：订单接口事件流 → 状态流——只改回测臂、`Order` 加 `limit_px`、快照身份键、市价路径零改

**日期**：2026-08-06（**2026-08-07 订正裁定三：键的分量从全三元组收到 `start_index`**，走 [#929](https://github.com/xy7365527-lang/NewChanlun/issues/929) 探针回填；**同日 [#938](https://github.com/xy7365527-lang/NewChanlun/issues/938) 两条探针回来，补裁 A/B 两条填掉「订正后浮出的缺口」**）
**裁定人**：编排者，走 [订单接口：事件流 → 状态流 #927](https://github.com/xy7365527-lang/NewChanlun/issues/927)（map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)）当场拍板
**状态**：已采纳（裁定三分量经 [#929](https://github.com/xy7365527-lang/NewChanlun/issues/929) 订正；**浮出的两个缺口已由 [#938](https://github.com/xy7365527-lang/NewChanlun/issues/938) 裁定 A/B 裁完**，见「订正后浮出的缺口」节）

## 背景

[ADR 0018](0018-limit-order-fill-model.md) 裁了五条限价成交假设，并把「订单接口从事件流改状态流」这件结构性改动移交本票（该 ADR「移交」节第 2 条）。本 ADR 只裁**形态**：改哪条臂、加什么字段、快照里怎么认「同一张单」、市价路径动不动、下游怎么排。**不出实装 spec**（见裁定五）。

上游两份：ADR 0018（五条成交裁定）+ [#926](https://github.com/xy7365527-lang/NewChanlun/issues/926) 的 AFK 只读勘察（生产成交回路现状，已回填进 ADR 0018「勘察回填」节）。

### 为什么这五条必须一起裁

ADR 0018 的五条裁定**落不进同一条代码路径**——这件事是本 ADR 的起点，也是裁定一的全部内容。发现它之前，「实现 ADR 0018」看起来是一件事；发现之后是两件，而其中一件本线不做。

### 本 ADR 的行号援引均经第二人独立复核

下表每一处援引都在本票交付时**逐条打开确认**（不靠 grep 命中数），订正 3 处，见「援引复核」节。这道工序在本仓在案 5/5 全中真错，其中最危险的一类是「结论正确但论据是假的」。

**2026-08-07 第二轮（[#929](https://github.com/xy7365527-lang/NewChanlun/issues/929) 回填）同样逐条复核**：订正 1 处路径错（`rebase_txn.rs` 在 `classifier/` 而非 `backtest/`）＋ 1 处实测口径收紧（wf8 七条里明写核心漂移的是 5 条），并**新增一处「结论对、论据假」的在案样本**——裁定五当初预判「若延伸会改写 ZG 则键只能用 `start_index`」，结论对，但机制描述是假的（延伸确实冻结核心；真成因是生产不走那条路径）。**在案数升为 6/6。**

**2026-08-07 第三轮（[#938](https://github.com/xy7365527-lang/NewChanlun/issues/938) 两条探针回填）同样逐条复核**（本次复核结果见「援引复核（#938 轮）」节）：**新增第 7 例在案样本，仍属「结论对、论据假」那一类**——`.chanlun/review-results/issue929-centerid-stability-20260807.md:134` 把生产的 frontier pop 接线写成 `classifier/mod.rs:1937-2001`，**实为 `classifier/mod.rs:1376-1415`**（`:1380` `let pop_n = lc.scan_cursor.last_window_emitted;`、`:1402-1403` `Rc::make_mut(&mut lc.centers)` + `cs.truncate(cs.len().saturating_sub(pop_n))`、注释 `:1377-1379` 逐字「pop 最后成立窗口的**全部**产出（frontier 域 = 整窗，重扫从窗口起点重产）」）。`:1937-2001` 区间内是 `extract_second_for_level`（`:1939`）与 `second_for_parent`（`:1967`）两个次级别背驰提取函数，**区间内无任何 pop / truncate / win_start 逻辑**（检索式：`awk 'NR>=1937 && NR<=2001' rust/src/theta_v0/classifier/mod.rs | grep -nE "^(pub )?fn |pop|truncate|win_start|resume"`，命中仅两个 `fn` 声明与两处 doc 注释里的「frontier 门控」字样）。**已排除「写时对、后来漂」**：`git diff --stat d6e9fd12e2 HEAD -- rust/src/theta_v0/classifier/mod.rs` 无输出（`d6e9fd12e2` = #929 报告落盘 commit）。**#929 的实质结论（生产延伸走 frontier pop + 重扫、`zd`/`zg` 会漂、`start_index` 不动）经本轮独立复核成立，不受影响。****在案数升为 7/7。**

## 名分速览

| 裁定 | 一句话 | 依据 |
|---|---|---|
| 一 | **只改回测臂 `pi_theta_fill_loop_overlay`，不动 NT 臂** | ADR 0018 五条落不进同一路径 ＋ NT 臂现无下游（#792） |
| 二 | `types.rs` 的 `Order` **加字段 `limit_px: Option<Tick>`** | `apply_order` 的 `px`/`fee_rate` 都由调用点传入 ⟹ 信息只能随订单到达 |
| 三 | 快照里「同一张单」按 **`(CenterId.start_index, 方向)` 身份键**认定，qty 冻结；**`zd`/`zg` 不入键** | ADR 0018 裁定四寿命绑中枢 ＋ W1 sizing 冻结先例 ＋ **[#929](https://github.com/xy7365527-lang/NewChanlun/issues/929) 探针实测：`zd`/`zg` 在产会漂** |
| 四 | **市价路径一行不动**，限价走独立槽；bit-exact 靠结构不靠测试 | ★M5 / ★LEE M1 两处旁挂先例 ＋ `cargo check` 假绿负控 |
| 五 | 探针挂 map #787，本票裁完形态即关；实装 spec 等探针回来再出且**出图** | 探针翻不动形态，只翻得动键的分量（**已回：分量确被翻，见裁定三订正**） |
| **A**（#938 补裁） | **挂单价跟随中枢核心**：核心被修订 ⟹ 挂单价跟着重读；中枢封口后自然固定。**暂不设漂移阈值** | 绑定中枢 100% 开放（4,677,752/4,677,752 零反例）＋ 核心漂 = 认识变准 ＋ 高级别最重（L4 71.43%）＋ 半绑半不绑说不通 |
| **B**（#938 补裁） | **身份键保持裸 `CenterId.start_index`**，不改用 `resolve()` 的锚 | 本窗零分歧实测：裸键误撤 **0/2,423**、锚漏撤 **0/258**；结构性原因 = 谱系判据只读种子 `(level, ordinal)`，`zd`/`zg` 不进判据 |

---

## 裁定一：只改回测臂，不动 NT 臂

**ADR 0018 的五条裁定落不进同一条代码路径。**

| ADR 0018 裁定 | 只能落在 | 为什么 |
|---|---|---|
| 一（穿越才成交）、二（开盘取优） | **回测臂** `pi_theta_fill_loop_overlay` | 成交判据在成交那一行，NT 臂的撮合归 venue（`nautilus/strategy.rs:133-137` 自陈），本仓不做撮合 |
| 四（GTC）、五（限价不扣滑点） | **回测臂** 同上 | 同上 |
| 三（同 bar 多单排序） | **NT 臂** `strategy::plan_orders` | 回测臂**每 bar 只产一个净额订单**（`sizing.rs:524` `schedule_order` 返回单数 `Order`，非 `Vec`），没有第二个单可排 |

⟹ **本 ADR 的裁定二/三/四全部只作用于回测臂**；ADR 0018 裁定三在本线上**不落地**（它已有 NT 臂实现 `ConflictKey`，见 ADR 0018 勘察回填订正二 + 本 ADR 对该订正的追加订正）。

**理由三条**：

1. **本线起点是「[ADR 0017](0017-chong-quantity-numerator-shortdiff-amplitude.md) 那批价位驱动短差规则验不出数」**（ADR 0018:127 逐字：「成交次数趋近 0，ADR 0017 那批规则又一次验不了」）。**能不能验，只在回测臂上定。** NT 臂改得再对，那批规则也还是没有数。
2. **NT 臂现无下游**：[#792](https://github.com/xy7365527-lang/NewChanlun/issues/792) 实测无任何一条路拿缠论信号实盘下单。
3. **回测臂改动自包含**，且 `overlay=None ⟹ 逐字节不变` 的 wrapper 机制**已有现成回归锁**（`backtest/fill.rs:764-766`）。

**代价（明账）**：本 ADR **不解决限价单在真实生产上怎么发**。NT 那条另开线——不在 map #787 上，也不由本 ADR 承接。

## 裁定二：`Order` 加字段 `limit_px: Option<Tick>`

**`None` = 市价，走现有 close 路径；`Some(tick)` = 限价，走 ADR 0018 裁定一/二。**

### 硬约束（这条是裁定二的物理基础）

回测臂真正成交的那一行是 `apply_order(o, px, fee_rate, ...)`（`backtest/fill.rs:4630-4638`）——**`px` 与 `fee_rate` 都是调用点算好传进去的**：

- `px = bar.close as f64 * config.tick.tick_size`（`fill.rs:4604`，主循环内每 bar 算一次）
- `fee_rate = (commission_bps + slippage_bps + tax_bps) / 10_000.0`（`fill.rs:4405-4406`，**主循环外算一次**）

而 ADR 0018 的两条裁定各要一份信息：

| ADR 0018 裁定 | 调用点必须知道 |
|---|---|
| 二（成交价 = `min(开盘价, 挂单价)`） | **挂单价** |
| 五（限价不扣 `slippage_bps`） | **这张单是限价还是市价** |

**这两条信息只能随订单到达那一行。**

### ⟹「让 fill 层自己从中枢推出挂单价」当场排除

理由不只是不好看：`fill.rs:5828-5829` 的 W1 段注释明写

> 「决策单源 = 本步 step_trace 五类生命周期事件（与 typed_ledger 消费同源，**禁第二裁决源**）」

**fill 层自推挂单价正面撞这条既有纪律**——它就是在 fill 层新开一个裁决源。

### 为什么加字段而不用包装型

本仓有过一次「`Order` 不加字段、冲突面零改」的规避先例：`LegOrder`（`strategy/mod.rs:944`，doc `:934` 逐字「**types.rs `Order` 不加字段**，冲突面零改」）。本次不照它走，四条理由：

1. **裁定一已把范围缩到回测臂**，净额回路只有**一个** `Order` 生产构造点（`sizing.rs:558`，`schedule_order` 内唯一的 `Order {` 字面量）。
2. **加字段的波及是编译期可见的**：`Order`（`types.rs:357-365`）仅 3 字段且全部字面量构造，**无 `..Default`、无 `#[non_exhaustive]`** ⟹ 加字段必然编译失败，不会静默漏。**这是本仓少有的能被编译器兜住的改动**——多数改动没有这个性质，这一条不可外推。
3. **`Option<Tick>` 不破坏 `Copy + Eq`**：`Order` 现 derive `Debug, Clone, Copy, PartialEq, Eq`（`types.rs:358`），`Tick` 是整数别名，`Option<整数>` 全部 derive 保住。
4. **包装型适合加正交维度**（`LegOrder` 加的是腿标记：长/短 × 开/平，与 `Order` 本身正交），**不适合二选一的模式**。限价/市价是同一个位置的两种取值，用包装型表达会退化成手搓 `Option`，还丢类型安全。

### 代价（明账）

- **测试侧构造点 [#926](https://github.com/xy7365527-lang/NewChanlun/issues/926) 诚实标注为未穷举**（只有 grep 命中，没打开过）⟹ **工作量现在数不清**。不写「工作量小」。
- **`Order` 是跨模块核心类型**，回测臂之外的生产构造点（`strategy/mod.rs:440`/`:462`、`backtest/dual_ledger.rs:258`）也被迫填 `None`——**行为零变化的无害溢出**，但溢出本身是真的。

## 裁定三：快照里的「同一张单」按 `(CenterId.start_index, 方向)` 身份键认定

> **★ 2026-08-07 订正（[#929](https://github.com/xy7365527-lang/NewChanlun/issues/929) 探针回来后）**：本裁定原文写的是**全三元组** `(CenterId, 方向)`，即 `CenterId = (start_index, zd, zg)` 三个分量一起入键。**探针实测把这一支翻掉了：`zd`/`zg` 不可入键，身份键只能用 `start_index`。** 形态（「按身份键认续挂」）不变，分量变——正是裁定五预告的那一支。下面的正文已按订正后的口径重写，原三元组表述不再有效。报告：`.chanlun/review-results/issue929-centerid-stability-20260807.md`。

**键相同 ⟹ 同一张单续挂**：qty **冻结**取首次挂出时的值，maker/taker 判为 **maker**。
**键变了或消失 ⟹ 旧单撤、新单挂**（taker）。

**⚠️ 「键相同」不再蕴含「价格不变」**——这是从全三元组降到单分量的**代价**，见下方「浮出的缺口」节，已开 [#938](https://github.com/xy7365527-lang/NewChanlun/issues/938) 裁。

> **★ 2026-08-07 补裁指针（[#938](https://github.com/xy7365527-lang/NewChanlun/issues/938)）**：本裁定当初留下的两处「未定」现已定完，**读本裁定必须连带读那两条**——
> - **键的分量已定**：保持**裸 `CenterId.start_index`**，**不**改用 `resolve()` 的归一锚（**裁定 B**）。即上面写的 `(CenterId.start_index, 方向)` 是终局形态，不是中间态的占位。
> - **价的跟随语义已定**：键相同 ⟹ 同一张单、qty 冻结、判 maker 这三条**不变**；但**挂单价不冻结**——中枢核心一被修订，挂单价跟着重读、改单（**裁定 A**）。⟹ 本裁定里「冻结」二字**只覆盖 qty 与 maker/taker 判定，不覆盖价格**。
>
> 两条全文见下方「订正后浮出的缺口」节。

### 为什么必须能认出「续挂」

ADR 0018 裁定四把订单接口改成「每 bar 输出应挂集合」之后，fill 层看到的是一份份长得一样的快照。有两处逼着必须区分「续挂」与「新挂」：

1. **maker/taker 判定原文就是按这个分的**。ADR 0018:159 逐字：「订单在这根 bar **之前**就已挂在簿上 ⟹ **Maker**；信号 bar 当根新挂…⟹ **Taker**」。**认不出续挂，maker 这一档永远拿不到**，ADR 0018 裁定五的费率区分等于没写。
2. **本仓已为同一个病做过一次修复**。`fill.rs:5830-5832` 逐字记着：「sizing 冻结：q_v 取开仓 bar 决策层 `SepLeg.q_units`…落簿后存续期不再随 NAV/价每 bar 重定（**治 runner `base_units=equity_nav/px` 每 bar 重算导致的声部目标漂移**）」。**快照语义会把这个病原样搬到限价单上**——每 bar 重出一份快照，量就每 bar 跟着 NAV 漂。

### 为什么用中枢身份做键（形态层，未被探针推翻）

`CenterId { start_index, zd, zg }`（`classifier/center_lifecycle.rs:162-169`，字段序即 `Ord` 派生序）：

1. **它是 ADR 0018 裁定四「寿命绑中枢存续」的直译**——寿命的载体就是中枢，键就该是中枢的身份。
2. **撤单彻底隐式**：中枢一失效，下一份快照里这个键就不在了。**不靠约定，靠键的存在性**。
3. **本仓已有把 `CenterId` 入键的先例**：`HistoricalSeenKey { level, owner: CenterId, source_index, bits_disc }`（`fill.rs:881-886`）。

**被否的候选：按挂单价认。** 会在中枢换代时**静默误判**——两个不同中枢可以有相同的 ZD 而 `start_index` 不同。按价认会把新中枢的单认成旧单续挂，沿用旧 qty 还判成 maker。**静默**是关键词：不报错，只出错数。

### ★ 为什么 `zd`/`zg` 必须出键（[#929](https://github.com/xy7365527-lang/NewChanlun/issues/929) 实证，本次订正的全部依据）

本 ADR 原文把三个分量一起入键，理由是「`CenterId` 就是中枢的身份」。**探针查出的事实是：`CenterId` 三元组在生产里不是「中枢的身份」，而是「本次扫描重算出来的派生值」。** 承重两层，两层**互相独立**：

**第一层：单次扫描内的 Step2 延伸确实冻结核心——本 ADR 原来的旁证没错。**

`classifier/recursive_tower.rs:745-752` 的延伸循环逐字只写三个字段：

```rust
let mut c = seed;
let mut j = i + 3;
while j < units.len() && units[j].lo <= c.zg && units[j].hi >= c.zd {
    c.end_index = units[j].end_index;   // :748
    c.dd = c.dd.min(units[j].lo);       // :749
    c.gg = c.gg.max(units[j].hi);       // :750
    j += 1;
}
```

三个被写的字段**都不在** `CenterId` 里；而且循环条件本身读 `c.zg`/`c.zd`（`:747`）——写核心会自毁循环语义。doc `recursive_tower.rs:265`「**ZD/ZG 核心冻结**」与实现**一致，未漂移**（本 ADR「有效域」原写的那条旁证经复核成立）。仓内另有机检锁 `center_lifecycle.rs:1371-1400`（`center_id_ignores_envelope_and_end_index`）断言延伸不改身份。

**第二层：但生产不以这种方式实现延伸——这一层是本 ADR 原来完全没看的。**

`recursive_tower.rs:704-713` 的 bit-exact 充要条件，第 1 条末尾逐字（`:710-711`）：

> 「frontier 协议天然正确：**pop 开放中枢 + 从其 seed 起点重扫** ⟹ 延伸在重扫中吸收新单元，bit-exact。」

第 2 条（`:712-713`）明写 `units[..start_i]` 之间**「只允许尾部追加或 frontier 段原地改写后重扫」**——即**frontier 段的原地改写是被允许的输入**。重扫时 `build` 重新执行 `center_from_segments`（`classifier/center.rs:223-230`），核心按 `zd = max(lo₁,lo₂,lo₃)`（`compute_zd`，`center.rs:133-135`）/ `zg = min(hi₁,hi₂,hi₃)`（`compute_zg`，`center.rs:125-127`）**重新求交**，而 `start_index = a.start_index`（`center.rs:228`）取第一源、不动。

⟹ **中枢还活着、`start_index` 不变、`zd`/`zg` 已变。** 全三元组键会把这种情形读成「键消失 ⟹ 中枢失效 ⟹ 撤单」，**与 ADR 0018 裁定四「寿命绑中枢存续」正相反**。

**实测（wf8 八条，`.chanlun/review-results/rebase-identity-d1-research-20260728.md:80-87`）**：

- **7/8** 归因为「同 seed、`start_index` 不变、**连续候选非分裂**」（seq=3/20/24/47/53/55/56）。其中 **5 条**（seq=3/20/47/53/56）表内**明写**「父 `zd` 漂移」或「父 `zg` 漂移」；另 2 条（seq=24/55）表内写的是「第三源 frontier 修订后 3 源延伸至 5 源」，**核心漂移未逐字写出**——照实标注，不并成「7 条都写了漂移」。
- 逐数（同文 `:220-236`，seq=3，L2）：`start_index = 24614` **不变**；第三源外缘下沿从 `2580000000000` 修订为 `2571704000000` ⟹ `old zd = max(2545760000000, 2556555000000, 2580000000000) = 2580000000000`，`new zd = max(2545760000000, 2556555000000, 2571704000000) = 2571704000000`（**约 −0.32%**）；`zg` 两次均为 `2598750000000`。
- **1/8**（seq=66）是 `start_index` 右移（238178 → 238612，同文 `:96-124`）；另有非核销样本 seq=59 同型（`:203-218`）。**两例的中枢确已重建**（seed 成员集合变了，旧三元交集变空），D1 裁「拒绝过继，保持核销」⟹ **撤单是正确语义，不构成 `start_index` 的反例**。

**机检（不是文档声明，是仓内测试断言）**：`classifier/rebase_txn.rs:1469-1487`，测试 `continued_center_pairs_only_admits_fully_bijective_continued_edges` 第 ① 段。helper 参数序 `center(start, end, zd, zg, dd, gg)`（同文 `:1109-1118`，逐行确认）：

```rust
let old_c = center(100, 130, 1080, 1100, 1000, 1180);  // start=100, zd=1080, zg=1100
let new_c = center(100, 136, 1060, 1100, 1000, 1200);  // start=100, zd=1060, zg=1100
```

该对被 `continued_center_pairs` 判为 `continued_1to1` ∧ functional/injective/unique/bijective 四布尔全真（注释 `:1470` 逐字「① 同 seed、第三源修订 ⟹ 一条可过继的连续边」）。**同一个中枢，`start_index` 不变、`zd` 从 1080 变 1060。** 该函数签名本身是 `-> Vec<(CenterId, CenterId)>`（`:744-773`）——「旧身份 → 新身份」这个返回类型就已经承认了身份会变。

**修补回路的存在即在产证明**：`lineage_book.rs:5-8` 模块 doc 把这条写死为已知病症——

> 「开放 frontier 每 bar 被 pop 后重扫，`CenterId=(start_index, zd, zg)` 是当次前三个构造单元的派生值：**第三源外缘一改，父核心就重新求交，旧三元组消失、新三元组出现**……这是**工程丢身份**，不是教义生死。」

而 `LineageBook` 是**生产默认开**的：`consumer_enabled()`（`lineage_book.rs:82-91`）实为 `!env_flag(THETA_REBASE_MIGRATE_SKIP)`，与 `OPSEM_DUMP_DIR` 解耦（doc `:27-29`）；生产接线在 `backtest/fill.rs:1017-1022`。**本仓已经为「全三元组会丢身份」专门写了一套 fail-closed 的迁移回路并默认开着——这件事本身就是该三元组不稳的在产证明**，无需再测。

**写入点穷举（本次独立复跑，非照抄报告）**：对**已存在** `Center` 实例的字段改写，在 `rust/src/theta_v0/` 全域**只有 3 处**——`recursive_tower.rs:748`（`end_index`）/ `:749`（`dd`）/ `:750`（`gg`），全在同一个延伸循环体内、全是**非身份字段**。检索式（覆盖 `rust/src/theta_v0/` 全部 `*.rs`）：

| 形式 | 检索式 | 命中 |
|---|---|---|
| 字段赋值 | `grep -rnE "\.(zd\|zg\|start_index)\s*(=[^=]\|[-+*/]=)"` | **0**（全部命中均为 `==`/`<`/`>` 比较、注释、或 `trading/` 侧同名异物） |
| 可变引用外借 | `grep -rnE "&\s*(')?[a-z_]*\s*mut\s+(super::)?(types::)?Center\b"` | **0** |
| 容器可变迭代 | `grep -rnE "\.(iter_mut\|last_mut\|get_mut\|first_mut\|values_mut)\(\)"` ∩ `Center` | **0** |
| 整体覆写 / swap / replace | `grep -rnE "\bmem::(swap\|replace\|take)\b"` | 命中 26 处，**无一作用于 `Center`**（全是 `Vec`/缓存缓冲） |
| `impl` 块内改自身 | `grep -rnE "^impl( <.*>)? .*\bCenter\b"` | 仅 `level_view/pan.rs:69` 的 `impl From<&Center> for PanCenterIdentity`（**只读**） |
| 解构式赋值 | `grep -rnE "\.(zd\|zg\|start_index)\s*,"` ∩ `) =` | **0** |

⟹ **三个身份分量从不被就地改写**；它们取到不同值的**唯一**途径是**重新构造一个 `Center`**。生产（非 `#[cfg(test)]`）的 `Center` 构造点共 **8 处**（同上目录，检索式 `grep -rnE "\bCenter\s*\{"` 后按各文件首个 `#[cfg(test)]` 行号切分，**含路径限定形式** `super::types::Center {`）：`center.rs:223`/`:260`、`recursive_tower.rs:782`、`level_view/projection.rs:157`/`:170`、`strategy/mod.rs:588`/`:597`（两处零占位）、`backtest/signal.rs:129`（零占位）。

### ★ 升级重切路径（本 ADR 原先完全未查）

`recursive_tower.rs:767-800`（#148，第 33 课）：窗口总段数 `n ≥ UPGRADE_TOTAL_SEGMENTS`（9）时，整窗按每 3 段重切为 `k = n / 3` 个本级子中枢。逐分量（承重 `:782-789`）：

| 分量 | 重切后 | 承重行 |
|---|---|---|
| `zd` | **继承母核心不变**——`zd: c.zd` | `:783` |
| `zg` | **继承母核心不变**——`zg: c.zg` | `:784` |
| `start_index` | `units[s].start_index`，`s = i + t * 3`（`:778`）⟹ **`t = 0` 时 `s = i`，头子逐值等于母中枢**；后 `k−1` 个子是新身份 | `:787` |

**两条含义，都对本裁定承重：**

1. **升级重切不会让「已挂单所绑的中枢」换身份**——头子完整继承母中枢的 `(start_index, zd, zg)`。同款教义已在仓内：`lineage_book.rs:33-39` 逐字「**下级分裂头子继承父种子**——头子唯一保住窗口起点与核心，身份延续无歧义」（用户 2026-07-29 裁定，宽读法已转生产默认；严格读法留档，`THETA_REBASE_MIGRATE_STRICT=1` 切回）。
2. **副产结论：`(zd, zg)` 单独做键会撞键。** 重切后 `k` 个子中枢**共享同一 `(zd, zg)`**（`:783-784` 全部继承同一个 `c`），**只有 `start_index` 能区分它们** ⟹ 无论怎么选，**`start_index` 必须在键里**。这条是「只能用 `start_index`」的**正面**理由，与上面「`zd`/`zg` 会漂」这条反面理由**互相独立**。

⚠️ **这一节是逐行读出、不是实测。** wf8 八条案例在**被观测的父级层**无一触发九段重切（研究文 `:185` 明写）。**照实追加一条订正**：`lineage_book.rs:33-35` 记着 seq=24/55 两条**在下一级**确实发生了九段分裂（「它们的第三个种子源在下一级发生了九段分裂（`split`），即『同 ordinal 但不是同一个对象』」）⟹ **九段重切路径在 wf8 里执行过，只是不在被观测的那一级**，#929 报告 §6 第 1 条「wf8 无一触发」说过头了。但那两条的比对口径是**原始种子 ordinal**（宽读法），**不是母中枢与头子的 `CenterId` 逐位相等**——所以它只是**间接一致**，仍**不构成**「头子身份保持」的直接实测。要当承重须另补探针。

### 代价（明账，这笔账要单独记）

**qty 冻结让挂单量与当下 `p_star` 脱钩**：挂出时 NAV 高则量大，之后 NAV 掉了，那张单还挂着大量；成交后仓位超出当下水位。

W1 当初接受了同款权衡（`fill.rs:5830-5832`），本裁定沿用它——但**这笔账要单独记**：**[ADR 0017](0017-chong-quantity-numerator-shortdiff-amplitude.md) 那批规则验出来的数字会含这份偏差。** 不许在读那批数时把它当成干净的。

## 裁定四：市价路径一行不动，bit-exact 靠结构成立而非靠测试

**`pending` 队列原样留给市价单，限价单走新增的独立槽；`limit_px == None` 时不进任何新代码。**

**理由**：

1. **本仓两处先例都是「新路径旁挂、旧路径零改」，不是「统一后靠测试锁」**：
   - ★M5：`fill.rs:764-765` 逐字「overlay=None ⟹ 现有净额路径逐字节不变（bit-exact）」
   - ★LEE M1：`fill.rs:766` 逐字「level_ledger=None ⟹ 级别账本镜像整段跳过（bit-exact 回归锁，#644）」
   （同款结构还有 M4 级别帽：`fill.rs:5180` 的 `if config.risk.enforce_level_cap`，默认 false（`config.rs:229`），门关整段不构造。）
2. **靠结构比靠测试强一个量级**：本仓有 `cargo check` 重放缓存假绿的实证负控——改了源文件仍 `Finished 0.03s`。测试可能没跑；结构不会没成立。
3. **统一成一套要先补一个与限价无关的历史语义差**（见下），那笔债不该由本线还。

### ★ 必须写进本 ADR 的那个语义差

**现状市价单不会重复挂，靠的是时序巧合**：bar `i` 产的单在 `i+1` 的成交段成交（`fill.rs:4624` 消费 `pending[i]`），`p_t` 随即更新；等 `i+1` 走到决策点时 `Δ = p_star − p_t` 已归零，`schedule_order` 退化为 `Hold`/`Wait`（qty=0），`fill.rs:5821` 的 `if order.qty > 0` 挡住不入队。

**但 `fill_bar_index` 会在落点 `untradable` 时顺延**（`strategy/exec.rs:42-53`，`while i < bars.len() { if !bars[i].untradable { return Some(i) } ... }`）。一旦顺延：

- 中间几根 bar 的 `p_t` 仍是旧值（成交还没发生）
- ⟹ 每根都产同一张单
- ⟹ 全部 `push` 进同一个 `pending[ei]`（`fill.rs:5824` 是 **`push` 不是覆盖**）**叠加**
- 而快照语义下，同样情形只留 **1 张**

⟹ **把市价单一起改成快照是真行为变化，不是重构。**

**⚠️ 差异面未穷举**：顺延路径上的 `untradable` bar 到底还走不走决策段（走了才会产第二张单），**本票未查**。⟹ 上面写的是「一旦顺延则叠加」这条机制成立，**不是**「实测叠加发生了 N 次」。**不许把这一段读成「不受影响」，也不许读成「已实测」。**

### 代价（明账）

- **两套并存 ⟹ 同 bar 既有市价单又有限价单时，成交次序未定义。** ADR 0018「有效域」已登记过一条同类缺口（`0018:169`：「裁定三③：同 bar 反向开仓单的次序**未定义**」）。
- **本 ADR 不定这个次序。** 净额臂上二者同 bar 并存，需要 `p_star` 同时要求两种执行方式，判断为不可达——**但这个不可达性没有举证**（没穷举过 `p_star` 的产生路径）。⟹ 按本仓「穷举断言必须附穷举方法」的纪律，只能写「**已查到的路径上不可达**」，**不许写「不会发生」**。
- **`pending` 队列长期留着会成为第二套机制** —— 这是**已知的、有名字的技术债**，写在这里就是给它一个名字，防止日后被当成「本来就该有两套」。

## 裁定五：下游排法

1. **探针票**（测 `CenterId` 的 `zd`/`zg` 在中枢**延伸**时会不会被改写）**挂 map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)**；
2. **本票（[#927](https://github.com/xy7365527-lang/NewChanlun/issues/927)）裁完形态即关**，不等探针；
3. **实装 spec 等探针回来再出，且出图**——#787 是**纯决策图**，实装票不得挂本图。

**为什么探针不挡本票**：五条裁定**没有一条等这个读数**。探针翻不动「用身份键」这个形态，只翻得动**键里放哪几个分量**——若延伸会改写 ZG，键就只能用 `start_index`（丢掉 zd/zg）。形态照裁，分量待测。

**遗留（照实）**：#787 的关图条件②「下游实施线已开、有票号、有主」**在本条线上暂时不满足**——实装票要等探针，现在没有票号也没有主。

**★ 探针回来后的复盘（2026-08-07）**：上面这段预判「若延伸会改写 ZG，键就只能用 `start_index`」——**结论蒙对了，理由错了**。真正的成因不是「延伸改写核心」（那条路径经复核确实冻结核心），而是**生产根本不以那种方式实现延伸**（frontier pop + 重扫）。**记在这里是因为「结论对、论据假」正是本仓在案最危险的一类错**：若当时据此写实装 spec，spec 会带着一条假的机制描述落地。

---

## ★ 订正后浮出的缺口 —— **已由 [#938](https://github.com/xy7365527-lang/NewChanlun/issues/938) 裁定，结论见本节「裁定 A / 裁定 B」**

> **本节的读法（2026-08-07 改写）**：下面「缺口一 / 缺口二」两小节是**原文照留的发生史**——它们记的是 2026-08-07 上午从全三元组降到单分量那一刻、缺口刚浮出来时的认识状态，包括当时写的「本 ADR 不裁」。**那句话现在已经不成立了**：同日两条 AFK 只读探针回来后，编排者当场把两个缺口都裁完了，走 [#938](https://github.com/xy7365527-lang/NewChanlun/issues/938)。**结论全文在两小节之后的「裁定 A」「裁定 B」里，以那两条为准。**
>
> 承重的两份探针报告：
> - `.chanlun/review-results/issue938-center-open-or-sealed-20260807.md`（绑定中枢开放性 + `core_drift` 计数）
> - `.chanlun/review-results/issue938-seed-moved-composition-20260807.md`（`seed_moved` 2,423 次的谱系构成）
>
> **两份均为 AFK 只读口径**：新增两个只读探针 bin（`rust/src/bin/p938_bound_center_openness.rs`、`rust/src/bin/p938_seed_moved_composition.rs`），**既有 `.rs` 一行未改**。

### 缺口浮出时的原文（发生史，照留不抹）

从「全三元组」降到「只 `start_index`」不是无代价的等价替换。**它换掉了一条本 ADR 原来隐含依赖的性质。**

#### 缺口一：键相同不再蕴含价格不变

**原三元组键下**，「键相同」自动意味着 `zd`/`zg` 也相同 ⟹ 中枢核心没动 ⟹ 从 ZD/ZG 算出来的挂价没动。**裁定三「键相同即续挂、qty 冻结、判 maker」是踩在这条隐含性质上的。**

**降到单分量后这条性质没了**：同一个 `start_index` 下 `zd`/`zg` 可以漂——实测 `zd` 从 `2580000000000` 变到 `2571704000000`，**约 −0.32%**（seq=3，见裁定三）。**而限价挂价正是从 ZD/ZG 读的。** ⟹ 键说「这是同一张单，续挂」，价却说「该改了」。**裁定三在这一条上有缺口，本 ADR 不补。**

**仓内有同形先例可直接抄**：`strategy/center_oscillation_trade.rs:619-631` 的 `on_trigger_side_bound`（#487 生产接线入口）——doc `:619` 逐字「开局腿把完整中枢框绑定到挂起；**重复触发只延续首次冻结框**」，函数体 `:626-630` 用 `assert_eq!(CenterId::of(&center), trigger.center())` 强制框与触发身份同源，后续清算按**旧框**判（ADR 补充十四）。⟹ 语义闭合的写法是**「键管身份、冻结框管价格」**：身份键回答「是不是同一张单」，另存一份首次绑定时冻结的四边框回答「挂在什么价」。**本 ADR 只指出这条路存在，不裁要不要走。**

#### 缺口二：更强的载体已经在产，但本 ADR 没用它

两件都是**生产默认开**、不是提案：

- **`CenterOscillationBook::resolve()`**（`center_oscillation_trade.rs:516-522`）：把**当前身份**折回**首次绑定的稳定锚**，未经谱系迁移的身份恒等返回。doc `:518-519` 自称「这是挂起表的**唯一 key 归一入口**：触发、生命周期事件、挂起查询三条路径都先经它」。
- **`LineageBook`**（`lineage_book.rs`）：`consumer_enabled()`（`:82-91`）= `!env_flag(THETA_REBASE_MIGRATE_SKIP)` ⟹ **默认开**；接线 `backtest/fill.rs:1017-1022`；证书缺失 / bar 不匹配 / 目标不在新链上 / 同一旧身份多候选一律 **fail-closed**（doc `:21-22`，明写「禁 fail-open」）。

#929 探针的建议是：裁定三直接以 `resolve()` 后的锚为键，可同时吃到「`zd` 漂移不误撤」与「seed 右移正确撤」两头，且不必自建第二套身份。**该建议本 ADR 未采纳**——它属**改裁**而非订正援引，**归 [#938](https://github.com/xy7365527-lang/NewChanlun/issues/938) 裁。** 本次只把 `zd`/`zg` 移出键，是**最小订正**：它由实测直接强制，不含选择。

**⚠️ 明账**：`start_index` 单分量键是**已知不完备**的中间态，不是终局。它挡住了「核心漂移误撤单」，**没有**回答「核心漂了之后挂价怎么办」（缺口一）。**不许把本次订正读成「键的问题已经解决」。**

> **⚠️ 上面这句「已知不完备的中间态」已被裁定 B 改判。** 裁定 B 以「本窗零分歧」的实测把裸键定为**当前的正式选择**（不是占位），并把翻盘条件写死为三条。改判的是**名分**（中间态 → 正式选择），不是**有效域**（仍限 BTC 前 100 万 bar）。

---

## ★ 裁定 A（补缺口一）：挂单价**跟随**中枢，不冻结

**中枢核心被修订时，挂单价跟着重读；中枢封口后自然固定。**

即：**键管身份、qty 冻结、maker/taker 判定冻结；价不冻结。** 裁定三里的「冻结」二字只覆盖前两者。

### 四条理由，每条都有实测垫底

**1. 必然遇上——绑定的中枢 100% 是开放的，不存在「运气好挂到封口中枢上」这条路。**

`alive_center()` 恒返回链尾（或 `None`）。这是**穷举结论**：`center_lifecycle.rs` 全文对字段 `self.alive` 只有 **8 处**引用（检索式 `grep -n "self\.alive" rust/src/theta_v0/classifier/center_lifecycle.rs`，覆盖该文件全文），其中**写点只有 3 处**，本轮逐条打开确认：

| `file:line` | 写什么 | 是不是链尾 |
|---|---|---|
| `center_lifecycle.rs:586` | `self.alive = Some((*c, idx));`，在 `for (idx, c) in chain.iter().enumerate().skip(k)` 循环体内 ⟹ 循环结束时 `idx == chain.len()-1` | **是** |
| `center_lifecycle.rs:611` | `self.alive = chain.last().map(\|c\| (*c, chain.len() - 1));`（`adopt()`，首次消费 / 重基共用） | **是**（字面 `chain.last()`） |
| `center_lifecycle.rs:742` | `let taken = self.alive.take();`（`take_alive`，教义死亡）⟹ `None` | `None` 不构造触发 |

`None` 那一支走不到绑定：`CenterOscillationTrigger::new` 第一行 `let center = alive.ok_or(TriggerError::CenterNotAlive)?;`（`center_oscillation_trade.rs:156`）。

**实测复核**（BTC 1min 前 100 万 bar，L0–L4）：`alive_center()` 为 `Some` 时其链下标 == `chain.len()-1` 的占比 = **100.000000%，4,677,752 / 4,677,752 级-bar，零反例**。前 20 万 bar 的独立较小窗同样零反例（765,984 / 765,984）。

⟹ **「挂到一个已封口、核心永不再动的中枢上」这条路在结构上不存在。** 缺口一不是理论角落，是每次都会碰上的常态。

**2. 飘 ≠ 标的变了，是认识变准了。**

核心 `zd = max(三段低点)`（`compute_zd`，`center.rs:133-135`）/ `zg = min(三段高点)`（`compute_zg`，`center.rs:125-127`），而段在实时推进中逐步确认——第三段多走几根 K 线后低点才看全，重扫时 `center_from_segments`（`center.rs:216-231`）用**已修订的** frontier 源重新求交，`start_index = a.start_index` 不动。

⟹ 新值不是「这个中枢变了」，是「原值当时没看全」。**挂在旧值上 = 挂在一个已知不准的读数上。**

**3. 高级别最重，而这条链正好跑在高级别。**

`core_drift`（`start_index` 不变、核心被改写）占链尾身份变化的比例**随级别单调上升**：

| level | same | **core_drift** | seed_moved | core_drift 占变化 |
|---|---|---|---|---|
| L0 | 997,531 | 25 | 1,876 | **1.32%** |
| L1 | 997,724 | 151 | 431 | **25.95%** |
| L2 | 991,664 | 62 | 94 | **39.74%** |
| L3 | 976,387 | 15 | 20 | **42.86%** |
| L4 | 711,760 | 5 | 2 | **71.43%** |
| **合计** | **4,675,066** | **258** | **2,423** | **9.62%** |

而中枢震荡短差这条链**从 L1 起跑**：`fill.rs:1223` 逐字 `for lvl in 1..n_levels`，注释 `:1221-1222` 逐字「level 0 无次级别（L0 是递归底），故从 lvl=1 起」。⟹ **缺口最严重的那几级，正是这条链实际工作的那几级。**

**4. 半绑半不绑说不通。**

[ADR 0018](0018-limit-order-fill-model.md) 裁定四已把**寿命**绑给结构（中枢失效即撤单）。价再不绑，订单就成了「寿命看结构、价格看一个过时的数字」——同一张单的两个属性挂在两个不同时刻的世界观上。

### ★ 暂不设「漂多少才改单」的阈值

**理由是实测频次**：`core_drift` = **258 次 / 100 万 bar = 0.026%**。防抖机制（阈值、迟滞、最小改单间隔）本身要引入参数、要测、要维护，**成本大于收益**。

**翻盘条件明写**：若将来实测频次显著上升，**或**按挂单存续期加权后占比变高（bar 次口径 ≠ 订单口径，见有效域），**重议本条**。

### ★ 边界：本裁定不推翻 #487

`center_oscillation_trade.rs:619-631` 的 `on_trigger_side_bound`（#487 生产接线入口）doc `:619` 逐字「开局腿把完整中枢框绑定到挂起；**重复触发只延续首次冻结框**」，函数体 `:626-630` 用 `assert_eq!(CenterId::of(&center), trigger.center(), "挂起四边框必须与触发 CenterId 精确同源")` 强制框与触发身份同源；冻结写入在 `:659-671`（`.and_modify(|frame| if frame.is_none() { *frame = frozen; })`）。

**那个冻结框冻的是清算用的框，冻结是对的。** 它的唯一消费者是 `fill.rs:1072-1077`——`osc_books[lvl].suspended_frames()` 逐个喂给 `classifier::historical_bound_third_cert(...)`，即**拿首次绑定时的旧框右边去配对紧邻三类点**，据此判这笔短差该不该收手回补/终结（注释 `:1061-1062` 逐字「挂起簿给出首次绑定时冻结的完整 `Center`（起止边 + ZD/ZG），再交 classifier 纯结构函数做**旧框右边**的紧邻配对」）。**清算判据必须与开仓时的判据一致，否则同一笔短差的开与平踩在两套标准上。**

⟹ **两件事分开管**：

| | 用哪个框 | 为什么 |
|---|---|---|
| **清算**（这笔短差该不该平） | **旧框**（首次绑定冻结的四边框，#487） | 与开仓时的判断同源 |
| **挂价**（限价单挂在哪个价） | **新框**（当前中枢核心） | 下单是对着当下的结构下的 |

**⚠️ 顺带订正一处读法（照实记）**：#487 那个补丁**是已经为「核心会漂」这个病打过的补丁**，不是「核心不会漂」的证据。`center_oscillation_trade.rs:481-483` 的 doc 自己就写死了这一点——逐字「`CenterId` 刻意不含会随延伸改写的右边，**不能反过来承担四边框快照**」。**主控在本次走票中途曾把它读反**（读成「已有冻结框 ⟹ 核心不会漂 ⟹ 票可关」），方向正好相反，记在这里防止后来人重犯。

---

## ★ 裁定 B（补缺口二）：身份键**保持裸 `CenterId.start_index`**，不改用 `resolve()` 的锚

**#929 探针建议的「以 `resolve()` 后的稳定锚为键」不采纳。** 理由**不是**「差别未知所以选简单的」，是**实测本窗零分歧**。

### 实测：两个键在本窗上逐帧给出同一串决定

把 2,423 次 `seed_moved` 按**生产同源判据**拆开（探针二，与探针一同窗口，`tail_same + core_drift + seed_moved` 逐级、合计均与探针一对得上）：

| | 谱系判**迁移** | 谱系判**重建** |
|---|---|---|
| **真重基 18 次**（`start_index` 变） | **0** | **18（100%）** |
| **核心漂移 258 次**（`start_index` 不变） | **258（100%）** | 0 |

**对角线满、反对角线空，276 次真重基上零例外。** ⟹ 「谱系判迁移」与「起点未变」在本窗**是同一个集合，逐帧重合**。

**两侧的错都算过，不只算一边**：

- **裸键会误撤多少次**（本该保住却撤了 = `seed_moved` 里被谱系判为迁移的）：**0 / 2,423**
- **锚会漏撤多少次**（本该撤却因锚没变而没撤 = `core_drift` 里被谱系判为重建的）：**0 / 258**

### 结构性原因已查清——不是巧合

谱系「迁移 vs 重建」的分界线**只由前三个构造种子的 `(level, ordinal)` 序列决定**：

- `lineage_book::record_edges`（`lineage_book.rs:198`）只收 `rebase_txn::continued_center_pairs` 的产出；
- 后者的门槛 `rebase_txn.rs:757-759` 逐字：

```rust
if e.relation != Relation::Continued1To1
    || !(e.functional && e.injective && e.unique && e.bijective)
{
    continue;
}
```

- 而**种子键**是该输出对象**前三个**源单元的 `(level, ordinal)`——`rebase_txn.rs:245-251` `TxnNode::seed_key`（doc `:243-244` 逐字「前三个源的 lineage（`(level, ordinal)` 三元组）——变换边判定的**种子键**」）：

```rust
fn seed_key(&self) -> Vec<(u32, u64)> {
    self.sources.iter().take(3)
        .map(|s| (s.lineage_id.level, s.lineage_id.ordinal))
        .collect()
}
```

⟹ **`zd`/`zg` 怎么漂都不进这条判据。** 而 `start_index = a.start_index`（`center.rs:228`）= 种子首段起点 ⟹ 种子首段不变则 `start_index` 不变。**两者在构造上本来就耦合**，本窗实测把这条耦合坐实为逐帧重合。

### ★ 前提订正（很重要）：「裸键会在 2,423 次量级上误撤」这个担心不成立

**主控在本次走票中途曾担心**：`seed_moved` 有 2,423 次，裸键在这个量级上会不会大批误撤？**实测答案是不会**，因为那 2,423 次里绝大多数根本不是重基：

| 类 | 次数 | 占比 | 含义 |
|---|---|---|---|
| `advanced` | **2,405** | **99.26%** | `consume_chain` 返回 `Advanced`（`center_lifecycle.rs:563-594`）——**链正常往前长了一节**，新中枢诞生、旧链尾仍在链上、只是被 `Superseded`。`on_chain_rebase_lineage` 根本不会被调 |
| 真重基（`no_cert`） | **18** | 0.74% | `Rebased` + 谱系无证书 ⟹ 保持 `RebaseVanished` 核销 = 判重建 |
| 其余六桶 | 0 | — | `rebased_survived` / `rebased_migrate` / `target_absent` / `ambiguous` / `bar_mismatch` 全 0 |

**而链前进 ⟹ 旧中枢不再 `alive` ⟹ 震荡短差被 `CenterNotAlive` 挡掉（`center_oscillation_trade.rs:156`）⟹ 那时撤单本来就是对的。** 这 2,405 次两个键**都撤，且都撤对**，不构成任何分歧。

真重基按级别拆开：L0 10 次 / L1 7 次 / L2 1 次 / L3 0 / L4 0，**每一级的迁移比例都是 0/n**。消费链跑在 L1+，L1 那 7 次全判重建。**没有出现 `core_drift` 那种「总体 9.62% 但 L4 占 71.43%」的级别反转。**

### ★ 而且 (乙) 可能在结构上就是空的

`anchor_of` **只在挂起表非空时才写**——门二，`center_oscillation_trade.rs:794-796` 逐字：

```rust
let mut tally = RebaseMigrationTally::default();
if self.suspended.is_empty() {
    return (Vec::new(), tally);
}
```

而 `suspended` 的键是 `(VoiceSide, CenterId)`（`center_oscillation_trade.rs:497`），服务的是**中枢震荡短差的挂起**。**限价单走不走这个簿，本次未查。**

若不走 ⟹ `anchor_of` 对限价单永不写入 ⟹ `resolve()` 恒等返回（`:520-522`，`self.anchor_of.get(&id).copied().unwrap_or(id)`，doc `:516` 逐字「未经谱系迁移的身份**恒等返回**」）⟹ **「用锚」≡「用裸键」，锚这个选项在结构上是空的。**

⟹ **单独登记一条实装前必查项：「限价单走不走挂起簿」。** 它**不影响本裁定**（裸键怎么都成立），但它决定「用锚」是不是一个**真实存在的选项**——若不存在，将来再提「改用锚」时要先回答这一条。已登记进「有效域」节。

### ★★ 翻盘条款（三条，照实写死）

**任一条被推翻，本裁定作废、改用锚。**

1. **只测了 BTC 前 100 万 bar**（全史 4,613,599 bar 的 **21.7%**），**其他标的未跑**。跨标的外推无依据（本仓在案：L3 跨标的池化实证，正号仅 BTC）。
2. **没按挂单存续期加权**——「0 次」是 **bar 次**口径，**不是订单口径**。两条探针都不重放买卖点触发、不构造挂单、不模拟撤单/成交。「一张单活了多少 bar、期间碰上几次身份变化」**没有测**。（方向上「超集为 0 ⟹ 子集为 0」这一步外推成立；但**每次误撤/漏撤值多少钱**完全未测。）
3. **前缀守卫有已登记盲区**——`center_lifecycle.rs:529-535` 自陈逐字：「本守卫的**盲区** =『链长不变或增长，且已消费末条身份未变，但**更深的前缀**被改写』。这类分叉本守卫**检不出**……**本窗没有该盲区的证据不等于没有该盲区**（无检出即无观测，检不出的东西也无从计数）」。⟹ **未检出的深层重基可能混在那 2,405 次 `advanced` 里。**

**另附一条口径限制（不构成翻盘条件，但读数时要知道）**：`core_drift` 比的是**相邻 bar 的链尾**。若某中枢在若干 bar 内先被换掉、后又被重基换回，三分法会记成两次 `seed_moved` 而非 `core_drift`——探针**不追踪跨 bar 的中枢个体轨迹**，只做相邻帧差分。

---

## 有效域

- **L0（形态裁定）**：五条均为口径选择，无实测背书。ADR 0018 的 L0 声明（无逐笔、无盘口、成交概率是假设不是实测）全部继承。
- **裁定一的范围声明**：本 ADR **不覆盖 NT 臂的限价下单**。NT 臂的 `entry_tick_for`（`nautilus/strategy.rs:317`）忽略入参恒返 `None`，doc `:316` 自陈「限价腿 entry 价回填待 `plan_orders` 接口扩展」——**这条缺口本 ADR 不补**。
- **裁定二的代价面未穷举**：测试侧 `Order` 构造点数量未知（#926 诚实标注）。
- **裁定三的偏差未量化**：qty 冻结引入的「挂单量 vs 当下 `p_star`」偏差**没有测过量级**。
- **裁定四的差异面未穷举**：顺延路径上 `untradable` bar 是否仍走决策段，未查。
- **裁定四的不可达断言降级**：「同 bar 市价 + 限价并存」只写「已查到的路径上不可达」，非「不会发生」。
- **★ 裁定五的探针已回（[#929](https://github.com/xy7365527-lang/NewChanlun/issues/929)，2026-08-07，本条已从「未知」改判）**：`CenterId.zd/zg` 在中枢延伸时**会不会被改写**，答案是**分两条路径、结论相反**——
  1. **单次扫描内的 Step2 延伸：不改写。** 本 ADR 原写的旁证经第二人复核**成立**：`classifier/recursive_tower.rs:265` doc 逐字「**ZD/ZG 核心冻结**」，实现段 `:748-750` 只写 `c.end_index` / `c.dd` / `c.gg`。**doc 与实现一致，未漂移。**
  2. **但生产不走这条路径。** 生产的「延伸」是 frontier 协议：**每 bar pop 开放中枢 + 从其 seed 起点重扫**（`recursive_tower.rs:710-711`），重扫时 `center_from_segments`（`center.rs:223-230`）用**已修订的** frontier 源重新求交 ⟹ **中枢还活着、`zd`/`zg` 已变、`start_index` 不变**。⟹ **旁证虽真但不承重**：它描述的那条路径在生产上不是延伸的实现方式。
  
  **原「不是穷举」的自我降级现已补齐**：本次订正在 `rust/src/theta_v0/` 全域按六种语法形式复跑检索（字段赋值 / `&mut Center` / 容器可变迭代 / 整体覆写与 `mem::swap|replace|take` / `impl` 块 / 解构式赋值，检索式全表见裁定三），对已有 `Center` 实例的改写**只有 3 处**（`recursive_tower.rs:748`/`:749`/`:750`），全为非身份字段。升级重切路径（原写「本票未查」）**本次已查**，见裁定三新增节。结论：**身份键只能用 `start_index`**。
- **★ [#929](https://github.com/xy7365527-lang/NewChanlun/issues/929) 报告自陈的未能确认项，原样转录（一条不省，090 照实）**：
  1. **九段升级重切的「头子身份保持」是代码逐行读出、不是实测**——wf8 八条案例在被观测的父级层无一触发。（**本 ADR 订正**：`lineage_book.rs:33-35` 记着 seq=24/55 的第三个种子源**在下一级**确实发生了九段分裂 ⟹ 该路径在 wf8 里执行过，只是不在被观测那一级；但比对口径是原始种子 ordinal、非 `CenterId` 逐位相等，仍不构成直接实测。）
  2. **`cargo expand` 未跑**：宏展开产生的 `Center` 字段写入未作机械排除；A–F 是源码层穷举，不是展开后穷举。（**本 ADR 补充、未消除**：`rust/src/theta_v0/` 全域 `macro_rules!` **仅 1 处**——`lineage_book.rs:312` 的 `counters!`，生成 `AtomicU64` 计数器与 `Counters` 结构，与 `Center` 无关；`Center` 的 derive 只有 `Debug, Clone, Copy, PartialEq, Eq`（`types.rs:121`），**无自定义 derive、无 `Deserialize`** ⟹ 无 serde 反序列化写路径。缺口已收窄到「未跑 `cargo expand` 机械确认」这一步，**不写成已排除**。）
  3. **`unsafe`/FFI 写入未作专项核对**。（**本 ADR 已核并关闭该条**：`grep -rnE "\bunsafe\b" rust/src/theta_v0/` 命中数 **0** ⟹ `theta_v0` 全域无 `unsafe`，不存在指针绕过写入。有效域限 `theta_v0`，仓内其他 crate 目录未查。）
  4. **「中枢连续（`continued_1to1` ∧ 四布尔全真）但 `start_index` 变」是否可能，未证亦未反证**——已查到的证据里没有反例，**不是**已证不可能。
  5. **首个脏源来自 parser 线段重分还是下一级窗口重算，未查**——研究文 `:246` 同样标「未能判定」。
  6. **`level_view/projection.rs:157-161` 的「携带核覆盖自算核」是否落在本 ADR 快照的读路径上，未查**。该处在自算核与携带核不等时用 `Center { zd: carried.zd, zg: carried.zg, ..own_center }` 覆盖（`SeedCoreProvenance::InheritedRecut`）⟹ **同一窗口，读 `level_view` 投影 vs 读塔链，`(zd, zg)` 可以不同**。快照若同时消费这两个源，实装时必须钉死读哪一个。
  7. **#929 探针未运行任何测试/回测**（AFK 只读口径）——全部结论来自源码逐行打开 + 已落盘的 wf8 实测报告。上述机检断言（`rebase_txn.rs:1469-1487`）**只读了源码，没有实际执行 `cargo test`**；本次订正同样未跑（纯文档 commit）。
- **裁定三的援引已由第二人独立复核**：本次订正的每一处 `file:line` 均逐条打开确认（不靠 grep 命中数），**订正 1 处路径错**——`rebase_txn.rs` 在 `rust/src/theta_v0/**classifier**/`，不在 `backtest/`。另有 1 处实测口径收紧（wf8 七条里明写核心漂移的是 **5** 条，非 7 条），见裁定三正文。

### ★ 裁定 A / B 的有效域（[#938](https://github.com/xy7365527-lang/NewChanlun/issues/938)，2026-08-07）

**三条翻盘条款（任一条被推翻 ⟹ 裁定 B 作废、改用 `resolve()` 锚；第 1/2 条同时是裁定 A「暂不设阈值」的翻盘条件）：**

1. **只测了 BTC 前 100 万 bar**（全史 4,613,599 bar 的 21.7%，覆盖 2017-08 起连续区间）。**其他标的未跑**，跨标的外推无依据。**不得把 258 / 2,423 / 18 这几个数当全史或跨标的数字外推。**（结构性结论「`alive_center()` 恒为链尾」不受此限——它是 3 个写点的穷举，不依赖窗口长度。）
2. **没按挂单存续期加权**：所有「0 次 / 258 次 / 2,423 次」都是 **bar 次**（链尾身份变化的帧数），**不是订单次数**。两条探针都不重放买卖点触发、不构造挂单、不模拟撤单/成交，**触发绝对次数一条都没数**。「加权后的经济影响」完全未测。
3. **前缀守卫有已登记盲区**：`center_lifecycle.rs:529-535` 自陈「链长不变或增长、末条身份未变、但更深前缀被改写」这类分叉**检不出**。⟹ 未检出的深层重基**可能混在那 2,405 次 `advanced` 里**，本窗无证据说明其规模。

**★ 实装前必查项（单独登记，本裁定不依赖它，但「用锚」是不是真实选项取决于它）：**

> **限价单走不走 `CenterOscillationBook` 的挂起簿？**
> `anchor_of` 的写入受门二 `center_oscillation_trade.rs:794-796`（`if self.suspended.is_empty() { return }`）限制，而 `suspended` 键是 `(VoiceSide, CenterId)`（`:497`），服务的是中枢震荡短差挂起。**本次未查限价单是否进这个簿。** 若不进 ⟹ `resolve()` 对限价单恒等返回（`:520-522`）⟹ 「用锚」≡「用裸键」，该选项在结构上是空的。

**其余未闭合项（两份探针自陈，照实转录，不省）：**

- **理论角落未测**：`last_window_emitted == 0` 时 `confirmed_watermark == len`，此刻链尾在形式上会落进「已封口」一侧。**该角落是否可达、可达时链是否非空，探针没测**（探针检的是 `alive == 链尾`，不是 `alive 下标 ≥ confirmed_watermark`）。**「没测到」≠「不存在」。**
- **`core_drift` 的口径限制**：比的是**相邻 bar 的链尾**，不追踪跨 bar 的中枢个体轨迹。中枢先被换掉、后又被重基换回，会记成两次 `seed_moved` 而非 `core_drift`。
- **非链尾中枢未测**：探针只看链尾。「绑定中枢恒为链尾」已在同窗证到 100%，故对**绑定中枢**这个用途充分；对「任意中枢的身份稳定性」不充分。
- **`consume_chain` 之外的生命周期驱动（`push_point` 死亡事件）未接入探针**：接入后 `alive` 会多出 `None` 的 bar（分母变小），但 `Some` 时仍恒为链尾（结构性结论）——方向不受影响，绝对分母会变。
- **严格读法（`THETA_REBASE_MIGRATE_STRICT=1`）下的构成未跑**：生产默认宽读法，全部读数是宽读法。
- **另挂一票的差异（本 ADR 不裁）**：证书产出点用 `txn_bar = l0.merged_bars.last().source_index`（`classifier/mod.rs:879-880`，doc `:874-875` 自称与 trades/tower_events 同坐标系），生产查簿用回测主循环的 `i`（`fill.rs:1017` 的入参，实参见 `fill.rs:5099`），**实测两者在 41.51%（415,114 / 1,000,000）的 bar 上不相等**。对本裁定无影响（重基那 276 个 bar 上 `txn_bar == i` 恒成立、两种口径逐格相同、`rebased_bar_mismatch` 两边都是 0），但那是**本窗实测**、**不是恒等的证明**（`bar_mismatch` 桶在生产里存在就说明不永真）。**不得据此认定 `fill.rs:1017` 传 `i` 是安全的。**
- **两条 #938 探针未跑任何回测/生产改动**：AFK 只读口径，既有 `.rs` 一行未改；新增两个只读探针 bin。本次 ADR 订正同样未跑（纯文档 commit）。

## 援引复核（2026-08-06，本票交付时逐条打开确认）

本 ADR 与本票下达的 24 条援引全部独立复核（不靠 grep 命中数）。**订正 3 处，结论均不变**——错的全是行号，这正是本仓在案「结论正确、论据是假的」那一类。

| # | 断言 | 实测 |
|---|---|---|
| 6 | 「`px = bar.close as f64 * config.tick.tick_size` 在 `fill.rs:4603`」 | ❌ 实为 **`:4604`**（`:4603` 是 `let bar = &bars[i];`）。ADR 0018:49 引的 `:4604` 是对的 |
| 7 | 「`fee_rate` 在主循环外算一次，`fill.rs:4402-4403`」 | ❌ 实为 **`:4405-4406`**。「主循环外」「含 `slippage_bps`」两点均成立（主循环 `for i in 0..n` 在 `:4602`） |
| 12 | 「W1 段注释在 `fill.rs:5833`」 | ❌ **`:5833` 只是「平单先挂、开单后挂」那一句**（ADR 0018:176 引它引对了）。「决策单源…禁第二裁决源」在 **`:5828-5829`**，「sizing 冻结 / 治 `base_units` 每 bar 重算」在 **`:5830-5832`** |

另有一处**非行号的归属订正**：ADR 0018 的「成交次数趋近 0，ADR 0017 那批规则又一次验不了」在 **`0018:127`**，位置是**裁定四**正文，**不在 §背景**（§背景 = `0018:7-55`）。逐字内容成立，段落归属原断言写错。

其余 21 条逐条打开后与断言一致（`Order` 3 字段 + derive、`schedule_order` 单数全函数、`Bar.high/low` 为 `Tick`、`pending: Vec<Vec<Order>>`、`apply_order` 调用点传 `px`/`fee_rate`、`push` 入队且 `qty>0` 才入、订单单点返回、M4 唯一覆写点且默认 false、★M5/★LEE M1 注释、`fill_bar_index` 顺延、`ConflictKey` 六元组、`sort_by_key` 落在 `plan_orders` 内、NT 适配层两处消费、NT doc 自陈撮合归 venue、`entry_tick_for` 恒返 `None`、`CenterId` 三字段、`HistoricalSeenKey` 已入 `CenterId`、`LegOrder` doc「`Order` 不加字段」、ADR 0018 的 `:169`/`:159`）。

## 援引复核（2026-08-07 第三轮，[#938](https://github.com/xy7365527-lang/NewChanlun/issues/938) 裁定 A/B 落文档时逐条打开确认）

**口径**：本轮所有 `file:line` 与全部数字**都是主控转述、未经第二人核实**，故逐条真的打开确认，不靠印象、不靠 grep 命中数。**基线核对**：`git diff --stat 77a8c9660e HEAD -- rust/src/theta_v0/` 与 `git diff --stat 09ed7224f8 HEAD -- rust/src/theta_v0/` **均无输出**（两条探针各自的基线到本次落文档的 HEAD 之间，`theta_v0/` 全域零改动）⟹ **两份报告的行号读数在本次落盘时仍然有效**。

| # | 断言 | 复核结果 |
|---|---|---|
| 1 | `alive_center()` 恒为链尾的三个写点 = `center_lifecycle.rs:586` / `:611` / `:742` | ✓ 三处全对。`:586` 在 `.skip(k)` 循环体内；`:611` 字面 `chain.last()`；`:742` 是 `take_alive` 的 `self.alive.take()`。`grep -n "self\.alive" …/center_lifecycle.rs` 命中 8 行（`:570`/`:586`/`:607`/`:611`/`:682`/`:742`/`:777`/`:879`），其中写点确为 3 处 ⟹ **穷举成立** |
| 2 | `rebase_txn.rs:245-252` `seed_key` | **⚠️ 行号微收**：`fn seed_key` 实为 **`:245-251`**（`:252` 是空行；doc 在 `:243-244`）。**内容成立**：`.take(3)` + `(s.lineage_id.level, s.lineage_id.ordinal)`。本 ADR 正文已按 `:245-251` 写 |
| 3 | `rebase_txn.rs:757-759` 收单条件 | ✓ 逐字对上：`if e.relation != Relation::Continued1To1 \|\| !(e.functional && e.injective && e.unique && e.bijective)` 恰好跨 `:757-759` |
| 4 | `center_oscillation_trade.rs:794-796` 挂起表非空门 | ✓ `:794` `if self.suspended.is_empty() {` / `:795` `return (Vec::new(), tally);` / `:796` `}` |
| 5 | `center_oscillation_trade.rs:619-631` #487 冻结框 | ✓ doc `:619` 逐字、`pub fn on_trigger_side_bound` `:620-625`、`assert_eq!` `:626-630`、`:631` 调 `on_trigger_side_impl(..., Some(center))`。**补充查明**（原转述未给）：冻结的**写入**在 `:659-671`，冻结框的**唯一消费者**是 `fill.rs:1072-1077` → `historical_bound_third_cert`（清算侧），据此坐实「冻的是清算用的框」这一读法 |
| 6 | `center_lifecycle.rs:529-535` 自陈前缀盲区 | ✓ 逐字对上，含「**本窗没有该盲区的证据不等于没有该盲区**（无检出即无观测）」 |
| 7 | `issue929-…md:134` 行号错 ＋ `classifier/mod.rs:1376-1415` / `:1937-2001` 实际内容 | ✓ **错确实存在**。`:1376-1415` 是 frontier pop（`:1380` `pop_n`、`:1402-1403` `truncate`），`:1937-2001` 是 `extract_second_for_level`（`:1939`）/ `second_for_parent`（`:1967`），区间内无 pop/truncate/win_start。`git diff --stat d6e9fd12e2 HEAD -- …/classifier/mod.rs` 无输出 ⟹ 排除「写时对、后来漂」 |
| 8 | 两份探针报告的数字（258 / 2,423 / 2,405 / 18 / 4,677,752 / 71.43% / 1.32%） | ✓ 全部与报告正文一致，且**逐级求和自洽**：25+151+62+15+5 = **258**；1,876+431+94+20+2 = **2,423**；2,405+18 = **2,423**；25/(25+1,876) = **1.32%**；5/(5+2) = **71.43%**；258/(258+2,423) = **9.62%**；258/1,000,000 = **0.026%** |

**★ 本轮自己查出、上游转述里没有的一处**：探针一报「4,677,752」（`alive_center()` 的级-bar 分母），探针二 §5.1 报「4,677,747 帧链尾比对」——**两个数不同不是矛盾**，差 **5** = 5 个级别各自的首 bar 没有「上一 bar」可差分。且探针一 §3.3 的 `4,675,066 + 258 + 2,423 = 4,677,747` 逐位对上。**两数各有所指，都对；引用时不可混用。**

## 移交

- **`CenterId.zd/zg` 延伸稳定性探针** → **已开、已回、已关：[#929](https://github.com/xy7365527-lang/NewChanlun/issues/929)**（挂 map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)）。报告 `.chanlun/review-results/issue929-centerid-stability-20260807.md`（commit `d6e9fd12e2`）。**结论：身份键只能用 `start_index`，`zd`/`zg` 不可入键**——裁定三已按此订正，键分量不再是「未定」。
- **★ 键相同 ≠ 价格不变（缺口一）＋ 是否改用 `resolve()` 锚为键（缺口二）** → **[#938](https://github.com/xy7365527-lang/NewChanlun/issues/938)：已开、已回、已裁**（2026-08-07）。两条探针报告：`.chanlun/review-results/issue938-center-open-or-sealed-20260807.md`、`.chanlun/review-results/issue938-seed-moved-composition-20260807.md`。**结论**：缺口一 → **裁定 A**（挂单价跟随中枢，不设阈值）；缺口二 → **裁定 B**（保持裸 `start_index`，#929 探针的「改用 `resolve()` 锚」建议**经实测后不采纳**，理由是本窗零分歧而非未知）。全文见「订正后浮出的缺口」节。
- **★ 限价单走不走挂起簿** → **实装前必查项**，见「有效域 § 裁定 A/B 的有效域」。它不影响裁定 B，但决定「用锚」是不是一个真实存在的选项。
- **★ 证书 bar 坐标（`classifier/mod.rs:879-880` 的 `txn_bar`）vs 生产查簿 bar（`fill.rs:1017` 的入参 `i`）在 41.51% 的 bar 上不等** → 探针二顺带查出，**本 ADR 不裁**，建议另挂一票。
- **实装 spec** → 探针已回、缺口已裁，可出，**出图**（#787 是纯决策图，实装票不挂本图）。**前置已更新**：不再是「等 #938」，而是「实装 spec 必须按裁定 A 写成**挂价跟随、不冻结**」，且**不得**把「键相同 ⟹ 挂价不变」当成已成立（那条性质已被裁定 A 明确否掉）。
- **NT 臂限价下单** → 本 ADR 不覆盖，另开线，不挂 #787。
- **`pending` 队列的第二套机制债** → 裁定四已命名，随实装线一并处置或另挂 `debt`。
