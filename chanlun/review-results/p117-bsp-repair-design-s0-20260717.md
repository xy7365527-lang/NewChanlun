# p117：BSP 侧口径修复 S0「取段坐标桥」施工图（113 案 = 73.4%，最大单因）

日期：2026-07-17 ｜ 性质：**设计工位·只读出图**（本轮不改任何生产源码；实装由编排者随后串行派发）
工位：主线阶段 1 关②（BSP 侧口径修复）——S0 分项。worktree：`/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`）
归因依据：`chanlun/review-results/p112-trend-predicate-caseaudit-20260717.md` §4 S0 行（:121）、§6-5（:162-164）；
案集坐标：`/tmp/p109_full2.txt` P109_CHAIN_DETAIL（754 链）+ `/tmp/p112_full.txt` P112_CASE（154 事件逐案）；
口径前提：R1–R3 已实装（`p115-predicate-caliber-impl-20260717.md`）——confirmed 趋势事件
`turn_source = t*`（全合取首个全成立时点，`level_view.rs:714-716`），seg_c = 全离开段（`level_view.rs:860-871`）；
教义判定书：`doc-trend-divergence-predicate-20260717.md`。
纪律：生产源码只读、p116 全量重放后台在跑（不 cargo build/test、不动 p116 bin）、主仓零写入、零 git mutation、
`rust/Cargo.toml` 零改动；教义引用已直读主仓 `docs/chanlun/text/blog/` 原文逐行核对（课号:段号 = md 文件行号）。

---

## 0. 结论先行（施工图要点）

**推荐方案：加映射层（lookup 侧坐标桥），不改 BSP 账本、不改事件模式、不碰塔/笔/线段/中枢/inclusion 主干。**

- 桥接对象：**终端背书的查法**（`terminal_of` 闭包），不是 BSP 生成器、不是事件坐标。
  现行查法 = `levels[k].bsp` 上 `point.source_index == event.turn_source` 的**位格等式**精确匹配
  （`p92_nest_replay_postruling.rs:940-953`，另 8 个归档探针同构副本）；S0 = 该等式在两个坐标格之间
  结构性不可命中（D2 turn = L0 腿坐标格；BSP 点 = 级别-k 单元窗终点坐标格）。
- 桥键（推荐口径，§3）：**c 责任单元** `u* =` 级别-k 单元序列中首个
  `start_index ≥ event.interval_b.0`（c_start，c 区域起点）`∧ end_index ≥ event.turn_source`（确认点 t*）的单元；
  账本查法改为 `bsp.find(p.source_index == u*.end_index ∧ p.bits.confirm_side(event.side))`。
  教义读法：u* = **c 内含次级别中枢（037:22）在级别-k 网格上首个完成于确认点的单元**——c 自己的
  次级别结构见证；`start ≥ c_start` 守卫把「B 自身单元」（见证的是 b-vs-a 的前序背驰）排除出桥键。
- 落点：`nest.rs` 新增单一来源函数 `terminal_bits_bridged`（lib 层，账本只读）；
  `p92` bin 预建 `units_by_level` 后把 `terminal_of` 闭包切到桥接查法。仅此两个文件。
- 映射性质：**函数**（每事件 ≤1 个责任单元），非单射（多事件可共单元，∃ 语义合法）、非满射；
  c 见证未形成 ⟹ None（诚实判负，037:18 否则条款域）。
- bit-exact：塔/笔/线段/中枢/inclusion/BSP 账本内容零触碰（§4），不标升级裁定；
  有意变更面 = nest 证书集合（终端门放宽为桥接口径，修复目的本身），按 p115 §6 纪律单口径替换、
  诚实报告失证风险，不叠加旧口径。

---

## 1. 病因（逐案坐标 + 代码坐标 + p112 归因引用）

### 1.1 机制链（全部坐标只读核对于本 worktree）

1. **终端门查法**：nest 装配的基例门 `assemble_typed_certificate`（`nest.rs:476-481`）要求
   `terminal_of(base)` 返回的 `BspBits` 满足 `confirm_side(base.side)`，否则 `None`（证书拒绝）。
   `terminal_of` 由调用方以闭包注入（`nest.rs:470`）；生产重放 p92 的注入 =
   `terminal_bits_new`（`rust/src/bin/p92_nest_replay_postruling.rs:940-953`）：

   ```rust
   classification.levels.get(event.level as usize)?.bsp.iter().find(|point| {
       point.source_index == event.turn_source && point.bits.confirm_side(event.side)
   }).map(|point| point.bits)
   ```

   即「同级（`levels[event.level]`）同 source（位格等式）confirm_side BSP」。同构副本另存于
   p102:346、p107:540、p108:826、p109:164、p111:149、p112:537、p113:730、p116:1080（归档探针仪器）。
