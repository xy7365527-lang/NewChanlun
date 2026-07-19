# p118 施工图：小转大显式分类分支（关④）——断链点定位 + 旁挂联合分类设计

日期：2026-07-18 ｜ 性质：**施工图（只设计不实装）**——零代码改动、零数据重放、零 git mutation、主仓零写入、零 cargo 调用（p116 后台重放不受干扰）
工位：主线阶段 1 **关④（小转大显式分类分支实装+单测，断链不得落入无类状态）**
输入在案：`doc-level-domain-20260717.md`（L0 地板/L5 天花板/小转大跳级三节，同目录）；`chanlun/escalate/bsp-terminal-endorsement-ruling-20260718.md`（T1 终端背书移位 levels[ℓ-1].bsp）；`p105-cert-level-spectrum-20260717.md`；`p107-level-funnel-audit-20260717.md`
教义引用格式 `0XX:段号` = 主仓 `docs/chanlun/text/blog/0XX-第X课.md` 行号，全部直读原文逐条核对（§8 自检清单）
代码坐标格式 `文件:行号` = worktree `/tmp/kimi-nest-mainline` 2026-07-18 现行状态直读核对

---

## 0. 结论速览

| 必答 | 一句话结论 |
|---|---|
| 1 断链点 | **装配环**：`nest.rs:647-650` rung 的 `divergence_confirmed` 真值被丢弃 + 输出分类词汇（`NestDivergenceKind` / p92 桶 trend/pan/mixed）无「小转大」值——小转大形态在证书对象上**无类**。两个次级落点：中间级缺事件时链退化为单级证书（`nest.rs:628-630/636-670` + p92:764-794 兜底循环）与未确认父事件孤儿化（039:34 defer 域无承载）。候选环 extreme 预滤（`level_view.rs:681-683`）杀「c 未破极值」形态属 037:20 否则条款域，**不**归本分支。 |
| 2 分支形态 | **旁挂联合分类**（推荐）：装配环恢复 rung 级 confirmed 向量（sidecar，约 6 行）+ 新纯函数派生 `NestTurnClass`（每证恰一类的 partition）；**否**新证书类型（双计同一转折、对账键 fork）、**否** BspBits 第七位（破 Lean parity + class_index）、**否** 六态第七态（破 ∃! partition，定义域不符）。必要条件（c′ 三类买卖点）在现有生产对象上**完全可判定**：c′ ∈ `levels[ℓ-2].centers`、三类点 ∈ `levels[ℓ-2].bsp`（`BspPoint.center` 契约 `signal.rs:558-569`）。缺的对象只有 rung confirmed 向量与分类词汇本身。 |
| 3 六态/位向量 | 三者正交：r（六态 partition，`TrendSixState.lean:82-89`）答「在哪」、b（{0,1}⁶ subset，`:252-259`）答「有什么信号」、`NestTurnClass` 答「链顶背书形态」（跨级链属性）。挂 ⊥ 态被否（`no_center_maps_bot :231-234` 语义冲突）；加第七位被否（`signalBits_not_collapsible_to_sum :309-311` 语境破坏）。相容性模式 = `TrendClass` τ 四态与 r 正交的既有先例（`:399-401`）：新立独立 inductive + ∃!，不动既有文件。c′ 必要条件可**读作** c′ 级六态查询（顶转 D¹/底转 U¹），但只作只读消费，不改 r。 |
| 4 冲突面 | `nest.rs`（sidecar 字段+2 函数，证书本体零改）+ 新增 `classifier/turn_class.rs` + `mod.rs`（注册 2 行）+ p92 bin（新增独立 `TURN_CLASS` dump 行，CERT 行零改）。**不碰**塔/笔/线段/中枢主干、不碰 signal.rs 判据链、不碰 level_view.rs provider 门、不碰 types.rs——**无需升级裁定**。 |
| 5 单测 | 9 个测试（§5）：044:16 正例、平台下破负例（066:198）、c′ 缺失/无三类/方向反/因果序反四个防误标负例、情况一对照、Defer 孤儿、partition 穷尽互斥锁。 |
| 6 验收线 | 证书集合三栏对照 存续=全/失证=0/新增=0（分类是旁挂不是过滤）；`P92_BIT_EXACT` 五维 diff=0；无类计数=0（terminal_confirmed 事件与 c 破极值未确认 Trend 事件 100% 有类）；候选占比诚实声明（044:32/044:48「少见」——高占比=检测聋度证据非产量）；负例全绿；Lean 漂移登记（rust 领先 Origin，T3 L1 先例）。 |

