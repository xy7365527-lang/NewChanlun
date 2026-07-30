# #668（N4）事件↔BSP 桥接对象实装——回炉后轮实施报告

- ticket：#668（blocked-by #666，母裁定票，含 2026-07-29 supersede 评论：键公式 v2）。
- 工位：`/tmp/wt-668b`，分支 `ticket-668b`，起点 = 干净 main 尖端 `9b68eb1967`（v1 工位
  `/tmp/wt-668`/分支 `ticket-668` 封存作史料，本轮零接触；v1 探针+停手报告经
  `git cherry-pick 692de98537` 带 hash 溯源进本分支）。
- 执行器：claude sonnet 5，全程前台单线程，未派发任何子代理/后台任务。
- **结论：v2 键公式真值表 BTC 三窗均判唯一（ambiguous_keys=0/0/0），按裁定进入桥接对象实装；
  实装完成，验收对拍 cmp=0；实装过程中发现一项独立于键唯一性的经验残留——N1 事件↔BSP 点的
  配对覆盖率偏低（尤以三类为甚），已如实登记入 ADR-0008 / CONTEXT.md / 本报告 §六，未在本票
  内解决，留后续票追查。**

## 一、v2 真值表读数（BTC 三窗，含锚段取法）

命令：`cargo run --release --bin p_issue668_bsp_key_truth -- <btc_1m_full.json> <max_bars>`
（`CARGO_TARGET_DIR=/tmp/wt668b-target`）。

### 方法（锚段坐标取法，逐类）

- **一类**（Buy1/Sell1）：反查生产候选事件流（`classify_with_tower_events` 第三个返回值
  `CandidateStreams`，v1 探针原弃用，本轮启用）按 `(level, side, parent_fingerprint,
  interval.1==point.source_index)` 找到同一次塔扫描产出的 Trend 候选（`CandidateKey.seg_a`/
  `c_start` 与本点共用同一 `first_structural_gates` 结构门），锚 = `(seg_a, (c_start,c_start))`。
- **三类**（Buy3/Sell3）：既有 `bits.third_class_entry`（`ThirdClassEntryIdentity`，#542 随证书
  直传，零新增）直读，锚 = `(leave_interval, (retest_interval.0,retest_interval.0))`。
- **二类**（Buy2/Sell2）：`OwnerRef::Type1Anchor` 反查同级一类锚点坐标，锚 =
  `(anchor_idx,anchor_idx)`——回抽段坐标 structurally 不可得（signal.rs「坐标 still-MISSING」，
  见 §四），诚实退化为 v1。

**★方法学订正（本轮内自查发现并修复，commit `00fa6fc3ea`）**：初版实现把本点自身所在段的右端
（一类 C 段/三类回试段的 `end_index`，恒等于该点自身 `source_index`）也塞进了锚——这会让键对
`source_index` 平凡单射，真值表「唯一」的判定退化成重言式（methodologically 空洞，不是真的验证
了结构去歧义）。订正为与 `CandidateKey`「C 右端与 as_of 均不在键中」同一纪律：本点自身右端一律
排除，已闭合、不再随 as_of 增长的历史段坐标全字段保留。**订正前后 ambiguous_keys 读数逐位不变**
（见下表），证实唯一性不是靠自身坐标注入制造的伪阳性。

### 读数

| 窗口 | levels | bsp 点数 | 置位 bit 数 | distinct_keys | anchor_unresolved | anchor_seg_unresolved | **ambiguous_keys** |
|---|---|---|---|---|---|---|---|
| 20,000 | 3 | 54 | 54 | 37 | 17 | 0 | **0** |
| 100,000 | 4 | 352 | 348 | 258 | 88 | 2（Sell1:2） | **0** |
| 300,000 | 5 | 1242 | 1199 | 901 | 280 | 18（Buy1:5, Sell1:13） | **0** |

