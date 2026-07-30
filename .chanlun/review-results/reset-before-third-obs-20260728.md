# #565 讣告抢跑观测：Reset 早于三类点逐案归因

- 日期：2026-07-28
- 票据：#565
- 仓库快照：`main@155a5db845feb72b08db0bb112aa7862d3560d6b`
- #489 终态：merge `5bce46f366` 可由本快照到达
- 性质：纯观测；未改主仓生产代码

## 0. 结论

按“Reset 是事件、同 bar 内以实际生产路由顺序判先后”的主口径，58 条漏发分为：

| 类别 | 数量 | 含义 |
|---|---:|---|
| (a) Reset 事件前已有相关三类点 | 24 | 层级错位 20；同级归属他框 4 |
| (b) C 截至 wf8 窗末从未产 exact 点 | 3 | 紧邻对价格失败 1；结构后来通过但 C 不在挂起候选、regular 归他框 2 |
| (c) Reset 后才产 exact 点 | 31 | 31 条最终均路由为 `Broken` |
| 合计 | 58 | 与 `reset_alive_center_leak` 58 条账平 |

这里的 `exact` 专指生产生命周期键 `(level, CenterId)`，其中 `CenterId=(si,zd,zg)`；它不等价于 ADR 的四边冻结框。31 条 (c) 中，只有 3 条晚到点的 `owner_ei` 与 Reset 见证的冻结 `died_ei` 相同，28 条在晚到生产前右边已经变化。因此本报告能确认“代码身份同一并最终 Broken”，不能把这 28 条扩大表述为“原冻结四边框的证书后来到达”。

建议另裁。原因不是“58 条已经全部证明教义违规”，而是样本混有三种不同事实：同源低级别点先到、同级 Owner 归属偏移、以及同一生命周期身份晚到；同时现有转储没有逐案标准趋势资格，不能把 37 课的趋势限定抹掉。

## 1. 统计口径与 090 边界

**统计口径行**：对象为 `/tmp/main542-on.2jRQ67/center_lifecycle.jsonl` 中 `kind=reset && alive_center_leak=true` 的 58 个事件；事件主键保留 JSONL 行号、`B=bar`、`level`、`trigger_src`、`trigger_side`、`slot`、`C=(died_si,died_ei,died_zd,died_zg)`；生产身份匹配使用 `(level,died_si,died_zd,died_zg)`，冻结框判定另使用 `died_ei`；先后使用 `observed_bar`，同 bar 时以生产代码的级别升序与 lifecycle JSONL 行序判事件先后；分类互斥优先级为 `(a) > (c) > (b)`。

### 1.1 为什么同 bar 的 19 案计入 (a)

生产接线按 `lvl=0..n_levels` 顺序处理（`rust/src/theta_v0/backtest/fill.rs:897-904`），每级再路由 BSP。19 案中，L0/L2 三类点与 L1/L3 Reset 同 bar，但低级别的 `broken/stale` lifecycle 行严格位于 Reset 行之前；所以相对“Reset 事件”它们已经发生，主口径计入 (a)。这 19 案都有后续 exact Broken。

若把“先于 B”机械限定为严格 `third.observed_bar < B`，不承认同 bar 内事件序，则敏感性口径变为 `a=5,b=3,c=50`。这是标签口径变化，不是数据变化；逐案表保留 JSONL 行号，另裁可无损改口径。

### 1.2 “相关三类点”约束

(a) 不从任意历史三类点中挑近邻。只接受两种可复算关联：

1. 与 Reset 的 `trigger_src` 相同；
2. 与 C 冻结框“右边后第一条 leave + 紧随 retest”在 Reset 时的候选 `source_index` 相同。

实际点还必须有 probe 产出行和 `broken/stale` 路由行。若级别不同记“层级错位”；同级而 `(si,zd,zg)` 不同记“归属他框”。若一案既有此前相关点又在以后出现 exact 点，按任务的“为何先产却没清到 C”问题优先归 (a)，并在表中照录后续到达。

### 1.3 数据完整性与对照

- on/off 三流逐文件 SHA-256 完全一致：
  - `center_lifecycle.jsonl`: `083fa44978265412147524d7edce00f408e5d38861848b211f2ad8537e6b6dc6`
  - `tower_events.jsonl`: `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f`
  - `trades.jsonl`: `06243bac166a241f03eb996e4565dddb64168792537e3bb362c59f52c1e4f9b9`
- on/off 都是 59 条 Reset，其中 58 条 leak、1 条场空；漏发桶为 `L0/Long=5, L0/Short=5, L1/Short=27, L3/Short=21`。
- 既有 `trades.jsonl` 只有进入决策账本的正证，无法证明 `cert=None` 或 `Silent`；`tower_events.jsonl` 无 BSP；所以按票面启用了 `/tmp` D0 全量旁路。
- D0 输出 `/tmp/research-565/full-third.jsonl`：1,227 条 regular 三类点、1,227 条实际路由、58 条 Reset 快照、53 条目标结构状态迁移、63 条目标挂起候选见证。
- 隔离重跑的新三流与既有 on 三流 SHA-256 仍逐文件相同，证明旁路未改变生产结果。

### 1.4 不能从本次数据判定的事项