---

## 1. 断链点定位（必答 1）：小转大形态在哪一环变成「无类」

### 1.1 形态定义（教义锚，逐字核对）

小转大 = 「背驰级别小于当下走势级别」的转折（043:32、044:14）：**父级无背驰段**（043:34「明确显示没有出现30分钟的背驰，也就是背驰段最终不成立」）+ **低级别有背驰**（044:16「如果c是一个1分钟级别的背驰，最终引发下跌拉回B里」）。链行为：父级无证书，链顶 = 小级别背驰证书，中间各级无需背驰证书（`doc-level-domain-20260717.md:105,136-140`）——「跳级」是原文合法形态。

### 1.2 现管线逐环追踪（候选→背驰确认→终端→装配）

**候选环（`level_view.rs:653-718` `provide_nest_candidate_events` Trend 分支）**：父级 Trend 事件**产生**。逐环：
- pair 存在性（`:653-661`）：D2 pair（A/C 同趋势方向、分属相邻中枢锚的离开 episode，`provide_divergence_pairs :798-799`）+ Trend 块匹配——30-min a+A+b+B+c 的 (b,c) 对结构性存在。
- extreme 预滤（`:669-683`，037:20 c 包络破 b 包络）：冲顶型小转大 c 创极值 ⟹ 通过。**若 c 未破 b 极值，事件在 `:681-683` `if !extreme { continue; }` 被击杀，父级对象在事件层无承载**——但此属 037:20 否则条款域（B 中枢小级别波动，教义路由到盘整/二类点通道，T3 裁决第 6 条明示「否则条款不改路由」），**不是**小转大分支的对象（§2.5 边界，防误标第一防线）。

**背驰确认环（`level_view.rs:689-704`）**：`trend_confirm_time`（`:535-626` R1 全合取 T4∧T3∧T2∧T5）对父级返回 `None`——T5 力度或关系（`:600-614`）不成立正是 043:34「c对b在30分钟级别并没有出现背驰」的定义性特征。`:701-704`：`None ⟹ (pair.seg_c, pair.seg_c.1, false)`——事件**保留**（诚实结构坐标，`divergence_confirmed=false`）。此环行为**已符教义**（父级本来就不该有背驰确认），不丢弃对象。

**终端环（`nest.rs:532-539` `terminal_bits_at_event`，基例门 `:567-570`）**：只查基例。低级别基例（1-min 背驰确认）在 `levels[exec-1].bsp` 有 confirm_side 点（098:18/044:16 的 1 分钟背驰是真实的该级别一类点）⟹ 通过。父级作 rung 永不经过终端门。**此环不丢弃小转大对象**。

**装配环（`nest.rs:553-671`）——断链点在这里**：
- 基例门（`:564`）要求 `base.divergence_confirmed=true`——未确认父事件**永不作基例**，只能作 rung（教义正确：父级无证书）。
- rung 选择（`:638-646`）按 D1 裁定**不看** `divergence_confirmed`（`:638-639` 注释「rung 级不再要求 divergence_confirmed」），只要同向 + `is_sub` 包含——链**可以**穿过未确认父级成证（教义正确：中间各级无需背驰证书，`doc-level-domain:138`）。
- **但 rung 的 confirmed 真值在 `:647-650` 被丢弃**：`rungs.push(NestRung::assembled(event.judge_at, *child, parent, true))`、`kinds.push(event.kind)`、`clocks.push(...)`、`ids.push(...)`——唯独 `event.divergence_confirmed` 不进任何向量。⟹ `TypedNestCertificate`（`:411-418`：certificate/kinds/judge_at/identities/caliber）**不携带**「链顶有无背驰」信息。
- 输出分类词汇：证书侧只有 `NestDivergenceKind::{Trend, Consolidation}`（`level_view.rs:466-471`）；p92 桶 = trend/pan/mixed（`p92_nest_replay_postruling.rs:920-934` `certificate_kind`）。**没有任何分类值表达「链顶无背驰 = 情况二 = 小转大」**。

**结论（断链点）**：小转大形态（父级事件 confirmed=false、低级别基例 confirmed=true、链结构成证）在**装配环 `nest.rs:647-650`** 被丢成「无类」——信息在装配时存在、在输出时消失；分类词汇中无对应值。证书照常发射（p107 实测 F3→F4 missed=0），但它与「情况一（链顶有背驰确认）」**不可区分**。

### 1.3 两个次级「无类」落点（同源变体，必须同关覆盖）

