# #1419 M8A：片 1 持有、前沿与读取接口

名分：绑定 #1419 的工作草稿。基线 `sandcastle/issue-1419@d29d9c7172ca0fad5c4350ff8fd58a204eb27775`；路径相对仓根；既有源码行号统一以 `git show d29d9c7172ca0fad5c4350ff8fd58a204eb27775:<路径>` 为准，本片新增测试模块单独标注工作区行号，避免 s_content.rs 新增 mod 声明后行号漂移。全文读过本目录 M7D-DESIGN（含 §7）、M2-NOTES 的 M7C/M7D/M7E，以及 `/Volumes/AgentStorage/m7d-design-review48x/REPORT.md`。采用方案 ii 的授权及合同边界沿用上述设计；本片没有回收、SQL 新后端、证书迁移或合同变更。

本片交付的是当前真实持有清单、局部前沿证明与明确的未证明项，以及接累计 capture 的私有测试接口。**当前生产仍累计持有全史 capture，尚未实现 O(Δ+前沿)，也未证明任意生产调用的整个工作集有历史无关上界。** 后续实际回收以前，本文未证明项须有可核查的迁移/预算证据。局部栈深证明不能代替完整调用共存证明。

## 1. 字节与生命周期的计量方法

记 `C(x)` 为真实 allocation 的 capacity 字节，`L(x)` 为逻辑正文长度；`V<T>(x)=capacity(x)×size_of::<T>()`；树按实际 Box Layout 累加，HashMap 的 bucket/control、许可、临时扩容共存另计，不能只算槽有效载荷。`B(S)=Σ C(body)` 按不同 backing 求和，借用不再加一份物理正文，但被借用的 backing 仍全额存活。`P(S)` 是 CheckedFrame/proof 的独立正文副本及 Box Layout。`D(S)` 是 ContentRef 的 format/root 字符串容量、容器及封印/上下文描述符。SourceClaim 的 `L(body)+97` 是角色合同额，单列为 E/O，不与父账物理 live 相加冒充堆内存。

在时刻 t 的前沿字节必须写作：`Σ活正文backing C(body) + Σ活独立frame/proof副本 C(body) + Σ容器实际Layout/容量 + Σ聚合/描述符实际Layout + 同时存活SQL/codec临时副本`。合法正文长度检查发生在读回之后；畸形节点在拒绝前可能很大，`MAX_FRAME_LEN`、4096 以及 32/40/64/72 字节的合法格式都不能替代拒绝前的实际 capacity。当前没有收集各帧容量分布，不填伪精确 RSS 或节省额。

以下“跨请求”指成功候选被 owner 留存到下一请求；失败路径随候选/请求析构。持久库继续保全历史，本表只讨论内存持有。读到描述符不等于可以删除其指向的原件；未来替换须绑定同生涯、同内容库/事务域、完整 ContentRef、Flavor/Mode、判据版本，并走同一读取核。

## 2. 持正文与借用清单

