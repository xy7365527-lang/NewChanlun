# 盘整走势类型终结与第一类买卖点只读考证

- 拟归档路径：`.chanlun/review-results/consolidation-termination-research-20260727.md`
- 研究方式：仅静态读取当前工作树代码与博文原文；未修改文件，未运行测试或回测。
- 权威口径：博文原文优先；代码结论仅描述当前实现，不反推为教义。

## 一句话结论

**[代码可证]** 当前生产分类链中，单中枢盘整没有 `prev_center`、没有趋势门、也无法取得趋势 `a_seg`，因此不能产出 `buy1/sell1`；盘整背驰另有 `PanDivCert`、Consolidation Nest 事件及 `pan_div_diag` 路径，但不置一类 bit。**[原文可证]** 标准第一类点锚定趋势背驰，普通盘整背驰本身不保证走势或中枢终结；回中枢则盘整继续，不回中枢才转成本级三类点。**[未能判定]** 原文没有把“走势完成”“三类点破坏”“九段升级”“中枢扩展”“在场取代”统一成一种死亡事件，#474 的盘整生命周期仍需单独裁定。

## A1．第一类 bit 的全部产出路径

### A1.1 唯一实际置位原语

**[代码可证]**

`EndpointSituation::is_first()` 要求：

- `below_last_center=true`
- `after_first_buy=false`
- `left_center=false`

`endpoint_to_bsp()` 随买卖侧把结果写入 `buy1` 或 `sell1`。锚点：

- `rust/src/theta_v0/classifier/bsp.rs:73-109`

当前 `signal.rs` 的生产调用中：

- `signal.rs:392` 是第一类路径；
- `signal.rs:451,469` 是第三类路径；
- `signal.rs:881` 是第二类路径；
- `signal.rs:3241` 位于测试代码。

因此，在结构分类器的生产路径里，能令 `below_last_center` 按第一类语义成立的来源是 `judge_first_cached()`。

`signal_bits_of()` 只是 `endpoint_to_bsp()` 的薄包装，没有新增结构判据，且当前非测试代码没有调用：

- `rust/src/theta_v0/classifier/six_state.rs:79-85`

### A1.2 `judge_first_cached`

**[代码可证]**

该函数的必要链为：

1. 当前段方向必须等于趋势方向，并严格突破最后中枢核心；
2. `a_seg` 必须为 `Some`；
3. C 还必须突破 A 段包络极值；
4. A/C 区间必须能映射到 MACD；
5. `AbcDivergence.is_trend=true`；
6. 背驰确认才把 `below_last_center` 置为真，继而写入 `buy1/sell1`。

关键代码：

> `let Some(...) = a_seg else { return None; }`

锚点：

- `rust/src/theta_v0/classifier/signal.rs:287-355`
- `rust/src/theta_v0/classifier/signal.rs:378-401`

所以 `a_seg=None` 时，连零 bit 的结构候选也不会返回，更不可能产生第一类 bit。

### A1.3 各调用链的 `a_seg` 传递

| 路径 | `a_seg` 来源与传递 | 单中枢结果 |
|---|---|---|
| L0 批式 | `mod.rs:450` → `extract_signals_with_hist` → anchored extractor → `judge_segment`；缓存类型为 `Option<...>`，由 `locate_departure_move_a(prev_center,last_center)` 产生并原样传给 `judge_first_cached` | 无趋势门，不进入第一类分支 |
| L≥1 批式 | `mod.rs:456` → `extract_first_third_for_level` → units 转 Segment → 同一个 anchored extractor | 与 L0 相同；无独立一类判据 |
| L0 增量 | `mod.rs:2079` → `extract_first_third_resume` → pointwise gate → 同一 `judge_segment` | 无趋势门，不进入 |
| L≥1 增量 | `mod.rs:2091` → resume；虽然传入 `units_anchors`，第一类 A/C 定位实际使用 `anchors_self` | 无趋势门，不进入 |
| `recursive_tower` | `level_cand_delta` 先取趋势门和 `prev_center`，再计算可选 `a_seg_entry`，传同一个 `judge_first_cached` | 无门时只可能产盘背诊断；`cand_delta=false`，不产一类 bit |
| `level_view` | `provide_divergence_pairs` 对 `block.dir=None` 直接跳过；有趋势时 `seg_a=None` 也直接跳过 | 该模块本身不写 `BspBits` |
| 泛型投影 API | 调用者可人工构造 `EndpointSituation` 或 `BspBits` | 不属于市场结构识别，不能证明单中枢可识别一类点 |