2. **D2 侧 turn 坐标（L0 腿坐标格）**：趋势基例事件由 `provide_nest_candidate_events`
   （`level_view.rs:652`）产出。R1 后 confirmed ⟹ `turn_source = t*`（`level_view.rs:714-716`；
   t* = `trend_confirm_time` 全合取首个全成立时点，`level_view.rs:548-638`，为某 L0 腿终点）；
   未确认 ⟹ `turn_source = seg_c.1`（全离开段末个同向腿终点，`level_view.rs:860-871`）。
   p112 测量时（R1 前）turn = seg_c.1 = **首离开腿终点**（#105 单腿窗口，`level_view.rs:839-845`
   正向 find + 旧 c_end=c_terminal.end）。两代的共同点：**turn 是 L0 线段腿终点坐标**。
3. **BSP 侧点坐标（级别-k 单元窗终点坐标格）**：`levels[k].bsp` 由
   `extract_first_third_for_level`（级别-N 一/三类，`mod.rs:414-417`）在
   `units = project_to_units(&tower[k], &levels[k-1].moves)`（`mod.rs:437`、`recursive_tower.rs:891-909`）
   上判定产出；一类点由 `judge_first_cached` 在破中枢单元端点落点（`signal.rs:368`
   `make_first_point(end.source_index, …)`）。**单元终点 = tower[k] 中枢窗（seed 三段+延伸段）的
   末腿终点**（`recursive_tower.rs:899-907`），是全时间轴上的稀疏子集（每中枢窗一个）。
4. **不可命中的构造性原因**：D2 的 c 取段锚在**投影种子**上——`last = projection.seeds[block.end_center].center`，
   seed 的 `end_index` = 窗的**第三腿终点**（`level_view.rs:395-402`：seed 只取首三段坐标），
   而级别-k 单元窗终点 = 同一中枢**含延伸段的整窗末腿**终点。c_terminal = seed.end 之后首个同向腿
   （`level_view.rs:839-845`）⟹ turn（首离开腿终点，或 R1 后 t*）一般落在「seed.end, 整窗.end」
   区间内部或更后的 c 区域内，**与任何单元窗终点相等纯属巧合**——154 案中仅 41 案命中
   （进入后续门链：S1a=37 / S2=3 / S4=1），113 案无单元端点落在 turn 上 ⟹ S0。
   位格等式把「同一结构」误写为「同一坐标格」——这是**装配工件，不是谓词判负**。

### 1.2 p112 归因引用（逐字）

- §4 S0 行（`p112-…:121`）：「S0 无单元端点落在 turn（取段/坐标错位）113（73.4%）（立即三买 48 / 延迟三买 65）——
  **装配工件**：D2 turn=首离开腿终点（L0 坐标）vs BSP 点=级别-1 单元终点，`source_index` 精确匹配
  （p92 查法，types.rs:216-221）结构性不可命中」。
- §6-5（`p112-…:162-164`）：「S0 取段/坐标系错位（73.4%，最大单因）。不是谓词判负，是身份桥/装配工件：
  D2 背驰点与 BSP 点坐标格不同。修法选项（装配裁定，p109 §6-5b 已列）：**终端背书改『含 turn 的单元
  区间匹配』，或 nest 自定义确认位替代 BSP 账本**。」
- 总归属：p112 已裁定 754 链在 c 全离开段口径下整体归 **(i) BSP 欠配错杀**（§0 表 + §3 敏感性：
  T1∧T2∧T3_ext∧T4∧T5 = 154/154 全合取成立）。

### 1.3 逐案坐标（113 案 = S0 全集；源 `/tmp/p112_full.txt` P112_CASE `bsp_gate=S0_no_unit_at_turn`）

链数合计 542（754 中其余 212 链属 41 个非 S0 案）。turn 为 R1 前旧坐标（seg_c.1）；
**R1 后 confirmed 事件的坐标迁移为 t\***（§1.1-2），坐标格错位的结构本质不变——p116 全量重放
（后台在跑）产出新坐标系下的门链分布，实装验证以新坐标为准（§5.4）。