| 持有者及源码证据 | 取得、释放与跨请求 | 当前跨请求真实字节 | 方案 ii 的拟替换及释放义务 |
|---|---|---|---|
| `NativeNode` / `NativeCaptureNodes`：`rust/src/s_content.rs:2286`、`:2340`；追加 `:3515-3590` | CodecVec 正文按值移入 digest 表；唯一 digest 累积，重复 digest 先核帧再逐字比 tag/body。正常 finish 不删 entry；owner/candidate 析构才释放 | `B(全史唯一原件)` + 索引实际 Layout/控制字节/许可；每 entry inline SourceClaim、proof 槽、dirty/next。body_len/body_capacity 是聚合值，不能当另一份正文 | 持久正文+本代 pending+活 ReadLease；活跃索引限这些材料，不留全史内存 digest→body 表。最后租约结束才析构 backing；不能仅把表换名为 Handle 继续 pin 全 capture |
| `finish_owner_step`：`s_content.rs:3593-3615` | 清 public/template AuditMemo，沿 dirty 链清本步 proof、重设 memo/reads；原件及根/owner 成功树继续保留 | finish 后仍为上项的全史 B 和索引；dirty proof 独立副本已析构。finish 是清证明，不是正文释放点 | 本片保持原样；未来回收边界必须在交接、全部借用结束之后，不能将清 memo 当回收证据 |
| effects 来源准备：`rust/src/bin/s_session_v3/audit_owner/effects.rs:1462-1493` | 同真实事务逐 `j.nodes.keys()` read_scoped，许可随 body 移入 capture；每次循环临时 frame/body 消亡；新增 entry 此后跨请求 | j.nodes 是当前请求材料集合，不是另一个全史正文表；移交后的 B/索引计 capture。若本事务仍有其他真实副本须分别计账 | 当前事务未提交正文必须持住；后续持 pending 描述符及活租约，提交前不淘汰。仍核 j.cap、同父账，不改来源分派或扩容时机绕 J |
| effects 尾部补入：`audit_owner.rs:3894-3918`，尤其 `:3911-3913` | 从 effects row[3] replace 为 Null 后取 CodecVec 按值移入；成功成为累计 entry，失败不提交 | 移交后只归 capture 的同一 backing；尚未移走的 effects row 副本按自身容量计 | 同上；`usize::MAX` 是此入口当前 cap，仍同父收费。不得迁移其他追加到此入口逃 J |
| `CaptureReader` / `ObservedCaptureReader`：`s_content.rs:4739-4853` | 借 `&AuditCapture`；read/checked 每次取得实际 entry 并复制 body，返回 frame 生命周期由调用帧决定；observer 不留在 capture | reader 本身不跨请求；capture backing 被整个借用期 pin。返回副本若被持有则另计 C；当前 proof 可在请求内保留 | 改为步域 reader+ReadLease，读结果离开消费帧即 drop；本片 drop 只结束借用，capture backing 仍存活。未来不得返回对可回收槽的长活 `&[u8]` |
| `NativeFrameProof` / memo：`s_content.rs:2362-2529` | 首次真实 F 成功后复制并保留 CheckedFrame，same-material 必须逐字 rebind；缺失/错误/变化先撤旧 proof；finish dirty 链清除 | 正常 finish 后这些 dirty proof 不跨请求。请求内 `P(实际保留proof)` 按真实副本 capacity+Box；entry_limit/body_capacity_limit 随累计 capture 扩大，并非固定前沿帽 | 租约与纯结论分开；frame 只活于本步必要比较，成功结论用封闭域描述符，撤销沿原核 |
| `NativeCountsSubsetReader`：`s_content.rs:3023-3104`，ledger `:2973-3019`，调用 `:3181` | 借完整 capture、ledger、work；读前用 `original.len()` reserve，再读/核；record 用原件长度+97 累加来源费；逐组结束可 rollback，最终裁剪 move 节点（`:3276-3294`） | reader/临时 ledger 不跨请求；若子集 capture 被保留，其 B、索引、SourceClaim、证书及 CodecVec capacity 均跨请求，不能只数新 digest。ledger 的 HashSet/order 也须计当前峰 | 原件长度与角色费来源改为同事务真实 frame 或已认证不可变长度记录，不能靠失效地址。已接纳整组三闭包的保留义务继续成立；不得提前回收后再从别处补读 |
| `VerifierBody`、`Descent.body`：`s_content.rs:1999-2030`；`rust/src/s_content/path.rs:958-963` | Borrowed 在 AllocationPrototype 特殊读取核（`s_content.rs:2050`）及 Native OwnerCapture 中都存在；后者 `rust/src/s_content/closed.rs:773-796` 核工作域健康、直接借 capture 正文并核 F；正文/索引的同父扫描是新测试接口构造器的额外边界，不冒称旧 read 核逐项检查。Plain/Metered/Scoped 则持独立 backing/许可；调用返回或局部覆盖时结束借用或释放独立载荷 | 普通局部 body 不跨请求；Borrowed 持有期间 pin 对应 capture，不能据 enum 名称推断 Native 零复制 | 活 ReadLease 随消费块结束；当前 Borrowed drop 只结束借用，未来租约真正自持 allocation 时才在析构正文后归还许可。描述符里不存裸 slice/槽地址 |
| `SourceFields` / payload：`rust/src/bin/s_session_v3/source_audit.rs:223-289`、`:916-927`；`audit_store.rs:1216-1263`、`:1511-1558` | SourceFields 包 RefFields；Plain 是指定字段子集的 NativeRefFields，Owned 为 RefOwner 描述符。Object.fields 与 Group.payload 可随 NativeSourceState 留存；payload 本身是 RefValue，不是整份 JSON 正文 | `D(字段引用、payload、refs)`+字段容器/key 容量；Object.id/kind、Group.members/sources 另持真实文本/坐标。不能把 payload 误算为完整原件，也不能漏掉渲染/解码产生的正文 | 引用迁为封闭域描述符；标量/文本按租约读出并计实际容量。继续保存实体身份与必要小字段，不能通过 RefOwner 间接 pin 全 capture |
| 字段/标量/身份借用缓存：`source_audit.rs:109-211`、`:1184-1301`；`rust/src/bin/s_session_v3/native_record_fields.rs:309-342` | NativeFieldMemo、NativeRecordFields、NativeWitnessIdentityMemo 借同一 capture/work；fields 保存引用、scalars 保存 PlainJson、identity 保存结果文本；clear/drop 释放，本次审计内复用 | 缓存自身不跨请求；capture 被借用期 pin。请求峰须加 NativeFieldRefKey 两字符串 capacity、字段 names/refs、JSON/string、树/表 Layout，不能按 entry 数×常量 | 拆出封闭域身份及本步缓存，正文受租约；旧地址身份不能跨回收/生涯充当认证。缓存任意 entries 的上界仍待逐核证明 |
| codec 成功能力：`s_content.rs:2523-2544`、`:2731-2755`、`:3940-3998`；请求闭包能力 `:2220-2270` | 根证书 HashSet、owner 的 seen/bytes/elems/fields/key_cmp 成功 AVL 跨请求；请求能力持本步 memo。只有整根成功 merge，任一错误 revoke；当前可信性依赖同一不可变 Native owner，不是外来成功 bool | 根 digest 集 capacity、AVL 实际 Box Layout、ByteAgg/FieldAgg/Ordering inline 聚合，不持原件正文；仍可能随全史增长。不能称全史成功表为前沿 | 同生涯/封闭持久域/完整上下文/判据版本的能力描述符；非活跃结论可派生台账保存，冷从 g1 重核。保持完整根头核验和模式隔离；序列摘要不单独授权 |
| `CompleteCandidate`：`audit_owner.rs:3298-3315`、`:4110-4159` | capture、audited、Q、originals、资格 reserve 共同交接；complete/pending slot 保存（`:323`、`:392-394`）。成功跨事务，discard/stale/drop 释放；resources 字段最后析构 | 所有前述正文/描述符，加 audited/Q/ExactRows 全史元数据、inline/permit/资源账。finish 后仍按累计 node_count×97、body_len/capacity 等重算 | 本片保持字段及交接不变；后续仅对真正析构的原件终止相应角色，描述符仍收费。降低 capture 不证明总门 `:4153` 可过 |
| 来源与历史资格：`effects.rs:1254-1315`；`rust/src/bin/s_session_v3/state.rs:1482-1563` | QualifiedSourceCut 绑定 seal/epoch/root/raw，consume 检查当前材料、deadline 后消费；HistoricalCertificate 持 AuditedState，QualifiedCertification 持父账身份/最高已消费资格，随候选跨请求；H1 不持 Ledger | 来源 counts/exports、资格根/规则/内容库/二进制身份字符串 capacity、封印/资格 Layout；HistoricalCertificate 的 audited 载荷另计，不重复加账 | 保持原 seal/epoch/生涯/根/单位及许可交接时点，正文回收不撤销/重签资格，不延长到新生涯；canonical 用同 reader 实化 |
| `SourceClaim`：`s_content.rs:2314-2337`、`:3555-3557`、`:4063` | 只对 Native-bound 同 backing 正文签出，`L+97`；随 entry 消亡；无父账独立 reserve | 每活 entry 有 inline bytes 字段；E/O 角色合同额为 `Σ(L+97)`，物理正文已计一次。本片跨请求角色额完全不变 | 后续必须证明角色持有与租约 backing 同时终止。不能仅减 node_count/费用而保留正文；旧 origin 释放只减 live、不返当前 remaining（`native.rs:1565-1609`、`:1643-1659`） |

