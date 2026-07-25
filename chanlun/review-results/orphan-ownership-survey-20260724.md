# 既有归属实装核查（orphan ownership survey）——issue #217 / map #126 子票

- 日期：2026-07-24 ｜ 角色：AFK 只读调研员（不拍板，裁定归 #211 grilling）
- 票据：issue #217（repo xy7365527-lang/NewChanlun，wayfinder map #126 子票）
- 问题域：map #126 owner 判定（终端背书 Trend 域合法合取 `point.center.start_index == event.b_center_start`）被双错配质疑（级别错配 + 锚错配）；核查既有 orphan/历史实现里是否已有正确的结构派生归属实装而未接线。
- 090 约定：【确证】= 源码直读/实测计数；【推断】= 基于机制的解释；【未核实】= 未及核查。
- 只读声明：rust 源码零改动；git 零 mutation（log/show/grep 只读）；本报告是唯一仓库写入。

## TL;DR

1. **「点↔其判定结构」的结构派生归属在生产点构造层已实装**（教义三对应物全在，§四-Q4）；双错配发生在**「事件↔点」跨级配对层**——owner=B 把点侧中枢（账本级 ℓ-1）与事件 B（事件级 ℓ）做 `start_index` 序号判同。【确证】
2. **现行核之外存在四套归属/身份判据实装**（§二）：塔扫描身份归属边（ElementId）、#206 两元锚跨级同点索引（已接 admission 未接 nest）、descend 同点递归锚定（生产门在用）、XZD 双序号判同（已证伪退役）。【确证】
3. **最接 #211 基座的是 projection.rs 锚索引**（#206 裁定锚的既有实装）+ admission.rs 既有 `cross_level_query` 调用模式；但须先分清要修的是「点↔中枢归属」还是「点↔事件同点」——两个谓词不同（§五）。【推断】

## 一、五套并行跨级实现的归属语义（Q1）

来源：`e2e-existing-implementation-synthesis-20260720.md` §一（agent-52 盘点）。逐套补查归属语义：

| 套 | 位置 | 归属（owner/背书）语义 | 判据形态 | 结构派生？ |
|---|---|---|---|---|
| A. 严格三门 DFS 装配 | `classifier/nest.rs`（`terminal_bits_in_book_core`:605） | Trend 域合法合取 = `confirm_side ∧ owner=B`；owner=B = `point.center.start_index == b_center_start`（:645-647）；事件级 ℓ 读账本级 ℓ-1（`event_bsp_book_level`:489-493 严格下移一级） | **跨级序号判同**：点侧中枢属账本级 ℓ-1，事件 B 属事件级 ℓ（`level_view.rs:771,807` B=block_end_center seed 的 start_index） | **否**【确证】 |
| B. chi_bool 递归核 + nest_confirm | `strategy/nest.rs:122-143` + `strategy/interp.rs:378-391` | **零 owner 判据**（grep owner/归属/center 判同零命中）——只判 cand_delta/confirm_side，无归属概念 | 无 | 无归属层【确证】 |
| C. 生产门多级构造 | `backtest/econ_positive.rs` | Type2/3 准入经 `descend_type1_anchor_depth`（见 D）；无中枢序号归属判同 | 同点递归式锚定 | **是**（经 D）【确证】 |
| D. 次级别一类锚定 | `econ_positive.rs:714-742` `descend_type1_anchor_depth` | 点↔次级别结构归属：`find_move_by_end_index(subs, source_index)`（端点对齐=同点递归的序号形态）+ 逐级 `div_cand`（次级别一类锚）+ 递归下钻到最低可用级 | 端点序号对齐 + 背驰判据，逐层收缩 | **是**（教义同点递归精神；锚=end_index 序号，非两元锚）【确证】 |
| E. task-106 必要条件检查器 | commit `6075840687`（`classifier/interval_necessity.rs` 256 行新增）——**不在本分支 HEAD**，在 main/codex-typed-close-t1 等分支 | 窗口式必要条件判据，只读不回写 bit | 【未核实：未及读文件内容】 | 【未核实】 |

## 二、现行核之外的归属判据实装（Q2）

