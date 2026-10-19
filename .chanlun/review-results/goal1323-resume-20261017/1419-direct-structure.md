# #1419 C/D/E/F 直接内容构造候选

> 工作草稿；绑定 #1419 / #1420 / #1424 / #1323。2026-10-17。尚未编译、运行测试或通过独审，不能作为全图验收。

本轮已实际移除旧完整业务 JSON→CValue 桥，实装同次段事实的 C/D obj3 构造以及同一次中心扫描的 E/F 投影。源码修改文件：

`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1419/rust/src/bin/s_session_v3/content_structure.rs`

`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1419/rust/src/bin/s_session_v2/content_producer.rs`

基础 HEAD：`018c6542e5ce24d56fde354330d2571f64ffaefc`。本轮归档 SHA256：

- `rust/src/bin/s_session_v3/content_structure.rs`：`1b3001a12ffd822700a14475b6b90650707242301cd329ca50deb53f2da9bda3`
- `rust/src/bin/s_session_v2/content_producer.rs`：`57c02cf2c4a8810e4031048533826f5174f80c88af97fe6154718bfd4ee0eacd`

补丁：同目录 `1419-direct-structure.patch`。这是相对 HEAD 的这两个文件累计未提交差异，包含先前 codec 修复，不能说成只有本轮增量。没有 commit/push；`content_producer.rs` 仅新增显式四轴类型和请求工厂，旧 A 默认请求推断不变；没有修改 StageOwner/三个禁区文件。

## 已落源码

1. `StructureEvidence` 同时借用层结果、`SegmentFacts`、`StrokeFacts` 与同次 `SecondFoldStore`。`restore_second_cursors` 从上一代已核 C/D version 根恢复；调用者先恢复、再调用现成 parser `_store` 接口一次，然后交 `build_cd`，投影器不重新解析。
2. C/D 直接消费同次 seed/FIRST/SECOND/strict/construction/fact_vector/termination 事实。事实类型走直接 Serde→内容值适配，数字沿 s-segment/3 成十进制文本；不经旧对象 Value、JSON 字符串、旧 Builder。共享事实、笔引用和 prior 的 data/known_at 通过值根接边。
3. 实体 ID 保持既有配方；对象 ID 只来自借入的实际 `Cc004Producer`。先生成 FIRST/SECOND obj3，再生成引用真实序列对象的段，再生成见证引用真实段对象的 relation/change。普通关系的 subject/object 仍是实体身份。输出完整候选映射，包含进行中候选，未使用完成段枚举索引作为身份或来源。
4. E/F 使用另一工位刚提取的 `tb02e::units_from_segment_rows` 共享输入合同，行直接来自同次候选、笔端价、成员笔下标与实际 C/D 对象映射。中枢扫描只执行一次，seed/window/roles/assignment、tested_positions、pair_facts 全部消费该同一次 `center_scan`，不重写中枢判据。
5. E/F 保留 Zn 外缘、角色来源、同价见证、连接/未完成单元、结算锚、完整比较矩阵；≥9 段保持既有 pending 与动态变量 null 语义，扩展/新生的 realization 与 destruction 仍使用 #1424 冻结字面量。更新/撤回与前值根按旧投影的生命周期规则构造。
6. Prior 只接记录根，完整 codec 闭包核验、当前实际 producer 重新派生 obj3、schema/semantic/policy/view 头核验、fact_key/slot/实体一致、实体去重。C/D 非空段 prior 必须携版本根；EF 非空 prior 必须各携一条中心/中枢对版本根。旧载荷只读取具名小字段和值根，不重渲染完整对象来构造本次对象。**这只证明给定记录自洽；活跃发布态完整选择仍由调用者独立索引负责。**
7. 保留三项 codec 测试并将其旧 JSON 适配 helper 限定在 `cfg(test)`。生产唯一保留的 JSON 解析为标量文本与 parser 的权威 checkpoint wire；主控明确允许后者，未复制 checkpoint 选择政策。

## 已冻结接线接口

```rust
StructureEvidence {
    segments: &[Segment], segment_facts: &SegmentFacts,
    stroke_facts: &StrokeFacts, second_store: &SecondFoldStore,
}
StructurePrior { segment_records: &[Built], version_record: Option<&Built> }
EfPrior { records: &[Built] }
StructureContext {
    fact_generation: i64, generation: i64, cut: &str, frontier: i64,
    known_at: &Built, all_sources: &[usize],
}
StructureOutput {
    objects: Vec<TypedObject>, relations: Vec<Built>, requests: Vec<Built>,
    segments: Vec<SegmentObject>, waiting_reasons: Vec<&'static str>,
}
SegmentObject { candidate_index: usize, entity_id: String, object_id: String, sources: Vec<usize> }
```

调用顺序：

```text
restore_second_cursors(nodes, producer, cd_prior, fold)
→ parser::parse_layer_with_group_evidence_store(state, config, fold)
→ build_cd(producer, nodes, evidence, cd_prior, context)
→ build_ef(producer, nodes, evidence, &cd.segments, ef_prior, context)
→ 根生成器合并对象/关系/requests 及真实等待原因
```

`known_at` 由调用者提供真实共享根；字段沿旧 wire：generation、input_frontier、receipt_id、received_at、semantic_commit_ns。`fact_generation` 来自当前权威全事件事实代，不等同发布 generation。producer 必须是当前实际规则/档案且已绑定真实来源的同一个实例。

EF prior 选择：CC-016.seed_attempt/center、CC-017.roles、CC-022.assignment、CC-018.position、CC-020.pair 的真实完整 kind（源码 KINDS 定义），加 CC-056.center_version/pair_version；不传 relation/change 历史全集。