### 其他全历史 owner 状态单账

| 对象 | 跨请求材料与独立计量 |
|---|---|
| `NativeSourceState`：`rust/src/bin/s_session_v3/source_contributions.rs:408-455`、`:496-568` | Rc<Object>、raw/raw_ordinals/order、全部对象/组/形态/见证/请求/成员/pending/依赖索引、seed 引用、持久树节点与文本/坐标容量。其当前态大小随领域历史增长；Object/Group 引用按上表另识别正文边界；不把全部索引塞进 w |
| `AuditedState` / `ImmutableHistoricalAudit`：`state.rs:47-85` | raw_events 真实载荷、历史根/前缀/控制事实、对象/辅助/来源前缀状态、资格身份；按各 Retained/Native backing 的实际容量收费，raw 正文确实跨请求。它们不是可随 capture drop 的附属“缓存” |
| Q：`rust/src/bin/s_session_v3/audit_owner/q.rs:25-59` | clock、sid/inc、raw_tails、coordinates、pending、frontiers、raw_fees 及树/HashMap/Vec 实际容量；全关系状态，`retained_bytes` 独立进账，不承诺历史无关 |
| ExactRows：`rust/src/bin/s_session_v3/request_audit.rs:773-815`；候选 `audit_owner.rs:3311` | 非节点 SQL 全集的 sql/scope/column/rows、SqlCell 的 Text/Blob 实际容量；新节点 body 移出后 row[3]=Null 不等于这些全集消失。Native/Owned 已持许可部分不手工重复退款；effects.apply 产生的新旧共存按峰计 |
| 构造差异/已提交描述符：`rust/src/bin/s_session_v3/construction_set.rs:54-64`、`:216-222`、`:299-315` | CommittedSet 只留 root/raw/knowledge 定位；PendingSet 同时持三路 arrays 和可选 requests 的 edits Vec、源检查/stage seal/parent_charge。本步材料 Δ 可大，按 edits 实际 capacity 计；不称固定窗口 |