`anchor_unresolved` = 基础指纹解析失败（`point.center` 非预期 owner 载体 / Type1Anchor 反查
无命中）；`anchor_seg_unresolved` = 指纹已解析但反查 Trend 候选事件未命中（仅影响一类，见
§六残留分析——~~这是**配对覆盖率**问题，不是键**唯一性**问题，两者正交~~【**已撤销，见 §十二**：
#670 评审证伪——覆盖率缺口正是判据错误的后果，二者不正交】）。三窗 `ambiguous_keys`
全部为 0 ⟹ v2 键公式判**唯一**（【**§十二订正**：这是检验域被同一判据窄化的算术必然，不是键的
真实区分力；「键唯一性」本身已不再是本对象的验收目标，见第三轮 supersede】），回炉裁定的实查项
通过。

## 二、对象定形（`rust/src/theta_v0/classifier/bsp_bridge.rs`）

一等塔对象命名 = `BspBridgeEdge`（bsp_bridge.rs:138），身份 = `BridgeKey`（:116，`(event:
CandidateKey, bsp: BspStructuralKey)` 对）。`BspStructuralKey`（:102）字段：`rule_version,
level, parent(ParentFingerprint), side, class(BspPointClass), anchor(Vec<(usize,usize)>)`。
`BspBridgeBook`（:356）append-only 修订簿，`BridgeStatus`（:125）二态 `Open`/`Invalidated`
（无 `Closed`——边只回答「配对现在还成立吗」，不像链证书有「不可再扩展」的终局语义）。

`CandidateEvent`/`BspPoint` 两个老对象**零改动**；`classify_impl`/`classify_with_tower_events`
签名**零改动**（新模块只读消费已产出的 `Classification`+`CandidateStreams`，同 `chain_cert`
先例的「纯产出零消费」接线方式——不是嵌进 classify_impl 循环体，而是复用其已内联产出的两条
主路径事实）。

## 三、双向产出接点（文件:行）

- `resolve_bridge`（bsp_bridge.rs:277）：单点单 bit → （BSP 结构键, N1 事件键）；三分支覆盖
  一/三类（trend_index 反查）与二类（继承锚点的候选事件）。
- `observe`（:322）：全量扫描 `Classification` 逐级逐点逐 bit 调 `resolve_bridge`，查无 N1
  事件的点静默跳过（Absent，不产占位记录）——「查簿命中才写」的机器落点。
- `latest_candidates`/`trend_index_by_interval_end`（:230/:250）：候选事件流折叠 + Trend 域
  反查索引，与 `cand_sub::latest_by_level`/`chain_cert::AliveIndex::build` 同方法学（纯折叠，
  非新判据）。

「双向」的落地方式：一类点的边 = 它自己触发的候选事件；三类点的边 = 其离开段对应的候选事件
（不要求背驰确认，只要求几何离开）；二类点的边 = **继承**其一类锚自己的候选事件——三条路径共用
同一 `trend_index` 反查表，一次折叠联合产出，不是三次独立扫描。

## 四、端死边死落点

`BspBridgeBook::apply`（:402）：`prior.status.is_terminal()` ⟹ 直接跳过（终态不复活）；否则
经 `make_revision`（:422）算出 `next`，`event_state`/`status` 单一来源 = N1 候选事件当次
`CandidateState`（`Invalidated` ⟹ 边 `BridgeStatus::Invalidated`，`invalidated_at` 一次写入
不后移）；`prior.projection() == next.projection()` ⟹ 跳过（同 as_of 重跑零 Delta，幂等）。
BSP 侧无独立状态机——一个 bit 一旦在某 as_of 置位即为冻结事实（`buy1`/`sell1` 严格背驰语义 +
#551「一次写入不后移」保证），故 `advance` 每次全量重扫 `classification` 天然覆盖历史点，
不需要 `chain_cert` 式的 `resident` 补集。

## 五、对拍读数（BTC 三窗，裁定⑦）