```text
726571 763996 808164 823853 838745 851310 867850 969804 992649 997630 1009211 1042575
1047437 1192257 1194220 1245613 1261171 1278877 1351399 1358466 1359194 1368906 1386329
1482582 1586017 1616025 1666918 1674705 1695693 1703299 1717339 1720364 1724885 1747631
1760996 1776717 1787174 1795938 1876509 1877936 1884887 1933751 1972667 1989599 1990816
1995115 2019313 2049451 2065447 2099840 2104623 2147037 2161199 2176153 2213942 2216653
2228837 2389541 2391143 2400699 2430974 2477951 2503123 2517903 2621898 2654849 2689855
2698231 2704489 2725063 2739381 2944132 3012725 3031349 3040514 3041538 3064116 3107052
3121152 3299978 3325009 3337750 3339068 3349108 3357216 3399697 3408050 3546705 3550639
3609266 3611752 3639582 3666649 3770761 3777964 3817099 3850754 3903030 3923677 3950436
4099871 4163480 4171192 4216333 4225787 4230079 4254346 4283186 4304572 4314109 4327794
4331234 4333140
```

代表样本（P112_CASE 全字段，§1.1-4 机制的直接见证）：

| turn | side/dir | seg_a（b 段） | seg_c（旧 c 窗） | chains | 三买（T3_ext，全离开段口径） | bsp 账本@turn |
|---|---|---|---|---:|---|---|
| 726571 | Short/Up | (725955, 726218) | (726527, 726571) | 1 | 立即 (726571, 726661) | none |
| 808164 | Long/Down | (807506, 807940) | (808070, 808164) | 2 | 立即 (808164, 808316) | none |
| 823853 | Short/Up | (823463, 823728) | (823752, 823853) | 4 | 立即 (823974, 824038) | none |
| 1194220 | Short/Up | (1193820, 1194132) | (1194164, 1194220) | 12 | 立即 (1195312, …) | none |
| 1192257 | Long/Down | (1191892, 1192055) | (1192236, 1192257) | 6 | 延迟（t3_ext leave=1217525） | none |

（113/113 `bsp_book=none`——turn 上完全无点；全 154 中仅 3 案 turn 有点且为反向位点，
见 p112 §4 :127-129。）

---

## 2. 教义依据（课号:段号，直读主仓博文原文核对；权威链：博文 > chan99）

1. **037:22（全离开段；桥对象的结构根据）**——`037-第37课.md:22` 直读：
   「对c的内部进行分析，**由于c包含B的第三类买卖点，则c至少包含两个次级别中枢**，否则满足不了
   次级别离开后次级别回拉不重回中枢的条件。」
   ⟹ c 不是点、是含次级别中枢的完整离开走势。c 内含的次级别中枢在工程对象上 = c 区域内新形成的
   tower[k] 中枢窗 ⟹ 其投影 = 级别-k 单元。**c 的级别-k 见证单元取「c 内含次级别中枢」是 037:22
   自己的对象**，不是工程新造。R1 的 seg_c 全离开段口径（`level_view.rs:855-871`）与本桥同源。
2. **031:883（比较对象唯一；桥不新造比较对象）**——`031-第31课.md:883` 直读：
   「盘整背驰一般都是中枢震荡时发生的，而**趋势背驰，是a+A+b+B+c中，cb间的比较**。」
   ⟹ 趋势背驰的比较对象是**段对 (c, b)**，不是任何点位。终端背书要证的是「这对 (c,b) 的背驰
   制造了买卖点」（024:18），不是「turn 这根 bar 上恰有一个账本题名」。桥保持段对身份
   （c 责任单元 = c 结构的级别-k 实现），只修坐标格，不替换比较对象——与 doc-trend §1 总判定
   （`doc-trend-…:32-56`：比较对象唯一 c vs b）一致。
3. **024:18（终端门的教义根）**——`024-第24课.md:18` 直读：
   「缠中说禅背驰-买卖点定理：**任一背驰都必然制造某级别的买卖点**，任一级别的买卖点都必然源自
   某级别走势的背驰。」终端门 = 该定理在 nest 基例的机械化（spec P5 §6 `Conf^δ_e`，`nest.rs:296-317`）。
   桥是定理存在性论断的正确读法（某级别 = 事件所在级 k 的网格），位格等式是对该论断的过度编码。
4. **029:18（锚点=邻域身份，非位格等式）**——`029-第29课.md:18` 直读：
   「最终通过1分钟以及1分钟以下级别的精确定位，最终**可以找到背驰的精确点**，其后就发生反弹。」
   「精确点」由次级别定位给出，是结构邻域身份；p116 §1.2/§3.4 同读法（锚点与转折点**邻域重合**，
   dt 容差分层）。桥的「c 责任单元终点」正是该邻域身份在级别-k 网格上的落点。
