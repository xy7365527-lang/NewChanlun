# 固定 Rust 映射与未完成的具名定理

工作草稿，绑定 #1419。固定545e与112e的 `types.rs / parser/feature_seq.rs / parser/second_fold.rs` 逐字相同，hash见 `RUST-IDENTITY.json`。以下行号均指这两棵树和当前未改的Rust。Lean 定义位于 `Origin/SegmentFeatureSeq.lean` 的 `SecondFold`；完整当前行号见 `DECLARATIONS.json`。

## 实际映射

| Rust固定字段/控制流 | Lean具体定义 | 已落证明体及边界 |
|---|---|---|
| types.rs:92 Stroke：direction、start_index、end_index、start_price、end_price | 直接复用Origin.Stroke五字段；Phase2.rawDirection/rawInterval/rawStroke | `ofStroke_rust_endpoints`逐值等于feature_seq.rs:201 stroke_high_low的max/min分支；尚无i64/usize机器编码桥 |
| feature_seq.rs:100 FeatureElement high/low/stroke_idx/stroke_indices | Element.range/strokeIdx/members | members按时间正序；strokeIdx是输入**笔数组**下标，不是Stroke.startIndex的K线索引 |
| second_fold.rs:42 SecondCandidateId seg_dir/b_stroke/candidate_extreme；:118 from；:129初始方向 | Candidate.segDir/bStroke/extreme；initial | P通过固定Candidate传入而不改写；Up初始Down，Down初始None；P来自主序列候选的合法性未证 |
| :190 next_index、:192 consumed_len、:195 through、:196反向跳过 | consume/step；indexedProjection | `step_position`、`step_reverse`；被过滤笔仍占一个预算单位；`run_undecided_consumed/raw`从实际原始列表构造消费量和raw元素 |
| :199 stroke_high_low、:201 reach、:205水位 | FeatureElem.ofStroke、reach、updateWatermark | `consumeFeature_watermark`、up/down_bound；使用合并前原始极值，单步上/下界、step/run保持及全消费前缀原始极值折叠有证明体 |
| feature_seq.rs:289 apply_inclusion、:301 effective_up | includeElem、effectiveUp、mergeInclusion | merge/append/direction/members定理；None和Up高高、Down低低；非包含时严格双端上升/下降更新方向 |
| second_fold.rs:230合并成员，:240最新stroke_idx | includeElem合并结构 | 末元素价格取方向合并，成员拼接、最新下标替换；余下标准元素原样保留 |
| :252末三元素、:260顶/底严格比较 | tailFractal/tailTriple | `tailFractal_iff`复用IsTopFractal/IsBottomFractal，不接任意oracle |
| :266任一分型→:267matched/:268Confirmed/:269return | consumeFeature第一分支 | `consumeFeature_confirmed_first`；即使本笔也越P仍确认，连续交替正例same_stroke_confirmation_priority |
| :274原始reach比较、:279SecondKill | breached、Kill、consumeFeature第二分支 | `consumeFeature_killed/open`；merged_extreme_still_kills锁住P10/原始12/合并high8，members=[1,3] |
| :324 while !decided与step | step守卫、run | `run_append/run_decided/run_terminal_prefix`；仅对不可改写的列表追加，不声称实现FNV/撤销/恢复 |
| :294 budget_end、:330 window_grant | evaluate drop(b+1)、take预算；None无限 | 有限预算单位是全部原始笔；机器饱和加法桥尚缺 |
| :302 finish语义与覆盖 | finish、Coverage、Result | resource_not_open、open_iff、decided_complete；预算不足时None，不把资源耗尽塞进三态 |
| feature_seq.rs:151 SecondTerminal到Claim10静态三态 | Phase2.secondFoldTerminal/secondFoldPrefix | finish资源为空、前缀OPEN iff；Phase2.second_fold_input_projection把第二序列接到既有featureElements |

Lean标准/原始列表按**最新元素在前**存放；对应Rust Vec须反转。members不反转，matched三元组仍按(a,b,c)顺序。模型逐步比较这一变换后的全部已建模字段。

输入域仍有真实义务：Origin价格是Int、下标是Nat；Rust价格是i64、下标是usize。端点取极值不产生价格算术溢出，但下标加法/预算饱和需要范围证明。输入应来自合法解析器、方向字段与端点走势一致、连续、b下标有效；本轮分类函数能计算任意结构记录，不把这种总定义当成其来源已证。

`run_initial_watermark`另给出包括终态提前停止的全消费前缀水位＝已消费原始元素的方向极值折叠；不是标准元素极值，也不带0初值。该计算不变量已落源码，仍待Lean类型检查。

## 未完成的具名定理（保留原 D_SECOND_FOLD_REFINEMENT）

以下名字是**待证明义务标识**，没有在Lean中创建占位声明，没有添加假设/公理替它们通过。