新增验收 bin `rust/src/bin/p_issue668_bsp_bridge_battery.rs`：`reference_join`（独立写就，
不调用 `bsp_bridge` 任何函数，直接拿 `Classification`/`CandidateStreams` 手工拼「(level,
source_index, class) → 命中的 N1 事件键」）与 `BspBridgeBook::heads()` 的产出逐条 cmp。

| 窗口 | reference_edges | produced_edges | missing_in_bridge | extra_in_bridge | **cmp** |
|---|---|---|---|---|---|
| 20,000 | 0 | 0 | 0 | 0 | **0** |
| 100,000 | 1 | 1 | 0 | 0 | **0** |
| 300,000 | 11 | 11 | 0 | 0 | **0** |

三窗逐条一致，`bsp_bridge.rs` 的产出与独立写就的参照集**完全等值**——本对拍验的是**接线**
正确性（两条独立代码路径共享同一 v2 键公式，键公式本身的唯一性已由 §一 真值表验过）。

## 六、经验残留（照实登记，未在本票内解决）

【**§十二订正**：本节标题「未在本票内解决」与下方「两件不同的事」「独立维度」的论断均已被
2026-07-29 #670 评审证伪并在修复轮改正，见 §十二。一类/二类部分不是独立残留，是本票判据的
一个 bug；三类部分（近零覆盖）仍照实维持「未解决，归 #688」。】

对拍读数本身很小（0/1/11 条边），远低于 §一 真值表里 37/258/901 个「已解出结构键」的点数——
~~这不是 bug，是**两件不同的事**：真值表问的是「解出的键唯一吗」（唯一），本节问的是「解出键的
点里有多少真找到了配对的 N1 事件」（覆盖率，独立维度）~~【**已撤销，见 §十二**】。用诊断输出
细分（`ISSUE668_BRIDGE_COVERAGE`）：

| 窗口 | trend_events | pan_events | b1(买1/卖1)点数 | b1 命中 | b2 点数 | b3 点数 |
|---|---|---|---|---|---|---|
| 20,000 | 0 | 41 | 0 | — | 17 | 37 |
| 100,000 | 7 | 227 | 3 | 1（33%） | 88 | 257 |
| 300,000 | 42 | 698 | 29 | 11（38%） | 280 | 890 |

三点发现：

1. **一类自身命中率仅约三至四成**（100k:1/3，300k:11/29），即便在 L0（两条产出路径共用同一
   `l0.segments`）也不例外（300k level0: 8/21≈38%）——`judge_first_cached`（BSP 抽取）与
   `cand_event::observations_for_level`（候选扫描）两条路径即便同源分段，命中率仍大幅低于
   直觉预期的「必然同步」（judge_first_cached 内部确实先调 `first_structural_gates` 才继续，
   逻辑上应与候选扫描同步）；根因未查——诊断已超出本对象「零改动判据函数」的授权范围，需要
   另票深入两条路径的 trend-gate 对齐细节（可能与 `center_trend_gate` 的中枢级 vs 段级粒度
   有关，未证实，照实存疑）。
2. **三类点几乎全员查无对应 Trend 候选**（`trend_events` 远小于 `b3_points` 一个数量级以上）：
   `judge_third_cert` 的离开段判据只要求几何破中枢（`leave.price > c.zg`/`< c.zd`），不要求
   `first_structural_gates` 的完整三门（趋势域 + 037:20 极值 + closes 可比较）——两者不同构，
   多数三类点的离开段根本不构成 Trend 候选。
3. **Pan 域候选事件数量级与三类点数量级接近**（`pan_events` vs `b3_points`：41/37、227/257、
   698/890，均同一数量级），提示 Pan 域候选（`judge_pan_div`，同一中枢两次同向离开的结构宽
   候选）可能是三类点更合适的 N1 配对对象，但 `judge_pan_div` 与 `judge_third_cert` 是两条
   独立定位逻辑（前者判「两次离开」，后者判「一次离开+一次回试」），`seg_c`/`leave_interval`
   是否对齐**未测**——本票不擅自扩大 N1 侧候选域（那是键/配对公式的再设计，超出裁定②授权的
   BSP 侧真值表实查范围），留后续票专项验证。