关键锚点：

- `rust/src/theta_v0/classifier/signal.rs:987-1125`
- `rust/src/theta_v0/classifier/signal.rs:1150-1255`
- `rust/src/theta_v0/classifier/signal.rs:1271-1348`
- `rust/src/theta_v0/classifier/mod.rs:262-295`
- `rust/src/theta_v0/classifier/mod.rs:438-463`
- `rust/src/theta_v0/classifier/mod.rs:2068-2097`

`judge_segment()` 有双重保险：

- 只有 `gate_dir=Some((pos,dir))` 才判第一类；
- `prev_center=centers_sorted[pos-1]`；
- `locate_departure_move_a(...)` 的结果仍是 `Option`；
- `None` 原样进入 `judge_first_cached` 后被拒绝。

### A1.4 单中枢为何没有趋势门

**[代码可证]**

单中枢被明确分解为：

- `MoveKind::Consolidation`
- `dir=None`
- 无任何相邻中枢关系

`center_trend_gate()` 只给 Trend 块中从第二个中枢开始的槽位置方向，因此单中枢的 gate 必为 `None`：

- `rust/src/theta_v0/classifier/decompose.rs:128-168`

这意味着单中枢甚至到不了 `prev_center=pos-1`，不是只靠叶函数的 `a_seg=None` 才被挡住。

### A1.5 `level_view` 严格合取链不是另一条一类 bit 路径

**[代码可证]**

`level_view` 的趋势资格要求：

- B 中枢内 DIF 回零轴；
- C 内三类点；
- C 破 A 包络极值；
- MACD 面积、DIF 峰、柱峰至少一种衰减；
- `seg_a` 必须成功定位。

但它输出的是 `DivergencePair`、`NestCandidateEvent` 和走势完成状态，不构造 `BspBits`：

- `rust/src/theta_v0/classifier/level_view.rs:528-635`
- `rust/src/theta_v0/classifier/level_view.rs:728-814`
- `rust/src/theta_v0/classifier/level_view.rs:903-936`

盘整分支另产 `NestDivergenceKind::Consolidation`，同样不写第一类 bit：

- `rust/src/theta_v0/classifier/level_view.rs:817-893`

### A1.6 `recursive_tower` 不是第二套判据

**[代码可证]**

该模块明确委托 `judge_first_cached/judge_pan_div`，不自建第一类判据：

- `rust/src/theta_v0/classifier/recursive_tower.rs:1021-1032`

趋势门缺失时，盘整分支只能产生：

- `cand_delta=false`
- `pan_div_diag=true`
- 无 `BspPoint`
- 无一类 bit

有趋势门时才取得 `prev_center`、可选 `a_seg_entry`，调用同一叶函数；`cand_delta` 最终仍由 `pf.bits.buy1 || pf.bits.sell1` 派生：

- `rust/src/theta_v0/classifier/recursive_tower.rs:2088-2164`

### A1.7 全仓机械构造的边界

**[代码可证]**

若把“产出”解释成“任何代码能够构造一个含一类位的值”，则存在不读中枢的机械路径：

- `endpoint_to_bsp()` 接受调用者提供的 `EndpointSituation`；
- `BspBits::from_class_index()` 可从 0–63 索引还原任意位；
- 离线 dump 复现器可按已记录的主类重建一类位；
- 测试夹具大量直接构造 `BspBits { buy1: true, ... }`。

锚点：

- `rust/src/theta_v0/types.rs:248-261`
- `rust/src/theta_v0/backtest/wverify_run.rs:2528-2563`
- 各 classifier 测试模块的 `#[cfg(test)]` 边界