| 义务名 | 精确源字段/行为 | 当前缺少的证明、失败例或不可外推例 |
|---|---|---|
| D_SECOND_MACHINE_ENCODING / rust_stroke_decode_preserves_step | types.rs:15 Tick=i64、:92 Stroke；second_fold.rs:118饱和b+1、:145 next_index、:294预算饱和 | 缺i64/usize→Int/Nat编码关系及可达范围保持。b=usize::MAX时Rust饱和b+1与Nat b+1不同；该值不是有效候选，必须证明生产候选域排除它，不能默认全Nat等价 |
| D_SECOND_VEC_SIMULATION / rust_step_refines_consume | second_fold.rs:72 State.elements 与 facts.standard双存储；:189 step，feature_seq.rs:289包含 | 已给具体逆序实现及局部定理，但未建立Rust操作语义→Lean完整State的模拟关系；尤其需证明elements和standard逐元素一致、pop/last可用、索引不越界。包含例[2,8]后[1,12]应raw长度2/standard长度1，不能逐笔直接当标准元素 |
| D_SECOND_STANDARD_PREFIX / inclusion_preserves_reachable_standard_and_tail_scan | feature_seq.rs:289 apply_inclusion、second_fold.rs:252只查尾三 | 缺从initial可达状态的全局标准化不变量及“此前无分型→只查新尾足够”的归纳桥。含合并的原始三元素不能直接喂FractalInSeq；例如上表合并例只有一个标准元素。局部tailFractal_iff并不证明整段扫描等价 |
| D_SECOND_FACTS_TRACE / step_preserves_complete_facts | feature_seq.rs:107 FeatureStep、:136 SecondCheck；second_fold.rs:244 steps、:265 checks、:267 matched、:279 killed_by | Lean仅存raw/standard与最小matched三元组、kill价格/下标/P；未存steps/checks全历史，matched的top/bottom/through和kill.comparison未逐字段证明重建。预算不足后持久事实必须逐位相同，几何同值不足以抵扣此义务 |
| D_SECOND_DIGEST / digest_refines_sealed_input_identity | second_fold.rs:91 digest_stroke、:193 consumed_digest | 未实现FNV-1a 64的五字段字节编码/环绕算术及其与恢复检查的关系。只改Stroke.end_index而价格不变，当前几何投影可能完全相同，Rust摘要会变；不能以几何相同代输入相同，更不能假定FNV无碰撞 |
| D_SECOND_BUDGET_GRANT / grant_resume_equals_uninterrupted | second_fold.rs:366 grant、:421 budget_end、:424封存while | run_append只证同一计算投影的列表分批，不证明累计预算、extra=0转无限、饱和累加或resume最终facts相等。预算1、后缀2笔例应先None后OPEN，不能把第一次资源态缓存成语义终态 |
| D_SECOND_TAIL_RECONCILE / unsealed_tail_recompute_and_one_step_undo | second_fold.rs:149 capture、:166 restore、:374 reconcile、:429末笔暂算撤销 | 未建FoldUndo/封存边界/一步回退、两步改写拒绝证明。末笔2→9改成2→12时同长度原输入结果OPEN→KILLED；这不是xs++ys，不满足run_append前件。MODEL-RESULTS保留此负控 |
| D_SECOND_WIRE_RESTORE / from_wire_verified_refold_equivalence | second_fold.rs:574 from_wire、:616 wire_match与恢复probe；:410版本拒绝 | 缺schema/rule/policy/候选身份/预算/消费摘要/标准成员/水位/终态/原因等完整线形对应与恢复归纳。若wire中标准成员[1,3]被改成[3]，几何仍同但不得当作已核恢复；本轮未触碰wire_match或执行恢复 |

这八项都源于**已经固定存在的接口**。留下它们是如实列出本轮未完成证明，不是将接口未定当豁免，也不批准换判据、缩小原义务或另开执行链。

## 第一种、其他域与执行欠项

`D_FIRST_RAW_SOURCE_BRIDGE`：rawTurnWindowState确实读取给定完整原始列表，P2原反例已锁；尚需从真实FeatureSeqState候选身份/笔切片证明a、b、P及其后所有已知笔无遗漏地映射到该输入。不能仅传projectStrokes然后再次自称完整原始前缀。

`D_FIRST_RUNTIME_BINDING`：feature_seq.rs:233严格分型、:550先试后合并、:670调用仍在。反例a=[0,8],b=[2,10],c=[3,9]与c=[1,10]已在v4独审查明，v5未改Rust。归#1493，按管理者给定状态仍PREPARATION_ONLY，#1405/#1420 OPEN。

`E_REFERENCE_FIXTURE_BINDING`、`F_EXPORT_EXECUTION/F_INDEPENDENT_ORIGIN`沿用旧缺项，不改fixture、不重跑C/E/F模型。`LEAN_TYPECHECK`（包括新import的7模块闭包）、`FINAL_SAME_TREE_PARITY`、真实产品/恢复仍NOT_RUN。本轮新增证明体需独审与获准环境中的真实Lean检查；源码存在不构成已通过证明核的证据。