因此整个 owner 的内存不能由 `B(Δ+前沿)` 表示。未来只允许对“原件及其读取工作集”给这个界，其余项继续逐项收费并经过原总保留门。

## 3. 逐调用族的 k / h / 工作窗证明

这里的 k 数的是同时保留的导航/比较 continuation，连续两次读取不自动成为两条路径。每行的 w 是**该调用的显式共存材料**，不是新政策帽。`h_s` 为实际序列最大导航深度，`d_v` 为实际值嵌套深度，`p` 为请求 path 长度，`h_i` 为各步序列深度，`G_T` 为实际容器容量/布局。对通过规范分解检查的序列，split_index/k 每层严格减小 count，深度随叶数对数增长；在完成检查前，使用实际走过深度 h，不把合法树深推给畸形输入。LeafIter 单独不核 k 形状，不能直接给所有输入 `h=log N`。

表中“局部已证”仅指控制流/正文共存形状；所有字节式中的 C/G 均需真实许可支持。总 w 未证的行明确保留义务。完整请求的峰是调用嵌套后所有活项之和，不能取这些行中最大值替代。

| 调用点及证据 | k、h 与 w 来源 | 字节界与证明结论 |
|---|---|---|
| capture 原件 lookup、checked_current、`read_scoped_checked`：`s_content.rs:2470-2519`、`:1809-1830`、`:4766-4853` | k=1 点查，h=0；w 包当前原件、返回副本、旧 proof 及可能新 proof。same-material 比较旧 proof 与本次真实复制正文，不把 digest 当字节相等 | 局部 `C(backing)+C(current)+C(oldproof)+C(newproof)+G_proof`，按真实同时存在取值，不将互斥分支叠算。索引/后端全史 B 仍活，故仅读帧局部已证，整体非有界 |
| 两 append 入口及同摘要异正文：`effects.rs:1462-1493`、`audit_owner.rs:3894-3918`、`s_content.rs:3540-3573` | k=1，无递归 h=0；w=当前候选 body+既有同 digest body；新索引 reserve 时旧索引仍活 | 局部比较 `C(new)+C(old)`；缺失新增分支加旧新索引许可/全史 B。未提交 j.nodes/Effects 的其余材料计 Δ。现整表扩容 G 不变；未证明本代所有 Δ 小于 J |
| 根声明/普通 value 待核：`s_content.rs:6204-6248`、`:5539-5597`、AuditMemo `:2168-2185` | 单个 root/header k=1、h=0；run 外帧再调用以下序列折叠。todo/pushed/seen/read_digests 与四类 memo 按本步闭包增长，w=`G_AuditMemo+待核值集合` | 外层活正文 `C(value)` 加被调行的字节；`G_AuditMemo` 必须全额计。**总 w 未证明**，不能由 k=1/h 导出 O(h) verifier |
| `byte_agg`：读 `s_content.rs:5632`、`:5656`；循环/Step `:5599-5770` | k=1 DFS；steps 的 Enter/Right/Combine 与 aggs 栈沿当前路径/未合并边界，h=实际字节序列深度；叶处序列帧和块正文同时活。ByteAgg 含 UTF-8/数字进态、总长/末长，不省略聚合态 | `C(外层value)+C(seq)+C(chunk)+G_steps+G_aggs+G_bytes_memo`；栈元素数 O(h)，实际 Vec capacity 另计。`bytes_memo` 按 (digest,UTF8,Num) 的整个本步已访问集合累积，**总 w 未证明** |
| `elem_count`：`s_content.rs:5773-5868`，读 `:5795/:5811` | k=1 DFS、h=h_s；叶同时持 seq 和 value 头；counts 与 Step 栈 O(h)，push_work 将每个值加入外部 todo/pushed | `C(外value)+C(seq)+C(leaf)+G_steps+G_counts+G_elem_memo+G_todo/pushed`。局部遍历栈已证；整个元素集合/memo 的 w 未证明 |
| `field_agg`：`s_content.rs:5873-6030`，读 `:5904/:5925`；`validate_string_node :6033-6055` | k=1 字段 DFS；叶持 seq+field，并进入键 string/byte_agg；组合期保留 FieldAgg 的首末 key，在 compare_keys 内同时两键流，故活导航数为外字段路径+2 键路径=3；h 用 `h_fields+h_key_a+h_key_b` | `C(外value)+C(seq)+C(field)+C(keyheader)+B(当前块)+G_field_steps/aggs+G_fields/seen+被调memo`；组合时换成双流式。**字段 memo/待核值/比较对 w 未证明** |
| `string_bytes_root`、`compare_keys` / `compare_key_streams`：`s_content.rs:6058-6173` | 两根头顺序读，头返回后释放；比较 k=2，h=`h_a+h_b`，两 LeafIter 栈与 buf_a/b、pos/digest 同存；LeafIter 推右再左（`:2117-2161`），一边推进时另一边保留 chunk，旧一边 buf 在 next 返回赋值前也还活 | `G_iter_a+G_iter_b+C(buf_a)+C(buf_b)+C(transient_seq)+C(next_chunk)+G_聚合`，再加外层 frame/栈；不能按“最多两个4096”计拒绝前/替换瞬间。key_cmp 保存双向结果（`:6085-6087`），**比较缓存总 w 未证明** |
| path 单根头/值读取、下行 `navigate_path_view`：`path.rs:974-1085`、`:1086`、`:1749`；`descend_seq :1305-1366` | k=1 外 path，h=`p+Σh_i`；Ancestor 每层保 branches，cur_body 赋值前和新 body 共存。descend 顺序读左右直接子仅存 SeqNode 描述符，非两条正文路径 | `ΣG_Ancestor/BranchFrame + C(cur)+C(next)+被调字段比较字节`；局部结构已证，p 及所有 branches/容量按输入计，不把 path.min(64) 的初始 reserve 当上限 |
| 字段二分：`find_field_optional path.rs:1441-1508`、`boundary_leaf :1370`、`read_field :1414` | 外字段路径 k=1，边界定位顺序完成并释放，只保摘要；比较边界两键调用 cmp_key_nodes（k=2），所以组合 k≤3；h 用外 branch 深度+双键深度；没有并行 reader | `G_branches+双流字节+当前读frame+外层cur_body`；局部已证。其来源在 `SeqNode/BranchFrame` 复制，不 pin 祖先正文；不据此证明整个 owner |
| 字段名/键比较：`read_short_key path.rs:685-707`、`cmp_field_key :709`、`cmp_key_bytes :1528-1568`、`cmp_key_nodes :1571-1637` | 短键 k=1、h≤单叶路径（声明超过 CHUNK_BYTES 只选择长键算法，并非失败重判）；键对 bytes k=1，键对键 k=2，h 为对应迭代栈。两键流外层比较使用完整核读取的实际块 | `ΣG_LeafIter+C(保留短键/双buf)+C(临时seq/新chunk)+标量进态`。4096 仅是已有合法字节块长度，不是内存前沿帽。与外 find_field/fields 的共存按上/下行加账 |
| 对象字段子集/全字段输出：`object_fields_output_inner path.rs:550-681` | k=1 外 DFS+最多2 键流=3；h=`h_fields+h_key_a+h_key_b`；previous_bytes/current_bytes 最多两短键，fallback 长键比较时仍可能持其中一短键；keys/result 按 names 长度增长 | `C(objectheader)+ΣC(短键)+双流临时字节+G_DFS+G_keys+G_result(D(names))`。SHAPE/TYPED 清单 12/7（`source_audit.rs:1155-1177`）可证特定输出个数，通用 names 仍是输入；**请求累计字段 memo 总 w 未证明** |
| 数组批取 `lookup_array_output_inner path.rs:292-340` 与 `NativeArrayCursor :2077-2199` | 批取 k=1、h=h_s，输出累积全部成员引用，w 包整个输出；cursor k=1、h=h_s，栈存 `(SeqNode,count)`，next 仅输出一个 Ref，调用者仍可能收集全部 | 批取 `G_输出+D(全体成员)+G_遍历` 未证固定窗；cursor 自身 `G_stack+C(当前读取)+D(单成员)` 局部已证，**调用者保留多少成员未证明**。cursor 不存正文不代表外 capture 可回收 |
| bytes range / key-at：`path.rs:749-835` | key-at 顺序 descent 当前/相邻字段，随后 cmp_key_nodes 双流比较，最大 k=2（外层对象 header 另持正文，已返回的 descent 不再保 continuation）；h 为 max(h_fields,h_key_a+h_key_b)。bytes_between 按覆盖区间的块 index 逐次 descend_seq，k=1、h=h_s，非 LeafIter；输出 WorkVec 收集完整命中范围 | `C(外层对象/字节值header)+G_branches/双流iter+Σ活正文C+G_out`；只有导航栈局部已证，输出由请求区间/正文决定，未证固定字节窗 |
| Renderer：`s_content.rs:6560-6845` | k 不是固定 1：当前 value 的每层 Elements/Fields 任务可保留各自 LeafIter，k≤d_v+1；h 应取各活 iter 深度和。token/string 单流追加当前路径，父 tasks 仍活；escaped/UTF8/Num/escape 状态共存 | `G_RTask+ΣG_活LeafIter+C(valueheader)+C(seq)+C(chunk)+G_escaped+G_sink`。任务深度局部按嵌套计；sink 若收集整段 canonical/JSON 则持整输出，**renderer 调用者总 w 未证明**，不能一概当流式常量内存 |
| `SourceFields.scalar/text`、canonical 比较/来源身份：`source_audit.rs:253-289`、`:308`、`:410-450`、`:1257-1301` | 一次读取/render 路径顺序 k 按 Renderer 行；canonical_matches 比较 sink 不另攒输出，但输入 PlainJson/原 bytes 同活；scalar JSON、复制 text 及 native identity 缓存均真实占字节 | `Renderer字节+G_JSON+C(text/bytes)+G_identity缓存`；**总 w 未证明**，不可把 payload/ref 的轻量表示套给 render 结果 |
| shape/见证/发展批来源读者：`source_audit.rs:2194-2225`、`:2515-2646`、`:2737-2789` | shape 固定三 part，3 份 high/low 文本/窗口、source_union 同存，实际读路径串行 k 按被调 path/renderer；w 至少三组 members/dependency 并集等。见证固定四 OHLC 依次比较引用，随后 elements(raw_bars) 收集成员；发展批 elements(entries) 收集全批 | `ΣC(high/low)+G_三槽/Decimal+G_coords并集/差集+G_成员Refs+被调字节`。三/四个语义字段已证；**各组/本批成员字节与变更集上界未证明**。bounded-layout/2 名称不是这里的固定字节证明 |
| `NativeCountsSubsetReader` 与三闭包组：`s_content.rs:3023-3104`、`:3146-3294` | reader k=1；内部 verifier/导航沿上述行；w=全部已接纳组的 ledger digests/order、完整三闭包原件、当前组及 rollback checkpoint。cap 决定停止而非固定 k/h/节点数 | `B(保留子集)+G_digests/order+P(proofs)+G_verifier+G_cert输出`；按费用 cap 接纳不足以证明固定前沿，**全体 w 未证明** |
| `construction_set::difference`：`construction_set.rs:108-211`，split `:72-100` | before/after 两栈，k=2、h=`h_old+h_current`；每次 split 顺序点读一个 fragment，栈留 Span；edits 保留全部 Retain/Remove/Add，不将序列摘要当单独认证（调用方先证两端完整域） | `G_before+G_after+G_edits+C(当前fragment)`；双 DFS 栈局部已证。PendingSet 同时保三数组+可选请求 edits（`:299-315`），**跨路 edits 总 w 未证明** |
| 完整原件集合比较 `exact_original_tuples_equal`：`s_content.rs:3639-3675` | k=2 表迭代/对应点查，h=0；逐 digest/tag/body 引用比较，无额外正文复制；两整 capture 必须同时活 | `B(left全史)+B(right全史)+G_left/right`；额外比较状态 O(1) 不能称持有有界，**整体未证明**；此入口迁移须逐位实化同域原件，不能比较摘要列表代正文 |
| SQL scoped 源 / SourceMaterial / ordinary audit wrapper：`effects.rs:1382-1394`、`:1198-1211`；`source_audit.rs:477-506`；`rust/src/bin/s_session_v2/content_store.rs:893-919` | 这些是同一 reader 的委托层，不另加并发 k；SQL 读结果、CheckedFrame 与 permit 按作用域活，work/tx/材料 seal 先检查，typed 错误直接传 | 将它们的 SQL 复制/预留加在实际调用行；**SQLite 临时行/页缓存可归属字节及真正读前许可尚未在本片证明**，不能从 scoped 包装推断实现完毕。stage miss 只在 test cfg，不作为生产恢复 |

