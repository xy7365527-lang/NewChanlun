# wave-1 plan 5b 实装卡：p123 run 语境细粒度 pan memo

> **复原卡（2026-07-27），非原文。** 原卡（2026-07-19）从未入 git 且已佚失（#479 登记）。本卡据 issue93 实装卡 §1、wave1-5a5b-blocker-clearance-20260720、r1-r3-ruling-confirmation-checklist-20260720（R1-R3 已确认）、scene-ledger 复原；行号锚已按 2026-07-27 工位代码重测。

- **票据关系**：#69 的 5b；复原票 #479。
- **工位基线**：`/tmp/wt-69`，分支 `ticket-69`，基于 `kimi-nest-mainline-20260717`。
- **本文性质**：纯文档复原，不声明 5b 已实装。现行 `RunEntry` 无 `pan_memo`（`rust/src/bin/p123_fast_replay.rs:374-383`），现行 `rust/src/` 无 `PanMemo` 符号。
- **裁定终态说明**：`r1-r3-ruling-confirmation-checklist-20260720.md` 自身仍写“待确认”；后续 `chanlun/escalate/g1-g2-window-ban-ruling-20260721.md:36-44` 已登记编排者逐项确认 R1-R3。本文随卡重锚，不重新诉讼。
- **锚定规则**：下文代码行号均为 2026-07-27 `/tmp/wt-69` 的 1-based 行号。历史段一能力若在现行树缺符号，照实登记为“历史曾核、当前需恢复 seam”，不以旧报告替代当前能力。

## 1. 作用面与缝形

### 1.1 唯一作用面

5b 只治理 `p123_fast_replay` 的 per-run pan 候选重算：

1. `run_targeted_prefix_pass` 维护 per-level `LevelDerived` 与
   per-(level, run_source_start) `RunEntry`（`p123_fast_replay.rs:781-782`）。
2. dirty run 经 `evaluate_run`（`:897-907`）生成 run 投影；center 来自该投影 seeds，blocks 来自该 run centers 的 decompose（`:1109-1112`）。
3. `evaluate_run` 组装 view 后调用 provider（`:1122-1147`）。
4. provider 的 pan 分支在 `level_view.rs:817-893` 对 as-of 内每个 lower segment 重做：
   最近确认中枢、run block kind 门、A/C 结构定位、Extreme、leave/block span、力度或关系与事件构造。

5b 不缓存全局 pan 结论，不改变 pan 教义谓词，不治理 runner/NestChainGate；memo 只减少同一 p123 run 在多次 dirty 评估中的重复读。

### 1.2 缝形

- `PanMemo` 放在 bin `RunEntry` 旁，按 run 驻留；不能放进 per-level `LevelDerived`。
- lib/provider 通过**显式可选参数**消费 memo；旧公开入口传 `None`，同入参同输出仍可从真冷路径验证。
- dirty 调用传 `Some(&mut entry.pan_memo)`；forced shadow 调用传 `None`。
- memo 的可复用性是两条链的合取：
  1. e_src 冻结水位链；
  2. `b_idx + 2 < blocks.len()` 的至少两个后继块门。
- 任一链不能证明即冷算且不回写稳定 memo；禁止用启发式 TTL、bar 数或 run 代次替代这两条构造性条件。

### 1.3 与 5a 的排期交叉

5a 与 5b 都改 B3-B5：

- B3：`evaluate_run` 签名；
- B4：dirty 调用点 `p123_fast_replay.rs:897-907`；
- B5：shadow forced 调用点 `:940-950`。

因此按 R4 在同一工位三段推进：段一 lib 暴露面【历史登记已落】→ 段二 5a → 段三 5b。5b 必须基于 5a 已定的最终签名追加参数，不得重新排列成第二条调用链。

## 2. 逐函数设计与读域表

幸存 export 指向原卡 §2 的“8 行函数封闭”。原八行逐字文本已佚失；按现行 provider 调用链重测，pan memo 必须封闭下列八个读面：