这些是投影、反序列化或测试数据，不是由单中枢行情结构识别出第一类点。

另一个边界是：`judge_first_cached` 的签名只接收计算好的 `a_seg`，不接收 `prev_center`，所以“该 A 确实属于前中枢”没有编码进叶函数类型；但当前两个非测试调用位置都在趋势门后从真实 `prev_center` 计算它。

### A1 结论

**[代码可证]** 当前所有生产分类路径中，没有任何一条能在单中枢、无 `prev_center`、无趋势 A 段的结构下产出 `buy1/sell1`。

## A2．双中枢趋势资格是否检查

**[代码可证]**

相邻中枢关系使用外缘 `dd/gg`：

- `next.dd > prev.gg` → `UpContinuation`
- `next.gg < prev.dd` → `DownContinuation`
- 其余，即外缘有重叠 → `LevelExpansion`

锚点：

- `rust/src/theta_v0/classifier/center.rs:93-103`
- `rust/src/theta_v0/classifier/center.rs:270-281`

走势分解随后映射为：

- Up/Down continuation → `MoveKind::Trend`
- `LevelExpansion` → `MoveKind::Consolidation`

锚点：

- `rust/src/theta_v0/classifier/decompose.rs:1-5`
- `rust/src/theta_v0/classifier/decompose.rs:54-73`

因此代码确实检查了“双中枢外缘完全分离才取得趋势资格；重叠关系不进趋势门”。

“同级别”不是 `classify_relation()` 内部的字段检查：`Center` 没有 level 字段；它依靠每个 `LevelState` 分别持有本级中枢链这一调用不变量。下游 `full_trend_qualification_evidence()` 还会拒绝 `LevelExpansion`，并检查递归组件 ID 同级且连续：

- `rust/src/theta_v0/classifier/recursive_tower.rs:1566-1610`

**[代码可证]** `LevelExpansion` 在此处只是“盘整/升级候选”标签；`classify_relation()` 本身不负责证明或构造一个已完成的更大级别中枢。

## A3．Reset 的 bit 触发源

**[代码可证]**

`center_lifecycle::push_point()` 只把以下位认作 Reset 候选：

- `buy1` → `Side::Long`
- 否则 `sell1` → `Side::Short`

随后经目标解析：

- 合法在场、容读目标或场为空 → `Reset`
- 过期目标可返回 `Stale`，并非每个携一类位的输入都无条件生成事件

优先级为：

1. 一类位
2. 三类位
3. 其他位

因此：

- 一类和三类同时为真时，一类优先，产 Reset；
- `buy1` 与 `sell1` 同时为真时，代码先取 `buy1`；
- `buy2/sell2` 不产生命周期事件；
- `buy3/sell3` 产 `Broken`，不产 Reset。

锚点：

- `rust/src/theta_v0/classifier/center_lifecycle.rs:551-624`

当前生命周期还存在链推进产生的 `Superseded`：

- `Broken`：三类点破坏
- `Reset`：一类点同死
- `Superseded`：新中枢成交后旧在场实例被取代

锚点：

- `rust/src/theta_v0/classifier/center_lifecycle.rs:247-315`
- `rust/src/theta_v0/classifier/center_lifecycle.rs:491-507`

所以“代码里的中枢只死于三类点”不是完整描述；但对单中枢的点信号来源而言，由于 A1 证明它不能自产一类位，点驱动的可达教义破坏仍只剩三类点，另有结构链驱动的 `Superseded`。

## A4．盘整背驰是否有独立信号路径

**[代码可证]**

不是完全不识别。当前代码有独立的：

1. `PanDivCert`
2. `LevelState.pan_div`
3. `NestDivergenceKind::Consolidation`
4. `recursive_tower` 的 `pan_div_diag`

其中 `PanDivCert` 明确：

> 不置任何 six-bit，不产 `BspPoint`，不冒充同级 B1/S1。

锚点：