5. **043:26（锚级别上界；桥不跨级）**——`043-第43课.md:26` 直读：
   「由于**背驰的级别不可能大于当下走势的级别**……」桥在事件同级 k 的单元网格内映射，
   不向更高级别账本取锚（`levels[k]` 不变），满足 ℓ_anchor ≤ ℓ_turn。
6. **037:18（诚实判负边界）**——`037-第37课.md:18` 直读：
   「c必然是次级别的，也就是说，c至少包含对B的一个第三类买卖点，**否则，就可以看成是B中枢的
   小级别波动，完全可以用盘整背驰来处理**。」c 的次级别中枢尚未在级别-k 网格形成（无 c 内单元
   完成于确认点）⟹ 见证不存在 ⟹ 桥返回 None——这是 037:18 否则条款域的诚实判负，非放宽、非误杀。

---

## 3. 修法设计（精确到函数/hunk；涉及文件清单 = 2 个）

### 3.1 方案选型（对 p112 §6-5 两个候选 + 第三候选的裁定材料）

| 候选 | 内容 | 触及面 | 判定 |
|---|---|---|---|
| **A（推荐）** | **映射层**：账本不动，终端背书查法把事件坐标桥接到 c 责任单元终点 | `nest.rs`（新增 1 函数）+ p92 bin（闭包切换） | **推荐**：教义对象正确（037:22 自己的 c 内含次级别中枢）、账本语义零扰动、不碰主干 |
| B | 改 BSP 侧匹配键到 D2 坐标系（一类点改登记/加签到腿坐标） | `judge_first_cached`/`BspPoint.source_index`（`signal.rs:368`、`bsp.rs:113-161`） | **拒绝**：`source_index` 是排序/去重/`class_index`/strategy 止损 single source/bit-exact digest 的公共键（`bsp.rs:91-111` 手写 PartialEq 铁律）——碰账本主干，须升级裁定；且把级别-k 点的坐标语义降格为腿坐标，破坏账本自身一致性 |
| C | nest 自定义确认位替代 BSP 账本（p112 §6-5 候选 b） | `nest.rs` 基例语义 + spec P5 §6 重锚 | **拒绝（本工位）**：以 D2 全合取自我背书替代 `Conf^δ_e`，切断 024:18「背驰⟹买卖点」的跨谓词链接，证书教义语义改变量远大于坐标桥；若主人裁定 BSP 账本本身不可信，另行立项 |

候选 A 的两个子口径（须实装实测后由编排者定稿，推荐 A1）：

- **A1 c 责任单元（推荐）**：`u* =` 首个 `start_index ≥ interval_b.0 ∧ end_index ≥ turn_source` 的单元。
  `start ≥ c_start` 守卫排除「B 自身单元」——B 单元的一类点见证的是**进入 B 的 b-vs-a 前序背驰**
  （其 I(C)=[λ_C, B.end] 不含 c），若选中则把前序背驰误记为 c-vs-b 的见证（跨结构误放，§6-F1）。
- **A2 含 turn 的单元区间匹配（p112 §6-5 原话候选 a）**：`u* =` 满足 `start ≤ turn ≤ end` 的单元。
  缺陷：turn 落在单元间隙（c 区域内次级别中枢形成前）时无覆盖单元 ⟹ 与 S0 同形的空洞残留；
  且 turn 落在 B 延伸窗内时选中 B 单元，与 A1 的 F1 误放同构。**列为备选，不推荐**。

### 3.2 Hunk 方案

**H1 — `rust/src/theta_v0/classifier/nest.rs`（新增 1 个 pub 函数 + 文档 + 单测；不改任何既有行）**

新增 imports：`use super::bsp::BspPoint; use super::center::UnitRange;`
（`BspBits`/`Side`/`NestCandidateEvent` 已在 :28-30 引入）。