1. 58 条各自是不是 37 课所限定的标准趋势，转储没有逐案 `A/B同级 + standard_trend` 资格字段，**未能判定**。
2. “且在一类点之前”是 ADR 依 37 课形成的裁定性时间次序，不是 37 课正文的逐字独立句。
3. 28 条 (c) 的生命周期 `CenterId` 相同但冻结 `ei` 已变，不能据此声称原四边框证书到达。

## 2. 权威链与实现链

### 2.1 教义链

- 37 课先限定讨论对象必须是趋势且 A、B 同级（`docs/chanlun/text/blog/037-第37课.md:16`），再说 c 至少包含对 B 的一个第三类买卖点（`:18`），并要求 c 创新高/新低（`:20`）。
- ADR 补充十三把它裁成带趋势限定的“第一类之前必有第三类”，并重申 Reset 只广播、不杀中枢（`docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:230-240`）。
- 补充十四裁“级别跟中枢不跟收编者”（ADR `:243-247`）。
- 补充十五冻结共享枢轴坐标：首 leave 取 `start_index >= end_index`，retest 只取紧随一条，方向或价格失败即 None，禁止越过同向续行找替代对；合法多 Owner 要全投（ADR `:250-253`）。

### 2.2 两条实现流必须分开说

- LevelView 趋势确认流已经接入 T3 全合取：`rust/src/theta_v0/classifier/level_view.rs:528-568`。
- 产出一类 BSP、继而触发 Reset 的流在 `judge_segment` 直接调用 `judge_first_cached`，随后才独立判三类点；这条一类判据没有把 T3 当前置 veto：`rust/src/theta_v0/classifier/signal.rs:289-404,1392-1468`。

因此只能说“Reset-producing BSP 流未接第一合取”，不能说“全系统未实现 T3”。

### 2.3 三类点、Owner、旧框与 Reset

- 严格价格证在 `signal.rs:448-500`：三买 leave/retest 都严格高于 ZG；三卖镜像严格低于 ZD；方向或价格失败返回 None。
- regular Owner 按 leave 段的最近已确认中枢选取：`signal.rs:1454-1466`。
- 旧框 historical-bound 只取右边第一条与紧随一条：`rust/src/theta_v0/classifier/mod.rs:262-298`。
- historical-bound 候选只来自策略挂起簿的冻结框：`rust/src/theta_v0/backtest/fill.rs:982-1018`；regular 与 bound 的 Owner 路由在 `:1020-1073`。
- 一类 bit 优先生成 Reset，Reset 只快照 alive、不 `take`；三类 bit 才解析 Owner 并 Broken：`rust/src/theta_v0/classifier/center_lifecycle.rs:597-696`。
- Reset 没有死亡身份，挂起簿只对 Broken 清算：`rust/src/theta_v0/strategy/center_oscillation_trade.rs:580-628`。
- 漏发桶只是 `(level,side)` 汇总，逐条身份在 lifecycle 行：`rust/src/theta_v0/strategy/oscillation_campaign.rs:991-995,1305-1321`。

## 3. 汇总

### 3.1 分侧分级账

| 漏发桶 | (a) | (b) | (c) | 合计 |
|---|---:|---:|---:|---:|
| L0 / Long | 3 | 0 | 2 | 5 |
| L0 / Short | 1 | 1 | 3 | 5 |
| L1 / Short | 10 | 0 | 17 | 27 |
| L3 / Short | 10 | 2 | 9 | 21 |
| **合计** | **24** | **3** | **31** | **58** |

### 3.2 (a) 的工程形态

- 层级错位 20 案：18 案有 L0 同源见证，4 案有 L2 同源见证，其中 2 案同时有 L0+L2。
- 同级归属他框 4 案：#3/#4/#5 共用 L0 source `112966`，但 Owner 是 `si=112101`；#40 source `208701` 归 `si=207533`。
- 时序：5 案严格早于 Reset bar；19 案同 bar、但低级别 lifecycle 路由行先于高级别 Reset 行。

### 3.3 (b) 的形态

- #2：Reset 时太年轻；首个紧邻对完成后 retest 触及/重回核心，最终没有 exact 点。
- #18/#19：Reset 时太年轻；bar `176930` 时冻结框结构证已经通过，但目标 `C=(L3,si=121297,zd=3715000000000,zg=3784600000000)` 不在挂起候选，regular 同源点产给另一框 `zd=3670700000000`。这里已能判定“候选缺席 + 归他框”，不能再写成纯价格失败。

### 3.4 (c) 的到达与滞后

- 31/31 最终到达并 exact Broken。
- 滞后期（`arrival_bar - B`）：min=60，P50=1,779，P90=10,386，max=27,475。

| 滞后区间 | 数量 |
|---|---:|
| 0 | 0 |
| 1–99 | 2 |
| 100–999 | 8 |
| 1,000–9,999 | 16 |
| ≥10,000 | 5 |

Reset 当时的结构状态：

| 状态 | 数量 |
|---|---:|
| 框右边后尚无 leave | 8 |
| leave 后尚无紧邻 retest | 8 |
| 同向续行挡道 | 5 |
| leave 未严格离开核心 | 7 |
| retest 触及/重回核心 | 3 |

