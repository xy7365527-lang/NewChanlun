# N4 桥接配对覆盖率根因（issue #688）

- 票：#688（map #529，research）
- 日期：2026-08-14
- 状态：只读研究，不改代码、不提交
- 方法：读票 #688/#666/#668/#670 全链 + 实施/修复/评审报告 + ADR-0008 七轮 supersede + CONTEXT.md；代码级定位判定点（file:line 逐条）。
- 结论前置：**一类命中三至四成已在 #668 修复轮查明根因并修到 100%（不是独立残留）；三类 Trend 域近零是候选域结构性错位（判据不同构），且 Pan 域被桥接反查域硬排除在门外——两问都已能落根因，处置方向见 §三。**

---

## 一、根因判定（file:line + 证据）

### 1.1 一类命中三至四成 —— 根因已查明并修复（#668 修复轮）

票面「一类命中三至四成（100k b1=3 中 1）」引用的是 #668 修复轮 **之前** 的旧读数。该缺口已在 #668 修复轮 1 定位根因并修复（报告 `chanlun/review-results/issue668-n4-fix-round1-20260729.md`，ADR-0008「第三轮 supersede」段），本票不翻案，仅把根因按票面要求的三分桶钉死：

| 分桶 | 结果 | 证据 |
|---|---|---|
| 候选不存在 | **0** | #670 评审独立探针实测：300k 窗 29 个一类点**全部**落在唯一 Trend 候选区间内（「29/29 全落唯一 Trend 候选区间，缺的是 join 不是候选域」，#668 comment） |
| 候选存在但键不合（join 判据错） | **62–67%**（修复前） | 旧判据 `point.source_index == event.interval.1`（Trend 候选 C 段**右端等值**）；右端按 N1 生长纪律明文不入身份、随 `as_of` 单调增长（`cand_event/key.rs:38`「C 右端与 as_of 均不在键中」）⟹ 生长后右端漂移，落在 episode 内部而非右端上的点被漏配 |
| 级别错位 | **0** | 300k level0 命中率同样 8/21≈38%，与整体一致——两条产出路径共用同一 `l0.segments`（实施报告 §六 第 1 点），非级别错位 |

- **修复后读数**：一类覆盖率 100k 33%→**100%**（3/3）、300k 38%→**100%**（29/29）（fix-round1 §二表 + 实施报告 §十二）。
- **现判定点（file:line）**：
  - `rust/src/theta_v0/classifier/bsp_bridge.rs:318-336` `find_episode` —— episode 区间覆盖反查（`c_start <= source_index <= interval_end`），替换旧右端等值；
  - `rust/src/theta_v0/classifier/bsp_bridge.rs:491-530` `resolve_first_class_episode_points` —— 逐物理点调 `find_episode`（事件侧驱动）；
  - 旧右端等值判据在现行代码中已不存在（仅存于历史报告/ADR 撤销段，090：检索式 `grep -rn "interval.1" rust/src/theta_v0/classifier/bsp_bridge.rs` 现仅命中注释与三类 leave_interval.1，无一类右端等值）。
- **结论**：票面「一类未命中 6–7 成分桶」的答案 = 全部落在「候选存在但键不合」（join 判据右端等值 bug），已修复，无需新处置。**#688 的一类部分实质上已闭环。**

### 1.2 三类 Trend 域近零 —— 候选域结构性错位（判据不同构）

三类点的配对链路：三类点判据（`judge_third_cert`）产点 → 桥接对象用 `leave_interval.1` 在 **Trend episode** 里 `find_episode` 反查 → 查无 ⟹ 不产边。近零的根因不在 join（join 已与一类同构为区间覆盖），而在**候选域本身**：三类点的判据比 Trend 候选结构门弱得多，二者不同构。

**判定点逐条：**