按裁定⑤「Absent（查无）非证伪」，上述低覆盖率本身**不违反本票裁定**——桥接对象按规格正确
实装、正确产出「查得到就写、查不到就不写」，验收对拍 cmp=0 证明产出与规格一致。但覆盖率数字
如实登记（ADR-0008 + CONTEXT.md 均已记录），供编排者判断是否值得开新票追查根因 / 扩展 N1 侧
候选域纳入 Pan。

## 七、测试计数（前后）

- **前**（本工位起点 `9b68eb1967`，探针提交之前，v1/v2 探针零依赖测试面，读数与本次实装前
  基线一致）：`cargo test --release --lib` = **2533 passed, 0 failed, 138 ignored**。
- **后**（含本票全部实装）：**2540 passed, 0 failed, 138 ignored**——净增 7（全部为
  `bsp_bridge::tests` 新增单测），0 failed，138 ignored 不变（既有 golden 零接触）。

## 八、新增测试清单（`rust/src/theta_v0/classifier/bsp_bridge/tests.rs`）

1. `first_class_point_pairs_with_matching_trend_event` —— 一类点命中匹配 Trend 候选，锚不含
   自身 source_index。
2. `no_matching_event_is_absent_not_an_error` —— 中枢指纹不匹配 ⟹ 查无静默跳过，不 panic。
3. `invalidated_event_freezes_edge_terminal_with_history_preserved` —— N1 事件缺席失效
   （`CandidateEventBook` 自有纪律）⟹ 边同步 `Invalidated`；终态不复活；旧 revision 留痕。
4. `same_as_of_rerun_is_zero_delta` —— 同 as_of 重跑幂等跳过。
5. `second_class_point_inherits_anchor_first_class_event_key` —— 二类边继承其一类锚自己的
   N1 事件键。
6. `third_class_point_pairs_with_leave_segment_trend_event` —— 三类边取离开段对应的 Trend
   候选（不要求背驰确认），回试段仅左端入锚。
7. `edges_for_bsp_point_finds_by_level_and_source_index` —— 查询入口按 (level, source_index)
   正确检索。

夹具原则（同 `chain_cert::tests` 先例）：N1 事件侧一律经真实 `CandidateEventBook::advance`
产出（不手搓 `CandidateEvent` 字面量）；`BspPoint`/`Classification` 是纯数据记录（无判定/
生命史协议），手搓字面量直接测试本模块自己的相关逻辑，不冒充判据测试。

## 九、ADR 与 CONTEXT.md diff

- 新增 `docs/adr/0008-bsp-bridge-object-and-structural-key-v2.md`：问题陈述 + Considered
  Options（4 项，含「BSP 六 bit 直接吸进 CandidateEvent」「另开字段存引用」「视图/投影方案」
  「二类回抽段坐标真实接线」四条被否选项及理由）+ Consequences（含 §六 经验残留原文摘要）。
- `CONTEXT.md` 新增两词条（插入「级别链证书」条目之后）：**事件↔BSP 桥接边**、**BSP 结构
  身份键 v2**，各含 `_Avoid_` 反面清单，格式对齐既有 N0-N3 词条风格。

## 十、Commit hash 列（本轮，`ticket-668b` 分支）

| hash | 说明 |
|---|---|
| `03b0a3339a` | cherry-pick v1 探针+停手报告（带 hash 溯源，090 纪律） |
| `1209166ee9` | v2 键公式真值表实查——三窗均唯一，回炉通过（含方法学误差，见下一条订正） |
| `00fa6fc3ea` | v2 探针方法学订正——锚不得含本点自身右端（防伪阳性唯一性）；同批含桥接对象
  完整实装（`bsp_bridge.rs`+测试+验收 bin）+ ADR-0008（`git add -A` 未拆分，内容正确，
  commit message 仅覆盖订正部分，实际改动范围见 `git show --stat`） |