```rust
/// ★p117 S0 取段坐标桥（037:22 c 责任单元；031:883 比较对象唯一）。
///
/// D2 事件的 turn 坐标（L0 腿坐标格：R1 后 = 确认点 t*）与 BSP 点坐标（级别-k 单元窗
/// 终点坐标格）是位格等式不可命中的两个网格（p112 §4 S0=113/154）。本函数把终端背书的
/// 查法从「位格等式」改为「c 责任单元」映射：
///   u* = units 中首个 start_index ≥ event.interval_b.0（c_start）∧
///        end_index ≥ event.turn_source（t*）的单元；
///   返回 levels[k].bsp 中 source_index == u*.end_index 且 confirm_side 的首个点。
///
/// - 语义：u* = c 内含次级别中枢（037:22）在级别-k 网格上首个完成于确认点的单元——
///   c 自己的次级别结构见证。start ≥ c_start 守卫排除 B 自身单元（其一类点见证进入 B 的
///   b-vs-a 前序背驰，非本案 c-vs-b）。
/// - 映射性质：函数（每事件 ≤1 单元）；非单射（多事件可共单元——nest ∃ 装配语义下合法，
///   证书基例身份仍是事件而非单元，去重键不变）；非满射（无事件单元不被读）。
/// - 边界：c 见证未形成（无 c 内单元完成于 t*）⟹ None（诚实判负，037:18 否则条款域）；
///   units 空/越界 ⟹ None。账本只读：find 语义与旧查法同（同 source_index 多点取首个
///   confirm_side 命中），不改 BspPoint 任何字段/排序/去重/class_index。
pub fn terminal_bits_bridged(
    bsp: &[BspPoint],
    units: &[UnitRange],
    event: &NestCandidateEvent,
) -> Option<BspBits> {
    let first_c = units.partition_point(|u| u.start_index < event.interval_b.0);
    let suffix = &units[first_c..];
    let rel = suffix.partition_point(|u| u.end_index < event.turn_source);
    let unit = suffix.get(rel)?;
    bsp.iter()
        .find(|p| p.source_index == unit.end_index && p.bits.confirm_side(event.side))
        .map(|p| p.bits)
}
```

（`UnitRange` 按 `start_index` 升序、窗互不重叠 ⟹ 两分正确；`partition_point` 与线性 find
逐位等价，取 O(log n) 形式。）

**H2 — `rust/src/bin/p92_nest_replay_postruling.rs`（生产重放，3 处小改）**

1. 在 classification + tower 同作用域处（终态快照通道 `:207` 附近、增量 prefix 通道 `:427/:468`
   回调内）预建 `units_by_level: Vec<Vec<UnitRange>>`：
   `units_by_level[0] = vec![]`（nest 不听 L0，p105 §3）；`k ≥ 1` 时
   `= project_to_units(&tower[k], &classification.levels[k - 1].moves)`
   （与 p112 §2 生产同源查法一致；增量通道每 checkpoint 重建或复用既有投影缓存，
   正确性锚 = 全量 `project_to_units`，`mod.rs:1400` 神谕守护同口径）。
2. `observe_certificates`（`:780`）加 `units_by_level: &[Vec<UnitRange>]` 形参；两处
   `assemble_typed_certificates` 闭包（`:794`、`:862`）改为：

   ```rust
   |event| nest::terminal_bits_bridged(
       &classification.levels[event.level as usize].bsp,
       &units_by_level[event.level as usize],
       event,
   )
   ```

3. 删除本 bin 内 `terminal_bits_new`/`terminal_bits_old`（`:940-966`）——单口径替换
   （禁双口径补丁，p115 §6 纪律）；归档探针（p102/p107-p116）的同名副本**不动**
   （一次性测量仪器，报告已归档；p116 后台运行中，严禁触碰）。

**明确不做**：不改 `NestCandidateEvent` 模式（事件身份/去重键/证书 ids 不变——R1 刚钉死
`turn_source=t*` 语义）；不改 `assemble_typed_certificate` 门序；不改 `mod.rs` 增量分类路径；
不加 Cargo.toml 任何条目。

### 3.3 对既有 BSP 账本语义的影响面

**零**。账本内容（`levels[k].bsp` 的集合、排序、`source_index` 键、`BspBits`/`class_index`、
`struct_break_dir`/`force` 旁挂、strategy 止损 single source）一字节不动；桥是账本上的**只读
查法变更**。语义变更只发生在 nest 证书的**消费侧**：基例背书的坐标解释从「位格等式」改为
「c 责任单元终点」——这是 p112 裁定 (i) 归属下的修复目的本身。消费 `Classification.levels[k].bsp`
的其他路径（econ_positive、strategy、closed_loop、μ 分桶）不经过 `terminal_bits_bridged`，不受影响。

---

## 4. bit-exact 冲突面（触及对象清单 + 主干判定）

### 4.1 触及对象清单（穷尽）

| 文件 | 改动 | 性质 |
|---|---|---|
| `rust/src/theta_v0/classifier/nest.rs` | 新增 `terminal_bits_bridged` + imports(2 行) + 单测 | **纯增量**；既有 `n_delta`/`assemble_*`/`extend_typed_upward`/`confirm`/`is_sub` 零行改 |
| `rust/src/bin/p92_nest_replay_postruling.rs` | `units_by_level` 预建 + `observe_certificates` 形参 + 2 处闭包 + 删 2 个本地查法函数 | bin（重放仪器/生产装配驱动），非 lib 主干 |

