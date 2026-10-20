# #1488 C1a：来源值与 memo 真实持有候选

日期：2026-10-17。工作树 `.sandcastle/worktrees/resume-1488`。只改 `rust/src/bin/s_session_v3/source_audit.rs` 与 `source_audit_oracle.rs`。编译、Rust 测试、六成功与全图验收均 **NOT_RUN**；仅 rustfmt2021（skip_children）与两文件 diff --check 通过。

## 已完成

- NativeFieldRefKey 的 format/root 与 NativeWitnessIdentityKey.object_id 改 NativeValue；保完整四字段、digest、capture/slot/mi 身份。构造在当前 CodecRead 同 scope 准入，去 native_admit 后裸复制。
- 字段缓存键的请求名用 CodecVec<NativeValue<String>>，Hash/Eq 按完整原顺序，包括重复项；未以排序或集合弱化缓存键。缓存内容用 NativeRefFields，返回命中结果时当前 scope 独立复制，旧缓存仍持原许可。新结果的规范唯一字段用于缓存，不把重复请求名字误送入禁止重复字段构造器。
- scalar 缓存改 BusinessMap<NativeFieldRefKey, PlainJson>；miss/hit 的独立 JSON 都走当前 native_json_clone，缓存与返回值各持对应真实分配。字段、scalar、witness entries、raw originals 均改 BusinessMap，原扩容点换 native_map_room。
- ArrayAuditSource 的 typed content_ref、path、borrowed-field、fields/subset、array 全接 Native API，parse_rendered_plain 返回 PlainJson。保 full_arrays、deadline、Owned role 临时切换及 same-capture 原件完整闭包资格；sidecar subset 直接移动 NativeRefFields 到 RefFields::Plain。
- SourceFields::get 返回借 RefView；get_admitted 仍提供当前 scope 的独立拥有副本。无 scope 测试 fixture 与空字符串构造适配新型；测试逻辑对拍的 JSON clone 明确只发生于 cfg(test) 比较材料。
- NativeRawFacts/NativeRawLedger.originals 持真实桶 token；跨 into_facts 移动原许可；clear 用 retain 清条目而保实际桶容量许可，drop 才归还桶费。RetainedDelta 仍仅统计，没有用统计退款。
- 接入主控新 `generation::witness_identity_native`；成功计算且 stored ID 比对通过后，原结果 owner 移入 memo。overlay 仍无缓存资格。测试原 expected closure 保留旧 `witness_identity` 作为普通结果对照。
- 生产来源与冻结 oracle 均接主控 `Cc004Producer::for_source_audit(profile,rule,s.codec_work())`，删除两处原初始化 native_admit。只读 wrapper 持初始化费；没有调用新增量来源核替代冻结 oracle，原循环、业务判据与错误保持。

## 新增必要测试源码（NOT_RUN）

1. `native_raw_facts_keep_original_bucket_permits_across_scope_and_move`：真实 ledger.advance 增量、回退再前进，核来源和 hash 复用；into_facts 不改变 live，离开 scope 保费，连续三轮最终 drop 回父账基线。
2. `source_identity_producer_outlives_scope_without_freeing_initial_strings`：新只读 producer 和旧普通 producer 的 typed_identity_scoped 同值；离域初始化费继续存在，连续三轮 drop 回基线。

原 source field 全四字段伪造/名字顺序与重复/overlay、witness success-only/cut faults/closure、raw winner/revision/collision/rewind/frozen oracle 测试均保留并适配。没有执行这些测试，不能把代码中断言当结果。

## 明确未闭合

- `ArrayAuditSource::read/checked_read` 仍委托底层兼容返回，底层 frame/body retain 缺口属于库/History 边界。这里保留 `checked_read_scoped` 委托；未靠新拒绝替旧成功，也未拆 token。
- 旧 CodecRead 的裸 ContentRef/CodecFields 兼容方法仍有实现；本包迁移 typed 消费入口，未擅改 trait 返回型或声称全部旧兼容 retain 已退役。
- source_prefix 共享 Rc/坐标、source_contributions、source_order、history、跨请求池等不在本包；SourceFields::get 借用签名已告知主控，其他模块的调用需对应工位核对。未修改 #1419 恢复树。
- 主控新 witness/producer 接口已静态找到，非本工位独立实现；其自身峰值/元数据/错误顺序须独审。SourceIdentityProducer 仅覆盖初始化拥有值，不承诺整个可变 Cc004Producer 已完成 RAII。
- 没有 cargo/rustc、服务/HTTP/浏览器、外盘访问、重型回放、commit/push/dispatch/main 合入或关票。16/8 MiB、原六成功与三禁区未改。

## 冻结 SHA256

- source_audit.rs：`cfc7587865864de4ec8391bace3b9e054283d9bfb7963531b309aa702b87062d`
- source_audit_oracle.rs：`41a7c0681eb801af236cbc0dda7f08fd6f8eac8ab140b00945a55ff2f02f51bf`

`1488-source-values-working.patch` 为这两文件相对恢复基线完整差异。需独立源码审查和后续获准远端编译；本报告不作全票完成结论。
