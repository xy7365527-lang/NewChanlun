# #1488 D：索引、控制与 owner 消费面迁移准备

日期：2026-10-17。票型：复用 #1488 实施票下的只读实施准备。恢复树：`.sandcastle/worktrees/resume-1488`。状态：**源码只读计划，未实施、未编译、未运行 Rust 测试，不构成验收**。

已读根 CLAUDE.md、A 实施报告、RAII 方案、raw 消费清单，向 B2 工位核实新接口。图工具对新 Native 符号返回零，因恢复树未被图覆盖，回退精读现文件。没有 cargo/rustc、服务、HTTP、外盘、重型回放；没有修改三个禁区或其他源码。

## 结论与实施顺序

D 可以拆成三个无文件重叠的写入包，但 owner/SQL 包必须最后接入：

1. **D1 索引**：object_index.rs、aux_index.rs、native_record_fields.rs、catalog_evidence.rs；只接 B2 通用接口与 A 容器。所有真实拥有值用 token owner，比较、查询、输出遍历借用。
2. **D2 控制兼容**：control_audit.rs、v2/mod.rs、v2/evidence_store.rs、q_runtime.rs、q_http_input.rs。优先消除接口类型不匹配，随后处理 Arc/parse 的真实持费；不得把无 scope 兼容构造当作 Native 准入。
3. **D3 SQL 与发布所有权**：request_audit.rs、audit_owner.rs、audit_owner/effects.rs、construction_set.rs、v2/content_store.rs。ExactRows 的 row/body move 与原 release 同时迁移；一条路径完成后再交接，不允许两边各改半套。

**尚有明确的库接口前置**：NativeValue 字节到 String 的原 token move、NativeRefFields 从已持费项构造/子集复制、CodecVec 的 retain/insert 等所需受控操作、带许可 Arc 共享/捕获原件接管。不能用 `.to_owned()`、裸 `.clone()` 或新 `into_inner` 绕过。

## 冻结接口（来自 B2 工位当前确认）

| 类型/方法 | 新合同 | D 用法 |
|---|---|---|
| TextValue::Plain | `NativeValue<String>` | `as_str()` 借用；深复制用 `try_clone_admitted(work)` |
| RefValue::Plain | `NativeValue<ContentRef>`，删除无条件深 Clone | `view()` 借用；`s.native_ref_clone(&r)` 独立复制 |
| RefFields::Plain | `NativeRefFields` | `get` 变 `Option<RefView>`；需持有则 `get_admitted(s,key)` |
| native_string/native_format | 返回 `NativeValue<String>` | 保存 owner，使用 `as_ref()/as_str()` |
| native_plain_ref | 返回 `NativeValue<ContentRef>` | 保存 owner，转 RefView 借用 |
| JsonOwner::Plain | `PlainJson` | `view()/as_ref()` 只读；无 take/into_inner |
| native_json_clone、parse_rendered_plain | 返回 `PlainJson` | 需要子项拥有权时在当前 scope 复制子项；不移出裸 Value |
| CodecVec/BusinessMap | 原容器私有字段携 Native token | reserve 用 `native_codec_room/native_map_room` |
| NativeTree | `audit_store.rs` 已有原 token owner | `insert_admitted`；不直接 `.entry()` 修改底层树 |
| ScopedFrame::into_native | 返回 `NativeValue<CheckedFrame>` | 只在真正持久化/跨 scope 保存 frame 的地方消费；普通读继续 scoped |

A 的 `NativeValue` 不提供 Clone/DerefMut/into_inner。B2 的 `From<String/Value/ContentRef>` 仅适用于**原本就无 scope 的兼容输入**，不能把 Native 载荷拆出后再 From 回去。

## D1：索引逐文件

### object_index.rs