| 新测试接口 `StepReader::new/read_checked`：`rust/src/s_content/step_reader.rs:31-90`（工作区） | new 顺序扫描所有既有节点的许可父身份，k=0 导航路径/h=0，但时间为 O(N)；read_checked 先同核 verify，再同核借实际根正文，k/h/w 继承 verifier，返回根lease的 k=1/h=0。当前禁止借此声称新生产读核有界 | new 不复制正文、额外 O(1) 控制状态，但整个 capture 继续存活；lease 为 Borrowed，返回正文不新增 backing。全史 B/索引/成功表依旧全额保留；**整体未证明有界**，测试接缝扫描不得进入后续生产复杂度证明 |

此枚举按真实读取/比较核及其 owner 调用族组织：委托包装归到对应核而非重复算一条路径；测试、普通完整 oracle 及 Native 动态入口均不因此获得新成功资格。测试专用新接口只复用已有核，下一片接生产前仍须用引用追踪检查是否新增调用族，并把新增点并入本表。

## 4. 当前可证结论与后续门

可证的是点查单帧、双键流、三路径嵌套字段查找、序列 DFS/数组游标及构造差异双栈的**局部控制流形状**。完整请求的 k 应按当前调用栈累加：例如外 field_agg 的导航 continuation 加两键流为 3；Renderer 则依值嵌套携多个 iterator。h 是实际导航深度，不是代数，也不是“最近若干代”。w 至少包含上述本代新增/变化材料、待核集合、聚合 memo、成员/来源向量、输出及 SQL 临时材料。