冻结边审计：31 条中 `owner_ei == died_ei` 仅 3 条，`owner_ei != died_ei` 为 28 条。后者仍是代码生命周期的 exact `CenterId`，但不是原冻结四边框的无歧义同证。

## 4. 58 条逐案表

表内 `life:N` 指 `/tmp/main542-on.2jRQ67/center_lifecycle.jsonl:N`；`probe:N` 指 `/tmp/research-565/full-third.jsonl:N`。所有 bar 都是实际观测到达 bar，`src` 是结构坐标，二者不混用。

| # | Reset 证据 | B / 桶 | 在场中枢 C（si,ei,ZD,ZG） | 分类 / 亚类 | Reset 事件前相关三类点 | C 的到达/失败证据 |
|---:|---|---|---|---|---|---|
| 1 | life:9, src=2468 | 2502 / L0-Long | 2270,2468,2614500000000,2625200000000 | **c** / 太年轻：框右边后尚无 leave | — | B时 太年轻：框右边后尚无 leave（probe:15）；C 到于 3047（+545, src=3017, probe:20/21, life:15, broken, owner_ei=2468=原冻结边） |
| 2 | life:667, src=73329 | 73783 / L0-Short | 73335,73747,2783928000000,2793080000000 | **b** / 证失败：retest 触及/重回核心 | — | B时 太年轻：框右边后尚无 leave（probe:596）；末态 证失败：retest 触及/重回核心（bar=73841, probe:603） |
| 3 | life:1034, src=113028 | 113090 / L0-Long | 112398,112708,3436670000000,3456357000000 | **a** / 归属他框 | L0@113004/src=112966→stale（probe:946,route:948,life:1033） | B时 证失败：retest 触及/重回核心（probe:949）；C 截至窗末仍无 exact 产点 |
| 4 | life:1035, src=113403 | 113433 / L0-Long | 112398,112708,3436670000000,3456357000000 | **a** / 归属他框 | L0@113004/src=112966→stale（probe:946,route:948,life:1033） | B时 证失败：retest 触及/重回核心（probe:951）；C 截至窗末仍无 exact 产点 |
| 5 | life:1036, src=113334 | 113440 / L0-Long | 112398,112708,3436670000000,3456357000000 | **a** / 归属他框 | L0@113004/src=112966→stale（probe:946,route:948,life:1033） | B时 证失败：retest 触及/重回核心（probe:953）；C 截至窗末仍无 exact 产点 |
| 6 | life:1125, src=121837 | 121874 / L0-Short | 121297,121486,3664060000000,3664277000000 | **c** / 证失败：retest 触及/重回核心 | — | B时 证失败：retest 触及/重回核心（probe:1043）；C 到于 122083（+209, src=121910, probe:1045/1046, life:1126, broken, owner_ei=121625≠原冻结边） |
| 7 | life:1171, src=127452 | 127522 / L3-Short | 92368,112338,3342124000000,3474191000000 | **c** / 证失败：leave 未严格离开核心 | — | B时 证失败：leave 未严格离开核心（probe:1085）；C 到于 137908（+10386, src=137848, probe:1240/1241, life:1304, broken, owner_ei=119258≠原冻结边） |
| 8 | life:1176, src=127808 | 127864 / L3-Short | 92368,112338,3342124000000,3474191000000 | **c** / 证失败：leave 未严格离开核心 | — | B时 证失败：leave 未严格离开核心（probe:1092）；C 到于 137908（+10044, src=137848, probe:1240/1241, life:1304, broken, owner_ei=119258≠原冻结边） |
| 9 | life:1178, src=127968 | 128017 / L3-Short | 92368,112338,3342124000000,3474191000000 | **a** / 层级错位 | L0@128017/src=127968→stale（probe:1094,route:1096,life:1177） | B时 证失败：leave 未严格离开核心（probe:1095）；C 后于 137908 到（+9891, src=137848, life:1304, owner_ei=119258≠原冻结边） |
| 10 | life:1179, src=128014 | 128064 / L3-Short | 92368,112338,3342124000000,3474191000000 | **c** / 证失败：leave 未严格离开核心 | — | B时 证失败：leave 未严格离开核心（probe:1098）；C 到于 137908（+9844, src=137848, probe:1240/1241, life:1304, broken, owner_ei=119258≠原冻结边） |
| 11 | life:1201, src=130929 | 130983 / L3-Short | 92368,112338,3342124000000,3474191000000 | **a** / 层级错位 | L0@130983/src=130929→broken（probe:1126,route:1128,life:1200） | B时 证失败：leave 未严格离开核心（probe:1127）；C 后于 137908 到（+6925, src=137848, life:1304, owner_ei=119258≠原冻结边） |
| 12 | life:1202, src=131017 | 131064 / L3-Short | 92368,112338,3342124000000,3474191000000 | **c** / 证失败：leave 未严格离开核心 | — | B时 证失败：leave 未严格离开核心（probe:1130）；C 到于 137908（+6844, src=137848, probe:1240/1241, life:1304, broken, owner_ei=119258≠原冻结边） |
| 13 | life:1204, src=131082 | 131111 / L3-Short | 92368,112338,3342124000000,3474191000000 | **a** / 层级错位 | L0@131111/src=131082→stale（probe:1132,route:1134,life:1203） | B时 证失败：leave 未严格离开核心（probe:1133）；C 后于 137908 到（+6797, src=137848, life:1304, owner_ei=119258≠原冻结边） |
| 14 | life:1205, src=131169 | 131190 / L3-Short | 92368,112338,3342124000000,3474191000000 | **c** / 证失败：leave 未严格离开核心 | — | B时 证失败：leave 未严格离开核心（probe:1136）；C 到于 137908（+6718, src=137848, probe:1240/1241, life:1304, broken, owner_ei=119258≠原冻结边） |
| 15 | life:1207, src=131189 | 131212 / L3-Short | 92368,112338,3342124000000,3474191000000 | **a** / 层级错位 | L0@131212/src=131189→stale（probe:1138,route:1140,life:1206） | B时 证失败：leave 未严格离开核心（probe:1139）；C 后于 137908 到（+6696, src=137848, life:1304, owner_ei=119258≠原冻结边） |
| 16 | life:1208, src=131211 | 131270 / L3-Short | 92368,112338,3342124000000,3474191000000 | **c** / 证失败：leave 未严格离开核心 | — | B时 证失败：leave 未严格离开核心（probe:1142）；C 到于 137908（+6638, src=137848, probe:1240/1241, life:1304, broken, owner_ei=119258≠原冻结边） |
| 17 | life:1219, src=131929 | 132013 / L3-Short | 92368,112338,3342124000000,3474191000000 | **a** / 层级错位 | L0@132013/src=131929→broken（probe:1152,route:1154,life:1217） | B时 证失败：leave 未严格离开核心（probe:1153）；C 后于 137908 到（+5895, src=137848, life:1304, owner_ei=119258≠原冻结边） |
| 18 | life:1391, src=146708 | 146768 / L3-Short | 121297,146708,3715000000000,3784600000000 | **b** / 结构证通过但 C 不在挂起候选，regular 归他框 | — | B时 太年轻：框右边后尚无 leave（probe:1356）；176930/src=176839 结构通过但产给 L3/si=121297（probe:1692,transition:1693,binding:1696=不在挂起候选） |
| 19 | life:1394, src=147131 | 147147 / L3-Short | 121297,146708,3715000000000,3784600000000 | **b** / 结构证通过但 C 不在挂起候选，regular 归他框 | — | B时 太年轻：框右边后尚无 leave（probe:1361）；176930/src=176839 结构通过但产给 L3/si=121297（probe:1692,transition:1693,binding:1696=不在挂起候选） |
| 20 | life:1415, src=149353 | 149455 / L3-Short | 121297,149353,3670700000000,3784600000000 | **c** / 太年轻：框右边后尚无 leave | — | B时 太年轻：框右边后尚无 leave（probe:1382）；C 到于 176930（+27475, src=176839, probe:1692/1698, life:1672, broken, owner_ei=152971≠原冻结边） |
| 21 | life:1428, src=150258 | 150285 / L3-Short | 121297,149353,3670700000000,3784600000000 | **a** / 层级错位 | L0@150285/src=150258→broken（probe:1392,route:1394,life:1426） | B时 太年轻：框右边后尚无 leave（probe:1393）；C 后于 176930 到（+26645, src=176839, life:1672, owner_ei=152971≠原冻结边） |
| 22 | life:1430, src=150282 | 150372 / L3-Short | 121297,149353,3670700000000,3784600000000 | **c** / 太年轻：框右边后尚无 leave | — | B时 太年轻：框右边后尚无 leave（probe:1397）；C 到于 176930（+26558, src=176839, probe:1692/1698, life:1672, broken, owner_ei=152971≠原冻结边） |
| 23 | life:1433, src=150670 | 150787 / L3-Short | 121297,149353,3670700000000,3784600000000 | **c** / 太年轻：框右边后尚无 leave | — | B时 太年轻：框右边后尚无 leave（probe:1400）；C 到于 176930（+26143, src=176839, probe:1692/1698, life:1672, broken, owner_ei=152971≠原冻结边） |
| 24 | life:1449, src=152044 | 152094 / L3-Short | 121297,149353,3670700000000,3784600000000 | **a** / 层级错位 | L2@152094/src=152044→stale（probe:1424,route:1426,life:1448） | B时 太年轻：框右边后尚无 leave（probe:1425）；C 后于 176930 到（+24836, src=176839, life:1672, owner_ei=152971≠原冻结边） |
| 25 | life:1455, src=152491 | 152511 / L3-Short | 121297,149353,3670700000000,3784600000000 | **a** / 层级错位 | L0@152511/src=152491→broken（probe:1430,route:1433,life:1453）<br>L2@152511/src=152491→stale（probe:1431,route:1434,life:1454） | B时 太年轻：框右边后尚无 leave（probe:1432）；C 后于 176930 到（+24419, src=176839, life:1672, owner_ei=152971≠原冻结边） |
| 26 | life:1458, src=152640 | 152690 / L3-Short | 121297,149353,3670700000000,3784600000000 | **a** / 层级错位 | L0@152690/src=152640→stale（probe:1436,route:1439,life:1456）<br>L2@152690/src=152640→stale（probe:1437,route:1440,life:1457） | B时 太年轻：框右边后尚无 leave（probe:1438）；C 后于 176930 到（+24240, src=176839, life:1672, owner_ei=152971≠原冻结边） |
| 27 | life:1460, src=152971 | 152999 / L3-Short | 121297,149353,3670700000000,3784600000000 | **a** / 层级错位 | L2@152999/src=152971→stale（probe:1442,route:1444,life:1459） | B时 太年轻：框右边后尚无 leave（probe:1443）；C 后于 176930 到（+23931, src=176839, life:1672, owner_ei=152971≠原冻结边） |
| 28 | life:1499, src=157469 | 157673 / L1-Short | 155232,156250,3936923000000,3945063000000 | **c** / 同向续行挡道 | — | B时 同向续行挡道（probe:1477）；C 到于 159233（+1560, src=159192, probe:1494/1495, life:1512, broken, owner_ei=156722≠原冻结边） |
| 29 | life:1501, src=157634 | 157705 / L1-Short | 155232,156250,3936923000000,3945063000000 | **a** / 层级错位 | L0@157705/src=157634→stale（probe:1479,route:1481,life:1500） | B时 同向续行挡道（probe:1480）；C 后于 159233 到（+1528, src=159192, life:1512, owner_ei=156722≠原冻结边） |
| 30 | life:1502, src=157747 | 157778 / L1-Short | 155232,156250,3936923000000,3945063000000 | **c** / 同向续行挡道 | — | B时 同向续行挡道（probe:1483）；C 到于 159233（+1455, src=159192, probe:1494/1495, life:1512, broken, owner_ei=156722≠原冻结边） |
| 31 | life:1516, src=159872 | 159900 / L1-Short | 157103,159192,4157138000000,4202600000000 | **a** / 层级错位 | L0@159900/src=159872→stale（probe:1498,route:1501,life:1515） | B时 太年轻：leave 后尚无紧邻 retest（probe:1500）；C 后于 160201 到（+301, src=160187, life:1524, owner_ei=159192=原冻结边） |
| 32 | life:1517, src=159899 | 159975 / L1-Short | 157103,159192,4157138000000,4202600000000 | **c** / 太年轻：leave 后尚无紧邻 retest | — | B时 太年轻：leave 后尚无紧邻 retest（probe:1503）；C 到于 160201（+226, src=160187, probe:1515/1518, life:1524, broken, owner_ei=159192=原冻结边） |
| 33 | life:1519, src=159974 | 160061 / L1-Short | 157103,159192,4157138000000,4202600000000 | **a** / 层级错位 | L0@160061/src=159974→stale（probe:1505,route:1507,life:1518） | B时 太年轻：leave 后尚无紧邻 retest（probe:1506）；C 后于 160201 到（+140, src=160187, life:1524, owner_ei=159192=原冻结边） |
| 34 | life:1520, src=160085 | 160106 / L1-Short | 157103,159192,4157138000000,4202600000000 | **c** / 太年轻：leave 后尚无紧邻 retest | — | B时 太年轻：leave 后尚无紧邻 retest（probe:1509）；C 到于 160201（+95, src=160187, probe:1515/1518, life:1524, broken, owner_ei=159192=原冻结边） |
| 35 | life:1532, src=160515 | 160606 / L1-Short | 159297,160515,4354503000000,4380500000000 | **c** / 太年轻：框右边后尚无 leave | — | B时 太年轻：框右边后尚无 leave（probe:1534）；C 到于 164572（+3966, src=164419, probe:1568/1569, life:1560, broken, owner_ei=162178≠原冻结边） |
| 36 | life:1533, src=160604 | 160635 / L1-Short | 159297,160515,4354503000000,4380500000000 | **c** / 太年轻：框右边后尚无 leave | — | B时 太年轻：框右边后尚无 leave（probe:1536）；C 到于 164572（+3937, src=164419, probe:1568/1569, life:1560, broken, owner_ei=162178≠原冻结边） |
| 37 | life:1608, src=168779 | 168832 / L0-Long | 168491,168671,4164325000000,4179760000000 | **c** / 太年轻：leave 后尚无紧邻 retest | — | B时 太年轻：leave 后尚无紧邻 retest（probe:1618）；C 到于 169861（+1029, src=169827, probe:1625/1626, life:1615, broken, owner_ei=169396≠原冻结边） |
| 38 | life:1800, src=191511 | 191661 / L0-Short | 190694,191377,4290000000000,4318560000000 | **c** / 太年轻：leave 后尚无紧邻 retest | — | B时 太年轻：leave 后尚无紧邻 retest（probe:1843）；C 到于 191762（+101, src=191722, probe:1845/1848, life:1803, broken, owner_ei=191302≠原冻结边） |
| 39 | life:1925, src=203084 | 203526 / L0-Short | 203096,203496,4388131000000,4415500000000 | **c** / 太年轻：框右边后尚无 leave | — | B时 太年轻：框右边后尚无 leave（probe:1976）；C 到于 203989（+463, src=203965, probe:1982/1983, life:1930, broken, owner_ei=203699≠原冻结边） |
| 40 | life:1971, src=208980 | 209054 / L0-Short | 208095,208344,4492598000000,4498826000000 | **a** / 归属他框 | L0@208741/src=208701→stale（probe:2008,route:2011,life:1970） | B时 紧邻对价格与方向均通过（probe:2012）；C 后于 209176 到（+122, src=209139, life:1974, owner_ei=208590≠原冻结边） |
| 41 | life:1995, src=211627 | 211671 / L1-Short | 208095,209666,4640790000000,4688802000000 | **a** / 层级错位 | L0@210925/src=210855→stale（probe:2035,route:2036,life:1992） | B时 同向续行挡道（probe:2039）；C 截至窗末仍无 exact 产点 |
| 42 | life:2423, src=253533 | 253568 / L1-Short | 252025,252737,4450539000000,4463378000000 | **a** / 层级错位 | L0@253568/src=253533→broken（probe:2489,route:2492,life:2422） | B时 证失败：leave 未严格离开核心（probe:2491）；C 后于 255598 到（+2030, src=254882, life:2437, owner_ei=253028≠原冻结边） |
| 43 | life:2424, src=253615 | 253661 / L1-Short | 252025,252737,4450539000000,4463378000000 | **c** / 证失败：leave 未严格离开核心 | — | B时 证失败：leave 未严格离开核心（probe:2494）；C 到于 255598（+1937, src=254882, probe:2508/2509, life:2437, broken, owner_ei=253028≠原冻结边） |
| 44 | life:2447, src=256457 | 256504 / L1-Short | 253980,255730,4704517000000,4750400000000 | **a** / 层级错位 | L0@256504/src=256457→broken（probe:2513,route:2516,life:2446） | B时 太年轻：leave 后尚无紧邻 retest（probe:2515）；C 后于 257337 到（+833, src=257000, life:2467, owner_ei=256098≠原冻结边） |
| 45 | life:2449, src=256529 | 256688 / L1-Short | 253980,255730,4704517000000,4750400000000 | **c** / 太年轻：leave 后尚无紧邻 retest | — | B时 太年轻：leave 后尚无紧邻 retest（probe:2519）；C 到于 257337（+649, src=257000, probe:2542/2545, life:2467, broken, owner_ei=256098≠原冻结边） |
| 46 | life:2453, src=256773 | 256813 / L1-Short | 253980,255730,4704517000000,4750400000000 | **a** / 层级错位 | L0@256813/src=256773→broken（probe:2522,route:2525,life:2452） | B时 同向续行挡道（probe:2524）；C 后于 257337 到（+524, src=257000, life:2467, owner_ei=256098≠原冻结边） |
| 47 | life:2454, src=256812 | 256877 / L1-Short | 253980,255730,4704517000000,4750400000000 | **c** / 同向续行挡道 | — | B时 同向续行挡道（probe:2527）；C 到于 257337（+460, src=257000, probe:2542/2545, life:2467, broken, owner_ei=256098≠原冻结边） |
| 48 | life:2456, src=256907 | 257002 / L1-Short | 253980,255730,4704517000000,4750400000000 | **a** / 层级错位 | L0@257002/src=256907→stale（probe:2529,route:2531,life:2455） | B时 同向续行挡道（probe:2530）；C 后于 257337 到（+335, src=257000, life:2467, owner_ei=256098≠原冻结边） |
| 49 | life:2457, src=257000 | 257093 / L1-Short | 253980,255730,4704517000000,4750400000000 | **c** / 同向续行挡道 | — | B时 同向续行挡道（probe:2533）；C 到于 257337（+244, src=257000, probe:2542/2545, life:2467, broken, owner_ei=256098≠原冻结边） |
| 50 | life:2459, src=257066 | 257110 / L1-Short | 253980,255730,4704517000000,4750400000000 | **a** / 层级错位 | L0@257110/src=257066→stale（probe:2535,route:2537,life:2458） | B时 同向续行挡道（probe:2536）；C 后于 257337 到（+227, src=257000, life:2467, owner_ei=256098≠原冻结边） |
| 51 | life:2460, src=257181 | 257277 / L1-Short | 253980,255730,4704517000000,4750400000000 | **c** / 同向续行挡道 | — | B时 同向续行挡道（probe:2539）；C 到于 257337（+60, src=257000, probe:2542/2545, life:2467, broken, owner_ei=256098≠原冻结边） |
| 52 | life:2468, src=257294 | 257337 / L1-Short | 256132,257294,4813333000000,4813598000000 | **c** / 太年轻：框右边后尚无 leave | — | B时 太年轻：框右边后尚无 leave（probe:2543）；C 到于 259916（+2579, src=259867, probe:2580/2581, life:2499, broken, owner_ei=258695≠原冻结边） |
| 53 | life:2474, src=258016 | 258075 / L1-Short | 256132,257294,4813333000000,4813598000000 | **c** / 太年轻：leave 后尚无紧邻 retest | — | B时 太年轻：leave 后尚无紧邻 retest（probe:2553）；C 到于 259916（+1841, src=259867, probe:2580/2581, life:2499, broken, owner_ei=258695≠原冻结边） |
| 54 | life:2475, src=257918 | 258079 / L1-Short | 256132,257294,4813333000000,4813598000000 | **c** / 太年轻：leave 后尚无紧邻 retest | — | B时 太年轻：leave 后尚无紧邻 retest（probe:2555）；C 到于 259916（+1837, src=259867, probe:2580/2581, life:2499, broken, owner_ei=258695≠原冻结边） |
| 55 | life:2477, src=258088 | 258117 / L1-Short | 256132,257294,4813333000000,4813598000000 | **a** / 层级错位 | L0@258117/src=258088→broken（probe:2557,route:2559,life:2476） | B时 太年轻：leave 后尚无紧邻 retest（probe:2558）；C 后于 259916 到（+1799, src=259867, life:2499, owner_ei=258695≠原冻结边） |
| 56 | life:2478, src=258114 | 258137 / L1-Short | 256132,257294,4813333000000,4813598000000 | **c** / 太年轻：leave 后尚无紧邻 retest | — | B时 太年轻：leave 后尚无紧邻 retest（probe:2561）；C 到于 259916（+1779, src=259867, probe:2580/2581, life:2499, broken, owner_ei=258695≠原冻结边） |
| 57 | life:2485, src=258787 | 258851 / L1-Short | 256132,257294,4813333000000,4813598000000 | **c** / 证失败：retest 触及/重回核心 | — | B时 证失败：retest 触及/重回核心（probe:2568）；C 到于 259916（+1065, src=259867, probe:2580/2581, life:2499, broken, owner_ei=258695≠原冻结边） |
| 58 | life:2486, src=258695 | 258885 / L1-Short | 256132,257294,4813333000000,4813598000000 | **c** / 证失败：retest 触及/重回核心 | — | B时 证失败：retest 触及/重回核心（probe:2570）；C 到于 259916（+1031, src=259867, probe:2580/2581, life:2499, broken, owner_ei=258695≠原冻结边） |