- `rust/src/theta_v0/classifier/signal.rs:605-631`
- `rust/src/theta_v0/classifier/mod.rs:203-207`
- `rust/src/theta_v0/classifier/level_view.rs:817-893`
- `rust/src/theta_v0/classifier/recursive_tower.rs:2098-2133`

但“无 A/C 配对”需要收窄表述：

- 它不要求趋势路径的 `prev_center + last_center`；
- 它仍要求盘整语义下的 A/A′ 与 C 力度比较；
- 窄锚取同一中枢前一次同向离开；
- 窄锚失败时可回退到中枢前最近同向段 A′；
- 无 A/A′ 或 MACD 区间不可映射仍返回 `None`。

锚点：

- `rust/src/theta_v0/classifier/signal.rs:652-708`
- `rust/src/theta_v0/classifier/signal.rs:721-747`
- `rust/src/theta_v0/classifier/signal.rs:774-819`

另有一个明确覆盖边界：

**[代码可证]** 两个盘背结构定位器均首先要求 C 端点突破中枢核心，即向下 `<zd` 或向上 `>zg`。第24课所述“C 不破中枢但 C 面积小于 A”的盘背不会通过当前 `PanDivCert`/Consolidation Nest 路径被识别。

因此当前实现是“盘整背驰有独立证书路径，但只覆盖破核心的一部分盘背结构”，不是完整覆盖第24课全部情形。

## B1．盘整背驰点是否等于第一类买卖点

### 早期标准背驰前提

**[原文可证]** 第16课强调：

> “一定要结合趋势来。记住，没有趋势没有背驰……背驰是两个趋势之间比较才有意义，和盘整里比较是没用的。”

`docs/chanlun/text/blog/016-第16课.md:114-122`

以及：

> “在一个盘整中也找什么第一类买点，那肯定要出问题的。”

`docs/chanlun/text/blog/016-第16课.md:564-572`

这是盘整背驰概念被单独命名前，对“标准背驰—第一类点”的限定。

### 盘整背驰被正式分出

**[原文可证]** 第24课明确区分：

> “一般不特别声明的，背驰都指最标准的趋势中形成的背驰。而盘整用，利用类似背驰的判断方法……称为盘整背弛判断。”

`docs/chanlun/text/blog/024-第24课.md:32-36`

第37课进一步定稿：

> “背驰，必须在趋势中，因为背驰意味这一个趋势的结束，而盘整背驰不一定，可能还是同一个走势类型里。”

`docs/chanlun/text/blog/037-第37课.md:148-152`

第53课称盘整背驰为对中枢震荡力度比较的“推广用法”：

- `docs/chanlun/text/blog/053-第53课.md:24-28`

### 第一类点与类第一类点

**[原文可证]** 第27课逐字：

> “第一类买点肯定是趋势背驰构成的，而盘整背驰构成的买点……在大级别里，这也构成一种类似第一类买点的买点……”

并规定：

> “这个级别，至少应该是周线以上。”

`docs/chanlun/text/blog/027-第27课.md:64-66`

第101课以买卖点总称再次归纳：

> “第一类买卖点就是背驰点，第三类买卖点就是中枢破坏点。”

`docs/chanlun/text/blog/101-第101课.md:16-22`

结合第24、37课中“背驰”与“盘整背驰”的明确区分，可证普通盘背不等于标准第一类点。

**[未能判定]** 原文没有授权把周线以上“类第一类买点”直接写入普通 `buy1/sell1` 位；“类似”“类第一类”不是“第一类”的同义词。

## B2．单中枢盘整完成、终结和后续路径

### B2.1 完成资格不等于立即死亡

**[原文可证]** 第17课定义：

> “某完成的走势类型只包含一个缠中说禅走势中枢，就称为……盘整。”

`docs/chanlun/text/blog/017-第17课.md:42-46`

第18课说明：

> “只要三个重叠的连续次级别走势类型走出来后，盘整随时结束都是完美的，但这可以不结束，可以不断延伸下去……”

`docs/chanlun/text/blog/018-第18课.md:38-42`

因此三段重叠给出完成资格，但不是强制终结时点。

### B2.2 严格的中枢破坏路径：三类点