全局未证明项为：① verifier 与 owner codec 成功表的当前/跨步累积；② Counts 子集闭包与 ledger；③ Native 字段/标量/身份缓存及每对象渲染结果；④ 三组/发展批成员、坐标并集和任意 changed object 的材料字节；⑤ 多路 edits 与跨调用游标/成员的共存；⑥ SQL 行复制/页缓存的读前归属及许可；⑦ 全部这些帧在真实生产调用栈上的容量和峰；⑧冷折叠在完成交接后不重攒旧正文的生命周期。每项目前都记“未证明”，不签有界。解决方式应是将必须保留的变化材料纳入 Δ，将完成子树转为可重建能力/描述符，提供逐步容量收据与同父生命周期锁；不能改判据或删去聚合/去重所需状态。

节点数式 `f≤k(h+1)+w` 只有在 w 明确到当前所有共存材料后才可使用；再以第 1 节逐 backing 字节式换算。本片不代入 4096 槽作为 w 或前沿帽。M7D 的 4096 槽只保留为敏感性情景，也不据 225 B 历史槽宽估算真实 body/proof/聚合字节。

## 5. 私有接口与本片验证边界

本片 `StepReader` / `ReadLease` 位于 `rust/src/s_content/step_reader.rs` 私有测试模块（工作区 `:11-27`），后端为当前 `AuditCapture`。`CaptureStep` 持同一 capture 与同一活动 WorkScope；`new :31-58` 检查活动域健康及既有 Native 正文/索引许可的同父身份，旧 origin 不必等于新请求 origin。该构造器当前遍历全表，不能作为有界 adapter 的实现或复杂度证明。`read_checked :60-90` 收完整 RefView/Mode，根租约固定 Flavor::Value，内部 JsonInner 沿原 Verifier 推导；不提供任意 Flavor 的裸摘要读口。lease 独占借用 reader 且不提供 Clone/into_body，Rust 借用寿命限制为此步内。正文由已有 OwnerCapture/同核读取路径借出，lease drop 只结束此借用，既有 capture backing 及正文许可仍保留；任何独立临时载荷沿其原生命周期释放。本片不签发新正文 allocation，也不在 lease drop 时释放 capture 的许可。它并不证明 SQL 事务域适配器已存在；真实 tx、内容库快照/生涯夹持及读前 SQL 许可属于下一片义务。