| # | 函数/步骤 | 现行读域 | 现行锚 | memo 规则 |
|---:|---|---|---|---|
| 1 | `nearest_confirmed_center_idx` | run centers + `segment.start_index` | `signal.rs:213-220` | center 身份来自 run 投影，不得跨 run 共享 |
| 2 | `center_block_kind` / `_at` | run blocks 的 ownership/kind | `decompose.rs:197-205,213-226` | 只接受已封 block |
| 3 | `locate_pan_div_structure` | center、segment、A/C 后向窗口、anchor dirs | `signal.rs:655-712` | 结构命中/稳定 miss 均须过两链 |
| 4 | `locate_pan_div_structure_front_anchor` | center 前最近同向段 + C episode | `signal.rs:724-751` | 与窄锚顺序不变，不能拆成第二谓词 |
| 5 | `pan_div_structure_extreme` | A/C span 内 segments envelope | `signal.rs:754-775` | 所读 segment 必须在 e_src 前封口 |
| 6 | leave/block span | run blocks、projection seeds | `level_view.rs:840-859`；`structural_pair_span :513-526` | block 还须满足两个后继块门 |
| 7 | `segments_diverge_or` | A/C 映射后的 hist/dif 前缀 | `divergence.rs:334-349`；调用 `level_view.rs:860-868` | 坐标/MACD 缺失不作负缓存 |
| 8 | 事件物化/去重/排序 | `as_of`、provider window、pending、锚供给 | `level_view.rs:869-899` | 动态字段每次重物化，不缓存旧 `judge_at` |

### 2.1 `evaluate_run`

现行 `evaluate_run` 在 `p123_fast_replay.rs:1097-1148`，从 run slice 独立构造 projection/centers/blocks。5b 只能在这一级把该 run 的 `PanMemo` 显式传给 provider；若在更外层按 level 共享，center 和 kind 的语义身份已经丢失。

### 2.2 provider resident seam

实施时在现行 `provide_nest_candidate_events{,_ext}` 的同一 pan 核上增加可选 memo seam。约束为：

- 旧 `provide_nest_candidate_events` 与 `_ext` 入口保持可走 `None` 的冷路径；
- resident 入口只复用已封 pan 子结果，不复制 `locate_*`、Extreme 或力度判据；
- trend 分支不消费 `PanMemo`；
- `judge_at`、当前 `provider_window`、锚 sidecar、最终去重与排序每次按当前调用重建。

原卡 resident provider 的精确函数名、参数位置和是否以内部 helper 实现已佚失，见 §9；本文只固定显式状态与单核要求。

### 2.3 memo 键和值

`PanMemoKey` 至少封闭：

- level 与 run/provider window；
- segment 身份（源坐标与方向/端点）；
- 最近确认 center 的稳定身份；
- run block 的 span/kind/ownership 身份；
- projection/version 与会改变 A/C/Extreme/force 的结构代次。

key 不能只用 `segment.end_index`，也不能只用 per-level `lower_gen`；两种都会把不同 run 语境混为一体。原字段布局已佚失，这里是读域封闭约束。

`PanMemoValue` 只保存与 as-of 无关、且已经两链证明稳定的 payload，例如：

- center/block 身份与稳定 kind；
- 窄锚或 front-anchor 选出的 `PanDivStructure`，或已证稳定 miss；
- Extreme 与 A/C 力度结果；
- 构造事件所需的稳定 source 坐标、side、fallback/span 信息。

不得跨 bar缓存：

- `judge_at`；
- 当前 `provider_window` 的临时对象；
- pending 集过滤结果；
- 缺坐标、hist/dif 尚不可读等可能随输入增长而改变的“未命中”；
- 未过两个后继块门的 tail block 结论。

### 2.4 写域与禁写域

允许写：

- 当前 `(level, run_source_start)` 对应的 `RunEntry.pan_memo`；
- memo entry 自身的 source 读域上界、block/key 身份与命中/失效诊断计数；
- 本次返回事件中的动态字段，但动态字段不回写 memo。

禁止写：

- 跨 run 的 `LevelDerived` pan 值；
- `TowerCache` 或 provider 内的隐式 pan 状态；
- e_src 的第二份公式/第二份水位；
- forced shadow 路径的任何 memo；
- 教义输出、事件键、排序或 pending 账本的替代缓存。

## 3. 裁定锚：R1-R3 已确认，不重新诉讼

### 3.1 R1：接受两条失效链与 TURN 残余

编排者接受：

- 链①使用严格 `segment.end_index < e_src`；
- 链②使用 `b_idx + 2 < blocks.len()`；
- 两链的充分/不过度构造性论证；
- TURN 已声明的残余风险：末窗一次重扫产出至少两个子中枢的 bar 理论上仍可能提前落盘。

残余不靠推测消失。拦截网固定为 V0 身份校验、`P123_SHADOW=1` 全程真冷对拍、全量双跑 dump diff；任一 mismatch/diff 即 090 停线。