| # | 实装 | 位置 | 归属语义 | 接线状态 |
|---|---|---|---|---|
| 1 | 塔扫描身份归属边 | `recursive_tower.rs:434-457` `CpScanOwnership`、:1139-1143 `CandDeltaCpEdge`、:1022-1028 `ParentCenterIdentity` | 背驰事件→B_p 中枢的稳定归属边，锚 = **ElementId**（确定性递归元素 ID），非序号；`CandDeltaEvent.b_parent/cp_ownership`（:1173,:1185）在生产 | **已接线**（事件↔中枢归属，非点↔事件归属）【确证】 |
| 2 | #206 两元锚跨级同点索引 | `projection.rs:85-180` `LevelProjectionLayer.triple_anchor_index`（极值价→[(组锚,坐标)]）+ `cross_level_query`(:168)；T5a #207 去方向 | **同点递归身份**：跨级同一拐点按极值价精确等值查（v3 无容差），组锚回执区分 W 底双脚 | **已接 admission.rs**（:787 脚身份组锚匹配、:858 恰好存在扫描，共 5 处调用）；**nest.rs/nest_index.rs 零引用**（grep 确证）——未接终端背书【确证】 |
| 3 | descend 同点递归锚定 | `econ_positive.rs:714-742`（见 §一-D） | Type2/3 信号↔次级别一类锚的结构派生定位 | **生产门在用**（Nest 通道 base gate，:1252；Type2/3 准入 :823,:842）；未接 nest 背书【确证】 |
| 4 | XZD 双序号归属判同 | `econ_positive.rs:1343-1359` `xzd_sub_last_zs_type3`、:1391-1397 `same_center_any` | 三类点↔最后次级中枢：`center.start_index == z.start_index && center.end_index == z.end_index`（比 nest 多 end 维） | **已证伪退役**：#44 探针确定性证伪为「归属链错位」（:1104-1108 注释在案），判据换第43课「背驰后新中枢+反向突破」；旧字段仅留诊断【确证】 |
| 5 | Q5 Unassigned 账本 | `q5-orphan-attribution-research-20260713.md` §4（草案 schema） | 「段→走势」归属（非本票「点→中枢」语义） | **生产零实装**（全树 grep `Unassigned` 仅 p83 研究 bin 注释命中）【确证】 |

另有旁证：`archive/c71-seam-20260714/level_view.rs` 的 `ownership_blocks`（段→MoveBlock 归属审计基线，Q5 语义域，非点↔中枢）。【确证】

## 三、对拍表：教义归属语义 vs 各实装 vs 现行 c==B

教义结构派生（票体给定口径）：一类=破 B / 二类=该走势一类后首次回拉成立 / 三类=离开中枢回抽不回。#206 裁定：身份=同点递归，锚=（极值价, 合并组锚），Avoid 序号判同（主仓 CONTEXT.md:49-54）。

| 维度 | 教义 | 现行 owner=B（nest.rs） | projection 锚索引 | descend 锚定 | XZD 双序号（退役） |
|---|---|---|---|---|---|
| 谓词对象 | 点↔中枢/走势归属 | 点.center↔事件 B（跨级配对） | 点↔跨级同点身份 | 点↔次级别一类锚 | 点.center↔最后次级中枢 |
| 判据 | 结构派生（破/回拉/回抽） | `start_index` 序号判同 | （极值价, 组锚）两元精确等值 | end_index 端点对齐 + div_cand | start+end 双序号判同 |
| 级别口径 | 点自身级别结构 | 点级 ℓ-1 vs 事件级 ℓ（级别错配在案） | 跨级不变量（极值价） | 逐级下钻（ℓ→ℓ-1→…） | 同级（次级别内） |
| 实测 | — | wf7 owner 相等率：一类 50%、二类 22.5%、三类 11.3%（#214） | — | — | #44 证伪（归属链错位） |

## 四、四问逐条答案

### Q1 五套各自归属语义、哪套结构派生

【确证】见 §一表。**C+D（生产门 + descend_type1_anchor_depth）是结构派生的**（归属从走势结构推出：次级别一类锚 + 端点同点对齐，非中枢序号判同）；A（nest 终端背书）是序号判同（现行被质疑路径）；B 无归属层；E 未核实。

### Q2 现行核之外有无第二套归属判据实装

【确证】**有，且不止一套**（§二 1-4）。语义最近 #206 裁定的是 **projection.rs 两元锚索引**（同点递归 +（极值价, 组锚），T5a 去方向形态 = 裁定书原文形态）；语义最近教义归属结构派生的是 **descend_type1_anchor_depth**；塔扫描 ElementId 归属边是事件↔中枢层的身份判据实装。

### Q3 更接近正确的实装：位置/语义/差异/接线面

【确证-位置语义】+【推断-评估】：

- **基座首选 = projection.rs 锚索引 + admission.rs 调用模式**。差异：现 owner=B 问「点判定中枢==事件 B」；锚索引问「点与事件是否同一拐点」（同点递归）。点侧两元锚**无需给 BspPoint 加字段**——`projection.rs:143-145` 已有从 `point.source_index` 经 T1 供给线（`fractal_at_source`/`merged_group_anchor`）的现成查法；事件侧 `NestCandidateEventExt` 已带（极值价, 组锚）（`level_view.rs:788-811`）。改动面集中于 nest.rs 判定核的 owner_eq 换锚 + 锚供给参数接线；判定集合会变 ⟹ GOLDEN digest 翻转族（#110 先例流程受控）。
- **就位插槽 = Exact 臂**（nest.rs:619-622，`point.source_index == turn_source`）——点↔事件同点判定的序号形态，两元锚化改造语义最近；但其谓词是「点=事件拐点」非「点归属=B」【推断：谓词语义差，见 §五警示】。
- **descend_type1_anchor_depth 复用**：已证明「Type2/3 信号↔次级别一类锚」结构派生可在生产存活（生产门吞吐在案）；若背书层复用，把「窗内最早合法点」换成「与事件 turn 同点递归的下钻点」。语义差：它是定位谓词非归属谓词【推断】。
- **XZD 双序号判同不可复用**（#44 已证伪为归属链错位——「同中枢序号」不等于「同归属链」，对本票是直接的反面教材）【确证-证伪在案】。

