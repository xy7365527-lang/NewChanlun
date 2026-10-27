# #1488 C：来源与历史消费面迁移方案

日期：2026-10-17。工作树：`.sandcastle/worktrees/resume-1488`。性质：只读源码调查与后续派工方案；没有修改生产源码，没有编译或执行测试，**不是 #1488 或六成功验收**。

## 范围与已核接口

本工位负责逐文件准备 `source_audit.rs`、`source_audit_oracle.rs`、`source_prefix.rs`、`source_contributions.rs`、`source_pending.rs`、`source_pending_oracle.rs`、`source_delta.rs`、`source_order.rs`、`cross_request_source_cache.rs`、`history.rs`。本轮只写此报告。已读根 CLAUDE.md、Serena manual、RAII 迁移方案、A 实现报告、raw 消费清单；图中恢复树新符号返回零，随后精读这些文件。行号为本轮读取快照，其他工位并行修改后须按符号定位。

已向 `migrate_1488_plain_owners` 核对其接口：

- `TextValue::Plain(NativeValue<String>)`；`native_string/native_format -> NativeValue<String>`。
- `RefValue::Plain(NativeValue<ContentRef>)`，无深 `Clone`；`native_plain_ref -> NativeValue<ContentRef>`；`native_ref_clone -> RefValue`，从当前 scope 独立准入。
- `RefFields::Plain(NativeRefFields)`；`get -> Option<RefView>` 为借用；需要拥有的调用点用 `get_admitted(s, key) -> Result<Option<RefValue>>`。
- `JsonOwner::Plain(PlainJson)`；`native_json_clone -> PlainJson`；只读 Value 及原 permit，不提供 take/into_inner。`parse_rendered_plain -> Result<PlainJson, String>`；`JsonOwner.view()` 保留。
- `TextValue::from`、`RefValue::from`、`PlainJson::from(Value)`只供原来无 scope 的普通/fixture 值，不能把生产未持费分配转入这些口规避准入。
- Vec 用 `CodecVec` 与 `native_codec_room`，散列表用 `BusinessMap` 与 `native_map_room`；顺序树用 `NativeTree`。不另造第二套 NativeValue。

## 必须先解决的接口缺口

1. 当前 `audit_store::NativeTree`只有只读 Deref、insert_admitted/remove/clear，没有 get_mut/remove_entry/有序消费出口。`source_contributions::Map`需要受控 mutate 与 remove_entry，Tree 需要有序集合迭代。应由 B 工位统一补**不会偷扩容的受控方法**；不得加通用 DerefMut 或裸树 into_inner。树高水位许可在真正 clear/drop 前保留；remove 不代表物理树容量立即释放。
2. 三类 Rc 分配不能靠元素有 NativeValue 自动闭合：`SourceCoords::Shared(Rc<[i64]>)`、`Rc<Object>`、`Rc<RefCell<Node>>`。应由 A/B 提供经原 reserve 点构造的共享 allocation owner（或批准的局部等价封装），token 放在共享 allocation 内，最后一个强引用析构载荷后释放；浅 Rc clone 不重收费，弱引用不得提前退费。包括 token/控制块本身实际布局；不能用 `NativeValue<Rc<T>>`的外部单 owner 使裸 Rc clone 逃逸。保持 slice/对象/节点的借用语义。
3. 库 `into_counts_original_subset_admitted_for_testonly`当前接收裸组 Vec/ContentRef。缓存组改型后需要该库接口接收持费集合或借用迭代，不能把 CodecVec/NativeValue 拆回裸值。属于 A/缓存交接工位共同接口，当前 C 文件单独无法完成。
4. `NodeSource::checked_read`要求裸 CheckedFrame，`HashReader::checked_read_scoped`已返回 ScopedFrame。生产应沿 scoped 链借用或用 into_native 持有；不能简单把 trait 方法返回类型改掉。旧 `read -> NodeRead`的 body 同理，需要列出真实生产调用并在库边界解决，不能用 scoped 方法存在就宣布兼容 retain 消失。

## 逐文件方案

### source_audit.rs