### 3.2 R2：5a per-level store

5a `ConfirmCursorStore` 放 `LevelDerived` 旁并让 forced 传 `None`。5b 不复用该 store 承载 pan 值；两者只共享显式冷/热 seam 与最终 `evaluate_run` 接线纪律。

### 3.3 R3：5b per-run memo

`PanMemo` 放 bin `RunEntry` 旁：

- center 来自 run 投影 seeds（`p123_fast_replay.rs:1109-1111`）；
- kind 门来自同一 run 的 blocks（`:1112`）；
- `LevelDerived` 是跨 run 的 per-level lower-leg 容器，不能表达 memo 键；
- TowerCache/lib 隐式 memo 会破坏同入参同输出与 shadow 可控性；
- 显式参数让 dirty 传 `Some`、forced 传 `None`。

## 4. 失效链①：e_src 水位

### 4.1 单一公式与现行能力漂移

现行唯一可见公式在 `signal::extract_first_third_resume`：

```text
e_src =
  if prefix_count >= 2
  then min(centers[prefix_count - 2].end_index, dirty_e)
  else 0
```

锚：`signal.rs:1130-1147,1187-1194`；调用侧说明见
`classifier/mod.rs:2056-2080`。现行代码已经用严格
`segment.end_index < e_src` 计算 stable prefix，并在缓存越界时清空（`signal.rs:1194-1202`）。

历史 blocker-clearance §3.2 曾核过 `freeze_boundary`、`last_freeze_boundary` 与
`TowerCache::freeze_boundary(level)` getter，但这些符号在 2026-07-27 现行
`rust/src/` **均不存在**；只剩上述内联公式。故“段一已落”只能作为历史工位事实，不能冒充当前公开能力。

5b 动手前必须先恢复一个**同公式、同 source 坐标量纲**的只读 seam，并让现行
`extract_first_third_resume` 也调用该单一公式；禁止在 p123/bin 再抄一份。若该 seam 未恢复，
5b 对 e_src 的读取能力即未具备，必须停线，不得用 `last_as_of` 或 `lower_gen` 代替。

### 4.2 ①a：写入/复用资格

只有同时满足以下条件的 segment 结果才能写入或复用 memo：

1. `segment.end_index < e_src`，必须严格小于；等于边界仍属可变 tail。
2. 该 segment、最近 center、A/C spans 所读的 segment 均落在同一已证 stable prefix。
3. memo key 的 run、block、center、version 身份与当前调用逐项相等。
4. hist/dif/close_src 对 A/C 的闭区间映射完整可读。

`e_src=0` 或 getter 越界/缺级时，稳定集为空：照常冷算本 bar，但不把 tail 结果写成 stable memo。

### 4.3 ①b：水位回退/结构回缩失效

出现任一情况即失效：

- 当前 e_src 小于 entry 写入时覆盖的 source 上界；
- stable segment 数小于 memo 已缓存数量；
- cascade/frontier rewrite 改变 run projection、center 或 block 身份；
- run 消失后重现且其结构 fingerprint 不同；
- 无法把旧 memo 的读域上界与当前 e_src 做同量纲比较。

能定位首个失效 entry 时截断受影响后缀；不能定位时清空该 `RunEntry.pan_memo`。禁止假设水位只增。

### 4.4 ①c：输入完备性与负缓存

- A/C source 坐标须由 `map_src_to_close_idx` 完整映射，且 hist/dif 切片覆盖闭区间。
- 映射失败、数据尚未到达、anchor 供给缺失等“当前不可验”允许冷路返回当前结果，但不写负缓存。
- 只有判定核完整执行后得到、且全部读域都在 e_src 前的结构 miss/force false，才可作为稳定负值。
- event 的 `judge_at` 与当前 pending 过滤永不进入 memo 值。

### 4.5 充分性论证

幸存证明链为：

1. e_src 前的 segment 已属于 confirmed 段前缀；当前代码用
   `centers[prefix_count-2].end_index` 保证最近 center 的 ownership 关系远离 frontier。
2. A/C 定位只向后读已封 segment 窗口；`locate_*` 无隐式时间状态。
3. center kind 来自同一 run blocks；链②另行保证其关系封口。
4. hist/dif/close_src 是 append-only 前缀，完整映射后的历史闭区间值不被未来 bar 改写。
5. cascade 的 `dirty_e` 取最小改变 source 坐标，公式用 `min` 把任何更早改写压入边界。