- **三类点判据**（弱，无趋势门/无前中枢/无 037:20/无力度）：
  - `rust/src/theta_v0/classifier/signal.rs:580-630` `judge_third_cert` —— 离开段破核心（`leave.price > c.zg` / `< c.zd`）+ 回试反向不重回（`retest.price > c.zg` / `< c.zd`）。**无任何趋势方向、无前中枢、无 037:20 破极值、无 MACD 力度要求**。
  - `rust/src/theta_v0/classifier/signal.rs:2092-2104` `judge_segment` 三类分支 —— 触发条件是 `if i > 0`（当前段作 retest、前段作 leave），**不要求** `gate_dir`（趋势门）。⟹ 三类点在盘整（Consolidation）块、单中枢、未破极值等情形都产点。
  - `rust/src/theta_v0/classifier/signal.rs:566-578` `third_class_entry_identity` —— `leave_interval = (leave_seg.start_index, leave_seg.end_index)`（**单段**坐标，非离开 episode）。

- **Trend 候选结构门**（严，四重门）：
  - `rust/src/theta_v0/classifier/decompose.rs:163-170` `center_trend_gate` —— 只对 `MoveKind::Trend` 块内中枢开门（`for g in &mut gate[b.start_center+1..=b.end_center]`），盘整块方向 = `None`。
  - `rust/src/theta_v0/classifier/cand_event/observe.rs:148-150` `structural_observation` —— ① `center_gate.get(c_idx).flatten()?`（趋势方向门，盘整中枢直接 `None` 返回）；② `centers.get(c_idx.checked_sub(1)?)?`（**前中枢必须存在**，即 ≥2 中枢）。
  - `rust/src/theta_v0/classifier/cand_event/observe.rs:152-153` —— ③ A 段必须可配对（`locate_departure_move_a` 非空）。
  - `rust/src/theta_v0/classifier/signal.rs:266-325` `first_structural_gates` —— ④ 破最后中枢方向 = 趋势方向（`:284-290`）+ 037:20 破 b 包络极值（`:311-314`，作 `extreme` 谓词进状态，不作缺席）+ closes 可比较（`:317-324`）。

- **桥接反查域**：
  - `rust/src/theta_v0/classifier/bsp_bridge.rs:290-303` `trend_episodes` —— **只过滤 `CandidateKind::Trend`**；三类点 `resolve_bridge` 三类分支 `bsp_bridge.rs:424-446` 用 `entry.leave_interval.1` 调 `find_episode`（`:431`）在 Trend episode 里反查，`find_episode` 查无 ⟹ `None` ⟹ 不产边（`:431` 的 `?`）。

**根因一句话**：`judge_third_cert` 的「离开段破核心 + 回试不重回」是**纯几何判据**，其产点域严格超集于 Trend 候选的 C 段域（Trend 候选还需：趋势方向门 + 前中枢 + A 段配对 + 037:20 破极值 + comparable）；桥接对象又只把三类点往 Trend 域里配 ⟹ 凡是落在「盘整中枢 / 单中枢 / 未破 b 包络极值 / A 段不可配对」的三类点，Trend 域无 episode 覆盖，**结构性查无**，覆盖率近零是必然而非 bug。

证据数字（实施报告 §六，`ISSUE668_BRIDGE_COVERAGE`）：`trend_events` 远小于 `b3_points` 一个数量级以上——20k `trend=0` vs `b3=37`；100k `trend=7` vs `b3=257`；300k `trend=42` vs `b3=890`。

### 1.3 Pan 域嫌疑 —— 静态判定：与三类「共享几何、不同构」，且被反查域硬排除