**[原文可证]** 第18课定理三：

> “某级别‘缠中说禅走势中枢’的破坏，当且仅当一个次级别走势离开该‘缠中说禅走势中枢’后，其后的次级别回抽走势不重新回到该‘缠中说禅走势中枢’内。”

`docs/chanlun/text/blog/018-第18课.md:62-66`

第20课把该结构命名为第三类买卖点：

> “一个次级别走势类型向上离开……回试，其低点不跌破ZG，则构成第三类买点；……向下离开……回抽，其高点不升破ZD，则构成第三类卖点。”

`docs/chanlun/text/blog/020-第20课.md:54-62`

第37课答疑：

> “只要走出第三类买卖点，自然就结束。”

`docs/chanlun/text/blog/037-第37课.md:251-265`

所以，若“死亡”严格指“中枢被破坏”，三类点结构是原文给出的充要条件。

### B2.3 中枢的完整命运分类

**[原文可证]** 第23课：

> “中枢有三种命运：延伸、扩展、新生。”

并解释：

> “不是中枢的延伸，就是中枢的扩展，也就是产生高级别的走势中枢；或者中枢的新生，就是趋势及延续。”

`docs/chanlun/text/blog/023-第23课.md:184-190,294-306`

对应关系是：

- 延伸：原中枢继续维持，不是终结；
- 扩展：吸收为更大级别中枢；
- 新生：产生新的同级中枢，形成趋势及延续。

第53课把中枢结束后的下游压缩为两类：

> “转成更大的中枢或上涨下跌直到形成新的该级别中枢。第三类买卖点就是告诉什么时候发生这种事情的。”

`docs/chanlun/text/blog/053-第53课.md:28-32`

第21课同样说：

> “中枢扩张导致一个更大级别的中枢，而中枢新生，就形成一个上涨的趋势。”

`docs/chanlun/text/blog/021-第21课.md:28-36`

卖点方向镜像。

### B2.4 九段延伸升级

**[原文可证]** 课号为第33课：

> “5分钟级别的中枢不断延伸，出现9段以上的1分钟次级别走势……一旦出现6段的延伸，加上形成中枢本身那三段，就构成更大级别的中枢了。”

`docs/chanlun/text/blog/033-第33课.md:14-18`

这是按结合律升级、吸收成更大级别中枢的路径。

**[未能判定]** 原文没有把九段升级称为旧中枢的“破坏”或“死亡”，也没有赋予它三类点的离开—回抽不回条件；不得与 `Broken` 直接画等号。

### B2.5 相邻中枢扩展为更大级别中枢

**[原文可证]** 第20课给出：

> “其后的走势有两种情况：一、该走势中枢的延伸。二、产生新的同级别走势中枢。”

若新旧同级中枢外围波动重叠：

> “由此产生更大级别的走势中枢。”

`docs/chanlun/text/blog/020-第20课.md:16-44`

中心定理二进一步把相邻中枢外缘分离与重叠区分为趋势及高级别中枢：

- `docs/chanlun/text/blog/020-第20课.md:54-58`

### B2.6 盘整背驰后的分叉

**[原文可证]** 第24课完整分叉是：

1. C 不破中枢、发生盘背：

   > “其后必定有回跌。”

   但没有继续规定最终走势类型。

2. C 已破中枢，回跌不重新进入：

   > “刚好这反而构成该级别的第三类买点。”

3. C 已破中枢，回跌重新进入：

   > “反之就继续该盘整。”

`docs/chanlun/text/blog/024-第24课.md:32-46`

因此：

- 回中枢不是终结；
- 不回中枢才通过本级三类点确认中枢破坏；
- 三类点之后再分扩展为更大中枢或中枢新生／趋势。

### B2.7 第29课三分支不能直接移植给盘背

**[原文可证]** 第29课定理的主语明确是“某级别趋势的背驰”：

> “某级别趋势的背驰将导致该趋势最后一个中枢的级别扩展、该级别更大级别的盘整或该级别以上级别的反趋势。”

并先说明：