因此两链均过且 key 同一时，memo 所封读域跨 bar bit-stable；重用等于冷算。

### 4.6 不过度论证

- e_src 增长只推进新晋 stable segment，不清已证前缀。
- append-only hist/dif/close_src 不因新增尾部使既有完整闭区间失效。
- 水位回退时只清覆盖越界的后缀；无法定位才保守全清。
- over-shrink 只增加重算，不改变输出；over-grow 会复用陈旧值，严禁。

## 5. 失效链②：至少两个后继块门

### 5.1 ②a：稳定资格

对 pan segment 找到其 run-local ownership block 下标 `b_idx` 后，只有

```text
b_idx + 2 < blocks.len()
```

才允许 memo 写入/复用。该式表示目标 block 之后至少有两个完整 block；与 TURN
`ready = m.saturating_sub(2)` 的“链尾两块不落盘”纪律同构
（`p123_fast_replay.rs:283-318`；原始规则书 `p116_turnpoint_anchor_existence.rs:19-24,183-203`）。

门必须在**当前 run 的 blocks**上计算。全塔其他 run 的后继块不能替代。

### 5.2 ②b：blocks 回缩/重折失效

以下任一变化使相关 memo 失效：

- `blocks.len()` 回缩，使 `b_idx + 2 < len` 不再成立；
- `b_idx` 的 start/end center、kind、ownership 或身份改变；
- 目标 block 后两个 block 的身份改变或消失；
- run 投影重分区导致旧 `b_idx` 不再对应同一 center；
- 末窗重扫改写 frontier 子中枢数量，无法证明旧 block 已远离可变尾。

能按 block 顺序定位时从首个受影响 block 清后缀；不能定位时清整个 run memo。

### 5.3 ②c：两链合取与 forced 冷路

- memo 命中条件是链① ∧ 链② ∧ key 等值；不能以任一单链替代。
- 未达两个后继块的 tail 始终冷算，不写 stable memo。
- forced shadow 始终传 `None`，完全绕过 memo 命中与写入。
- 冷路仍执行当前 provider 的完整窄锚→front-anchor、Extreme、span、force、事件去重与排序；“不缓存”不等于“不计算”。

### 5.4 充分性与不过度

`decompose` 的 sealed 关系常规边界是 `i < m-2`，现行说明见
`decompose.rs:23-27`；`decompose_resume` 对多 frontier centers 的保守边界见
`:95-141`。目标 block 有两个后继时，其完成关系在常规 fr=1 情形远离临时尾；结合链①的 source 水位，center/segment 与 block kind 两个读域都已封，故可复用。

门只排除链尾两块；更早 block 保留，不因追加新 block 清空。block 追加使既有目标从 tail 晋级 stable 时可开始缓存，因此不是整 run 永久禁用。

### 5.5 已接受残余风险

`p116_turnpoint_anchor_existence.rs:19-23` 已声明：末窗一次重扫产出至少两个子中枢的 bar，理论上仍可能让常规“两个后继块”门提前成立。R1 明确接受 5b 继承该风险，不在本卡发明未经裁定的第三门。

残余兑现时应表现为：

- V0 身份校验不一致；
- `P123_SHADOW=1` mismatch；
- 全量双跑 dump diff。

任一信号出现即停止能力声明并保存现场；不得以“理论残余已接受”为由忽略实际 mismatch。

## 6. TDD 切片与接线顺序

以下测试名/fixture 是逻辑切片，不宣称复原了原卡的原始测试函数名。

### T0：冷路径刻画

- 固定现行 pan provider 的事件键、顺序、`interval_a/b`、`intake_fallback`、
  `divergence_confirmed`、`turn_source` 与 `b_center_start`。
- 覆盖窄锚成功、front-anchor 回退、Extreme 失败、坐标映射失败、span fallback、重复事件。

### T1：单一 e_src seam

- 把 `signal.rs:1187-1194` 的公式提成唯一函数/访问器，现行 resume 与 p123 读同源。
- 网格测试覆盖 `prefix_count<2`、`dirty_e` 更小、center end 更小、相等边界严格不封。
- getter 量纲为 source index；禁止与 5a legs count 混用。

### T2：链①

- e_src 增长：新晋 stable entry 一生一算，既有 entry 复用。
- e_src 回退/segment 回缩：截断或全清后与冷路相等。
- `segment.end_index == e_src` 必冷算、不落 stable memo。
- 坐标/MACD 未到不写负缓存；后续到达可产出冷路事件。