## 测试源码与当前验证

现有三 codec 测试均保留。新增三项行为测试源码：

- `construction_requests_distinguish_false_outside_domain_and_missing_evidence`：同一显式请求工厂实构造含 false 的完整向量、已知偶数域外、奇数片段缺向量三种请求，逐请求核四轴，不能用 false 代缺证据。
- `cd_direct_frozen_cases_match_oracle_and_close_real_object_references`：所有封存 TB-02-D raw cases 经真实 parser；新旧对象按事实键归一身份后逐字段对拍；非真空 FIRST/SECOND 与对象引用闭包；同事实重放不伪造修订/change；缺源和错误空游标簿拒绝。
- `ef_direct_prefix_revisions_pending_and_retained_history_match_oracle`：有界段层输入的全部前缀，上一 cut 真正产物作下一 cut prior；新旧 E/F 全投影对拍、relation witness 闭包；必须实际经历已成立 pair→≥9 pending。该段层测试不冒称覆盖真实 parser→E/F 接线。

两项全投影测试还核 exact 五字段请求、规范二元键、真实 obj3 主体及对象 fact_key 一致、完整四轴、源坐标同根、无重复请求、每实际子轴计数；E/F 覆盖待升级 pair 请求 incomplete 与已判 pair / 已知未归属 complete 的区别。

已运行且成功：两文件 `rustfmt --edition 2021`、对应 `git diff --check`。**按主控禁止本机 cargo，未编译、未执行这六项测试，也没有远端 CI 成功证据。**

## 实际分类请求四轴与来源

请求 exact 字段为 `request_id, subject_id, axes, reasons, source_coords`。键固定为 `[object_fact_key, [cc, axis, "structure-classification/1", path, obj3]]`，与消费者二元槽位合同一致；主体是真实本次 obj3，版本及事实内容由其身份绑定。第一元复用对象 `kind` 和 `payload.slot` 内容边；显式工厂在同一节点读集渲染键，不能将含外部 slot 边的值交给 A 的私有 scratch。主控另行修目录按具名 CC 分轴。

| 请求 | 同次证据与四轴口径 |
|---|---|
| CC011 length | seed、candidate.construction 及每实际 termination_attempt 的 ConstructionFact.length；已知长度 COMPLETE。 |
| CC011 construction_conditions | vector Some（包括 false）为 SUFFICIENT/IN_DOMAIN/COMPLETE；已知非奇数>=3为 OUTSIDE_DOMAIN；域内缺向量才 INSUFFICIENT/DOMAIN_PROOF_MISSING/INCOMPLETE。 |
| CC011 direction | 1111且direction已知为完整；明确构造失败为 OUTSIDE_DOMAIN；1111却缺direction为缺证据。 |
| CC013 semantic_termination_result | 同次 first/gap/second/strict/termination结果；NO_FIRST、FAILED_GEOMETRY、KILLED和完整前缀PENDING为已计算语义；资源截断且未有决定性结果为 SUFFICIENT/IN_DOMAIN/INCOMPLETE。 |
| CC014 known_fact_vector | 同次 BreakFacts 的 new_segment_formed/segment_broken Option；Some(false)完整，资源截断None为 SUFFICIENT/IN_DOMAIN/INCOMPLETE。 |
| CC014 scan_coverage | 范围与覆盖事实已由同次parser给定；INCOMPLETE_RESOURCE_BOUND是已求出的覆盖状态，请求自身COMPLETE。 |
| CC016 seed_condition_vector/core_order | 共享中心扫描的完整三单元五谓词和核心比较；false向量完整，strict=None缺证据；core_order直接来自完整 typed 三元组。 |
| CC018 unit_core_position | 同次tested_positions及冻结strict核心和完整单元范围，COMPLETE。 |
| CC020 center_pair_development | 同次相邻pair_facts；Evaluated完整，即使上级realization未构造；≥9 pending缺有效外缘，INSUFFICIENT/DOMAIN_PROOF_MISSING/INCOMPLETE。 |
| CC022 assignment_status | 同次完整读法unit_facts及进行中单元，member/connector/not-yet已知归属均COMPLETE，未完成单元未归属不是false或漏算。 |

上述本次请求 validity=CURRENT；旧对象历史与 prior 保持，不把旧CURRENT事实冒充本次缺证据请求。等待原因独立汇总，不替代请求四轴。

## 尚未完成的验收与边界

- generation/prepared/object_index 实际接线由另工位负责；规则域、初始化与完整 22 轴目录由主控负责。本文件不擅改 V3_RULE_REVISION 或预算，不构造新的宽松分支。
- 实际请求按 `SPEC-COVERAGE-INPUT.md:185–197` 的具名子轴构造，`TypedObject.request=None`，唯一请求集合是 `StructureOutput.requests`；CC012/017/055/056 不臆造逐对象请求。generation 接线代理已确认只收该集合。分类来源、目录分轴、恢复/查询仍需联合独审与运行验证，不宣称知识请求已全链验收。
- `StructureEvidence` 生命周期保证借用有效，不从类型上证明其字段出自同一次 parser；生产接线与错配测试仍需证明因果。
- 完整恢复/进程重启、完整来源账、增长构造/资源审计、同输入双跑、新旧域兼容及远端执行仍待验证。新代码使用真实共享节点，但尚无成本/资源合同通过证据，不允许用放宽预算掩盖。
- 需要独审实际 diff 和行为测试结果后才有接纳资格。#1419/#1323/#1279 不能据此关闭。
