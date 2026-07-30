# 事件↔BSP 桥接对象 + BSP 结构身份键 v2

买卖点（BspPoint）与塔内原生候选事件（CandidateEvent，N1 #540）此前互不相识——BSP 六 bit 只活在
`LevelState.bsp`，候选事件只活在 `CandidateStreams`，链证书（N3 #641）等下游消费者若要同时问
「这个候选事件对应哪个买卖点」与「这个买卖点的结构身份从哪个候选事件来」，只能各自 sidecar 重扫，
重复判据、易漂移。#666 四问四裁（2026-07-29，编排者）裁定补一个独立塔对象把这条关系立户口：**桥接
边**（`BspBridgeEdge`，`rust/src/theta_v0/classifier/bsp_bridge.rs`），身份 = （N1 事件键，BSP
结构键）对；`CandidateEvent`/`BspPoint` 两个老对象零改动。

裁定②要求 BSP 侧的结构身份键先过真值表：候选式「被破中枢指纹 + 方向 + 点类」（v1）在 BTC 三窗
（20k/100k/300k bar）实测**不唯一**（ambiguous_keys=10/74/260——同一被破中枢在趋势延续期可产多个
一类点、同一中枢可被反复离开回试产多个三类点，报告
`chanlun/review-results/issue668-n4-impl-ticket668-20260729.md`）。回炉裁定 v2 = v1 + 锚段坐标
（一类=背驰确认段 `seg_a`+`c_start`、三类=离开段+回试段起点、二类=一类锚坐标——回抽段坐标
structurally 不可得，诚实退化）；~~v2 在同三窗实测 **ambiguous_keys=0**（报告
`chanlun/review-results/issue668-n4-impl-ticket668b-20260729.md`）~~，本 ADR 落地这把键。
**订正（R2-LOW-1，见「第四轮 supersede」段）**：`ambiguous_keys=0` 出自已撤回的「键唯一性」
旧口径检验，是检验域被同一判据预先窄化的算术必然，不是键的区分力实证——见下文「第三轮
supersede」段的完整订正；现行真值表 bin 已不再产出这个字段。

## Considered Options

- **BSP 六 bit 直接吸进 CandidateEvent**（裁定①选项 a）：被否。手术面切老对象（`BspPoint`/
  `CandidateEvent` 均要改字段），收益不抵——N1/N3 先例（背驰段候选事件、级别链证书）都是「老对象
  不动、新关系自立户口」，本对象延续同一路线。
- **BSP 侧另开字段直接存事件引用**（裁定①选项 c）：被否。BSP 侧此时仍无稳定结构身份（要用什么键
  存引用是本票要先解决的问题），且链式再拼时还得倒回去重建，等于绕远路。
- **视图/投影方案**（裁定①选项 d，同 #540 先例）：被否，维持 #540 既有裁决——查询期望走稳定对象
  的字段，不走每次重算的投影闭包。
- **BSP 结构键塞 `source_index` 自身**：被否。塞了会让键对 `source_index` 平凡单射，真值表
  「唯一」的判定退化成重言式（methodologically 空洞）——与 `CandidateKey`「C 右端与 as_of 不入键」
  同一纪律，本点自身所在段的右端一律排除出锚。
- **二类回抽段坐标真实接线**（重跑 `find_second_type_structure` 取次级别走势起点）：搁置。
  `extract_second_signals` 入参层已把回拉走势坐标剥离（signal.rs「坐标 still-MISSING」），真实
  接线需要新判定路径，违反本对象「零改动 judge_*/extract_* 判据函数」的方法学；二类 v2 诚实退化为
  v1（仅一类锚坐标），如实登记。~~真值表在此退化下仍 0 ambiguous。~~ **订正（R2-LOW-1）**：同上，
  该读数出自已撤回的旧口径；且二类在生产数据上实测 0/280 命中（`resolve_second_class_anchor`
  的第二个条件——反查锚点自身是否持有一类 bit——恒假，见「第四轮 supersede」段 R2-MED-2），
  该正面断言当前无检验域支撑，按未实测处置。

## Consequences

- **命名独立、双向产出、端死边死、append-only** 全部落地（`bsp_bridge.rs`）：边的活死单一来源 =
  N1 事件侧状态（`CandidateState::Invalidated` ⟹ 边同步终态，留痕不复活；BSP 侧无独立状态机，一次
  置位即冻结事实）；「查簿命中才写」，查无 = Absent 非证伪，不产边、不产占位记录。
- **零消费接线**：p92 电池 / π runner 的 `source_index` 拼缝零触碰；本模块唯一调用点 = 单测 +
  `p_issue668_bsp_bridge_battery` 验收对拍 bin（BTC 三窗对拍 cmp=0）。
- **经验残留（第三轮 supersede 后收窄，见下）**：三类点绝大多数查无对应候选事件（Trend 候选的
  结构门 `first_structural_gates` 比三类离开段的纯几何判据严格得多，二者不同构），根因未查，归
  #688。一类点当时实测命中率只有约三至四成——**这一条已在第三轮 supersede 中查明并修复，不是独立
  残留，见下**。