### T3：链②

- 0/1 个后继块不缓存，2 个后继块开始缓存。
- blocks 回缩、kind 翻转、span/ownership 改写均使受影响 memo 失效。
- run A 与 run B 即使 segment/source 相同也不共享 memo。
- 构造末窗多子中枢场景，至少由 shadow/身份校验捕获任何不等价。

### T4：memo 值与动态字段

- resident 命中仍按当前 `as_of` 重写 `judge_at`。
- pending 缩减只影响本次过滤，不污染 memo。
- provider window、锚 sidecar、去重和排序与冷路逐字段一致。
- 稳定负值只在完整读域且两链均过时缓存。

### T5：`RunEntry` 持有与替换

现行 dirty 分支用 `entries.insert(... RunEntry { ... })` 整体替换条目
（`p123_fast_replay.rs:912-920`）；直接加字段会在每次 dirty 时把 memo 丢掉。实施必须改为：

1. 先取得或新建该 `(level, run_source_start)` 的 entry；
2. 把 `Some(&mut entry.pan_memo)` 传入 dirty `evaluate_run`；
3. 评估后更新 `self_gen/lower_gen/last_as_of/events`，保留有效 memo；
4. 若 run identity 已变，先按 §4/§5 失效，再评估。

### T6：真冷 shadow

- dirty 路径传 `Some`；forced 路径传 `None`。
- 人为污染 memo 时 forced 应产生 mismatch，证明 oracle 未共享 memo。
- `P123_SHADOW=1` 正常靶向跑 0 mismatch。

### T7：集成

- `cargo test` 全绿。
- 250k/1M diff=0。
- 在线 pending/views 计数一致。
- 最终 wave-1+5a/5b 全量双跑合并封印。

## 7. 验收协议

5b 只有同时满足以下条件才可声明完成：

1. `cargo test` 全绿；以实装提交当时输出为准，不复用历史通过数。
2. 250k 与 1M 两档对拍 diff=0；stdout 语义行、约定 dump、正规化及任何物理字段白名单都须在交付报告逐项列出。
3. `P123_SHADOW=1` 全程 `shadow_mismatches=0`；forced 明确传 `None`。
4. 在线 pending/views 计数一致；不得只报 memo hit-rate 掩盖事件账不一致。
5. 全量双跑 dump diff=0，按 scene-ledger 的“wave-1+5a/5b 合并封印”口径结案。

`.chanlun/scene-ledger.md` 的历史 `:16/:17/:18` 在现行工位为 `:18/:19/:20`：

- `:18` 要求解决 5a/5b 后再做一次性合并总重跑；
- `:19` 如实登记 wave-1 只完成 6/8 与在线一致至 3.0M；
- `:20` 只豁免 wave-1 自身 4.6M 封口。

该豁免不覆盖 5a/5b。若只通过 cargo、250k/1M、shadow 与在线计数，
但全量合并双跑尚未完成，结论必须写“靶向验收通过、全量封印待完成”。

## 8. 2026-07-27 重锚表与复原来源

### 8.1 重锚表

| 符号/缝 | 文件:行 |
|---|---|
| `LevelDerived` | `rust/src/bin/p123_fast_replay.rs:358-370` |
| `RunEntry` | `rust/src/bin/p123_fast_replay.rs:374-383` |
| dirty `evaluate_run` 调用 | `rust/src/bin/p123_fast_replay.rs:897-907` |
| `RunEntry` 整体替换 | `rust/src/bin/p123_fast_replay.rs:912-920` |
| forced shadow 调用/比对 | `rust/src/bin/p123_fast_replay.rs:940-973` |
| `evaluate_run` | `rust/src/bin/p123_fast_replay.rs:1097-1148` |
| run projection centers/blocks | `rust/src/bin/p123_fast_replay.rs:1109-1112` |
| provider pan 主循环 | `rust/src/theta_v0/classifier/level_view.rs:817-893` |
| 最近确认中枢 | `rust/src/theta_v0/classifier/signal.rs:213-220` |
| block kind | `rust/src/theta_v0/classifier/decompose.rs:197-205,213-226` |
| 窄锚/前锚 | `rust/src/theta_v0/classifier/signal.rs:655-712,724-751` |
| Extreme | `rust/src/theta_v0/classifier/signal.rs:754-775` |
| 力度或关系 | `rust/src/theta_v0/classifier/divergence.rs:334-349` |
| e_src 证明/公式/回退守卫 | `rust/src/theta_v0/classifier/signal.rs:1130-1147,1187-1202` |
| e_src caller | `rust/src/theta_v0/classifier/mod.rs:2056-2080` |
| cascade dirty_e | `rust/src/theta_v0/classifier/mod.rs:1654-1657,1721-1735` |
| decompose 冻结不变量 | `rust/src/theta_v0/classifier/decompose.rs:23-27,95-141` |
| TURN 门/残余 | `rust/src/bin/p116_turnpoint_anchor_existence.rs:19-24,183-203` |
| p123 TURN 同构门 | `rust/src/bin/p123_fast_replay.rs:283-318` |

