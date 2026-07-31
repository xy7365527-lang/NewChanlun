# #809 总缝规则过筛：#789 的约 40 套口径逐条分类

- 日期：2026-07-30；票据：[#809](https://github.com/xy7365527-lang/NewChanlun/issues/809)（parent map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)）
- 性质：**普查票，不裁任何概念的口径**。只读，零代码改动。本文件是唯一产物。
- 判据正本：`AGENTS.md`「缠论教义正本」节（总缝规则 [#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) / 收敛通则 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十 / 管辖 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定六），commit `dac8ff9fa1`。
- 过筛对象：[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 报告 `.chanlun/review-results/issue789-multi-impl-census-20260730.md`（commit `b7eaff1097`）的 70+ 实体 / 约 40 套独立教义口径。

---

## ⚠ 开头三件必须先看的事

### 1. 例外候选上限：**未触顶，1 / 3**

**合规例外候选总数 = 1 条**，即已知占用的那一条（`compose_level` 的 `is_l0` 中枢分派）。
**本次过筛没有找到第二条够格的举证候选。**

但要写明一条**逼近触顶的预警**（090 照实，不粉饰）：本次判成「违规-已定性」的 6 条里有 **2 条**
（`recursive_t::leg_strength` 的 level=0 退化、`a_nested_divergence` 的力度按级别换判据）在形式上
与 E1 同型——都是「上级/下级输入缺这个维度」。**若第二批概念票选择给它们也开举证书而不是收敛，
上限 3 条会在中枢票 + 背驰票两张票之内用光。** 现在还没用，但余量只有 2。

### 2. 三个数

| 类 | 条数 | 口径行数 |
|---|---|---|
| **违规-已定性** | **6** | 6（涉及落点 11 处） |
| **合规例外候选** | **1** | 1（涉及落点 5 处，均为同一条缝） |
| **真分歧-待裁** | **34**（按分歧组计，分 7 概念） | 涉及约 40 套口径的全部竞争分支 |
| **不适用** | 19 | 19 |

「真待裁」计的是**分歧组**（一组 = 同一个问题的若干个互不相容答案），不是口径行数——
因为一组里的每个分支都要在同一张概念票上一起裁，按口径行计会重复计数同一个待裁问题。

### 3. 本报告对「不适用」这一类做过一次口径扩张，此处明写

票面把「不适用」定义为**管辖外**（对照臂 / 只读观测器 / 验证装置），后续列写的是「不计入分歧面」。
本报告的 19 条「不适用」里：

- **13 条是票面原义的管辖外**（对照臂 / 零调用件 / 只读观测器 / 验证装置 / 账本 / 透传层）；
- **6 条是「非分歧项」**——它们在生产判定路径内，但实测**不产生第二套口径**（要么级别差异只以
  传入值出现、下游是同一个判据函数，要么根本不是判据而是缓存键 / 输入投影）。这 6 条严格说
  应该叫「合规-实测通过」，票面四类里没有这一格。**为不擅自新增第五类，归入「不适用」，
  并在表中用「非分歧项」前缀标出，可随时拆回。**

---

## 一、违规-已定性（6 条）

判据：踩收敛通则（两套判据接在同一 if-else 上 / 宽档接管严档失败）或总缝规则
（同一判定按级别换判据），且违规事实清楚、无书面举证。

| # | 口径名 | 落点 | 踩哪条 | 一句理由 |
|---|---|---|---|---|
| **V1** | 次级别背驰确认双档 + C1 降级 | `rust/src/theta_v0/classifier/cand_predicate.rs:107` `div_cand`（四条件含 Extreme） ↔ `classifier/mod.rs:1929` `sublevel_diverges`（无 Extreme）；分派点 `backtest/econ_positive.rs:1743` `build_gate_certificate` | 收敛通则 | `if let Some(cert)=build_nest_certificate(..) { Nest } else { build_xzd_fallback(..) }`——严格档失败即落宽松通道，实测 52 个候选 51 个经此放行（[#796](https://github.com/xy7365527-lang/NewChanlun/issues/796)）。**本条即通则的在案来源，非我新判。** |
| **V2** | `Cand^δ` 的 per-rung 恒真档 | `rust/src/theta_v0/backtest/econ_positive.rs:1052-1060` `cand_delta` 分派：Type1 → `div_cand` 四条件；Type2/3 → **恒 `true`** | 收敛通则 | 同一个 `Cand^δ` 符号在同一个 dispatcher 的两个分支上，一支是四条件合取、一支是常真——常真不是判据的一个取值，是判据缺席。**反方辩护登记在案**：可主张这是限定词①「不同判断」（Type1 与 Type2/3 判的对象不同，存在性已由 base gate 的 descend anchor 门控）；但该辩护**代码里没有写成举证书**，且 `nest.rs:155-157` 同时自陈 `Cand^δ_ℓ` **spec 无独立定义式**，两处叠加 ⟹ 现状按违规记。 |
| **V3** | `recursive_t` 力度的 level=0 退化 | `rust/src/recursive_t/divergence.rs:72-81` `leg_strength`：`if nest>0 { 嵌套深度 } else { hi−lo 几何振幅 }` | 收敛通则 + 总缝规则 | 两个量纲完全不同的力度接在同一个 if-else 上（系统自己声明二者可互换）；且 `nest==0` 实际就是 level 0（下级无内嵌中枢）⟹ 同一判定 L0 与 L≥1 跑的不是同一个判定。已亲手核对源码，**零 MACD**。 |
| **V4** | MACD 缺失时力度 fallback = 振幅×时长 | `rust/src/divergence.rs:188-197`（`compute_force` 尾部 Fallback 分支）＋ 其 Python 对位 `src/newchan/a_divergence_v1.py` | 收敛通则 | 同一个 `compute_force` 内：有 MACD 走面积、无 MACD 走 `(high−low)×duration`。严格档取不到数据即由宽松档接管并照常产信号，是「判据失败 ⟹ 换判据重判 ⟹ 放行」的标准形。 |
| **V5** | 中枢关系判定的「弱化版本」兜底 | `src/newchan/a_trendtype_v0.py:143-155` `_centers_relation`：`if has_gg_dd: 严格理论判定 else: _centers_relation_by_zg_zd` | 收敛通则 | **源码 docstring 自陈**「GG/DD 缺失时用 ZG/ZD 的兼容路径（§8.2 **弱化版本**）」——宽严两档、接在同一 if-else 上、宽档接管严档的输入缺失。附带事实：该弱化档实现里用的是 `high`/`low` 而非函数名声称的 `zg`/`zd`（第三种同向判据）。 |
| **V6** | 区间套内力度按级别换判据 | `src/newchan/a_nested_divergence.py:118-134` `_amplitude_force`（level 2+：振幅×组件数，docstring 自陈「level 2+ 无 MACD 数据」）↔ `:405` 起 `_finalize_with_level1`（level 1：MACD 真算） | 总缝规则 | 同一条区间套链上，「背驰力度」这一个判定在 level 1 和 level ≥2 是两套判据。按判别式：同一份数据喂两级，输出必不逐位相同。**注意这条 grep `level ==` 抓不到**，判别式走的是语义。 |

**违规条数：6。涉及落点 11 处。**

三条附注（090）：

- V1 与 V2 都落在 `econ_positive.rs` 的准入门上，是**同一条门链上的两处**，但踩的是两个独立事实
  （一处是通道降级，一处是谓词恒真），故分别计。
- V3–V6 **不是** #799/#804 已点名的在案实例，是本票新判。判据是通则原文，不新创口径；但既然是新判，
  **由第二批对应概念票复核后再定性生效**更稳——本报告只做分类，不代替裁定。
- 6 条里 4 条落在背驰/力度上（V1、V3、V4、V6）。**「力度」是本仓通则违规最集中的一处。**

---

## 二、合规例外候选（1 条，上限 3，未触顶）

| # | 口径名 | 落点 | 现状 | 一句理由 |
|---|---|---|---|---|
| **E1** | 中枢构造按级别分派：L0 含方向交替 / L≥1 不含 | 判据对：`rust/src/theta_v0/classifier/center.rs:210` `center_from_segments` ↔ `:252` `center_from_window`（**签名逐字相同**，实测只差一句 `dir_alternates`）。分派落点 5 处：`classifier/recursive_tower.rs:304` `compose_level`、`:859` `compose_level_resume`、`classifier/mod.rs:431` `classify_level`、`classifier/mod.rs:3275`、`backtest/wverify_run.rs:3092` | **现状不合规，够格但未成立** | 举证理由本身够格（上级单元是中枢外缘区间、无内在缠论方向，援引 `Origin.CenterStates.classifyDevelopment` 外缘判据背书，见 `center.rs:235` 起的「★诚实有效域」文档注释）；但按 `AGENTS.md` ②③ **缺退场条件、且载体在代码注释而非 `.chanlun/definitions/`** ⟹ 须由第二批中枢概念票补齐后方为合规例外。 |

**本次没有找到第二条例外候选。** 落点 5 处算 **1 条缝**（同一条判据分歧的 5 个调用点），与
`AGENTS.md`「实测现役只有 1 条缝」一致。其中 `mod.rs:3275` 与 `wverify_run.rs:3092` 属验证/回测装置，
真生产落点是前三处。

---

## 三、真分歧-待裁（34 组，按概念分组）

通则判不了（不是级别分叉、不是同 if-else 兜底），必须由概念票裁教义口径。
「已记账」= 上游票已明记归第二批，不必重推。

### 中枢（7 组）

| 组 | 问题 | 竞争答案与落点 |
|---|---|---|
| Z-1 | 核心区间取哪几段 | 全三段 `max(3lo)/min(3hi)`：`rust/src/zhongshu.rs:185`、`level.rs:118`、`a_zhongshu_v1.py:142`、`recursive_t/center.rs:95`、`theta_v0/classifier/center.rs:10`、`CenterConstruction.lean:92` ↔ **第 1、3 段**：`a_center_v0.py:145-150`、Lean 对位 `CenterConstruct.lean:490`。`native_decide` 已钉反例 `(11,18)` vs `(12,15)`（`CenterConstruct.lean:494/497`）。 |
| Z-2 | 单点核心 `ZD==ZG` 成不成立 | Rust 严格 `<`（`center.rs:219/257`、`zhongshu.rs:251`）↔ Lean 弱 `≤`（`CenterConstruction.lean:116`、`CenterComplete.lean:148`）。`center.rs:204-208` 自陈「Lean 侧未随 #321 跟进」。按 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793)，这是**裁错了层**（端点问题归 Lean 管），不是跟进不及时。 |
| Z-3 | 中枢成立要不要方向判据 | 方向交替（`center.rs:210`、`CenterComplete.lean:146`）↔ s1/s3 同向不查 s2（`a_center_v0.py:252`）↔ 完全不查（`zhongshu.rs`、`level.rs:118`、`recursive_t/center.rs:95`、`CenterConstruction.lean:116`）。**已记账**（[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 明记归第二批）。 |
| Z-4 | 有无 ≥9 段升级重切 | 有（`recursive_tower.rs:57-70` `UPGRADE_TOTAL_SEGMENTS=9`，第 33 课，且 `:771` 明写重切子中枢不复验 seed）↔ 无上限（`zhongshu.rs:193`、`a_zhongshu_v1.py:104`、`recursive_t/center.rs:101`、`a_center_v0.py:209`、`CenterStates.lean:180`）。同段数据中枢**个数**不同 ⟹ 走势类型跟着翻转。 |
| Z-5 | 中枢结算后从哪继续扫 | `i=max(break−2,seg_end)`（族 I / Lean）↔ `i=last+2`（`recursive_t/center.rs:70`，自述 c 段命中率 21%→100%）↔ `i=j`（`recursive_tower.rs:742`）↔ `i=seg1+1`（`a_center_v0.py:298`）。 |
| Z-6 | 有无「候选/确认」中间态 | `a_center_v0.py:179` 独有 `sustain_m` 可调阈值（默认 2）↔ 其余全部几何成立即成立、零参数。 |
| Z-7 | 成员数固定 3 还是可配置 / 中枢建在什么对象上 | 固定 3（全部笔段实现）↔ `a_ph_zhongshu.py:329` `policy.min_members` 可配置、输入是 persistence barcode 的 merge bars、重叠判据可由 policy 关掉。 |

### 区间套（6 组）

| 组 | 问题 | 竞争答案与落点 |
|---|---|---|
| N-1 | 坐标系是什么 | 时间（`classifier/nest.rs:71`）↔ `source_index`（`classifier/cand_sub.rs:38`）↔ bar 区间（`a_nested_divergence.py` `_level_move_to_bar_range`）↔ **根本不是区间**（`Divergence.lean:172` `NestNecessary` 是纯蕴含）。 |
| N-2 | 下降到哪一级停 | 停执行级 e 且 partial chain 合法（`nest.rs` + `econ_positive.rs:1039-1045`，注释明写「设计选择：partial chain 合法」）↔ 末级必须恰为 ev（`strategy/nest.rs:129`）↔ 必须降到 a0（`recursive_t/divergence.rs:552`）↔ 降到 level 1 停（`a_nested_divergence.py:368-405`）。 |
| N-3 | 「区间套」这个词指什么 | 纵向级别下钻 ↔ `src/newchan/nesting/nesting_operator.py:20-24` 的 `NestingType{HORIZONTAL,VERTICAL}`，**横向 = 搜索空间收缩（配置→角→板块→标的）**，与纵向并列为同一算子 N 的两个实例。同名异物，且现役（`pipeline.py`/`pipeline_backtest.py`/`multi_tf_pipeline.py` 都在消费）。 |
| N-4 | 装配方向与递归骨架 | top-down（`econ_positive.rs:994`）↔ bottom-up（`:1142`，标为对照基线）；Lean 侧 `NestingCertificate.lean`（Nat 级别差递归 + 平移不变）↔ `IntervalNestCertificate.lean`（列表链递归），**文件头互相声明「刻意不合并」，无等价证明**。 |
| N-5 | `Cand^δ_ℓ` 的定义式是什么 | 无定义式（`nest.rs:155-157` 标 [需人工确认]，「spec 中仅作符号出现」）↔ 四条件合取（`cand_predicate.rs:107`）。**教义空洞**。（其「Type2/3 恒真」那一档已判 V2 违规，此处只余定义式缺失。） |
| N-6 | 两条生产 `N^δ` 严格度不同 | `classifier/nest.rs` `n_delta`（宽：partial chain 合法，回测准入门用）↔ `strategy/nest.rs` `chi_bool`（严：末级必须恰为 ev、级别严格递减，`strategy/interp.rs:586` 环2 用）。**两者都在生产判定路径，但不接在同一 if-else 上、也不按级别分叉 ⟹ 两条通则都判不了，须概念票裁。** |

### 背驰（5 组）

| 组 | 问题 | 竞争答案与落点 |
|---|---|---|
| D-1 | 「力度」是什么 | MACD 段面积（`theta_v0/classifier/divergence.rs:231/247`）↔ 三维度 OR + T4 零轴穿越（`rust/src/divergence.rs:305-321`）↔ 振幅×时长 ↔ 中枢嵌套深度（`recursive_t/divergence.rs:72`）↔ persistence Wasserstein-1（`a_divergence_topo.py:96`）。源码自陈「与缠师第 17 课原文相悖」（`theta_v0/classifier/divergence.rs:256-269`）。（其中「振幅×时长」和「嵌套深度」的**兜底接法**已判 V3/V4 违规；**它们作为力度候选口径本身仍待裁**。） |
| D-2 | 次级别背驰要不要 `Extreme` 条件 | 要（`cand_predicate.rs:107` 四条件）↔ 不要（`classifier/mod.rs:1929`）。**已记账**（[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十③ 明记归第二批）。 |
| D-3 | 盘整背驰比较哪两段 | 最后两个同向离开段（`rust/src/divergence.rs:504`）↔ 进入段 vs 离开段（`recursive_t/divergence.rs` `judge_consolidation_divergence`）↔ 只定位不判力度、力度另由面积门确认（`signal.rs:1089` `locate_pan_div_structure`）。 |
| D-4 | 判据形状：严格面积 `<` 还是三维 OR | `segments_diverge`（严格面积）↔ `segments_diverge_or`（面积 ∨ DIF 峰）——后者**是生产件**（`signal.rs:1285` 盘整背驰真值路径，已核实），非只测试。两者服务不同判定（趋势 vs 盘整）⟹ 可援引限定词①，故不判违规，但「同一概念两种判据形状」须概念票收口。 |
| D-5 | 力度判据能不能常驻可配置四档 | `divergence.rs:549-561` `DivergenceGauge{MacdArea,ThetaDom,Conjunction,ThetaLex}`，`confirm_divergence` 按 gauge 分派，`config.divergence_gauge` 进生产热路径。**灰区，照实登记**：它明确写了「不 fallback 回 MACD 口径；no-workaround」⟹ **不踩**收敛通则的「兜底」那半条；但踩了「同一判断不得存在宽严两档实现」的字面。文件自陈是三套预注册 Θ 的 OOS 实验设计，既非对照臂也非诊断模式 ⟹ 三条限定词都不完全覆盖。**判不了，归待裁。** |

### 买卖点（3 组）

| 组 | 问题 | 竞争答案与落点 |
|---|---|---|
| B-1 | 一类点判据含不含背驰 | 含（`BspClassification.lean:99` `IsType1 = brokeCenter ∧ IsDivergence`）↔ 不含（`theta_v0/classifier/bsp.rs:78-80`，且 `:159` 明说与 `macd_c_lt_a` 无关）↔ 背驰是生成源且用比值阈值（`rust/src/buysellpoint.rs:195-226`，`force_c/force_a ≤ 0.9`）。 |
| B-2 | 二类点的锚是什么 | 一类点的价（`buysellpoint.rs:267`、`a_buysellpoint_v1.py:217`）↔ 次级别第一类 + 回拉不创新低（`rmove_compose.rs:144` + `descend.rs:183`，真下钻）↔ `afterTypeOne ∧ ¬brokeCenter`（`BspClassification.lean:110`，且 `SellClosedLoop.lean` **明文拒绝冒充次级别递归**）↔ `after_first_buy ∧ is_pullback_end`（`bsp.rs:82`）。 |
| B-3 | 三类点「第一次回抽」在不在判据 | 在（`BspClassification.lean:113-118` `firstRetrace`）↔ 不在（`bsp.rs:86-88`）↔ 隐式承担（`buysellpoint.rs:418` 用「break 后首个反向段」）。`recursive_t` 自陈「第 38 课严格序数未单独编码」。 |

### 笔（4 组）

| 组 | 问题 | 竞争答案与落点 |
|---|---|---|
| S-1 | 「至少 N 根独立 K」算的是什么量 | `merged_gap≥2 ∧ raw_gap≥3` 净空原始 K 数双条件（`a_stroke.py:176-181`）↔ `b.source_index − a.source_index > 3` 锚 index 之差单条件（`theta_v0/parser/stroke.rs:60-63`）↔ `noSharedBar ∧ barsBetween ≥ 3` 不考虑包含关系（`SegmentFeatureSeq.lean:374`）。三者在有包含合并的区间上给出不同笔集合，**零对拍**。 |
| S-2 | 旧笔存废 | 族 I 支持 `mode=strict` 老笔 `min_strict_sep=5` ↔ theta_v0 头部「启用新笔，**旧笔禁用**」。（这是配置档并存，不是失败兜底 ⟹ 不判 V 类。） |
| S-3 | gap 不足时怎么回退 | `j+=1` 不动起点（族 I）↔ `i+=2` 放弃起点（theta_v0）。 |
| S-4 | 已确认的笔能不能回写改写 | `_extend_prev_stroke` 原地改写上一笔终点（族 I）↔ `collapse_consecutive` 配对前预坍缩、不回写（theta_v0，`mod.rs` 头部明文「已确认结构不可回写重分解」）。 |

### 线段（5 组）

| 组 | 问题 | 竞争答案与落点 |
|---|---|---|
| G-1 | 线段的构造范式 | 三笔重叠法恒 3 笔无终结判据（`a_segment_v0.py:147-163`）↔ 特征序列增量状态机（`a_segment_v1.py` / `rust/src/segment.rs` / `theta_v0 divide_segments_with_tail`）↔ 遇第一根反方向笔即切（`SegmentConstruction.lean:63-83`）。**且 `Pipeline.lean:95` 的 Lean 端到端管线用的就是第三种** ⟹ 形式化层跑的不是生产那套线段定义。实测量级差：无状态批处理 406 段 vs 参考 237 段（70% 过分段）。 |
| G-2 | 特征序列包含处理是不是方向性的 | 方向性（`a_segment_v1.py:201`、`segment.rs:337`、`feature_seq.rs:186`、`SegmentFeatureSeq.lean:153`）↔ 外包络并区间无条件 min/max（`theta_v0/parser/segment.rs:196-208` 静态臂，自称「中性实现」）。 |
| G-3 | 第二特征序列是什么 | 由触发笔之后的**同向**笔新建、任意分型即确认（`a_segment_v1.py:370`、`segment.rs:416`、`feature_seq.rs:229`）↔ 直接取首条序列分型之后的**剩余（反向）**元素、只找对偶分型（`SegmentAutoConstruct.lean:177-199`，`:174` 自承是「代理」）。**根本不是同一条序列。** |
| G-4 | 缺口封闭降级要不要 | `a_segment_v1.py:431` `if has_gap and self._extend_mode == "strict"`（67 课严格延续：缺口被 c 封闭 → 按第一种情况处理）↔ `optimized` 不做。配置两档并存（默认 strict）。灰区同 D-5：不是失败兜底，但两档并存。 |
| G-5 | 第二特征序列扫描窗口 | `MAX_SECOND_SEQ_SCAN=50`（Py / 顶层散件）↔ `second_seq_scan_window=0` 无限（theta_v0，`second_kind.rs:44-48` 明说「不复制 Python 的性能启发式」）。`theta_v0/parser/segment.rs:76` 附单点实测「窗口 7→∞ 输出不变（237→237）」——**单点实测一致，未证等价**（090）。 |

### 走势类型 / 级别（4 组）

| 组 | 问题 | 竞争答案与落点 |
|---|---|---|
| M-1 | 「盘整」是什么 | 恰好 1 个中枢（`moves.rs:126`、`a_move_v1.py:134`、`TrendCompleteClassification.lean:66`）↔ 一段 `LevelExpansion` 关系 run、可含任意多中枢（`decompose.rs:3-5`）↔ 方向断裂即收尾、方向另取首尾中点净位移（`recursive_t/trend.rs:47-77`）。 |
| M-2 | 「同向」判据 | 核心分离 `c2.zd>c1.zg`（`moves.rs:63`、`level.rs:176`、`a_move_v1.py:71`）↔ 外缘分离 `next.dd>prev.gg`（`center.rs:274`、`recursive_t/center.rs:55`、`CenterStates.lean:215`）↔ 双升双降 `high/low`（`a_trendtype_v0.py:131-139`）。三者接受集互不包含。（第三种的**兜底接法**已判 V5 违规；作为口径本身仍待裁。） |
| M-3 | 「级别」是递归级别还是数据涌现的簇 | 递归级别 `level_id = 下级+1`（`level.rs:46/106`、`a_zhongshu_level.py`、`classifier/mod.rs:512`）↔ persistence log-gap 递归分裂簇（`a_level_detection.py:282` `detect_levels`，经 `CalendarPeriodNamer:77` 映射到「30 分钟/日线」）。**直接命中 map #787 待裁的「统一递归算子 T / 每级别一个 T 实例」。** |
| M-4 | 「小转大」的教义位置 | 全仓**只有一处**实现（`a_xiaozhuan_da.py:142`）；Rust 三族 + Lean 五文件全无对应。`theta_v0/classifier/turn_class.rs` 的 `XiaozhuandaCandidate` 自陈「纯只读派生，永远只是必要条件」。**空洞，不是分歧**——列在这里是因为它同样只能由概念票裁。 |

**真待裁合计：7 + 6 + 5 + 3 + 4 + 5 + 4 = 34 组。**

---

## 四、不适用（19 条，不计入分歧面）

### 4.1 票面原义的管辖外（13 条）

| 口径/实体 | 落点 | 归类依据（已实测核实） |
|---|---|---|
| `ref_v1` 参照中枢 | `theta_v0/classifier/ref_v1.rs` | 对照件；生产零调用，消费者只有 `rust/tests/theta_v0_center_parity.rs`（grep 坐实）。附：`:9/37-40` 文件头仍断言 `center.rs` 用「前两段」，与现状矛盾（陈旧谱系记录，[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 已订正）。 |
| `interval_necessity_tower` | `theta_v0/classifier/interval_necessity.rs:72` | 必要条件检查器；grep 全仓消费者只有本文件 `#[cfg(test)]`，verdict 不回写。 |
| `cand_sub` + `chain_cert` 一族 | `theta_v0/classifier/cand_sub.rs`、`chain_cert/` | 模块头明说与 `NestCertificate`「不共享类型、不互相调用、不互为验证」；实测消费者只有 `chain_cert` 自身与两个只读探针 bin（`issue550_event_battery.rs`、`p126_d3_descending_clock.rs`）。 |
| `build_nest_certificate_bottomup` | `econ_positive.rs:1142`，唯一消费点 `:6319` | 已声明的独立对照基线（限定词③）。 |
| `nest_lifecycle` 活假设 sidecar | `theta_v0/classifier/nest_lifecycle.rs:350` | 文件自陈「仅审计载荷，不进真值路径」，明令禁入证书真值路径。 |
| `analyze_termination` 线段静态臂 | `theta_v0/parser/segment.rs::analyze_termination` | 生产零调用者（仅本文件 `#[cfg(test)]`）。**注意：这只解除它的管辖，不解除 G-2 的分歧**——G-2 的另一支在生产臂里。 |
| `turn_class::XiaozhuandaCandidate` | `theta_v0/classifier/turn_class.rs` | 自陈「纯只读派生，永远只是必要条件（044:30 纪律）」。 |
| `six_state.rs` / `force_conformance.rs` | 同名文件 | 在编译树、生产零调用（[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 6.2 项 12）。 |
| `a_divergence_topo.py` / `a_geometric_momentum.py` | 同名文件 | 仅测试路径；后者自带 521 号声明「背驰的动量闸必须 MACD」。 |
| `a_feature_sequence.py` / `a_fractal_feature.py` | 同名文件 | 仅 `tests/test_segment_v1.py` 消费。 |
| `nest_index` 的 exec 从 lvl 1 起扫、L0 设计性跳过 | `theta_v0/classifier/nest_index.rs:7/275-276` | 只读计数索引（`:58` 自陈「只读计数」）。**照实标注**：这处 L0 跳过在形状上是级别分叉，若日后该索引进入真值路径，须立刻重新过筛。 |
| `wverify_run.rs:3092` 的 `is_l0` 中枢分派 | 同 | 验证装置（W-VERIFY），非生产判定路径；口径本身与 E1 同源。 |
| `trading/nested_interval_fugue.rs`、`recursive_nested_fugue.rs` | 同名文件 | 操作语义（BSP→仓位映射），模块头明说「不改信号层」。 |

### 4.2 非分歧项：在生产路径内，但实测不产生第二套口径（6 条）

| 口径/实体 | 落点 | 实测判定 |
|---|---|---|
| 一/三类信号提取的 `is_l0` 分派 | `classifier/mod.rs:547`（L0 `extract_signals_with_hist`）↔ `:562`（L≥1 `extract_first_third_for_level`） | **已亲手核实两侧殊途同归**：`signal.rs:1514-1536` 的 L0 入口只是 `extract_signals_with_hist_anchored(.., None, ..)`；`mod.rs:341-372` 的 L≥1 包装是把 units 投影成 `Segment` + 传结构方向锚，再调**同一个** `extract_signals_with_hist_anchored`。级别差异只以传进去的值出现 ⟹ **总缝规则实测通过**。 |
| `candidate_scan_inputs` 的 `is_l0` | `classifier/mod.rs:245-260` | 输入投影（L0 借用 parser 线段 / L≥1 由 units 还原 Segment），下游是同一个 `observations_for_level`；方向锚一律取单元结构方向。非判据分叉。 |
| memo 前缀 `stable` 的 `is_l0` | `classifier/mod.rs:1047` | 增量缓存跳前缀的安全界（L0 有 segments 证书、L1+ 保守取 `dirty_from`），不进判定。 |
| 结构长度键 `struct_len` 的 `is_l0` | `classifier/mod.rs:1481` | memo key 选择（L0 用 `segments.len()`、L≥1 用 `units.len()`），不进判定。 |
| `w_is_l0` | `econ_positive.rs:4276` | walk-forward 报表分组标签。 |
| 全量↔增量成对件（`compose_level`↔`_resume`、`decompose`↔`_resume`、`compute_macd`↔`MacdState`、`bi_engine` checkpoint、`segment_layers.rs`、`tower_cache.rs`、`level_view_store.rs`、`retrace_ledger/`、`bsp_bridge`、`center_lifecycle.rs`、`core/recursion/*_engine.py`、`spiral/ffi.rs`/`fugue_v3/ffi.rs` 透传） | 各自文件 | 判据函数复用同一份，多处自述并有测试守 bit-exact；`spiral`/`fugue_v3` **零自有中枢/走势/笔/段定义**。折为一行计。 |

**不适用合计 19 条。**

---

## 五、判不了的（090 照实，不写成「不受影响」）

1. **总缝规则的判别式要求「同输入同输出对拍」，而本仓跑不出这个证据。**
   `AGENTS.md` 已明写本条载体只能是测试锁，前置是「先让 CI 跑 `cargo test`」——而
   `.github/workflows/` 全仓 `cargo test` 零命中（[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 3.1 已实测）。
   **本报告的总缝判定全部是源码语义推断，不是对拍实测。** V6、V3 的「同输入输出必不相同」是
   从判据量纲不同推出的（振幅×组件数 与 MACD 面积不同量纲，不可能逐位相同），这一步可靠；
   但**没有一条是跑出来的**。真要定性生效，须补对拍。
2. **V2（`Cand^δ` 恒真档）的性质判不死。** 反方辩护（限定词①「不同判断」）在技术上站得住，
   但它没有被写成举证书，而 `nest.rs:155-157` 又自陈该谓词无定义式——**在定义式补出来之前，
   无法判定 Type2/3 分支到底是「另一个判断」还是「同一个判断被架空」。** 现按违规记，
   由区间套/背驰概念票复核。
3. **Lean 侧口径与生产口径的分歧，两条通则都不直接管辖**（通则管生产判定路径），
   但按 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 权威分层，Lean 管边界、
   Rust-only 裁定属「裁错了层」。故本报告把 Z-2、B-1、B-3、G-1、G-3、S-1 的 Lean 分支
   **归入真待裁而非不适用**。这是一次判据推断，若 map 认为 Lean 应一律归「验证装置」，
   待裁数会从 34 降到约 28——**此处标出，供人拍。**
4. **`formal/Origin/BuySellPredicate.lean` 疑似编译不过**（[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 6.3 项 18 登记，
   `NestingCertificate.lean` 文件头逐字记载实测不通过，但它仍列在 `lakefile.toml` roots）。
   本票未跑 lake 复验，**不判真伪**，只登记它可能影响买卖点 Lean 侧口径的可信度。
5. **本报告未重跑 [#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 的全仓勘察。**
   口径清单、落点行号采信 #789；本票亲手复核的是与分类判定直接相关的 11 处源码
   （`center.rs:210/252`、36 处 `is_l0` 全表、`mod.rs:341/430/547`、`signal.rs:1514`、
   `recursive_t/divergence.rs:72`、`rust/src/divergence.rs:188/305`、`a_trendtype_v0.py:143`、
   `a_nested_divergence.py:118`、`econ_positive.rs:1030/1720`、`divergence.rs:540-600`、
   `a_segment_v1.py:431`，以及 `ref_v1`/`interval_necessity`/`cand_sub`/`bottomup`/`chi_bool` 的调用面 grep）。
   **未复核的部分若与 #789 记载不符，本报告的分类跟着错。**

---

## 六、给第二批概念票的建议（建议，不是裁定）

### 6.1 建议开 6 张概念票 + 2 张前置 task

次序按**教义依赖链**排（下游口径不能先于上游定），但第 1 张有额外的时限理由。

| 序 | 票 | 覆盖组 | 为什么排这个位置 | 必答项 |
|---|---|---|---|---|
| **0a** | **task：CI 接 `cargo test`** | — | `AGENTS.md` 已把它定为总缝规则下游实施链第一条动作；34 个 Lean↔Rust parity 断言现在只编译不执行，**任何教义裁定落盘后都没有看守**。 | 接上后 34 个断言的红绿实况；真实行情用例（现被 `-m "not slow"` 摘掉）要不要进 CI。 |
| **0b** | **task：6 条违规登记入票** | V1–V6 | [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 关票判据：**分歧只能存在于已开的票里，不能只存在于注释里**。V3–V6 是本票新判，现在只存在于本报告。 | 每条：违规成立/不成立的复核结论；归哪张概念票的受影响代码清单。 |
| **1** | **中枢概念票** | Z-1 … Z-7（7 组）+ E1 | 中枢是走势类型、背驰 A/C 段划分、买卖点、区间套的共同输入，不定它下面全悬空；**且唯一一条例外候选 E1 的举证书要它来补退场条件**，拖着就是让全仓唯一的例外长期不合规。 | ①核心区间取全三段还是第 1、3 段（`native_decide` 反例已在）；②单点核心 `ZD==ZG`——**并明确这是 Lean 管的边界，不许再只裁 Rust**；③方向交替要不要（[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 记账）；④E1 的退场条件是什么、正本落 `.chanlun/definitions/中枢.md`；⑤受影响代码清单点名到行号（Rust/Lean/Python 三侧）。 |
| **2** | **笔 + 线段概念票**（合并 1 张） | S-1 … S-4、G-1 … G-5（9 组） | 笔/段是中枢的输入，理论上应最先；但它们的分歧后果**局部**（段数差量级），而中枢的分歧后果**全局**，故排第 2。合并成一张是因为 G-1/G-3 的争点直接依赖 S-1 的笔集合，拆开会来回踢皮球。 | ①「至少 3 根独立 K」算哪个量（三种）；②线段构造范式定哪一种——**并回答 `Pipeline.lean:95` 用朴素切分这条断链怎么处置**；③第二特征序列由同向笔还是反向剩余元素构成；④旧笔、`extend_mode`、扫描窗口三处配置档留不留（涉及 D-5 同型的灰区，建议与第 3 张同口径处理）。 |
| **3** | **背驰概念票** | D-1 … D-5（5 组） | 6 条违规里 4 条落在力度上，是通则违规最集中的一处；且背驰是一类买卖点与区间套的共同前件。 | ①力度是什么（五选一或分场景）；②`Extreme` 要不要（[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十③ 记账）；③盘整背驰比较哪两段；④**D-5 的灰区裁一刀：可配置多档判据能不能常驻生产门**——这一刀会同时决定 G-4、S-2 的处置；⑤V1/V3/V4/V6 四条违规各自收敛到哪一套。 |
| **4** | **走势类型 + 级别概念票** | M-1 … M-4（4 组） | 依赖中枢（第 1 张）已定；且 M-3「级别是递归级别还是 persistence 簇」直通 map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787) 待裁的统一递归算子 T。 | ①盘整是什么；②同向判据（核心分离/外缘分离/双升双降）；③**级别的定义——递归还是涌现簇**，并回答 `a_ph_zhongshu` 的「同级」算不算缠论级别；④小转大的教义位置（全仓一处实现的空洞怎么补）。 |
| **5** | **买卖点概念票** | B-1 … B-3（3 组） | 依赖背驰（第 3 张）与中枢（第 1 张）。 | ①一类点判据含不含背驰、比值阈值合不合法；②二类锚是不是必须靠次级别第一类——**并回答 Lean 侧明文拒绝次级别递归与生产走定律一的正面冲突**；③三类点 `firstRetrace` 在不在判据；④可直接确认的既定：三类点回抽用 ZG/ZD 严格 `>`（全仓 5 处一致，[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 已核）。 |
| **6** | **区间套概念票**（含算子 N） | N-1 … N-6（6 组） | 排最后不是因为不重要——**它是分歧最深、全仓零对拍的一个**，而是因为它的每一条都以「中枢/背驰/买卖点已定」为前提（`Cand^δ` 引用背驰，rung 引用中枢，停止级引用买卖点级别）。前面不定，这张只能空转。 | ①坐标系定哪一个（时间/source_index/bar/纯蕴含）；②下降到哪一级停 + partial chain 合不合法；③`Cand^δ_ℓ` 的定义式（**必须写出来，这是空洞不是分歧**）；④`n_delta` 与 `chi_bool` 收敛到哪一套；⑤`src/newchan/nesting/` 的横向算子改名还是并入——同名异物必须消灭一个；⑥Lean 两套骨架合不合并。 |

### 6.2 三条给拍板人的提示

1. **例外余量只剩 2。** 若第 1 张（中枢）除 E1 外再开例外、第 3 张（背驰）给 V3/V6 开例外，
   上限 3 条当场用光，之后任何级别分叉都只能走强版（把缺的维度上载进数据类型）。
   建议在第 1 张票里就把「例外还剩几条」写进裁定正文。
2. **0a（CI 接 `cargo test`）建议与第 1 张并行开，不要串在后面。** 概念票的产出是文字正本 +
   受影响代码清单，落地要靠对拍锁；锁的执行腿现在是断的。
3. **本报告第五节 3 那条口径请先拍**：Lean 分支算「真待裁」还是「验证装置不适用」。
   这一刀决定第二批的待裁面是 34 组还是约 28 组，也决定第 1/5 张票要不要带 Lean 侧动作。

---

*勘察树：worktree `agent-a6faa36a91f9a7dcf`，HEAD 已对齐 `docs/grill-with-docs-entry`（`dac8ff9fa1`，
含 #793/#799/#804 三节教义正本）。该 worktree 初始 HEAD 是 `19b4015927`（2026-04-20 的旧线，
`rust/`/`formal/` 尚未入仓），已 `git reset --hard` 到教义正本 tip 后勘察——原 HEAD 经
`git branch --contains` 核实同时可达自 `research/i252-doctrine-expectation` 等多条分支，无提交丢失。
本 worktree 内除本文件外零改动。*