- `Entry`（约225）已有 TextValue/RefValue，保持 struct move；比较、SQL 参数借 `as_str()/view()`。`Entry::try_clone`（约5696）及 Slot/LifeOwner 的深复制需接当前 admission；不能保留 `.clone()` 裸复制 RefValue。测试纯无 scope 构造使用兼容 From。
- `NativeProjection.wires`（约712）、`OwnerProjection.wires`（约2558）改 `CodecVec<[Option<RefValue>;2]>`，新行容量在创建前签；`remember_wire` 先完成受准入新 clone 再替换旧 owner，失败保旧，替换时旧引用 token 自动 drop。缓存命中依旧执行 validate_life_view/verify_original，不能用 RAII 迁移删判据。
- `native_dual_at_cut`（约836–946）的 active/dead/births/template/public 临时 Vec 改 CodecVec。`pairs` BTreeMap 改 NativeTree，元素是两份有独立 token 的 RefValue；返回或 move 入输出时 token 随元素走，迭代器仅负责 buffer。
- load/restore/update 临时 wires/changed/rows（约1656、1865、2014、2135）改 CodecVec；Entry 原始载荷不经 serialize/deserialize 重建。
- `HistoryEvents`（约2255）：by_id 已是 BusinessMap；births/deaths 改 `NativeTree<i64, CodecVec<usize>>`；ranks/withdrawn_ranks 改 `CodecVec<OrderKey<NativeValue<String>>>`。不能只改字符串类型仍保留 OrderKey 的裸 Clone/admit 分离；新 `try_clone_admitted` 在原复制点签实际新字符串 token。
- `HistorySweep.active`、ProjectionSweep.active/withdrawn 的 BTreeSet 改 `NativeTree<Key, ()>` 或专门可审的 set owner。既有 `advance_admitted/seek_admitted` 先 admission 后由 `advance/seek` 裸 clone/insert 的两段式必须合成一次真实受准入操作，否则预付费与 owner 分离。相同 cut/正向/逆向 seek 的成员与排序语义保持。
- `HistoryEvents` 被两个游标 Arc 共享（约773、2430）。**目前 NativeTree 不能自动覆盖 Arc 控制块费用**：需要共享分配 owner 包在 Arc 内，末个 Arc 引用 drop 才退原 permit；不能两个外层 wrapper 各持一个 permit，也不能每次 Arc::clone 深收费。原事件 payload 保守统计口径是否重复计入仍由 Retained 统计合同决定。
- `RetainedDelta.release/add`（约733、1899）只是结构保留统计，不是 Budget.release，不能见 release 就删除。统计继续记录实际替换，父账以 token 为准。

### aux_index.rs

- Witness/Relation/Observation（约261/284/305）去 RefValue 无条件 Clone；文本和引用统一借用或 `try_clone_admitted`。`Index::try_clone/clone_in`（约343）须显式当前 scope，不从旧 scope 暗续约；复制的 CodecVec 要先 reserve，旧新两份同时收费。
- Runtime Cache 的 `insert_admitted`（约435）已能接 BusinessMap 原 token；命中返回借用，确需保存副本时 native_ref_clone/native_text。discard/clear 释放元素，表 buffer 若保留则不得假退容量费。
- `native_tables/NativeAuxFacts`（约1206–1278）：三 HashMap 改 `BusinessMap<NativeValue<String>,usize>`，三个 born 表改 `NativeTree<i64,CodecVec<usize>>`，raw_revisions 改 CodecVec；重复身份替换通过 move 新 key/drop 旧 key，不能按 len 推算退费。
- `relation_memo/raw_memo` 改 BusinessMap，RelationCore 内文本/引用跟随 owner；audit 失败清 memo 后仍可保留表容量，但元素应立即退费。
- update_from_owner（约1882）与 native_cut_members（约2105–2272）同步换新容器接口。born 桶的 `.entry().or_default()` 必须换先查/再受准入插入，不能借 DerefMut 逃逸。
- into_facts_retained（约1401）保留统计 delta；`facts` 按值 move 进长期 owner，memo 本次 drop，不把 memo 费用带入事实统计或永久 retain。

### native_record_fields.rs

这是一个独立、较小的优先垂直片。

- RecordKey（约8/170）format/root 改 NativeValue<String>，删除 derive(Clone)；lookup 先用借用身份 key，或采用原固定 digest/kind/extent 加完整 format/root 比较，不能省掉原完整四字段身份校验。
- records（约205）改 BusinessMap<RecordKey, NativeRefFields>，或 NativeTree 持已收费 `(key,ref)`，不要保留 `HashMap<_,BTreeMap<String,ContentRef>>` 裸 payload。
- seed（约225）以 `fields.get(name)` 的 RefView 借用核验，复制时每项单独在当前 scope 签费；成功后 move 整个 fields owner；替换旧项真实 drop 旧 owner。
- subset（约270）不能再返回 `CodecFields::from_plain(collect())`：改 `Option<NativeRefFields>`，在原核核完重复请求名/缺字段/同 capture 后逐项受准入复制。若调用者只即时消费可另加借用迭代器，不能让 lifetime 越过 records。
- clear（约308）释放字段/引用/键 token，保留真实 hash buffer token；现测试“clear 后 live 完全不变”是旧 retain 行为，应改为“项费减少、剩余 buffer 费非零，最终 drop 归零”，保持业务子集等价/overlay/capture 隔离断言。