### Q4 教义结构派生三对应物

【确证】三者在生产点构造层全部已实装——**判据本身是结构派生的，序号判同只是把结构事实压成 `center.start_index` 等值**：

| 教义 | 实装 | 归属载体（构造时填载） |
|---|---|---|
| 一类=破 B | `judge_first_cached`（signal.rs:340-399：破最后中枢几何 ∧ 037:20 破 b 包络 ∧ A/C 配对） | `make_first_point` center = **last_center**（被破的最后中枢，a+A+b+B+c 的 B 同型对象，:539-560）——归属教义成立 |
| 二类=该走势一类后首次回拉 | `find_second_type_structure`/`extract_second_signals`（signal.rs:820-877：i1=一类离开 + i2=回拉不创新低/新高，i1<i2，相对 `parent` RMove=「该走势」） | `make_second_point` center = **c1**（次级别一类离开所破中枢；生产实参 = `parent.rmove` Compose `centers.first()`，mod.rs:2275-2282）——**归属载体是次级别中枢，非「该走势」身份**：级别错配的构造根源 |
| 三类=离开中枢回抽不回 | `judge_third_cert`（signal.rs:430-478：离开最近确认中枢 + 回抽不入 [ZD,ZG]） | `make_third_point` center = **c**（所离开回抽的中枢，:566-577）——归属教义成立，但与事件 B 是否同中枢无构造保证 |
| 区间套同点递归（定位） | `descend_type1_anchor_depth`（econ_positive.rs:714-742）+ projection 锚索引（projection.rs:168） | — |

## 五、可接线性评估与概念警示

【推断】**两层谓词须先分清**（#211 执行票的前置问题）：

1. **点↔中枢/走势归属层**（教义「一类=破 B」等的本义）：结构派生已在点构造层成立（§四-Q4）——每个生产点的归属由其判定结构内在给出。此层要修的不是判据而是**载体**：二类点 center 载的是次级别 c1，若要载「该走势」身份需改 `extract_second_signals` 的填载（parent 身份/B 中枢可得——parent 与 Compose.centers 都在手）；一/三类载体教义已对齐。
2. **点↔事件同点层**（区间套背书配对）：#206 裁定锚=（极值价, 组锚）+ 同点递归，实装=projection 锚索引，已接 admission 未接 nest；nest 的 owner=B 序号判同与 Exact 臂序号同点都在此层。换锚接线面小（§四-Q3 首选），但语义从「归属合取」变为「同点合取」——**这是否等价于 map #126 想要的 owner 语义，归 #211 裁定，本报告不拍板**。

接线改动面分级（推断）：(a) 换锚（nest 判定核 owner_eq→两元锚）= 单函数 + 锚供给参数 + GOLDEN 翻转流程；(b) 二类载体改填（c1→该走势 B/身份）= signal.rs 构造器 + owner 判定式语义重定 + 全消费面复核（止损不进 owner 载体，消费面小，事实账 §4.1 已盘点）；(c) 两层都动 = (a)+(b) 正交可叠加。

## 六、证据锚清单

- 现行核：nest.rs:489-493（账本下移一级）、:605-665（owner_eq 判定式）、:619-622（Exact 臂）
- 事件 B 构造：level_view.rs:771,805-807（Trend）、:879-881（Pan 同写归因不作门）；两元锚 :788-811
- 点构造器：signal.rs:539-560 / :566-577 / :590-601；二类结构识别 :820-877；c1 实参 mod.rs:2275-2282
- 结构派生归属：econ_positive.rs:714-742（descend）；:1343-1403（XZD 双序号 + #44 证伪注释 :1104-1108）
- 身份实装：recursive_tower.rs:434-457 / :1022-1028 / :1139-1143 / :1151-1194；projection.rs:85-180；admission.rs:787,858
- 裁定/实测：主仓 CONTEXT.md:49-54（#206）；`endorsement-failure-instrument-readings-20260724.md` §1.1（wf7 相等率）；`p1-resupercede-fact-ledger-20260724.md` §4.1；`e2e-existing-implementation-synthesis-20260720.md` §一
- 未接线/未实装：task-106 commit 6075840687（非本分支 HEAD）；Q5 Unassigned（生产零实装，grep 全树仅 p83 bin 注释）
