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
structurally 不可得，诚实退化）；v2 在同三窗实测 **ambiguous_keys=0**（报告
`chanlun/review-results/issue668-n4-impl-ticket668b-20260729.md`），本 ADR 落地这把键。

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
  v1（仅一类锚坐标），如实登记，真值表在此退化下仍 0 ambiguous。

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