### 4.2 主干判定：**不碰主干，不标升级裁定**

- **塔**（`recursive_tower.rs` compose/detect/advance、`mod.rs` 全量+增量构造）：零触碰。
- **笔/线段**（parser 账本、`lower_legs_from`）：零触碰。
- **中枢**（`center.rs`、`detect_centers_windowed{,_resume}`、`compose_level`）：零触碰。
- **inclusion/包含**（`is_sub`、`extend_typed_upward` 闭包含边、身份门）：零触碰。
- **BSP 生成**（`judge_segment`/`judge_first_cached`/`extract_*`/`bsp.rs`/`decompose.rs`/
  `signal.rs`/`divergence.rs`/`level_view.rs`）：零触碰 ⟹ **`levels[k].bsp` 账本内容逐字节不变**，
  账本自身的 bit-exact 断言（07a resume/full debug_assert、`bit_exact_*` 系列、GOLDEN digest）不受扰动。
- 验收推定：`P92_BIT_EXACT` 的 old_path/tower/moves/centers/bsp/pan 六维 diff 必须 =0
  （lifecycle_cp_ownership=1 为基线既有 frontier 工件，p115 §5.0 已复现归档，与本设计无关）。

### 4.3 有意行为变更面（诚实声明，非 bit-exact 违约）

- nest **证书集合**变化（终端门由位格等式放宽为 c 责任单元桥接——修复目的本身）：
  `P92_CERT`、`CKPT_STATS` A/B 计数、`CERT` dump 行预期增加；既有 66 张证书中基例命中单元
  `start < c_start`（B 自身单元）者可能失证（§6-K3）——按 p115 §6 纪律单口径替换、重放对照
  诚实报告，不为保产量叠加旧口径。
- 证书字段语义不变：`terminal: BspBits` 仍来自账本单一点；`judge_at`/identities/区间边全部沿用
  事件字段，桥不向证书注入任何新字段（**无前视入证**：BspBits 无时间字段，见证单元完成时刻
  不进证书；快照重放只读 ≤ as_of 数据）。

---

## 5. 单测计划（测试名 + 断言 + 数据构造）

### 5.1 lib 单测（`nest.rs` `#[cfg(test)]`，新增 8 个；数据全合成）

公共夹具（镜像 113 案代表样本 turn=726571 的几何关系）：

```text
units = [
  U0 {start:726400, end:726540},   // B 自身单元（延伸窗覆盖 c_start=726527）
  U1 {start:726541, end:726600},   // c 内首个次级别中枢单元
  U2 {start:726601, end:726680},   // c 内第二个次级别中枢单元
]
event: kind=Trend, side=Long, interval_b=(726527, t*), turn_source=t*
bsp: 在 726540/726600/726680 按需放 BspPoint（buy1/sell1 组合）
```

| 测试名 | 构造 | 断言 |
|---|---|---|
| `terminal_bridge_exact_hit_degenerates` | t*=726600=U1.end；U1.end 放 buy1 | 桥返回 Some(buy1)——与旧位格等式在命中时逐位一致（向后兼容锁定） |
| `terminal_bridge_skips_center_being_left_unit` | t*=726560（落在 U0 延伸窗内）；U0.end 放 buy1、U1.end 无点 | **不选 U0**（`start<c_start` 守卫）⟹ 选 U1；U1 无点 ⟹ None——防 b-vs-a 误记（§6-F1） |
| `terminal_bridge_selects_first_c_unit_covering_tstar` | t*=726650（深在 c 内）；U2.end 放 buy1 | 跳过 U1（end<t*）选 U2 ⟹ Some——037:22 多中枢 c 的覆盖语义 |
| `terminal_bridge_witness_not_formed_returns_none` | t*=726560；U0/U1/U2 终点均无点 | None——c 见证未形成，诚实判负（037:18 否则条款域） |
| `terminal_bridge_wrong_side_rejected` | t*=726600；U1.end 只放 sell1 | None——`confirm_side` 方向过滤保持 |
| `terminal_bridge_first_confirm_at_shared_endpoint` | t*=726600；U1.end 放两点（无 bit 点 + buy1 点） | 返回 buy1 点 bits——find 语义与旧查法同（同坐标多点取首个 confirm） |
| `terminal_bridge_pan_event_same_contract` | 同夹具，event.kind=Consolidation、interval_b=(726541, 726600)、turn=726600；U1.end 放 buy1 | Some——pan 事件共用契约（其 interval_b.0=pan C 起点）；仅契约同形冒烟，pan 验证归其工位 |
| `terminal_bridge_past_last_unit_returns_none` | t*=726999（越过末单元）；账本任意 | None——`suffix.get(rel)` 越界守卫 |

