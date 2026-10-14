# #1489 受影响代码清单（S 内容布局 bounded-layout/1）

> 票：[#1489](https://github.com/xy7365527-lang/NewChanlun/issues/1489)；正本：[ADR 0037](../../../docs/adr/0037-s-content-bounded-layout.md)、`CONTEXT.md`「内容布局」。
> 基线：S 写者 #1394 整合分支 `0fc0c165df0e80ac1fe704a0f24c09ea6678f373`；main `7c6ee7502656f337101f2f1169ececbf45278d48`。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归「归属」列的实施票（段 A = [#1488](https://github.com/xy7365527-lang/NewChanlun/issues/1488)，段 B = [#1491](https://github.com/xy7365527-lang/NewChanlun/issues/1491)），本清单本身不改任何代码。Lean 无受影响项（本票不动教义）。

| 侧 | 文件:行 | 处置 | 归属 |
|---|---|---|---|
| Rust | `rust/src/bin/s_structure_session.rs:98-106` | V3_RULE_REVISION 升 s2-axis-quantifiers+new-bi/1+bounded-layout/1；文档写组件组合规则与「v3 写者任何修订串必含 bounded-layout」 | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/mod.rs:443-444` | init 写入新 rule_revision | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/mod.rs:590-592` | 旧库 expect_meta 允许旧串（只读读取 layout/1 库） | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v2/tb02a_facts.rs:945-980` | CatalogAxisTable::for_rule_revision 改按组件解析；旧整串继续映射 NewBi/SegmentSecond | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/source_audit.rs:2110` | 测试夹具取新 V3_RULE_REVISION | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/object_index.rs:287-301` | OrderKey/order_key：新增布局感知比较器（活动按 first_known_generation、撤回按 withdrawn_generation，同代内沿 layout/1 键）；按库内 rule_revision 分派，单一比较器 | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/object_index.rs:784-785,896-897,979,1983,2089,2122,2656,3813,4392-4394,4564` | 全部排序调用点改用分派后的比较器；不得出现「先新键失败再旧键」 | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/object_index.rs:873-897` | at_cut_projected：段A 增量维护（不每代全量重排+hash_array）；段B 出生序下只需尾部追加/有界删除 | #1488（段 A）+#1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/aux_index.rs:762-789` | witnesses/relations/observations 三处 stable_sort 键改 (published_generation, 旧键) | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/aux_index.rs:690-800` | 段A：快照合并从全量重排改增量；段B：键换 | #1488（段 A）+#1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/aux_index.rs:924-949` | public_sets 不变（已走 SequenceMemo::sync）；登记 | — |
| Rust | `rust/src/bin/s_session_v3/generation.rs:700-712` | CC-054 requests 数组按 (window_start, label) 排列，竞争请求不再插头 | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/generation.rs:737-748` | 本代 typed 排序不改（本代集合有界）；登记 | — |
| Rust | `rust/src/bin/s_session_v3/generation.rs:1020-1110` | SequenceMemo::sync 后缀整段写入，不再逐元素 sequence_append 产生作废中间根 | #1488（段 A） |
| Rust | `rust/src/s_content/sequence.rs:1-60` | 模块文档补 batch-append 上界 ≤ m·(bits+1) 并加测试 | #1488（段 A） |
| Rust | `rust/src/s_content.rs:383-384` | split_index 不改；§5.2 编码不动（登记） | — |
| Rust | `rust/src/bin/s_session_v3/history.rs:44-56` | compare_cut_array：审计数组次序判据改「代际单调不减 + 同代内原键」，按布局分派 | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/history.rs:782-845` | audit_one_published_cut 的 at_cut/compare：段A 增量；段B 位置继承恒成立（M4 修正一退回） | #1488（段 A）+#1491（段 B）+#1481 |
| Rust | `rust/src/bin/s_session_v3/prepared.rs:766-830` | templates 字段不变；注释登记数组次序由布局组件决定 | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:1236-1330` | CC-054 build_knowledge_state 内容不改；加单代新增节点实测锁 | #1491（段 B，锁） |
| Rust | `rust/src/bin/s_session_v2/content_bi.rs:1461-1503` | CC-056 version 不改；加实测锁 | #1491（段 B，锁） |
| Rust | `rust/src/bin/s_session_v3/audit_owner.rs:写前门（行号由实施票定位）` | S 存储 max_store_bytes 帽（64 GiB@9011）超帽 stale 停写；随 #1481 保留帽一起落 | #1481 |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:1580-1660` | build_fractal_impl：保留 memo/sequence_split/append/floor；FractalDevelopment{root,count,union_tail} 作输出返回，不再写进分型载荷 | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:1661-1686` | 单元对象 {bar, relations} 不变（登记） | — |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:2845-2868,2968` | fractal_relations / order_of 不变；全仓唯一判定（登记） | — |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:1740-1810` | 分型载荷删 subsequent_development；all_sources 改基础来源（shape.sources ∪ 右组 group_sources_coords） | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:1836-1880` | subsequent_development 请求仅 no_future 时发出，sources=基础来源+右组末坐标 | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v2/content_producer.rs:新增` | build_development_batch：kind CC-007.development_batch，fact_key [kind, bar.source_coord]，payload {bar, entries:[{fractal_object_id, unit, development:{root,count}}]}，entries 出生序；走 typed_object | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v3/generation.rs:516-600` | 收集各分型 FractalDevelopment（含往代分型，从 input.prior 续 append），每代组一条批记录推进 typed | #1491（段 B） |
| Rust | `rust/src/bin/s_session_v2/tb02a_facts.rs:1010-1012` | kind→轴映射加 CC-007.development_batch → CC-007；轴表不变 | #1491（段 B） |
| Python/JS | `s_session/s_query_integrity.py:1647-1655` | 六数组重建按出生序并按库 rule_revision 分派；顺带核 withdrawn Typed 键与 Rust 的不一致 | #1491（段 B） |
| Python/JS | `s_session/s_readonly_server.py:529` | withdrawn 构造按出生序 | #1491（段 B） |
| Python/JS | `s_session/browser/q3-typed.js:124-137` | CC-007 卡片去 subsequent_development，改三跳只读：分型对象 → 最新批记录 entry → development 序列 §5.6 分片；不重算 order | #1491（段 B） |
| Python/JS | `s_session/browser/tb01c-client.js:2727-2740` | CC-007 卡片去 subsequent_development，改三跳只读：分型对象 → 最新批记录 entry → development 序列 §5.6 分片；不重算 order | #1491（段 B） |
| Python/JS | `s_session/browser/index.html:1028` | 文案：已实现后续发展见单元批记录 | #1491（段 B） |
| Python/JS | `s_session/browser/q3-consumer.js:Delta 应用处（行号由实施票定位）` | base+Delta→next：upserts/withdrawals 按出生序追加 | #1491（段 B） |
| Python/JS | `s_session/tests/tb02a_oracle_check.py:216-551` | oracle 按新 rule_revision 重算；旧夹具留作 layout/1 读法锁 | #1491（段 B） |
| Python/JS | `compare_r7_r6.py（#1394 线）:—` | 段A 用；段B 新增 crosswalk_layout2.py：分型对象去 subsequent_development 相等 ∧ 每代 development root == 旧载荷 bars root | #1491（段 B） |