## 第三轮 supersede（2026-07-29，#670 影子评审 FAIL 回炉，修复轮报告
`chanlun/review-results/issue668-n4-fix-round1-20260729.md`）

`#670` 影子评审发现上面「不影响本票裁定②的真值表判据（唯一性已证成立）」这句断言与事实相反：
一类点 62% 命中率缺口**正是**判据错误的直接后果，不是与键唯一性无关的独立现象——
`resolve_bridge` 的一类配对判据用 `point.source_index == event.interval.1`（Trend 候选 C 段
**右端**等值），而右端按 N1 生长纪律明文**不入身份**、随 `as_of` 单调增长；真值表「唯一性成立」
只是因为同一判据把 62% 的样本预先排除出了检验域（一个 Trend 候选只有一个当前右端，62% 落在
episode 内部而非右端上的点被记作 `anchor_seg_unresolved` 直接丢弃）。纳入这些样本后 300k 窗
5 组撞键，最坏一组 4 点共键。

裁定：
1. **一类点身份 = episode**（v2 锚不变：`seg_a` + `c_start`）。同一 episode 内的多个物理一类点
   （多段递进背驰，教义必然）是**同一候选身份的修订史**——`source_index`/pivot 是修订载荷，不入
   身份；撞键自动消解，不需要任何区分量。验收目标从「键唯一」改为「episode 归属唯一」（一个一类
   点只能属于一个 episode，`find_episode` 的 `debug_assert` 机器化此不变量）。
2. **配对判据改 episode 区间覆盖**：一类走事件侧驱动——遍历 Trend episode，回挂其
   `[c_start, interval.1]` 区间覆盖的全部一类点（同 level/side/中枢指纹）；二类同样按其一类锚
   坐标走 episode 区间覆盖（沿用同一失效模式的修复）；三类判据未改（v2 锚已验不空洞）。修复后
   BTC 实测一类覆盖率从 38%（100k 窗 1/3、300k 窗 11/29）升到 **100%**（3/3、29/29）。
3. **对拍验收门重定**：旧版本对拍参照集与生产模块共享同一 v2 键公式且逐字段同构复写，验不出
   判据层错误。新版本 = 独立实现的 episode 区间覆盖参照集（对拍 `edges()` 全量修订历史，非仅
   `heads()`）+ 跨 `as_of` 平价锁单测（对齐 N1/N3 先例）+ 负控用例（生长后不产新匹配点时，旧
   判据会让边永久卡在生长前的旧状态，新判据下必须继续跟随事件转 Invalidated）。
4. 对象骨架（append-only、终态不复活、幂等、零消费接线）与三类锚公式**不动**，回炉不推倒。

## Consequences（第三轮更新）

- 一类/二类覆盖率缺口已定位为判据错误并修复（不是独立候选域结构性问题）；三类近零覆盖仍归 #688。
- 「覆盖率与键唯一性正交，不可互相反推」这一论断整体撤回：本票的实测史正是「低覆盖率反推判据可疑」
  这条路径的正面案例，`chanlun/CONTEXT.md` 对应 `_Avoid_` 词条已订正。

## 第四轮 supersede（2026-07-29，#670 影子评审第 2 轮 FAIL 回炉，修复轮报告
`chanlun/review-results/issue668-n4-fix-round2-20260729.md`）

`#670` 影子评审第 2 轮发现第三轮 supersede 裁定①（「同一 episode 内的多个物理一类点是同一
候选身份的修订史」）在**实现**上被映射成了错误的载荷形态：`resolve_first_class_episode_edges`
把 k 个同时并存的物理点按 `source_index` 升序产出 k 条**独立观察**，全部共享同一 `BridgeKey`，
交给 `apply()` 顺序处理——而 `apply()` 的幂等/终态判据（prior vs next 二元比较）隐含假设**每次
`observe()` 每个 key 只产一条观察**。k 条独立观察打破这个前提：
1. **幂等破**（R2-HIGH-1）：同一 `(classification, streams, as_of)` 重跑，k 个点被重新逐个
   `apply`，与「当前链头」比较逐一不等 ⟹ 无条件 append k 条 churn revision，300k 窗实测每次
   重跑 +12、边数无上界增长（29→41→53…）。
2. **终态挡提前生效**（R2-HIGH-2）：失效时，遍历序第一个物理点的 `apply` 已把该 key 判
   `Invalidated`（终态），后续物理点的 `apply` 因终态挡直接 `return None`——链头因此**回退**到
   遍历序第一个物理点（而非最新物理点），其余物理点的 `Invalidated` 永不落簿，查询入口
   `edges_for_bsp_point`（按 `heads()` 过滤）永久丢失这些点（300k 窗实测 7/29 点查无）。