> “在某级别的盘整中……不存在转折的问题，除非站在次级别图形中……”

`docs/chanlun/text/blog/029-第29课.md:14-26`

该课还专门区分：

> “这种情况和盘整背驰中转化成第三类卖点的情况不同……”

`docs/chanlun/text/blog/029-第29课.md:28-34`

**[未能判定]** “最后中枢扩展／更大级别盘整／反趋势”不能直接写成普通盘背的三种必然后果。盘背最终形成三类点后，可以再进入扩展或新生，但这不是第29课趋势背驰定理的直接适用。

### B2.8 分解规则差异

**[原文可证]** 第38课指出，固定同级别分解可以不使用中枢延伸、扩展概念：

> “如果这5分钟次级别延伸成6段，那么就当成两个30分钟盘整类型的连接……是允许盘整+盘整情况的。”

`docs/chanlun/text/blog/038-第38课.md:20-26`

**[未能判定]** 不先锁定“中枢递归分解”还是“固定同级别分解”，无法形成唯一、分解无关的“盘整死亡事件全集”。

## B3．“走势必完美”的约束

**[原文可证]** 第17课首次正式提出：

> “任何级别的任何走势类型终要完成。后面一句用更简练的话，就是‘走势终完美’。”

并解释：

> “趋势终完美，盘整也终完美。”

> “任何走势，无论是趋势还是盘整，在图形上最终都要完成。另一方面，一旦某种类型的走势完成以后，就会转化为其他类型的走势。”

`docs/chanlun/text/blog/017-第17课.md:20-36`

最低结构约束为：

- 完成走势至少包含一个中枢；
- 任何走势类型至少由三段以上次级别走势构成。

`docs/chanlun/text/blog/017-第17课.md:50-58`

第24课据此判断尚未形成内部中枢的 C 段“肯定没完”：

- `docs/chanlun/text/blog/024-第24课.md:40`

但第17课同时明确，当下究竟继续延伸还是改变不能预先判定：

- `docs/chanlun/text/blog/017-第17课.md:30`

第102课进一步把“走势必完美”解释为有级别的递归唯一分解和完全分类，而不是新增一个终结信号：

- `docs/chanlun/text/blog/102-第102课.md:20-44`

**[未能判定]**

“走势必完美”没有提供：

- 有限时间上界；
- 固定终结时刻；
- 必须由一类点、三类点或某一 bit 终结的规定；
- 九段升级如何登记旧中枢死亡的生命周期语义。

它只能证明盘整最终必须完成并转化，不能替代具体的当下结构确认。

## B4．盘整背驰翻转的级别位置与中枢后果

### 级别位置

**[原文可证]**

普通盘背首先是所选本级中枢震荡中的力度比较：

- `docs/chanlun/text/blog/053-第53课.md:24`

但精确定位可发生在更低级别。第24课明确：

> “在次级别的第一类买点回补，刚好这反而构成该级别的第三类买点。”

`docs/chanlun/text/blog/024-第24课.md:32-36`

因此同一转折区域可以同时具有：

- 次级别第一类点；
- 本级第三类点；

但不能因此把它改名为本级第一类点。

第31课也说明单中枢转折有两重结构：

> “一是A与C之间的比较产生的盘整背弛，二是C内部的次级别背弛……两者有着类似区间套的关系。”

`docs/chanlun/text/blog/031-第31课.md:652-656`

### 对本级中枢生死的后果

**[原文可证]**

盘背本身不保证本级走势终结：

> “盘整背驰不一定，可能还是同一个走势类型里。”

`docs/chanlun/text/blog/037-第37课.md:148-152`

第38课进一步区分：

> “背驰、盘整背驰，都是走势分段的依据，所谓第三类买卖对盘整结束的确认……”

`docs/chanlun/text/blog/038-第38课.md:252-258`

所以：

- 盘背可作为分段或候选转折依据；
- 回到中枢，原盘整继续；
- 不回中枢，本级三类点确认破坏；
- 至少周线级别以上可称“类第一类”，仍不是普通第一类。

第88课有条件式表述：