新接口对第一次可达、同 digest 异正文、缺失/篡改、完整 ContentRef 头与上下文、typed origin、失败撤销都委托当前核；没有“新接口失败后改走旧接口”的分支。完整单位仍为 `(digest,Flavor,Mode)`，序列只是所属值核的材料；根 format/kind/extent 每次照核，不能凭裸 digest 或调用者 bool 授权。新锁在独立旧路径“verify_owner_refview 后 OwnerCapture.read”与同序列新接口之间，对拍结果正文、原 CodecError 类别/文本及单独核验的 typed origin 桥（CodecError::PartialEq 本身不比较 origin）、F/reads/peak 及许可/事件；具体锁名和 CI 过滤步骤记 M2-NOTES 的 M8A，运行状态以真实 CI 为准。

本片生产零行为变化的理由是接口及新锁均在 test cfg 内，生产调用点、累计存储、finish、两追加入口、帽/费用式、effects/资格/证书、deadline/W2 串和 ci.yml 全部保持。没有启用淘汰/回收，没有增加存储重试，也没有使用 content_store 的 test-only stage miss 修补当前缺失。测试后端仍 pin 全 capture，这仅是同核对拍支架，不能宣称已迁移持有者或已经履行“不常驻全部原件”。

本文件没有制造 Rust 通过收据。本机禁止 cargo/rustc，Rust 编译与对拍待 CI。原 M3 四锁、admission 六代、certified/qualified、effects/少 1B/首错、ordinary full_some 冻结轨迹及具名重型性能/恢复门继续按 M7D §5 保留；本片不以新靶向锁替代它们。后续实际回收须在本文未证明项已解决且片 1 经非作者独审有效后进入。