- `NativeFieldRefKey`（约 95）：format/root 改持费字符串，构造器接当前 CodecRead；删除 `native_admit(length)`后 `.into()`的分离收费路径。Hash/Eq 仍按四字段和 digest，不能缩键。去掉导致无准入深拷贝的 derive Clone。
- `NativeScalarFields.values`、`NativeFieldMemo.fields`（约 117–137）换 BusinessMap。scalar 值保存 PlainJson；命中通过同当前 scope 的 native_json_clone 返回独立 owner，不能借缓存值转为拥有值。字段 key 的名字序列用 CodecVec<NativeValue<String>>，保留顺序与重复项。
- `ArrayAuditSource::decoded_scalar/lookup_subset_ref`（约 440–610）是关键：原 `fields.clone()`及 collect/cloned 路径改为逐项受准入复制。memo 中字段用 NativeRefFields 或等价持费字段 owner；仅需检查时借 RefView，需要返回独立拥有结果时逐字段同父复制。NativeRecordFields.subset 的类型交接由 D 工位负责。
- `SourceFields::get`改借用 RefView；有拥有要求的调用用已有 get_admitted。不能通过对 RefView 调 clone 得到新拥有数据。
- `ArrayAuditSource`的 parse_rendered_plain 签名跟 B；非 Owned lookup/content_ref/field/array 路由全部用 A 的 typed native API，保持原 full_arrays/deadline 资格分支和完整闭包验证。外面包 NativeValue 不会恢复底层已 retain 的许可。
- `NativeWitnessIdentityMemo`（约 1080–1180）：key.object_id、值 want 持费，entries BusinessMap；key 函数接 admission，删除 native_admit+to_owned 分离路线；成功 stored ID 比较后才入缓存，overlay 不获缓存资格。
- `NativeRawLedger.originals`（约 1337）从 std HashMap 换 BusinessMap，保留原 ordinal/raw winner 键和源哈希路径；sources 已 BusinessMap 不重复签账。RetainedDelta 的容量统计跟实际类型更新。
- `highs/lows`空字符串数组（1867/1868）用无分配空 owner 或允许的空构造；真实读取仍 native_text。测试 fixture RefValue/JsonOwner 构造按新类型适配；测试不得用进入真实 scope 后裸 Value clone 冒充受准入结果。
- `Cc004Producer::new`前 `native_admit`（约 2168）仍是临时 profile/rule 字符串分配：需让 producer 借用或将其整个实际分配的 token 保持到 producer drop，不能简单删除。producer 文件不归 C，需接口工位协同。

### source_audit_oracle.rs

仅 cfg(test) 的冻结 oracle，保留循环次序、错误、判断、计数与源版本。跟随 SourceFields/RefValue/TextValue/PlainJson 类型和相同准入口；约 335/337 空值初始化适配。其 `Cc004Producer`临时收费同上。不能把 oracle 改为调用被验的新增量核，不能为省迁移删除完整对照路径。

### source_prefix.rs

- `NativeSourceMemo`的 intervals/coords/references/intern（约 410）都是持久 std HashMap，应换 BusinessMap；intervals 的每个 Vec<Interval>改 CodecVec。winner_intervals_admitted 的临时 identity_coords/coord_identities/winners（约 455–545）改持费树，借用 raw 的 &str 不另造字符串，不换身份/坐标/赢家判据。
- `share`（约 550–575）同时存活旧 CodecVec 和新共享坐标，必须先为新 Rc allocation 在**同一个父账**准入，再复制，随后旧 owner 自然 drop。intern 的 key/value 与各 SourceCoords 共享同一 allocation，不能按引用数重复收费，也不能 memo.clear 时在仍存活 SourceCoords 前提前退费。
- `SourceCoords::Shared`与 Proof.values 接统一共享 owner；保留 Rc::ptr_eq 所表达的同源共享语义，Original 仍 CodecVec。native_admit(layout.size())退场须与共享 token 真正接入同时发生。
- 保留 intervals_ready/None 的现有资格含义、ParseDomain/numeric 键、完整原件校验和前缀区间。None 是既有“不可复用证书”分支，不因迁移增加失败兜底。
- 当前测试 `native_prefix_certificates_and_shared_coords_use_current_parent_scope`明确断言 drop 后旧保守费用仍保留（约 113–124）；此断言是待退役的旧收费行为，应更新为真实剩余 owners/容器容量的准确预期，而不删除同父和共享正控。

### source_contributions.rs