- **Pan 域候选判据**：`rust/src/theta_v0/classifier/signal.rs:1322-1351` `judge_pan_div` —— 盘整中枢（`judge_segment` 的 `if kind_consol` 分支，`signal.rs:2083-2087`）+ 破核心 + **同一中枢两次同向离开**（A 锚 + C 段）+ Weak 力度（面积/黄白线/同向柱任一衰减）。产出投影 `rust/src/theta_v0/classifier/cand_event/observe.rs:221-247` `pan_observations_for_level`，kind = `CandidateKind::Pan`。
- **两类几何对比**：
  - 三类 `leave_interval` = `(leave_seg.start_index, leave_seg.end_index)`（**单段**，`signal.rs:575`）；
  - Pan 域 `seg_c` = `(lambda_c, seg.end_index)`（**离开 episode 区间**，`signal.rs:1222`；`lambda_c = departure_move_c_start`，`signal.rs:1191`）。
  - 右端：三类 leave 段端点与 Pan C 段破核心段端点**可为同一段**（都在「破核心离开」这一几何事件上），右端可能对齐；左端语义不同（单段起点 vs 离开 episode 起点 λ_C）。**是否对齐在 #668 未测（实施报告 §六 第 3 点原文「seg_c/leave_interval 是否对齐未测」），本票静态层面判「右端同源、左端不同构」，未见任何代码路径保证二者对齐——090 照实：数据级对齐验证查不到，未做。**
  - 判据本质差异：Pan = 「两次**同向**离开 + 力度」，三类 = 「一次离开 + 一次**反向**回试」——不是同一个事件序列（`signal.rs:1311-1318` vs `signal.rs:588-590` 两段注释互证）。
- **被反查域硬排除**：`bsp_bridge.rs:293` `filter(|event| event.kind == CandidateKind::Trend)` —— Pan 域候选（`CandidateKind::Pan`）**从不进入** `trend_episodes`，因此无论 Pan 域与三类是否对齐，现行桥接对象都**不可能**拿 Pan 候选配三类点。这是「Pan 域嫌疑」问题里唯一可以代码级钉死的事实。

**数量级接近的含义（照实标注为未验证的提示，非证据）**：`pan_events` vs `b3_points` = 41/37、227/257、698/890（实施报告 §六 第 3 点）。两者都锚定「离开中枢核心」这一几何事件，数量级接近提示在盘整中枢上高度重叠，但 `pan_events` 是**事件计数**（Pan 域候选）、`b3_points` 是**物理点计数**，计数域不同，数量级接近不能直接推导「Pan 是三类正确配对对象」——这正是 #668 §六 留待验证的那一步。

---

## 二、影响面

1. **一类**：根因已修复（右端等值 join → episode 区间覆盖），覆盖率 100%。影响面收窄为零，仅需在 #688 关票时把「一类部分」标记为「已由 #668 修复轮闭环」。
2. **三类**：BTC 三窗 `trend_events`（0/7/42）相对 `b3_points`（37/257/890）恒近零。当前桥接对象对三类点产出的边数≈0（三窗对拍 `hit_by_class` 中 Buy3/Sell3 空域，`ISSUE668_BRIDGE_BATTERY_BY_CLASS` 显式标注「此类空域，cmp 无信息量」，ADR-0008 第五轮 §4）。**N4 桥接对象的「三类边」在 BTC 上实质是空产物**——不违反 #668 裁定⑤「Absent 非证伪」（正确实装 + 正确查无），但「事件↔三类点互挂」这一裁定③承诺对三类点名存实空。
3. **二类**：0/280（`resolve_second_class_anchor` 第二条件「锚坐标持一类 bit」恒假，ADR-0008 第五轮 §3 + 第六轮 §3 补充数据 145/280 在簿无一类 bit、135/280 不在簿）。与三类同属「候选域/锚语义结构性错位」，已在 #668 定性为结构性不可解，待编排者裁（是否归 #688 或二类键公式重选锚）。
4. **下游**：零消费接线（#666 裁定④），p92/π runner 不受影响；但 N7（Consume_at）将来若消费「事件↔三类点」边，会直接吃空。桥接对象本身生产行为无 bug，影响面在**覆盖率承诺**与**后续 N7 消费面**。

---

## 三、建议方向（喂 #529 后续裁定/实装）

> 以下均为候选方向，裁定权在编排者；本票只做根因测量，不做裁定。