1. **单级退化**：中间级无任何同向包含事件时 `extend_typed_upward` 返回 false（`nest.rs:628-630` `events_by_level.get` 缺级 / `:636-670` 扫描无命中）⟹ 该 (exec,top) 配对装配失败；基例由 p92 的 exec..top 全配对循环（`p92_nest_replay_postruling.rs:764-794`）兜底为 (exec,exec) 单级证书——与「本级普通转折」同桶，断链事实（父级转折未入链）无分类承载。p105 实测 exec=2/3 的 6 张证书全为深度 1 单级（`p105-cert-level-spectrum-20260717.md` §2.1），p107(b) 坐实 exec>1 三证卡几何包含门（is_sub@interval_b）。
2. **父事件孤儿化**：未确认 Trend 事件若永远不被任何链选为 rung（无包含子链/方向不合），只进 `book.candidates` 计数层（`p92_nest_replay_postruling.rs:746`），输出层无形态分类承载——这正是 039:34 defer 策略（「没有盘背就等同级别分解的段组成中枢后……再比较」）的对象域，现管线无 defer 分类值。

### 1.4 非断链点（排除声明）

- 候选环 extreme 预滤击杀（`level_view.rs:681-683`）：属 037:20 否则条款域，教义出口是盘整背驰/二类点通道，本分支**不救回**（救了就是把盘整形态误标小转大，违反 044:30 纪律，§2.5）。
- 背驰确认环与终端环：行为已符教义（§1.2），不动。

---

## 2. 分支设计（必答 2）

### 2.1 形态评估（四个候选形态对账本语义/互斥分类的影响）

| 形态 | 评估 | 判 |
|---|---|---|
| **A. 新证书类型**（XiaozhuandaCert 与 NestCertificate/PanDivCert 并列第三证书种） | 同一转折的基例已持 NestCertificate，再发一张 = 双计；CERT 对账主键（ids 身份向量，`p92:833-838`）fork；PanDivCert 并列先例不成立——027:20 盘整背驰与趋势背驰**异构**（不同对象），而小转大与情况一共享同一背驰对象、只是父级背书形态不同（044 两分类是同一定理下的情形分支，不是两种证书物种）。 | 否 |
| **B. BspBits 第七位** | 破 Lean `SignalBits` {0,1}⁶ parity（`TrendSixState.lean:252-311`，含 2B/3B 非塌缩与 1B/2B 互斥两证明语境）；破 `BspBits::class_index` bit-exact（`types.rs:229` 附近）；且**范畴错误**——b 是「点」的买卖点分类，小转大是「链」的背书形态分类。 | 否 |
| **C. 六态第七态 / 挂 ⊥ 态** | ⊥ 已有语义（无确认中枢，`no_center_maps_bot :231-234`），小转大链各级都有中枢（c′ 按定义存在）；挂接即破 `sixState_unique` ∃! partition（`:170-174`）；§6 边界条件明示扩态须重形式化（`:379-381`）；r 是单级位置态，小转大是跨级链属性，定义域不同。 | 否 |
| **D. 旁挂联合分类**（sidecar confirmed 向量 + 派生 NestTurnClass 账本） | 信息恢复恰在丢弃点（`nest.rs:647-650`），分类函数纯（只读账本）；证书/BSP/六态/信号位/判据链本体零改；每证恰一类的 partition 由构造函数保证；Lean 侧按 TrendSixState 同模式**新立**文件（不动既有）。 | **推荐** |

### 2.2 推荐形态 D 的结构（三个新对象 + 一处恢复）

**D-1 恢复（`nest.rs`，约 6 行）**：`extend_typed_upward` 在 `:647-650` 同位追加 `confirmed.push(event.divergence_confirmed)`；`assemble_typed_certificate` 以 `base.divergence_confirmed`（门已断言 true）播种；`TypedNestCertificate` 加 sidecar 字段 `confirmed: Vec<bool>` + 访问器 `confirmed()`——与 kinds/judge_at/identities 同序（**高→低，含基例**，`:409-410` 既有约定）。`NestRung`/`NestCertificate` 本体零改（证书真值不动，`certificate.n_delta()` 契约不动）。

**D-2 分类代数（新文件 `classifier/turn_class.rs`）**：