- `Map<K,V>`/`Tree<K>`（约 76–244）底层换 NativeTree；保留原 BTree 顺序和对 key replacement 的语义。`bytes`及 RetainedDelta 是诊断/历史保留账，不是释放真实 token 的指令。Map.put/remove/mutate 应随真实移入/移出 owner 变化，返回的旧 value 在调用者 drop 前仍持费。
- `owned -> NativeValue<String>`，`vec_push`用 CodecVec。NativeSourceState 的 rule、ids/shapes/shape_slots/missing_shapes/shape_witnesses/slots/step_subjects/request_keys/request_subjects 等 String 键，Witness.id、Request.key/subject，均改持费文本；Borrow<str>维持现有查找。元组键借用若原调用依赖 String 类型，需显式借视图比较，禁止生成无费临时 String。
- Rc<Object>的控制块分配（约 1055）与 SourceCoords共享问题同级；Object payload 自带 token 不能替代 Rc 控制块。Object 为多个 entry/StepContribution 引用时，只在最后引用消失释放 payload/控制块。
- SourceFields retained_bytes 接 NativeRefFields 的真实容量统计；SourceCoords 字节统计改跟新共享 owner实际布局。不要把 Rc 多个索引中的逻辑 heap_bytes 求和当父账退款。
- `audit_native_bound`（1769起）仍只能接 SourceMaterial 的当前 WorkScope；previous NativeSourceState 的原 token 随 move 带入，不新建父 Budget/不重新预付其全保留费。dirty trees/Changes 临时向量销毁时退本次费用，增量 reverse dependency、错误时丢 pending、不full-audit fallback均保持。

### source_pending.rs

StepContribution 已按值持 fields/key/reasons并共享 Object；这些字段随基础类型迁移自然持费，关键是共享 Object 的实际 owner。只读 StepView继续借用；payload/nulls/strings等读取用 RefView 或 get_admitted明确区分。已有 CodecVec/BusinessMap+native_codec_room/native_map_room复用，不再包第二个 token。请求键、raw bar、等待原因、判据、错误顺序不得改变。

### source_pending_oracle.rs

仅 cfg(test) 冻结核；绝大多数容器已 BusinessMap/CodecVec。跟 B 的 TextValue与RefValue视图即可；保留全部 steps/prefix/pending/requests/bound分支及完整对照，不能改为调用新 NativeSourceState。

### source_delta.rs

仅 cfg(test) 的 FixtureCut/Contributions，已大量使用 BusinessMap/CodecVec。移出 original 用 native_ref_clone当前 scope（约 281/489），不恢复 RefValue深Clone。测试中 raw serde_json::Value的输入构造 clone 可保留，和真实 scope 下的 owner复制区分。FixtureCut 不新增生产提升能力；Pending.next_fixture保持仅测试。必须仍能对拍完整 source oracle，不能因迁移合并两实现。

### source_order.rs

Order.nodes 的 BTreeMap换 NativeTree；Node的 Rc控制块持独立真实许可，child/parent浅克隆保留图结构不收费第二次。insert两段现有 native_tree_room/native_admit（约 252–255）改原位签发到树与共享 Node；错误须发生于树状态修改前，不能在 parent已改后才准入。remove只改 live并脱树；reclaim移除索引后若仍有 Link借用，allocation继续持费；树节点高水位费不能按一个 handle 删除推测归还。AVL旋转/顺序/handle/复杂度保持，不能改为每轮全排序。

### history.rs

- NativeCutProjection.batch/binding（38–42）、native_cut_binding（57–114）：batch→持费文本，binding→CodecVec<持费文本>；check_scope比较用切片/借字符串。每次重建比较 binding 的临时副本与原 binding同时计费，比较后自动drop；不可改成摘要弱化完整绑定。
- public_array临时 Vec<ValueView>（约 162）改 CodecVec；仅借 nodes无需复制 NodeDigest 所属对象。
- PrefixArray.stack（1762）改CodecVec，admit_append保当前 scope；append/truncate/drop后 backing capacity仍持费，不按栈长度退费。
- `HashReader::parse_rendered_plain`（2713）必须直接返回 PlainJson，走 B 的 strict_json_owner_admitted(..., self.codec_work())。**原 native.reserve(...).commit(0, heap_bytes) 与 scope.reserve_native(...).retain 两条旧路径一并替换**，不能保旧收费再包新 owner。保 strict_json词法/重复键/错误合同和解析峰先准入。
- HashReader非Owned lookup_ref/fields/array/content_ref等全接typed API，不能先拿旧裸返回再收费复制。与SourceFieldStore adapter使用同一父scope。
- checked_read_scoped已是正确起点；checked_read中的 ScopedFrame::into_frame（2590）和read中的retain（约2555–2570）需A边界协同查生产入口。不能单文件声称去掉全部frame/body泄留。
- OverlayOwner::Dynamic及其put也在本文件，必须在实施时覆盖：其BTree索引/Vec body与旧内容替换是另一真实allocation，不因返回frame迁移自动受管。Bounded Owned路径保留既有域。