> “如果说前一个走势类型的背驰或盘整背驰宣告了前一个走势类型的死亡……”

`docs/chanlun/text/blog/088-第88课.md:18-20`

但该句是“如果说”的结束语境，不能覆盖第24、37课明确存在的“盘背后仍为同一盘整”情形。

**[未能判定]**

原文没有授权以下等式：

- `PanDivCert = 本级 buy1/sell1`
- `盘整背驰 = 本级 Reset`
- `周线以上类第一类 = 普通一类 bit`
- `任一盘背 = 中枢立即死亡`

## 未能判定清单

1. **[未能判定]** “盘整走势完成”“盘整延伸结束”“中枢破坏”“升级吸收”“在场取代”应否映射为同一个生命周期出口。
2. **[未能判定]** 九段升级或相邻中枢扩展时，旧中枢应登记 `Broken`、`Superseded`、新设 Upgrade，还是只改变上级结构解释。
3. **[未能判定]** 周线以上“类第一类”是否需要独立 bit；原文不足以授权复用普通 `buy1/sell1`。
4. **[未能判定]** `PanDivCert` 是否应直接触发生命周期事件；原文只授权分段依据和经三类点确认破坏。
5. **[未能判定]** 当前代码遗漏“C 不破中枢”的第24课盘背，是有意收窄还是尚未实现。
6. **[未能判定]** #474 所谓“全部终结路径”采用中枢递归分解还是第38课固定同级别分解；两者的完成表述不同。
7. **[未能判定]** `classify_relation()` 的同级别前提是否需要进入类型系统；当前仅由调用容器保证。

## 引用锚汇总

### 代码锚

- `rust/src/theta_v0/classifier/bsp.rs:73-109`
- `rust/src/theta_v0/classifier/signal.rs:220-258,287-402`
- `rust/src/theta_v0/classifier/signal.rs:605-819`
- `rust/src/theta_v0/classifier/signal.rs:962-1125,1150-1348`
- `rust/src/theta_v0/classifier/mod.rs:262-295,438-463,2068-2097`
- `rust/src/theta_v0/classifier/center.rs:93-103,270-281`
- `rust/src/theta_v0/classifier/decompose.rs:1-73,128-188`
- `rust/src/theta_v0/classifier/level_view.rs:528-635,728-936,1097-1176`
- `rust/src/theta_v0/classifier/recursive_tower.rs:1021-1032,1167-1215,1531-1610,2088-2164`
- `rust/src/theta_v0/classifier/center_lifecycle.rs:247-315,491-507,551-624`
- `rust/src/theta_v0/types.rs:248-261`

### 原文锚

- 第16课：`docs/chanlun/text/blog/016-第16课.md:114-122,564-572`
- 第17课：`docs/chanlun/text/blog/017-第17课.md:20-58`
- 第18课：`docs/chanlun/text/blog/018-第18课.md:38-66`
- 第20课：`docs/chanlun/text/blog/020-第20课.md:16-62`
- 第21课：`docs/chanlun/text/blog/021-第21课.md:28-36`
- 第23课：`docs/chanlun/text/blog/023-第23课.md:184-190,294-306`
- 第24课：`docs/chanlun/text/blog/024-第24课.md:22-46`
- 第27课：`docs/chanlun/text/blog/027-第27课.md:14-24,64-66`
- 第29课：`docs/chanlun/text/blog/029-第29课.md:14-38,52`
- 第31课：`docs/chanlun/text/blog/031-第31课.md:652-656`
- 第33课：`docs/chanlun/text/blog/033-第33课.md:14-18`
- 第37课：`docs/chanlun/text/blog/037-第37课.md:148-152,251-265`
- 第38课：`docs/chanlun/text/blog/038-第38课.md:20-26,252-258`
- 第53课：`docs/chanlun/text/blog/053-第53课.md:24-32`
- 第88课：`docs/chanlun/text/blog/088-第88课.md:18-20`
- 第101课：`docs/chanlun/text/blog/101-第101课.md:16-22`
- 第102课：`docs/chanlun/text/blog/102-第102课.md:20-44`


