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
- **经验残留（未在本票内解决，留后续票）**：BTC 实测配对**覆盖率**（不同于键**唯一性**）偏低——
  三类点绝大多数查无对应候选事件（Trend 候选的结构门 `first_structural_gates` 比三类离开段的纯几何
  判据严格得多，二者不同构；Pan 域候选事件数量级与三类点数量级接近，是否为更合适的配对对象未测）；
  即便一类点自身，命中率在实测三窗中也只有约三至四成，根因未查（候选扫描与 BSP 抽取两条路径即便在
  L0 同源分段下仍有相当比例不重合，超出本对象「零改动判据函数」的诊断权限）。这不影响本票裁定②的
  真值表判据（唯一性已证成立），但影响本对象实际产出的边密度，留作独立票追查。