裁定：
1. **载荷形态改集合**：`BspBridgeEdge::bsp_source_index: usize` 升级为
   `bsp_source_indices: Vec<usize>`（有序去重的物理点集合，`head_source_index()` 取集合内最大值
   为链头/兼容旧单点语义）。`observe()` 内部先产原始逐点观察（`RawPointObservation`），再**按
   `BridgeKey` 分组折叠**（`BTreeSet<usize>` 归并）成一条 `BridgeObservation`——保证每个 key
   每次 `observe()` 恰好一条观察，`apply()` 的幂等/终态二元比较前提重新成立。这不是撤销裁定①，
   是把裁定①的语义（多物理点=同一身份的并发载荷，非顺序时间序）正确落地到实现里。
2. **三类判据同步迁移到 episode 区间覆盖**（撤销第三轮裁定 2「三类判据未改」的例外）：三类原用
   `leave_interval.1` 对 `trend_index_by_interval_end`（右端精确等值）反查，与一类修复前的
   失效模式相同——Trend 候选 `growth_revision` 后索引键随当前 `interval.1` 漂移，而三类点自身
   记录的 `leave_interval.1` 是过去时，二者错位后点永久失联。迁移到 `find_episode` 区间覆盖
   反查后与一类/二类同构，`trend_index_by_interval_end` 连同其 `debug_assert`/单测一并移除
   （静默覆盖风险迁移到统一的 `find_episode`，新增
   `overlapping_episodes_trip_find_episode_debug_assert` 覆盖）。
3. **查询入口改集合语义**：`edges_for_bsp_point` 从 `bsp_source_index == source_index` 精确匹配
   改为 `bsp_source_indices.contains(&source_index)`，300k 窗查询完备性从 22/29 回升到 29/29。
4. **跨 `as_of` 平价锁换真形态**（撤销第三轮替代验收物①的具体实现，保留其「跨 as_of 平价锁」
   意图）：旧测试 `bridge_book_incremental_equals_full_replay_across_as_of` 是「同一 `inputs`
   列表跑两遍」（`f(x)==f(x)`，结构上不可能失败，对齐的是 N3 最弱一面且未抄 N3 的三条非真空
   锁）。新测试 `bridge_book_incremental_final_state_equals_fresh_full_replay_from_empty` 改为
   两个真正不同的驱动（对齐 N1 `..._full_replay_equals_incremental` 先例）：驱动 A = 逐 `as_of`
   递进推进的增量簿；驱动 B = 仅用终态一步输入、从空簿单次 `advance`。经实测核实（poison 注入
   验证，见修复报告）：该锁对「同一 `observe()` 调用内多条同 key 原始观察被错误顺序 `apply`」
   这类缺陷**不具判别力**——因为该缺陷是 `observe()`+`apply()` 管线内部对固定终态输入的确定性
   函数，driver A 与 driver B 最终都会经过同一条（可能有缺陷的）管线收敛到同一个（可能错误的）
   不动点，二者不会分道。该锁的真实职责收窄为「验证增量簿的记账不携带隐藏状态偏移」——一个不同
   于 R2-HIGH-1/R2-HIGH-2 但同样值得锁住的性质。R2-HIGH-1/R2-HIGH-2 这类缺陷的实际回归锁是本轮
   新增的 4 条直接单测（`multi_point_episode_same_as_of_rerun_is_zero_delta`/
   `invalidation_after_multi_point_episode_covers_all_points_and_stays_queryable` 等）——已用
   poison 注入法逐条验证：临时还原「不分组」的旧实现后，这 4 条单测（含三类同构版）确实转红，
   现行实现下全绿。
5. 二类结构性不可解（`resolve_second_class_anchor` 第二条件在生产数据上恒假，0/280 命中）、
   `find_episode` 的 `debug_assert` 在一类路径无机器保证、真值表检验域 20k=0 报 SUCCESS 是空域
   重言式——三者（评审 #670 第 2 轮 R2-HIGH-3/R2-MED-2）**本轮不处置**，dispatch 明确将其排除
   在本次修复单之外，交编排者另裁范围与优先级。

## Consequences（第四轮更新）

- R2-HIGH-1/R2-HIGH-2 根因（多物理点载荷形态映射错误）已修复；300k 窗实测：同输入连续
  `advance` 三次 delta2=delta3=0、边数稳定（22 条 revision，覆盖 29 个物理点不变）；失效后
  查询完备性 29/29（原 22/29）。三类判据与一类同构（迁移到 episode 区间覆盖），MED-1 三类残留
  处置为「已同构，`trend_index_by_interval_end` 移除」。
- 跨 `as_of` 平价锁改真两驱动形态，但据实登记其对本次缺陷类别不具判别力（见上）——不作为
  R2-HIGH-1/R2-HIGH-2 的证明义务方，直接单测才是。
- R2-HIGH-3（episode 归属唯一机器保证缺位）、R2-MED-2（二类锚生产不可解）、R2-MED-3（对拍规模
  信息量）**未处置**，留待编排者下一轮裁定范围。