## 5. 每类证据链演示

### 5.1 (a) 先产但未清到 C

**案例 #3：同级归属他框 + 对 C 价格失败**

1. Reset 现场：`center_lifecycle.jsonl:1034`，B=113090，L0/Long，C=`(si=112398,ei=112708,zd=3436670000000,zg=3456357000000)`。
2. 早在 bar 113004，source `112966` 已产 L0 三类点，但 Owner=`si=112101`：`full-third.jsonl:946`；实际路由为 stale：probe `:948`、lifecycle `:1033`。
3. 对 C 的同一紧邻对，retest 端价 `3446600000000 >= C.zd 3436670000000`，触及/重回核心，probe `:947/:949` 判 `retest_reentered`。代码严格价格门见 `signal.rs:477-499`。
4. 所以这不是“三类点不存在”，而是同源点产给另一框；对 C 自己价格判负，C 到窗末没有 exact 点。

**案例 #11：同 bar 低级别先路由**

1. Reset：lifecycle `:1201`，B=130983，L3/Short，C=`si=92368`，trigger source=`130929`。
2. 同 bar/source 的 L0 三类点先产：probe `:1126`；先路由 Broken：probe `:1128`、lifecycle `:1200`。
3. 随后才写 L3 Reset：lifecycle `:1201`。代码按 level 升序处理见 `fill.rs:897-904`，JSONL 行序与之同证。
4. 对 L3 冻结框，同一 source 的 leave 未严格越过 L3 核心，probe `:1127` 判负；L3 生命周期身份直到 bar 137908 才以 source `137848` Broken（probe `:1240/:1241`，lifecycle `:1304`），滞后 6,925。