### catalog_evidence.rs

- Native 主通路已有 field_ref/scalar_ref/text_ref/array_field_ref、AxisState/CodecVec，可直接保留 owner；count 应从 text_ref 返回 owner 的 as_str 借用解析。
- 旧 `field/scalar/text/normal_ref`（约80–136）分别返回裸 ContentRef/Value/String，不能直接解包新 Plain。把 Native 调用迁到 typed ref 辅助函数；纯无 scope 兼容函数只允许无 token 输入，或用单独普通入口在 None scope 构造。不能在任意 `impl CodecRead` 下静默深复制拿裸结果。
- parse_json_owned 的无 scope strict_json 分支（约862）用明确兼容 From；有 scope parse_rendered_plain 分支保持 B2 新 PlainJson，不丢 permit。
- reference_text（约264）目前 serde_json::to_value + canonical_json 两次裸分配。Native 路需受准入规范写入 CodecVec/NativeValue<String>；仍用原规范编码核，不能替成调试串。
- build_template 的 metadata clone/V 构造是另一组真实分配面；若在同 publication WorkScope 到达，必须通过受准入 builder 或 scoped输出类型，不能仅因未调用 native_* 就认定无需归属。此计划不改其业务字段和证据内容。

## D2：控制、Q 与 V2 兼容

| 文件 | 精确迁移点 | 不得改变 |
|---|---|---|
| control_audit.rs | schema_ddls_admitted 已用 CodecVec + native_codec_room；返回调用借 slice。普通 schema_ddls 仅 None scope 可转 Vec。返回 ControlHistory 按值 move | schema列名/DDL/marker/原错误顺序 |
| v2/mod.rs | control_decode_admitted（约210）`native_admit(parse_peak)` 后裸 control_decode 要改同 parser 的 PlainJson 持原 permit，解析临时峰与最终 payload 分开；ControlFacts::resume_owner（约1122）删除预收 batch+commit_ns 后 TextValue::Plain 裸构造，直接两次 native_text；control_extend→TextValue::from_bytes 需要原 token 的 bytes→String 转换 | 原 strict/legacy parser选择、generation连续性、control header唯一性 |
| q_runtime.rs | with_control_instance_id（约96）None预算入口使用 TextValue::from(id)；测试 plain_wire 返回 `&Value` 用 PlainJson.as_ref；try_into_plain（约307/427）仅已证明无 Native token 的普通 reply 分支可留 | Q资源域、TCP回复、快慢请求/取消合同 |
| q_http_input.rs | FailureDetail::from、copy_text、format_text（约64、481、499）无 WorkScope 本来是普通路径，用 TextValue::from 保持；Owned 路不改 | HTTP分段/首错误/状态码，不能引入 Native 新拒绝 |
| v2/evidence_store.rs | batch_root_key_admitted 的 CodecVec bytes→TextValue 同样须 token move；Reader 内 slots/to_vec、frontier/ids、内容摘要文本改 typed 容器；缓存 Cert/RootCert/parsed Arc 的独立元数据和 payload 需要真实共享 owner | 同原件比较、证书只复用纯核验、不跳过当前SQL |

`evidence_store::admit_byte_arc`（约96）只 native_admit 然后返回 `()`，无法持有许可证；约1928/2174 的 Arc 元数据同型。不能只改调用函数返回签名而让 cache 继续持裸 Arc。应在 cache 类型拥有的 Arc allocation 内放 payload+原 permit，Arc clone 只共享，末引用释放后才退费。该闭环会涉及 `v2/cache.rs` 中的缓存定义，**超出当前 D2 初始文件列表，须主控加归属或单独派包**；不应冒称本包单独完成兼容边界。

## D3：ExactRows、effects、集合与帧

