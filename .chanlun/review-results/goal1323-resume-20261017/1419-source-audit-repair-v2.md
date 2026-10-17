# #1419 来源审计返修 v2

2026-10-17。绑定 #1419 的工作草稿。针对 `1419-source-kernel-review.md` 两项 HIGH；不代表运行验收。

## HIGH-1：区分事实代际与事件代际

已保留 payload.object_generation 的 fact_generation 原义。full 从当前 state 顶层 generation 读取；Native 从 SourceMaterial.root 的同一字段读取；test-only source_delta 从已核 FixtureCut.root 读取。可信值沿单对象共享核传参。change 的规范 slot[0] 与 data.known_at.generation 均须等该可信 cut 代际；不读取 first_known_generation，不改变生产者事实代际。

核查真实组装链：generation.rs build_cd/build_ef 的本次输出追加到本次 typed，随后当前 shape+typed 生成 object_nodes，objects_seq.sync 以该当前列表取代旧列表（约 1058–1141）；prepared full 与 Native 入口均送 generated.generated_state；construction_set::qualify 从 current.objects 建差分。当前材料不合并持久 object_index 的全部历史 change。历史 cut 独立审计仍用其自身根 generation，不以最新视图覆盖。Native 新增活跃 change 索引并计入 retained_bytes，每代检查这一真实代际依赖，即使恶意保留旧 change、内容键未变也不能沿缓存免审。

## HIGH-2：按见证原件投影关系

共享 audit_relation 按实际发出的关系分支核 source/target、版本和来源，均不运行分类器：

- C derived_from：当前 first/second sequence 对象必须与段 witness 指向同一 obj3，target 为该段实体，version 为段 entity_revision；来源等段来源。
- C member_of：当前 target sequence 属于 witness 段，source 为当前 CC010 笔，并在 sequence.stroke_refs 与 facts.raw_elements 以同一 stroke_index 对应；来源等笔 start/end 的 source_coords 并集。
- E attempted_from：source 为 attempt，target 为 units[0] 当前段，version=1；来源等 attempt。
- E member_of：source 在 center.members，target 为当前 witness center，禁止 UPGRADE_PENDING；版本等中心修订，来源等该成员段。
- E connects_left/right：source 与 assignment.unit 的实体/obj3 一致，target 对应 left/right_center_id，版本等当前目标中心修订；来源等该段。
- F position_of/against：source 为当前 position witness，target 分别对应 unit/center，版本等 witness.center_revision；来源分别等单元/position witness。
- F pair_from/to：source 为当前 pair，target 对应 A/B 中心，关系版本等 pair.entity_revision；双方 a_revision/b_revision 对当前中心修订，来源等 pair。

未知关系名或错误 kind/witness 组合直接拒绝，不只查白名单。change 同时核事件实体、目标 kind、当前实体索引、after 原件；withdrawn 要求 before.entity_id 一致且当前实体缺席。

完整路径新增当前实体索引；Native 新增 structure_entities 与 structure_entity_dependents 并计保留费，实体删除/修改经反向索引使关系重核。CDEF 的关系端点以及 pair 的双中心、assignment 的左右中心、position 的中心、change 的主体均登记实体依赖；既有 obj3 反向依赖保留。没有全量来源审计兜底。

位置关系版本的边界：按生成器真实规则核 witness.center_revision。content_structure 会在位置几何未变时保留旧 position 事实与其中心修订，因此未擅自强制它等于目标中心最新 entity_revision。pair 的 a/b_revision 则是生产者显式当前双中心修订，照此核验。

## 测试源码与检查

新增六项测试源码（含一个仅 native feature 启用）：

1. 十个实际发出关系分支正控，以及不存在/存在但错绑端点、未知关系名、错误版本、换 witness 负控。
2. 见证仍在而当前中心端点删除，关系必须拒绝。
3. Native 实际 Map/Tree 反向依赖与 NativeObjects 查找器的删除端点负控；调用生产 dirty_from 和共享核。此测试不是完整 SourceMaterial/owner 集成测试，不冒称完成该门。
4. C/E/F change 的 fact_generation=1、事件 generation=28，formed/updated/withdrawn 的代际正负控；过期 known_at 与错误当前代际拒绝。
5. after 字节真实但变更实体错绑的负控。
6. 冻结 TB02D 数据经真实 parse_layer_with_inclusion_facts_store/build_cd 工厂生成非空 change，实际 1/28 正控；随后同份 prior 暖续跑到 29，确认未复制旧 change；旧 change 用 29 审计必须拒绝。断言 change 数大于零，避免空真。

原五项请求/头/子边测试继续保留。单文件 rustfmt2021 与 --check exit 0；限定已跟踪四文件 git diff --check exit 0。**所有 Rust 测试及编译 NOT_RUN**，没有 cargo/rustc、服务/HTTP、外盘或重型验证，没有 commit/push。

本轮实际修改 source_audit.rs、source_structure.rs、source_contributions.rs、source_delta.rs；source_pending.rs 保持上一候选哈希，未改四轴逻辑。未改生成器/分类器或其他代理文件。

## 冻结 SHA256

- `source_audit.rs`：`0c43214deaa9fa49184fb3bec6d484cf15ee9bee36dbbbbf22c56a7fe6fcad73`
- `source_structure.rs`：`5dd619d794f97a06800d4b92210f253c7afc0ddad84ae80b83c3516685255111`
- `source_pending.rs`：`ec80d4e7784e23ffa87fcb30e54467bcac7941f1bff8dea064c3af63ed707633`
- `source_contributions.rs`：`507a66ef81681f02f7ae580fac2493488b54a6de218b42e88b5e224a83c307ce`
- `source_delta.rs`：`f5975d169300e27d4e094454736dd64f71aef5897631f1c54d16133a08f4b810`