### 5.2 (b) C 从未产 exact 点

**案例 #2：太年轻后转价格失败**

1. Reset：lifecycle `:667`，B=73783，L0/Short。
2. B 时框右边后还没有 leave：probe `:596`。
3. bar 73841 首紧邻对完成；Up leave 端 `2794484000000` 虽越 ZG，但 Down retest 端 `2787500000000 <= ZG 2793080000000`，重回核心：probe `:603`。
4. 此后状态不再转 pass，且全量 regular/routed 流无该 `(L0,CenterId)` 点，故归 (b) 价格失败。

**案例 #18（#19 同框第二个 Reset）：结构通过但候选缺席**

1. #18 Reset：lifecycle `:1391`，B=146768；#19 Reset：lifecycle `:1394`，B=147147。两者同一 L3 C，B 时都无 leave：probe `:1356/:1361`。
2. bar 176930，C 的冻结框紧邻对结构转 pass，source=`176839`：probe `:1693`。
3. 同时的挂起候选见证明确 `target_is_suspended=false`：probe `:1696`。生产代码只遍历 `suspended_frames()` 产 old-frame 候选（`fill.rs:982-1018`）。
4. regular 的 L3/source `176839` 实际 Owner 是同 si/zg 但 `zd=3670700000000` 的另一框：probe `:1692`，并路由 Broken `:1698`。目标 C 截至窗末仍没有 exact 点。因此 #18/#19 都归 (b)，具体原因已闭合为“C 不在挂起候选，regular 归他框”。