### request_audit.rs

- SqlCell::Text/Blob（约237）从 Vec<u8> 改持费 CodecVec<u8>；ExactRows（约277）sql/columns 文本为 NativeValue<String>，scope/columns/rows/row_cells 均 CodecVec。借用 SqlCellView 与 storage class/REAL bits 比较不变。
- read_admitted 逐原 SQL 借用、先准入再复制，列头和数据各自 owner；不要整行借用结束后再建另一份无费 Vec。
- read_optional_bounded_params_in_parent（约1654）当前是 legacy `Budget.reserve → commit(0,heap)` 体系，不能只改数据类型再留原 commit。把父 Budget 的活动 WorkScope 明确传入受准入构造，保留局部 cap/optional None 语义与旧新分配峰；同一次读不能在外层 reserve remaining 的同时再对同父全额签 Native token。
- ExactRows 的结构 Retained/统计仍存在，但不是资源许可；Eq/columns比较逐 payload，不比较 token身份。

### audit_owner/effects.rs

必须与 request_audit 的类型/构造交接一起提交：

| 原位置（约） | 原费归属 | RAII 后动作 |
|---|---|---|
| 542–556 changes_permit.commit | changes Vec buffer | CodecVec 原 token；删这个旧 commit |
| 568–599 row_permit.commit + row_header release | ExactRows 新行及剩余表头 | 逐 owner move/pop，drop表头自动退；去相应commit/release，局部峰上界仍留 |
| 650–654 journal_owned_bytes release | nodes/stages/consumed实际旧分配 | 仅那些容器迁为 token 后删对应项，未迁旧 journal仍按原账释放 |
| 704–759 old_payload/remaining release | 原件旧行、未移走changes | 替换行/销毁changes自动退；删对应旧release；original_charge继续作结构统计 |
| 1686 StageOriginals::Drop | 旧合并 read_permit 费用 | ExactRows整体RAII后删预算/bytes手工释放；into_row move整CodecVec row，不按row.heap_bytes拆许可 |
| 1860–1871 read_permit.commit | stage读整行外预留 | 改同次工作域真实owner，不保留全remaining预留加token双收费 |

Journal 已有的 Weak身份、先放锁再读SQL、local cap/完整父帽区分、阶段资格及尾检均不变。

### audit_owner.rs

- CompleteCandidate.originals 从 Vec<ExactRows> 改 CodecVec<ExactRows>，其 resources 仍是原父 Budget 最后析构；Budget持有本身不等同所有临时值已持费。
- append_nodes（约2070）：目前从 SqlCell::Blob move 裸 body 到 `AuditCapture::append_original_for_owner_testonly`；迁 CodecVec 后不能 `try_into_plain` 脱费，也不能 clone 再保留旧token。需要 capture 原件入口能消费携 token body，并把同一个 token 移入实际持久节点。已有原件重复时新 body 自动 drop，删除约2089 `budget.release(body_capacity)`。**这是 capture API 前置，不可在当前函数补负费用抵消。**
- objects/auxiliary rowid 临时向量（约2232–2241）迁 CodecVec 后删对应 `budget.release(row_ids_charge)`；借用行号期间 token 存活。
- CompleteCharge/RetainedDelta 的统计与 gate 必须区分“结构总保留”与“父账已收费”。不删所有 charge.total，不用 `live` 差求重复收费。逐原收点登记：冷候选 header/current SQL/inline 是否已 token，迁一个删除一份旧 reserve/commit。约1716 inline `size_of<Self>` 是候选结构本体旧费，不会因内部字符串有 token 自动成为重复费。
- `self.limit` 完整父帽修复保留；不退回缩小 validation_limit 与完整 resources.peak 相比。

### construction_set.rs

- SequenceDelta.edits、diff临时 before/after stack（约53–192）改 CodecVec；grow 通过同 publication 的 WorkScope 签费。清栈不退真实 buffer，临时栈 drop 后自动退；删除仅对应旧 Vec 的 `b.release(released)`。
- PendingSet 按值持 arrays/requests，move到CommittedSet时保留许可证；现 parent_charge/resource_hold 不可全删，它们分别承接旧外层预留/父账身份。要先证明具体新token覆盖的分配与旧外层cap重叠再精确交接，不能依 sum(retained_bytes) 推测释放。
- 此文件可改，但不触及 construction_state.rs 的禁区字段/接口；既有 raw winner_edits 跨禁区 move 由通用 CodecVec 自动携token。