```text
pub enum NestTurnClass {
    /// 情况一（043:28/30）：背驰级别=走势级别——链顶 rung confirmed=true。
    NestedConfirmed,
    /// 情况二候选（043:32/34 + 044:22-26 必要条件过滤通过）——
    /// 链顶 confirmed=false ∧ 基例 confirmed=true ∧ c′ 三类点必要条件成立。
    /// ★044:30：只有必要条件没有充分条件——本值语义=「通过必要条件过滤的候选」，
    /// 永远不是「小转大确认」；不提供 confirm_side/BspBits 语义（类型层无此成员）。
    XiaozhuandaCandidate { evidence: XzdEvidence },
    /// 链顶 confirmed=false 且必要条件不成立（含 c′ 正常震荡 044:20 域、方向反、
    /// 因果序反）、或 top<2（c′ 落背驰地板之下，065:94/066:198 类背驰域）——
    /// 背书证据仅及基例本级，诚实判负，不是误杀。
    ExecEvidenceOnly,
    /// 039:34 defer 域：c 破极值的未确认 Trend 事件未被任何链消费（孤儿）——
    /// 等三段成中枢后再比较，挂起观察而非丢弃。
    DeferOrphan { identity: NestEventIdentity },
}
```

派生函数（纯，只读）：`classify_nest_turns(events_by_level, certificates, classification) -> Vec<(CertKey, NestTurnClass)>`。partition 由构造保证：每张证书按 `confirmed[0]`（链顶）分流——true ⟹ NestedConfirmed；false ⟹ c′ 过滤，过 ⟹ Candidate、不过 ⟹ ExecEvidenceOnly；事件层未被任何证书身份向量覆盖的 c 破极值未确认 Trend 事件 ⟹ DeferOrphan。每证/每事件恰一类，互斥穷尽。

**D-3 c′ 必要条件过滤（044:22-26 的生产对象映射）**——设链顶 rung nest 级 ℓ（其中枢链 = `levels[ℓ-1]`，T1 裁定事实链）：

1. **c′ 定位**：`classification.levels[ℓ-2].centers` 中 `[start_index, end_index] ⊆ 链顶事件 interval_b`（c 段区间）且 `end_index ≤ 基例.turn_source` 的**最后一个**中枢（044:20「c`是c中最后一个5分钟的中枢」「顶背驰只能出现在c`之后」——因果序 c′.end ≤ 基例.turn）。ℓ<2 ⟹ c′ 落背驰递归地板之下（065:94 线段以下无背驰、066:198 类背驰域）⟹ 直接 ExecEvidenceOnly。
2. **三类点命中**：`classification.levels[ℓ-2].bsp` 中存在点 p：`p.center == Some(c′)`（3 类点携中枢契约，`signal.rs:558-569` `make_third_point` + `bsp.rs:122-126` 不变量）、`p.source_index > 基例.turn_source`（回拉入 c′ 后出三类，044:20）、方向匹配——顶转（side=Short）须 `sell3`、底转（side=Long）须 `buy3`（044:24/26）。
3. **补充证据（不作门）**：父级二类点（053:28「在小级别转大级别的情况下，第二类买卖点就是最佳的，因为在这种情况下，没有该级别的第一类买卖点」）——`levels[ℓ-1].bsp` 在 `[基例.turn_source, 三类点 src]` 窗内的 buy2/sell2 记入 `XzdEvidence.second_class: Option<usize>`，供下游操作层消费；有无不影响候选成立（053:28 是操作层补充，非必要条件合取项）。
4. **六态读法（只读消费）**：顶转候选的 c′ 条件 ⟺ c′ 级六态 r = D¹（belowS3，中枢下方·已有三卖）；底转 ⟺ U¹（aboveB3）（`six_state.rs:29-36` canonical 表）。平台下破（066:198）= c′ 级 r 落反向裂态（底转候选见 D¹）⟹ 非候选。r 只作查询原语，不改其 partition（§3）。

### 2.3 044:30 纪律：如何防止把「非小转大」误标

- **命名即诚实**：分类值 = `XiaozhuandaCandidate`（候选），类型层无 Confirmed 变体；evidence 结构不含 `BspBits`、无 `confirm_side` 方法——候选**不能**作终端背书、不能置任何 six-bit、不能进 strategy 触发链（类型层封锁，非注释承诺）。
- **四个反例封锁位**（全部是必要条件的否定式，无一充分断言）：c′ 缺失 ⟹ 非候选；c′ 正常震荡无三类点（044:20「在最后一个次级别中枢正常震荡的，都不可能转化成大级别的转折」）⟹ 非候选；三类点方向反（= 平台反破，066:198）⟹ 非候选；三类点因果序反（先于基例背驰点）⟹ 非候选。
- **占比诚实线**：缠师两次标注此形态少见（044:32「毕竟少见点」、044:48「并不常见」）⟹ 候选占链比例异常高是**检测聋度证据**而非教义形态（`doc-level-domain-20260717.md:141,176` 判别式）——验收报告须按 §6 如实声明，不得把高占比当产量。
- **不救回否则条款域**：c 未破极值的父级形态（`level_view.rs:681-683` 击杀）不进入本分支任何分类值（§1.4）——把盘整/二类点形态捞进小转大 = 误标，结构上封锁。