### 5.3 (c) Reset 后才到

**案例 #1：太年轻，+545 到达**

1. Reset：lifecycle `:9`，B=2502，L0/Long；B 时无 leave：probe `:15`。
2. bar 3047 产 exact C/source `3017`，证书区间 leave=`2922..2977`、retest=`3000..3017`：probe `:20`。
3. 同 bar 路由 Broken：probe `:21`、lifecycle `:15`；滞后 `3047-2502=545`，且 `owner_ei=2468` 与原冻结边一致。

**案例 #7：先价格失败，+10,386 到达但右边已变**

1. Reset：lifecycle `:1171`，B=127522，L3/Short。
2. B 时冻结框首 leave 未严格离开核心：probe `:1085`。
3. bar 137908 生产相同 `(L3,CenterId)` 的 source `137848` 并 Broken：probe `:1240/:1241`、lifecycle `:1304`，滞后 10,386。
4. 但晚到点 `owner_ei=119258`，不同于 Reset 的 `died_ei=112338`。所以能确认生命周期身份晚到，不能声称原冻结框证书晚到。

## 6. 另裁材料

### 6.1 若裁“教义违规”

先决裁定应是：哪些 Reset 确属 37 课的标准趋势第一类，哪些是盘整背驰/非标准形态。若裁 Reset-producing BSP 的一类点必须先有对最后 B 的同级 T3，则影响面包括：