| `3e55c261c9` | 补提交遗漏的 `p_issue668_bsp_key_truth.rs` 订正（`00fa6fc3ea` 暂存误差的
  补丁，内容属同一次订正） |

## 十一、遗留（照实）

1. **一类候选命中率约三至四成，根因未查**（§六 第 1 点）——超出本对象授权范围，需另票深入
   `judge_first_cached` 与 `cand_event::observations_for_level` 两条路径的 trend-gate 对齐
   细节。
2. **三类候选覆盖率近乎 0（Trend 域）**，Pan 域候选数量级接近但未验证是否为正确配对对象
   （§六 第 2/3 点）——需另票验证 `judge_pan_div` 的 `seg_c` 是否与 `judge_third_cert` 的
   `leave_seg` 对齐，若对齐则可扩展 N1 侧候选域覆盖三类。
3. **二类回抽段坐标 structurally 不可得**（`extract_second_signals` 入参层坐标剥离），v2
   诚实退化为 v1（仅一类锚身份）——若日后有需要，真实接线需要给 `RMove`/`LeveledMove` 补一条
   坐标供给线（`sub_moves[i2].start_index` 已存在，缺的是从 `extract_second_signals` 入参层
   到输出结构的传递），非本票判据函数零改动方法学下可做之事。
4. **N7 消费/并轨未涉及**——本票严格遵守裁定⑥「零消费接线」，p92/π runner 拼缝零触碰。
5. **未做 TowerCache 增量路径接线**——`BspBridgeBook`/`bsp_bridge.rs` 目前只在「一次性 fresh-
   full 调用」场景下验证（BTC 三窗单次 `classify_with_tower_events` + 单测多次 `advance()`
   调用），未接入 `classify_with_tower_events_incremental`/`TowerCache`（对齐 `chain_cert`
   同等阶段的现状——`chain_paths`/`ChainCertificateBook` 同样只在测试/诊断中被动态调用，未
   接生产增量路径）；`BspBridgeBook::advance` 的 revision/幂等/终态协议本身与调用次数无关，
   通用正确，接线增量路径属未来扩展，非本票判据范围。

## 十二、订正（2026-07-29 #670 影子评审 FAIL 回炉修复轮，
`chanlun/review-results/issue668-n4-fix-round1-20260729.md`）

`#670` 评审证伪本报告 §一/§六 的核心论断——「配对覆盖率」与「键唯一性」**不是**两件独立正交的
事：一类点 62% 命中率缺口正是判据错误的直接后果。`resolve_bridge` 的一类配对用
`point.source_index == event.interval.1`（Trend 候选 C 段**右端**等值），右端按 N1 生长纪律
明文**不入身份**、随 `as_of` 单调增长；真值表判「唯一」，是因为同一判据把 62% 的一类点预先排除
出了检验域（一个 Trend 候选只有一个当前右端，其余样本记 `anchor_seg_unresolved` 直接丢弃）。
纳入这些样本后 300k 窗实测 5 组撞键，最坏一组 4 点共键——§一「回炉裁定的实查项通过」这句在
「键唯一性是验收目标」这个前提下站不住。

第三轮 supersede 裁定改写了验收目标本身（一类点身份 = episode，同 episode 多物理点是修订史，
撞键自动消解，见 ADR-0008 第三轮 supersede 段）并把配对判据从「右端等值」改为「episode 区间
覆盖」。修复轮实测：一类覆盖率从 38%（300k 窗 11/29）升到 **100%**（29/29），100k 窗从
33%（1/3）升到 **100%**（3/3）。三类近零覆盖判据本轮未改，仍照实归 #688。

§一「anchor_seg_unresolved...两者正交」、§六标题与「两件不同的事/独立维度」两处论断撤回；
ADR-0008 Consequences 原句「不影响本票裁定②的真值表判据（唯一性已证成立）」同一批撤回；
CONTEXT.md 对应 `_Avoid_` 词条（「拿配对覆盖率低反推键公式错误」）已订正为非禁令。

不关票。