### 2.4 必要条件判据的生产对象可判定性（已有 vs 缺）

| 判据构件 | 生产对象 | 状态 |
|---|---|---|
| 父级无背驰段（043:34） | rung 事件 `divergence_confirmed=false`（`level_view.rs:701-704`） | **有，但装配时丢弃**（`nest.rs:647-650`）——D-1 恢复 |
| 低级别有背驰（044:16） | 基例门 `base.divergence_confirmed`（`nest.rs:564`）+ 终端 confirm（`:567-570`） | 已有 |
| c′ 存在与定位（044:18-20） | `LevelState.centers`（`mod.rs:172`）按区间包含 + 因果序筛选 | 已有（可判定） |
| c′ 三类买卖点（044:24/26） | `LevelState.bsp`（`mod.rs:183`）+ `BspPoint.center` 契约（`bsp.rs:122-126`、`signal.rs:558-569`） | 已有（可判定） |
| 因果序（c′ → 基例背驰 → 三类点） | `Center.end_index` / `BspPoint.source_index` / `event.turn_source` 同坐标系（source_index 原始 K 序） | 已有（可判定） |
| 父级二类点补充（053:28） | `levels[ℓ-1].bsp` 的 buy2/sell2（`signal.rs:577-588` `make_second_point`） | 已有（可判定，作证据不作门） |
| rung confirmed 向量 | — | **缺**（D-1 恢复，约 6 行） |
| 分类值与账本 | — | **缺**（D-2/D-3 新立） |
| 039:34 defer 状态 | — | **缺**（DeferOrphan 分类值承接，不做时序状态机——本关只交付分类口径，defer 的重判调度列遗留项） |

### 2.5 分支域边界（显式）

对象域 = nest 链对象（证书 + c 破极值未确认 Trend 孤儿事件）。**不在域内**：c 未破极值形态（037:20 否则条款 → 盘整/二类点通道）、L0 类背驰域（065:94/066:198 地板之下，另立「类背驰定位」工件、`doc-level-domain:172` 建议 1）、PanDivCert 通道（Q4 已有独立承接）。两通道（nest 候选 vs BSP 二类点）的联合归类另立裁定，本关不动。

---

## 3. 与六态/位向量的关系（必答 3）

**结论：`NestTurnClass` 挂独立联合类，与 r/b 正交；⊥ 态与第七位均被否。**

- **六态 r（partition）**：`TrendSixState.lean:82-89` 六构造子 + `:134-147` 穷尽 + `:170-174` `sixState_unique` ∃! + rust 镜像 `six_state.rs:42-63`。r 的定义域 = 「Option 中枢 + 价格点」的**单级位置态**；小转大的定义域 = 「跨级证书链」的**背书形态**——定义域不同构，挂不进 r。挂 ⊥ 的具体冲突：⊥ = 无确认中枢（`:231-234` `no_center_maps_bot` 任意 price/b3/s3 恒 bot），而小转大链以 c′ 存在为前提——把「有中枢的链形态」塞进「无中枢的态」直接翻转 `no_center_maps_bot` 见证语义；且六态任何扩态触发 §6 边界条件（`:379-381`）的重形式化义务。**否**。
- **信号位 b（subset）**：`:252-259` `SignalBits` 六独立 bool + `:298-311` 2B/3B 共存非塌缩 + `:320-326` 1B/2B 互斥；rust 镜像 `types.rs:173-221` + `bsp.rs:77-89`。加第七位 = 改结构定义 ⟹ 两条证明语境失效 + `class_index`（`types.rs:229` 附近）bit 布局翻转（GOLDEN 链全面重算）+ 点/链范畴混淆（§2.1-B）。**否**。
- **正交先例**：τ 四态（`TrendCompleteClassification.TrendClass`）与 r 的正交关系已在 Lean 侧立为纪律（`TrendSixState.lean:399-401`「FULL §5 的 5 元组明确二者是不同分量——不混淆 τ 与 r」）。`NestTurnClass` 按同模式入列：FULL §5 分量之外**新立**分量，不扩既有分量。
- **Lean 相容性（新文件，不动既有）**：`formal/Origin/NestTurnClass.lean`（formal-chain 后续工位）——`inductive NestTurnClass`（四构造子）+ `turnClass_unique`（∃! partition，镜像 `sixState_unique :170-174` 证明模式）+ c′ 必要性谓词的 Decidable 实例（镜像 `signalBitsOf` 的判据投影模式 `:285-291`）。rust 侧本关实装即领先 Origin——按 T3 裁决 L1 先例登记漂移，本关不动 Lean。
- **r 的只读消费**：c′ 必要条件可读作 c′ 级六态查询（§2.2-D-3.4：顶转 D¹/底转 U¹；平台下破 = 反向裂态）。这是对既有 partition 的**查询**，零修改——相容性的正确方向（消费而非扩态）。