1. 第一类生产门：`signal.rs:289-404,1392-1443`；
2. 现有 T3 helper 与 LevelView：`signal.rs:510-544`、`level_view.rs:528-568`；
3. 同点一类优先规则：`center_lifecycle.rs:625-655`；
4. 下游所有一类信号、Reset 数量/时点、trades、生命周期、仓位退出、wf8 金标准与经济指标。

不能直接把 `trend_third_class_in_c` 扫描 helper 拼进 old-frame 紧邻规则：前者在 c 窗内扫描相邻对，补充十五要求旧框只取右边第一对。若进入实施，先做离线信号差异面枚举；这不是零风险接线修复。

### 6.2 若裁“层级错位工程后果”

可选工程路线：

1. **补同级生产**：让 L1/L3 中枢在自身 level 产证，保持“级别跟中枢”；
2. **加强 exact old-frame 候选**：解决最近 Owner 遮挡，但需先裁候选是否应依赖交易挂起簿；#18/#19 已实证“结构 pass 但非 suspended 就不产”；
3. **显式多 Owner**：同一结构事件对多个冻结框各自验价、合法者全投，继续以 `(level,CenterId,source,side)` 去重；
4. **显式跨级 lineage**：只有另裁允许 L0 代表 L1/L3 时才可做，必须有可审计 parent/equivalence；不能凭相同 source 或相似价格带跨级清算。这条会触碰 ADR “级别跟中枢”，风险最高。