### cross_request_source_cache.rs

- Binding.rows、Trial/PendingPool/Pool.proofs改CodecVec；ExactRows内层与SqlCell由D工位负责，保持六份SQL原件、列/行顺序、未知meta键负依赖。
- Key.state/profile/rule改typed持费，去derive Clone；matches_cut使用借用。proof copies通过同当前scope准入，Counts纯数值clone不额外堆费。
- build_bounded_counts_admitted中groups（约278）持费集合与引用，需上述库边界协同。保持逆序整组选择、恢复代序、原S7口径。
- proofs compact（约300–320）原permit.retain不能保留。用CodecVec受准入重建，先全新buffer同父签费，移动元素，旧buffer真实销毁才退旧费。排序scratch permit原局部drop正确，元素变大后size_of按新Proof实际类型。
- Trial→PendingPool→Pool→Keeper移动所有原token；prior/next两池同时活时同父同时计费，finish/reject/clear/drop按实际释放。不能保留旧pool只在S7统计减费用而让父token脱离，不能把新请求临时budget当旧pool持费的父账。
- optional cache失败不得升级为业务失败；取消缓存不能吞原SQL/资源错误。source_cost逻辑S7费、capacity统计、真实父live三者不能混成一个数。

## 费用交接与主控集成点

本范围直接可见旧收费是 native_admit/native_std_map_room/native_vec_room/native_tree_room、history JSON commit/retain、cache compact retain。每个迁移须从这些**原构造点**移交 token；消费者端补新token会双计，简单删旧收费会漏计。

`RetainedDelta::release_bytes`与Map.bytes是普通统计，并非 native::Budget.release；不能见“release”就删除。反过来真正退款不能从retained_bytes统计推断。本轮在C文件中未发现直接Budget.release调用；主控D/owner工位需检查audit_owner约2089的body_capacity及2241的row_ids_charge，在相关载荷改为RAII时移除对应旧退费，避免双退。

另：audit_owner::with_bound_raw_advance约1306仍能看到新建Budget并把previous.retained_bytes作为entry charge的旧路径；是否仍生产可达由owner工位核实。本C计划不修改它，也不把文档的同父设计当作该路径已经完成。跨代source previous、同publication raw/runtime/material/tail、跨request source Pool均必须核对原父身份；仅字段换型无法证明。

## 建议后续派工边界

1. **C1来源值/缓存**独占 source_audit、source_audit_oracle、source_prefix；等待B typed接口及共享owner。与D native_record_fields只协商返回类型，不改其文件。
2. **C2增量贡献/顺序**独占 source_contributions、source_order、source_pending、source_pending_oracle、source_delta；等待B NativeTree受控变更接口及共享owner。不改construction_state或native禁区。
3. **C3历史/跨请求池**独占history、cross_request_source_cache；协调D request_audit的ExactRows、A codec subset/frame API。此包尤其需要owner主控审查跨request父账，不能凭C1/C2通过自行验收。
4. A/B新增共用API由原owner独占；D/owner负责candidate旧收费交接。主控合并时对这四接口做一次完整调用链审查；不要让C工位各造一套共享owner。

## 最小必要验证（本轮均未执行）

- plain无scope、Native有scope、Owned既有域三路保相同业务结果/错误；frozen source/pending oracle仍独立；现有增量路径调用计数保持，不能用full audit回退。
- memo miss/hit/clear，shared coords/Object/node先后drop：一份allocation计一次，最后owner消失才退；clear仍保实际backing capacity，drop后归基线。
- NativeSourceState跨代move、删除/重插、错误丢弃：旧/新共同存活峰覆盖，删除后不得残留永久累计费；原判据和逆依赖仍执行。
- History JSON同父解析成功/失败峰，替换掉commit+retain的重复计费；bindings不弱化。
- 两池交接及裁剪失败/取消：旧池、新池、ExactRows/RefValue/组集合真实持费，失败不产生业务授权。
- 最后才由主控安排具名远端六成功验证，16/8 MiB、C>L/2、peak等于完整帽及尾检禁COMMIT正负控保持。单文件rustfmt/静态阅读不替编译、更不替完整验收。

本轮无cargo/rustc、服务/HTTP/浏览器、外盘、重型回放、commit/push/dispatch或关票。