---

## 4. bit-exact 冲突面（必答 4）

### 4.1 触及文件清单（设计，实装时逐 hunk 核对）

| 文件 | 改动 | 性质 |
|---|---|---|
| `rust/src/theta_v0/classifier/nest.rs` | `extend_typed_upward`/`assemble_typed_certificate` 追加 confirmed 向量（`:576-609`/`:647-650` 同位，约 6 行）+ `TypedNestCertificate` sidecar 字段与访问器（`:411-448`）+ 头注 | sidecar；`NestCertificate`/`NestRung`/装配逻辑/终端查法零改 |
| `rust/src/theta_v0/classifier/turn_class.rs` | **新文件**：`NestTurnClass`/`XzdEvidence` + `classify_nest_turns` + c′ 过滤 + 单测（§5） | 纯只读派生 |
| `rust/src/theta_v0/classifier/mod.rs` | 模块注册 + re-export（2 行） | 机械 |
| `rust/src/bin/p92_nest_replay_postruling.rs` | 新增**独立** `TURN_CLASS` dump 行（证书身份键 + 分类值 + evidence）+ DeferOrphan 发射 | 侧信道只写不判（#98/#99 纪律）；**CERT 行格式零改**（p105/p107 归档仪器前缀解析不受影响的更强形式——根本不动旧行） |

### 4.2 不碰清单（主干零触碰，无需升级裁定）

塔/笔/线段/中枢主干（`recursive_tower.rs`/`decompose.rs`/`center.rs`/parser）、`signal.rs` 全部判据链（`judge_segment :1248+`、`judge_first_cached`、`trend_third_class_in_c :497-521`、make_*_point）、`divergence.rs`、`bsp.rs`、`level_view.rs` provider 门（含 `:681-683` extreme 预滤、`:689-704` R1 全合取）、`types.rs`（BspBits/class_index）、`six_state.rs`/`level_state.rs`、Lean 全部既有文件。τ 门/037:20 门/锚门/T1 终端查法一概不动。

### 4.3 bit-exact 论证

- **构造保证**：本分支对账本（moves/centers/bsp/pan_div）与塔只读；证书真值对象零改；sidecar 向量不进 `NestCertificate` 相等性语义（`TypedNestCertificate` 的 PartialEq 派生比较含 sidecar——证书去重键 `certificate_key`（`p92:936+`）按 ids 字符串构造，不读该字段，去重行为不变；实装时以 §5-T8 回归锁钉死）。
- **重放复验**：`P92_BIT_EXACT` 五维（old_path/tower/moves/centers/bsp/pan）预期 diff=0；`P92_PROVIDER` complete=true、future_violations=0。CERT 行零改 ⟹ p105/p107 口径对账不受污染；`TURN_CLASS` 新行独立成维。
- **p116 对账**：本分支不改 TERM 谓词、不改事件/证书集合 ⟹ p116 归因口径无新义务（T1 的 C2 平移义务已由关③承担，本关不叠加）。

---

## 5. 单测计划（必答 5）

落点 `turn_class.rs` 测试模块；夹具风格沿用 `nest.rs:1009+` 既有模式（手工 `NestCandidateEvent`/`events_by_level` + `Classification` 假账本：`levels[k].centers`/`bsp` 直造，`BspPoint.center` 按 `make_third_point` 契约填）。坐标全部 source_index 同系。

数据构造公共件（044:16 形态几何）：`levels[1].centers` = A[100,200]（0-12）、B[150,250]（20-32）；父事件（level=2，side=Short，`divergence_confirmed=false`，interval_b=(33,50)，turn_source=50）；基例（level=1，side=Short，`divergence_confirmed=true`，interval_b=(44,48)，turn_source=48）；`levels[0].centers` 含 c′[420,440]（40-46）⊂ (33,50)；终端闭包对基例返回 sell1 bits。链 (top=2, exec=1) 成证，confirmed 向量 = [false, true]（高→低）。