1. **一类**：关票时登记「已闭环（#668 修复轮）」，无新处置。
2. **三类（核心待裁项）**：根因已钉死为「三类判据（纯几何）与 Trend 候选门（趋势+前中枢+A 段+037:20+comparable）不同构」。可选处置三择一（对应 #688 票面「扩配对域 / 收窄桥接声明 / 不动」）：
   - **(a) 收窄桥接声明**（最小面）：N4 桥接对象只承诺「一类（事件↔一类点）+ 二类继承」，三类边在教义上明确「仅当离开段同时构成 Trend 候选 C 段时才有」，文档（ADR-0008/CONTEXT.md）如实收窄。代价：裁定③「三类点出生回挂事件」承诺对绝大多数三类点不成立。
   - **(b) 扩配对域纳入 Pan**：先做数据级对齐验证（`judge_pan_div` 的 `seg_c` 与 `judge_third_cert` 的 `leave_interval` 是否右端同段），再决定是否把 `trend_episodes` 的过滤扩到「Trend ∪ Pan」（`bsp_bridge.rs:293` 是唯一改动点）。**前置风险**：Pan 是「两次同向离开+力度」、三类是「一次离开+一次回试」，二者不同构，扩域后配对语义会从「三类点回挂其离开段候选」变成「三类点回挂同中枢 Pan 候选」，需要编排者裁「这是否仍是 N4 要的桥接边」。且 #666 裁定②键公式（被破中枢指纹+方向+点类+锚段坐标）当前**不含 Pan**，扩域等于改键域，属再裁定。
   - **(c) 不动**：维持现状（三类近零覆盖照实登记），等 N7 消费面出现真实需求再裁。
   - 本票倾向证据：**(a) 或 (c) 是低风险路径**；**(b) 需要一次新的键域/语义裁定，不能当「修 bug」直接实装**（Pan 与三类判据不同构，`signal.rs:588-590` vs `1311-1318` 已坐实）。
3. **二类**：与三类同捆待裁——二类锚 `Type1Anchor(type1_src)` 载的是走势 m1 终点坐标，m1 终点要过背驰确认门才是一类点，二者生产不重合（`bsp_bridge.rs:376-406` `resolve_second_class_anchor` 文档 + ADR-0008 第五轮 §3）。要么重选二类锚语义，要么收窄二类桥接声明。
4. **落盘建议**：#688 关票时把「一类已闭环」与「三类=判据不同构的候选域错位（Trend 域），Pan 域被 `bsp_bridge.rs:293` 排除」两条根因写进 #529 的 Decisions so far，供 N7 出笼时作为消费面前置事实。

---

## 附：检索式与穷举断言（090）

- 票链：`gh issue view 688/529/666/668/670 --json number,title,state,body,comments`。
- 代码定位：`grep -rn "fn judge_third_cert\|fn judge_first_cached\|fn judge_pan_div\|fn first_structural_gates\|fn center_trend_gate\|fn observations_for_level\|fn make_third_point\|fn make_first_point" rust/src/`；`grep -rn "CandidateKind::" rust/src/theta_v0/classifier/cand_event/*.rs`。
- 报告源：`chanlun/review-results/issue668-n4-impl-ticket668b-20260729.md`（§六/§十二）、`issue668-n4-fix-round1-20260729.md`、`docs/adr/0008-bsp-bridge-object-and-structural-key-v2.md`（七轮 supersede）。
- 穷举断言：「旧右端等值判据已不在现行 bsp_bridge.rs 中」——检索式 `grep -n "interval.1" rust/src/theta_v0/classifier/bsp_bridge.rs`，命中 6 处，逐一核对无一是一类右端等值（`65/66/300` 注释、`431` 三类 leave_interval.1、`428/429` 注释）。
- 查不到项（照实）：① Pan 域 `seg_c` 与三类 `leave_interval` 的**数据级**对齐验证——#668 未做，本票未重跑数据，静态层面只判「右端同源、左端不同构」；② 三类点落在盘整 vs 趋势块的比例分桶——现有诊断 bin 未输出该维度（`trend_events` 是 Trend 事件数，不含三类点按中枢块类别的分解），查不到。