### v2/content_store.rs：ScopedFrame 的真实接入

1. `NativeReadLease.current_scoped` 与 `TxStore.checked_read_scoped` 已携 SQL 正文原 permit；临时 codec reader 应直接消费 ScopedFrame 借 `body/tag/usage`。不必每次转 NativeValue。
2. `NodeSource::checked_read` 的返回类型固定 `CheckedFrame`，`read` 固定 NodeRead 内裸 Vec。因此**不能把约663/941 的 into_frame 文本替换成 into_native 就宣称接上**。先追生产调用者使其走 scoped入口；确需跨 scope 保存 frame 的内部接口返回 NativeValue<CheckedFrame>。无 scope普通兼容接口仍可用 CheckedFrame。要保留带 scope 的旧接口成功，需更广的 NodeRead/CheckedFrame owning类型设计，不能简单加拒绝冒充验收。
3. `SameMaterial.frames`（约294）从 `(CheckedFrame,usize)` 改 `(NativeValue<CheckedFrame>,usize)`；命中读取从旧 owner 借 body，经 `native_body_reserved`复制新正文/permit，再原 checked_same_material；cache插入也用原 reserved body+ScopedFrame::new(...).into_native，替代约361 native_body_copy retain。树上界的原 tree_permit已经存在，不能再额外按 NativeTree 给同一树收费。
4. OriginalScanScratch（约2345）HashSet 改 BusinessMap<NodeDigest,()> 或专用持费set；begin clear保留实际桶容量，插入扩桶替换token，销毁scratch退费。现每次增长 permit.retain 必须消除，不能仅在 clear按len估算释放。
5. `capture_request_nodes`（约2455–2500）memory nodes payload 和 tree_permit最终retain尚未闭合。需要 AuditCapture 接管真实原token（树+逐body），或持久包装owner随capture整个生命周期；现函数返回裸AuditCapture不能靠局部guard释放。与 audit_owner append节点同属capture API工作，不在三个禁区内可做，但须单独明确库文件归属。

## 公共 String/Value/ContentRef 返回兼容的硬边界

裸 `String` 只有 ptr/len/cap，本身没有存原 NativeReservation 的位置。保持同一签名返回裸 String，同时要求token随这个String的真实drop退账，无法靠局部适配实现。

本轮正确选择是：内部 Native 主链返回 TextValue/NativeValue<String>，观察者借 `&str`；序列化在owner活着时借payload写入另一个独立已准入输出。原公开/旧兼容 `String` API保留无scope入口。若调用者确实要求有scope的裸String，则需上层响应owner/API改变或明确生命周期闭包，不允许 retain永久计费、提前drop token、to_owned脱费、以拒绝代替原成功。

同理，带Native token的 `CodecVec<u8>` 到String必须在库内受控移动Vec与原permit，UTF-8错误先销毁payload再退费；不能走现 try_into_plain。B2 正处理 TextValue::from_bytes，但需独审确认它并未使用裸转换。

## 准入与验证清单（都未运行）

- D1：same capture/overlay隔离、重复/缺字段错误、双投影cut前进/回退/重复输出等价；缓存替换先签新值、失败旧值仍可用；clear项费退而capacity保留，最终drop归基线。
- D2：普通/Owned旧HTTP与控制错误顺序不变；Native strict JSON错误路径也退费；Arc首建收费、clone不双收、末个drop归还。
- D3：ExactRows保持NULL/REAL bits/列名原字节；row从effects→candidate、body从row→capture移动后费仍在；重复原件、新旧row替换、early Err/None、rollback、COMMIT失败不得泄费或双退。
- ScopedFrame：SQL真读取/同素材证明/失效顺序不变；临时frame drop退费、cache frame跨scope存活；实际frame owner最终drop才退。
- 最后才可执行原六成功合同、16/8 MiB父帽、peak恰等父帽正控、raw/set非零及尾检失败不得COMMIT。未经运行不能把静态类型接入当六成功。

当前阻塞是 owning API 的有限缺口与旧记账交接，并非“只差格式化”。本报告只供分派和审查，不批准远端运行、合入或关票。