| # | 测试名 | 断言 | 教义锚 |
|---|---|---|---|
| T1 | `turn_class_nested_confirmed_when_top_confirmed` | 链顶 confirmed=true ⟹ `NestedConfirmed`（c′ 账本留空也不影响） | 043:28/30 情况一 |
| T2 | `turn_class_xiaozhuanda_candidate_044_16_positive` | 公共件 + `levels[0].bsp` 置 sell3@52（center=c′）⟹ `XiaozhuandaCandidate`，evidence：c′=[420,440]/(40,46)、third_src=52、causal 序 46≤48<52 | 044:16/044:24 正例 |
| T3 | `turn_class_not_candidate_when_c_prime_missing` | `levels[0].centers` 无包含中枢 ⟹ `ExecEvidenceOnly` | 044:18（c 至少含一个次级别中枢） |
| T4 | `turn_class_not_candidate_when_c_prime_normal_oscillation` | c′ 在但 `levels[0].bsp` 无三类点 ⟹ `ExecEvidenceOnly`（正常震荡不可能转化，防误标主负例） | 044:20 |
| T5 | `turn_class_platform_breakdown_negative_066_198` | 底转镜像件（side=Long）+ c′ 处只有 **sell3**（平台下破，方向反）⟹ 非候选；另构造 buy3 版 ⟹ 候选（镜像正例） | 066:198 负例 + 044:26 镜像 |
| T6 | `turn_class_not_candidate_when_third_precedes_base` | sell3@45（先于基例 turn=48）⟹ `ExecEvidenceOnly`（因果序否决） | 044:20「顶背驰只能出现在c`之后」 |
| T7 | `turn_class_defer_orphan_event_039_34` | 父事件不被任何链消费（基例方向不合/无包含）⟹ 账本含 `DeferOrphan{identity}` 且不带候选语义；孤儿事件 100% 有类 | 039:34 defer |
| T8 | `turn_class_partition_unique_and_sidecar_neutral` | 枚举合成链（depth 1..3 × confirmed 全组合 × c′ 状态）⟹ 每证恰一类；同输入下 `certificate()` 与去重键不受 sidecar 影响（旧测试零改通过的等价锁） | partition 纪律 |
| T9 | `turn_class_candidate_carries_no_confirm_semantics` | evidence 结构无 `BspBits` 成员、无 `confirm_side` 方法（编译期构造保证 + 文档化断言）；候选不进任何 six-bit 置位路径 | 044:30 只有必要条件 |

辅助断言：T2 同步验证 `XzdEvidence.second_class`——`levels[1].bsp` 窗内置 sell2 时记入、不置时 `None` 且候选仍成立（053:28 证据不作门）。

---

## 6. 验收线建议（必答 6，供裁定用）

1. **测试**：`cargo test --lib` 全绿（既有测试零**非授权**变红）；新增 §5 九测试落账；`cargo check --bins` 零错误。
2. **bit-exact**：`P92_BIT_EXACT` 五维 diff=0（构造保证 + 重放复验）；`P92_PROVIDER` complete=true、future_violations=0；CERT 行逐字节不变（`TURN_CLASS` 独立新行）。
3. **证书集合三栏对照**（基线 = 实装时点最新全量重放）：**存续=全部、失证=0、新增=0**——分类是旁挂派生不是过滤/增产；任何证书增减 ⟹ 回票（p115 §6 纪律：不为保产量叠口径）。
4. **无类清零（主验收线）**：terminal_confirmed 基例事件 ∪ c 破极值未确认 Trend 事件，在 `TURN_CLASS` 账本上 100% 有类（NestedConfirmed/Candidate/ExecEvidenceOnly/DeferOrphan 之一）；missed-class 计数 = 0，逐检查点落账（对照 p107 F3→F4 missed=0 口径）。
5. **防误标**：T3-T6 四负例全绿；重放层抽审——全部 Candidate 逐案过 c′ 三要件探针（c′ 定位/三类点/因果序），误标率 = 0；任一负例被标候选 ⟹ 回票。
6. **候选占比诚实声明**：Candidate 占链比与逐检查点计数落账；若占比显著高于「少见」基线（044:32/044:48），报告须显式援引 `doc-level-domain-20260717.md:141,176` 判别式声明「候选泛化指向检测聋度/装配工件，非教义形态」，不得记为产量。
7. **044:30 类型层封锁核验**：Candidate 消费面审查——无 confirm_side/BspBits 语义泄漏、不进终端门、不进 strategy 触发链（编译期 + grep 双重核验）。
8. **调度**：p116 后台重放完成前禁 cargo build/test（T4 纪律沿用）；本关排在关③（T1）之后串行——T1 落地改变 `levels[ℓ-1].bsp` 消费口径，本分支的 c′ 查法与其同源（`levels[ℓ-2]`），须一次性锁定。
9. **Lean 漂移登记**：rust `NestTurnClass` 领先 Origin，列 formal-chain 遗留项（新文件 `Origin/NestTurnClass.lean`，∃! + Decidable 投影，T3 L1 先例）。

---

## 7. 边界与遗留（本关不裁）

- defer 的**时序重判**（DeferOrphan 挂起后何时按 039:34 三段成中枢再比较）是调度问题，本关只交付分类口径与账本值；重判状态机另立任务。
- nest 候选通道与 BSP 二类点通道（053:28 操作层）的联合归类另立裁定；本关 evidence 只记录二类点坐标。
- 098:18（「（注：这个背驰非区间套，1分级别小转大）」）的形态实例地位：`doc-level-domain-20260717.md:181` 把该「（注：…）」列入注家文字不作证据。本设计全部承重教义 = 043:34/044:16-30/053:28/039:34/066:198（原文），098:18 仅作形态参照，不落证据位——两读法差异不影响任何判据。
- exec≥2 泛化：c′ 查法对任意链顶级 ℓ≥2 同构适用（级别算术只依赖 T1 事实链），实测按 exec 分层如实报告，不逐案预承诺（T1 裁决第 5 条同款口径）。

## 8. 090 纪律自检

- 教义直读清单（全部主仓 `docs/chanlun/text/blog/` 原文逐条核对）：044:16 @`044-第44课.md:16`、044:18 @`:18`、044:20 @`:20`、044:22-26 @`:22-26`、044:30 @`:30`、044:32 @`:32`、044:48 @`:48`、053:28 @`053-第53课.md:28`、039:34 @`039-第39课.md:34`、043:34 @`043-第43课.md:34`（043:26/28/30/32 邻段同文核对）、067:236 @`067-第67课.md:236`、066:198 @`066-第66课.md:198`、098:18 @`098-第98课.md:18`（注家地位见 §7）、037:20 @`037-第37课.md:20`、065:94 引自 `doc-level-domain-20260717.md:29` 转录。
- 代码坐标抽查复核：`level_view.rs:681-683`/`:689-704`、`:535-626`、`:466-480`；`nest.rs:564`/`:567-570`/`:628-630`/`:636-671`/`:647-650`/`:411-418`/`:383-407`/`:473-478`/`:499-539`；`signal.rs:497-521`/`:558-569`/`:577-588`/`:1248-1279`；`bsp.rs:64-67`/`:122-126`；`mod.rs:161-195`；`six_state.rs:29-63`；`p92_nest_replay_postruling.rs:746`/`:764-794`/`:813-815`/`:920-934`/`:967-972`；`TrendSixState.lean:82-89`/`:134-147`/`:170-174`/`:231-234`/`:252-259`/`:298-326`/`:379-381`/`:399-401`——全部与本图引用一致。
- 声明与能力一致：本图只设计不实装；c′ 级别算术（`levels[ℓ-2]`）由 T1 事实链静态推导，实装时以 §6-1 测试钉死（标为实装验证点 W1）；未运行任何 cargo/重放（p116 不受干扰）；零 git mutation；主仓零写入；新增文件仅本文。

## 签字位

- [x] 断链点定位：装配环 `nest.rs:647-650` confirmed 丢弃 + 分类词汇缺失（次级：单级退化、孤儿化；候选环否则条款域排除）
- [x] 分支形态：旁挂联合分类（D）——sidecar confirmed 向量 + `NestTurnClass` 四类 partition；否新证书类型/第七位/第七态
- [x] c′ 必要条件全构件可判定性表（§2.4）+ 044:30 防误标四封锁位（§2.3）
- [x] 六态/位向量正交性论证（⊥/第七位否决，r 只读消费）
- [x] 不碰主干（升级裁定豁免）+ 冲突面清单（§4）
- [x] 单测 9 个（含 044:16 正例、066:198 平台下破负例）+ 验收线 9 条（含无类清零主线）
- [ ] 编排者/裁定者复议（空位）