### 8.2 复原来源清单

1. `chanlun/review-results/issue93-implementation-card-20260721.md` §1：5a/5b 作用面、缝形及禁第二套水线/游标。
2. `chanlun/review-results/wave1-5a5b-blocker-clearance-20260720.md`：阻塞终态、历史段一暴露面、R1/R3 与验收豁免边界。
3. `chanlun/escalate/r1-r3-ruling-confirmation-checklist-20260720.md`：原卡 §2/§4/§5/§9 引用与 R1-R3 原口径。
4. `chanlun/escalate/g1-g2-window-ban-ruling-20260721.md` §4：R1-R3 逐项“确认”的终态登记。
5. `.chanlun/scene-ledger.md:18-20`：中止令、wave-1 验证登记、豁免与合并封印。
6. `.chanlun/review-results/on2w3-07a-impl-20260704.md`：e_src 严格 `<`、A/C + kind + hist 的稳定性与回退证明。
7. `/Users/silencehan/Projects/NewChanlun/kimi-export-session_-20260721-173955.md:2095-2117`：佚失卡历史节号/行号、两链与签名引用。
8. GitHub issue #69、#67、#479 票体。
9. 2026-07-27 `/tmp/wt-69` 现行 Rust 代码，只读重测行号与能力现状。

## 9. 签名落点与未能复原清单

### 9.1 重建后的签名约束

最终同工位的 `evaluate_run` 语义签名为：

```rust
fn evaluate_run(
    /* 现有参数原序保留 */,
    confirm_cursors: Option<&mut ConfirmCursorStore>, // 段二 5a
    pan_memo: Option<&mut PanMemo>,                   // 段三 5b
) -> Result<Vec<NestCandidateEvent>, String>
```

- dirty：两个 resident 状态按各自持有者传 `Some`；
- forced：两个都传 `None`；
- legacy/provider 冷入口：pan memo 为 `None`；
- memo 归 `RunEntry`，不得被每次 dirty 的整条目替换丢失。

这是依据幸存 R2/R3/R4 与现行 B3-B5 重建的接口约束；原卡精确参数顺序和命名没有幸存证据。

### 9.2 未能复原清单

以下内容没有幸存证据，本文未推测填空：

1. 2026-07-19 原卡逐字正文、完整目录、原八行读域表的原文与原作者措辞。
2. `PanMemoKey`、`PanMemoValue`、`PanMemo` 的原始字段名、精确 Rust 类型、容器、可见性、容量与淘汰 API。
3. resident provider 的原始函数名、`evaluate_run` 最终精确参数顺序、borrow 拆分方式与错误类型。
4. 原卡对稳定正值/稳定负值具体缓存到哪个中间层的精确选择；只复原了动态字段不得入 memo 的边界。
5. 历史 `freeze_boundary`、`last_freeze_boundary`、getter 的原始源码与提交身份。blocker-clearance 证明它们曾在 2026-07-20 工位被核，但现行树符号已不存在。
6. e_src 从 classifier level 映射到 p123 event level 的原卡精确索引表达式；实施须以 source 量纲网格测试重证，不能猜 `level`/`level-1`。
7. ①a/①b/①c、②a/②b/②c 的原卡逐字分段与原测试函数名；本文只按幸存清单和现行证明链复原其语义。
8. 原卡对 V0 身份校验的精确函数、计数器和失败输出格式。
9. 250k/1M 与全量双跑的原脚本、dump 文件名、正规化脚本、白名单全文和预期绝对耗时。
10. 原卡对 memo hit/miss/evict 的预期计数、内存预算与性能验收阈值。

这些佚失项不得在实施报告中倒写为“原卡规定”。任何新定的 struct/API/淘汰策略都须标明是依据本复原卡约束作出的现行设计，并以 forced 冷路、靶向 diff 与全量封印约束。