### 5.2 回归护栏（零新写，全绿为验收）

- `cargo test --lib`：基线 1647（p115 §4）+ 新增 8 = 1655，**零既有测试变红**——含 07a
  resume/full bit-exact debug_assert、`bit_exact_*` 塔/笔/线段/中枢/包含全部微分测试、
  signal.rs digest guard（账本 Debug/digest 不变 ⟹ GOLDEN 不翻）。
- `cargo check --bins`：p92 编译通过；归档探针零改动零影响。

### 5.3 集成验证（实装工位执行，方法复刻 p115 §5）

1. **250k 冒烟**（P92_MAX_BARS=250000）：基线副本（rsync 源码 + H1/H2 回退）同跑对照；
   `P92_BIT_EXACT` 六维 diff=0、`P92_PROVIDER` complete=true、future_violations=0。
2. **全量重放**（4,613,599 bar，P92_CKPT=250000）：`CKPT_STATS` 末检查点 A/B vs
   基线 A=41/B=25 与 R1-R3 后 v2 读数；`diff <(grep '^CKPT caliber=A' 基线) <(同上)` 逐行对账。
3. **S0 重分布测量**（验收关键指标）：113 案坐标集（§1.3）在新口径下的门链首杀分布——
   预期 S0 击杀归零、按 S1a/S2/S3/S4/S5/S6 重分布；逐案列出进入 S6（证书）与落点门环，
   并报告见证单元 `u*.end − t*` 的距离分布（§6-F3 的量化）。
   **声明边界：本设计不预承诺救回数**——后序门环属兄弟工位（§7），090 声明与能力一致。
4. 验证坐标以 **R1 后新坐标系**（confirmed ⟹ turn=t*）为准；p116 后台重放产物
   （`/tmp/p92_ckpt_dump_v2.txt` + TURN/DIV/TERM dump）完成后直接可读，不重跑干扰。

---

## 6. 风险与边界（误放/误杀评估）

### 6.1 误放（把不该背书的事件放过终端门）

- **F1 跨结构误记（设计级已防）**：B 自身单元的一类点见证的是进入 B 的 b-vs-a 前序背驰
  （其 I(C)=[λ_C, B.end] 不含 c）。`start ≥ c_start` 守卫把 B 单元结构性地排除出桥键——
  这是 A1 相对 A2/裸后继映射的核心防误放设计（§3.1）。**残余**：c 内单元的终点可能携带
  非本结构的同侧 bit（级别-k 网格粗化，同单元终点可落多结构的点）——由 `confirm_side`
  方向过滤 + 重放逐案复核（§5.3-3 要求报告每张新证基例的桥中单元与门链）收口；
  不引入任何概率/统计判据（v3 硬禁令）。
- **F2 见证晚于确认点（语义边界，诚实声明）**：u*.end 可晚于 t*（首个 c 内中枢完成常晚于
  三买确立点）——背书单元在 t* 之后才完成。**无前视入证**：快照重放中事件与账本都只读
  ≤ as_of 数据；prefix 观察只在事件与见证点都已存在时发生；证书字段（BspBits/identities/
  judge_at/区间）不含见证完成时刻 ⟹ 无前视进入任何证书字段。语义差（背书晚于确认坐标）
  是跨粒度见证的内禀形态，由 §5.3-3 的 `u*.end − t*` 距离分布量化公示；p116 §3.4 的
  dt 分层（0/240/1440）为其提供既有读法。
- **F3 门序不变**：桥只改基例背书的坐标解释；`divergence_confirmed` 前置门
  （`nest.rs:476`）、方向门、闭包含边、Cand 门全部不动 ⟹ 不放行任何结构链身不合的证书。

### 6.2 误杀（把该救回的事件仍杀掉）

- **K1 c 见证未形成 ⟹ None**：t* 早于首个 c 内单元完成时诚实判负（037:18 否则条款域——
  该事件此时按盘整背驰处理是教义允许的形态）。70 延迟案 t* 平均晚 ~45k bar（p112 §3），
  c 内中枢多已形成，预期主要落在 84 立即案中的早熟子集——数量由重放实测，不预承诺。
- **K2 后序门环不归本项**：过桥后仍须过 S1a（τ 门上下文，24.0%）、S2/S4（anchor 门，2.6%）、
  S3/S5——那些是兄弟工位的修复对象（§7）。本项只承诺：**S0 这一装配性击杀归零**，
  门链重分布如实报告；整体 754 的救回率是四项合力结果，不在本设计声明。