不建议把 20 条层级错位直接改成“低级别点跨级杀高级别 C”；那会把本次观测事实变成新教义。

### 6.3 若裁“如实形态（非标准趋势/小转大）”

生产规则可不动，建议把观测口径补完整：

1. 每条 Reset 落 `standard_trend_eligible`、A/B 级别、last-B 身份、`c_start`、T3 pair 状态，以及盘整背驰/小转大候选标签；
2. 标准趋势、盘整/非标准、小转大分母分开；只对标准趋势分母应用 37 课 T3 先行期望；
3. 同时保留 event-order 与 strict-bar 两个时序口径，same-bar 单列，不把定义差异伪装成数据差异；
4. 滞后使用生存分析口径：到达事件、lag、窗末右删；Owner 必须同时落 `(level,CenterId)` 与 frozen/current `ei`；
5. `cert=None` 原因至少分 `missing_leave/missing_retest/same_direction/leave_not_outside/retest_reentered`，并记录 regular、historical-bound、候选缺席与实际路由。

## 7. 复现与无侵入验证

- 分析脚本：
  - `/tmp/research-565/analyze_existing.mjs`
  - `/tmp/research-565/classify.mjs`
- D0 工位：`/tmp/research-565/wt`
- 独立 target：`/tmp/research-565/target`
- 探针输出：`/tmp/research-565/full-third.jsonl`
- 重跑口径：

```bash
RESET_BEFORE_THIRD_PROBE_OUT=/tmp/research-565/full-third.jsonl \
RESET_BEFORE_THIRD_LEAKS=/tmp/main542-on.2jRQ67/center_lifecycle.jsonl \
M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_CENTER_OSCILLATION=1 \
OPSEM_DUMP_DIR=/tmp/research-565/wf8-on-2 \
CARGO_TARGET_DIR=/tmp/research-565/target \
cargo test --release --lib \
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
  -- --ignored --nocapture
```

结果：`1 passed; 0 failed`；生产 stdout 仍报 leak 桶 `5/5/27/21`、Reset 清算 0、`historical_bound_by_level={}`；新旧三条生产转储 SHA-256 全同。探针只存在 `/tmp` 隔离 worktree，未进入主仓。

## 8. 最终答复

- 主口径：`a=24, b=3, c=31`；
- `24+3+31=58`，与漏发警报桶账平；
- 建议另裁：**是**。先裁标准趋势资格与同 bar 时序口径，再裁同级生产/Owner 候选；不要把三种机制压成一个“教义违规”结论。