- **K3 既有 66 证书失证风险**：单口径替换（禁双口径补丁，p115 §6 已立纪律）。旧基例若
  靠 B 自身单元（`start < c_start`）命中，桥键迁走后可能失证——重放对照逐张列出
  （存续/失证/新增三栏），诚实报告，不为保产量叠加旧口径。

### 6.3 边界

- exec ≥ 1 恒成立（nest 不听 L0，p105 §3）；`event.level ≥ tower.len()` 或 units 空 ⟹ None。
- 未确认事件（`divergence_confirmed=false`，turn=seg_c.1）在 `nest.rs:476` 先被门，
  桥只在 confirmed 事件上生效；探针侧对未确认事件的桥读数只作诊断。
- pan 事件共用查法：契约同形（`interval_b.0` = pan C 起点），实装后 pan 基例同样过桥——
  属一致性收益；**验证域限趋势基例 113 案**（本工位范围），pan 侧全量验证归其工位。
- v3 硬禁令：全桥为确定性结构谓词（整数比较 + 账本 find），无概率/统计推断、无回测验证
  策略、无 EMH 假设；分档/分布计数仅作事实清点。

---

## 7. 与其他三项的潜在文件冲突（本设计触及文件 = 2 个）

本工位触及：**`nest.rs`**、**`rust/src/bin/p92_nest_replay_postruling.rs`**（仅此 2 个）。

| 兄弟项（p112 §6 清单） | 预期触及文件 | 与本工位冲突 |
|---|---|---|
| §6-3 BSP 缺 037:20 破极值项（破核心判据升级为破 b 包络） | `signal.rs`（`judge_first_cached` broke 判据 :289-303、`judge_segment` 消费点） | **无 lib 重叠**（我不碰 signal.rs）；同属 BSP 生成器语义，实装顺序上建议先于我落地或同日无交集合并——我重放验证时须以其基线为准 |
| §6-4 BSP 非教义 anchor 资格门（Q7-#1 裁定C 降级/去除） | `signal.rs`（:294 anchor 门、`locate_departure_move_a`） | **无 lib 重叠**；门链协同：我的桥把 113 案送进门链后，S2/S4 击杀数会因该项落地而变化——重分布报告须标注各项落地状态 |
| §6-6 S1a τ 门上下文（37 案归属中枢落盘整块） | `decompose.rs`（:159-169 `center_trend_gate`/`center_own_dir_at`）或 `signal.rs` 消费点 | **无 lib 重叠**；S1a 是过桥后最大承接门环，两项的验收口径（谁算「救回」）须编排者统一 |
| **共享面：p92 bin** | 四项的重放量测都要改/重跑 p92 | **唯一真实冲突面**——必须串行派发；各 hunk 行区不交叉（我的 :794/:862 闭包 + :940 删函数 + units 预建 vs 他项的探针行/门参），合并时按派发序依次 rebase 验证 |
| p116 bin（后台运行中） | — | **严禁触碰**（运行中 + 归档仪器）；其 TERM 行查法是旧口径副本，不影响本设计 |
| `rust/Cargo.toml` | — | 零改动（bin 自动发现），与 p115 同纪律 |

---

## 8. 边界与纪律声明

- 本文件为设计施工图：**生产源码零改动**（上述 hunk 为派发用方案，本轮未实装）；
  新增文件仅本报告；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入（教义原文只读核对）；
  零 git mutation；`rust/Cargo.toml` 零改动；未 cargo build/test（p116 后台重放不受干扰）。
- 090：教义引用全部直读主仓 `docs/chanlun/text/blog/` 原文核对（037:22 @037-第37课.md:22、
  031:883 @031-第31课.md:883、037:18 @037-第37课.md:18、024:18 @024-第24课.md:18、
  029:18 @029-第29课.md:18、043:26 @043-第43课.md:26）；代码坐标逐只读核对（nest.rs:470-481、
  level_view.rs:652-731/839-871、signal.rs:276-368/1209-1269、recursive_tower.rs:891-909、
  mod.rs:404-449、p92 bin:780-966、p112 bin:330-386/884-891）。
- 声明与能力一致：本设计承诺「S0 装配性击杀的机制消除方案 + 验证计划」，**不**承诺整体
  754 救回率（后序门环归兄弟工位）、不声明任何择时 alpha；113 案过桥后的逐案几何复核
  依赖实装后的全量重放（§5.3-3），未做的不写进结论。
