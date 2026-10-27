# #1488 raw Native 许可持有实现

日期：2026-10-17。状态：**raw 垂直切片源码已落，3 个生命周期锁 NOT_RUN；整体 Native 生命周期仍未闭合，不是六成功或 #1488 验收。**

工作树 `.sandcastle/worktrees/resume-1488`。B 独占修改 `rust/src/bin/s_session_v3/{audit_store.rs,business_json.rs,audit_retention.rs}`；经主控补充授权，仅在 `audit_owner.rs` 末尾追加测试模块。`v2/content_store.rs` 没有修改，S 的 selective flush/租约实现保留。三个禁区没有修改。没有 cargo/rustc、服务、HTTP、外盘、commit/push。

## 实际变更

- `CodecRead::native_codec_room/native_map_room` 调用 A 的 `try_reserve_native`；原 reserve token 存进 CodecVec/BusinessMap 实际拥有者，无 retain 或二次新账。
- RawPrefixFacts 两张 HashMap 改为 BusinessMap，winner 身份键为 `NativeValue<String>`；key 内部 String 的原许可随键持有，hash buffer 的许可随 map 持有。保持最终 revision/first-on-tie 决策与 identity 排序核。
- removed/added/winner_edits 全部使用 CodecVec，编辑的既有 move 签名保持；禁区 PendingRawState/BoundRawAdvance/IssuedRawAdvance 接收原类型，不添加生产旁门。
- ordered/changed 使用 NativeTree：树先析构，许可后析构；节点上界随最高同时条目数增长，删除重插不按历史累计；clear 真清树后释放节点许可。许可列表自身为 CodecVec，buffer 许可也跟 owner；clear 保留真实 buffer 时保留该费用。
- 保守 Retained 统计继续独立于预算退账。加入 NativeValue 与 NativeTree 的 Retained 实现；ordered 新许可列表容量变化在实际变更处记入 RetainedDelta，没有按全 live 差或统计差释放。
- 新增 `native_owned_text(&str)` 和 `native_owned_ref(RefView)`，直接从原准入点返回 A 的 NativeValue；没有通过旧 retain 接口后二次包装。

## 禁区 move 与旧释放核对

新增 audit_owner 私有测试模块直接调用现有 `PendingRawState::advance`，观察其实际 `RawPrefixFacts` 和 winner_edits move；再走 `BoundRawAdvance::into_issued` 与 `IssuedRawAdvance::bind`，scope 返回之后仍持费；分别 drop pending/token，最后父账回基线，重复 3 次。测试只用测试模块可见的既有私有构造，未开放新的生产接口。

business_json 另有 2 锁：真实 RawPrefixIndex→into_facts_retained 与 edits 跨 scope 移动，RetainedDelta 与完整 Retained 核对；NativeTree 删除重插不累计、clear 释放节点但 buffer 仍按实际生命周期持有。

静态检查 raw 路径没有既有 `budget.release(raw_payload)` 需要抵消：旧费用点就是 audit_store reserve→retain；本次将这些对应许可移动到实际 owner。`RetainedDelta::release/tree_len` 只修改候选保留统计，不调用 Native 父账 release。construction_state 的 retained_bytes 是统计/上限输入，未改成资源退还。非 publication 的独立旧 Budget 资格路径未修改；其整个父账逻辑仍需主控全图复核，本切片不将统计与 permit 两种准入相加视为已验证正确。

## 验证

四个修改文件 rustfmt --edition 2021（skip_children=true）成功，git diff --check 成功。**三个 Rust 锁均 NOT_RUN，未编译。** 16/8 MiB 及原六成功合同没有改动。类型包装带来的 inline/保留统计变化真实保留，不能声称六成功已通过。

## 仍需 C/D 迁移的明确消费面

`1488-raw-consumer-migration.txt` 枚举当前 117 处 native helper/裸转换命中，仅作分诊，不能当闭合判据。

- C：source_audit/source_audit_oracle 的 TextValue::Plain(String::new())，history 的 `TextValue::Plain(u.batch_id)`，source_contributions/source_pending/source_delta/source_order/source_prefix/cross_request_source_cache 的 native_* 返回与拥有容器。
- D：object_index/aux_index 的临时 Vec/map/tree，catalog_evidence 的 Plain 文本裸返回，native_record_fields/control_audit/request_audit 的 refs/JSON/fields。
- 扩展消费面：q_runtime/q_http_input、v2/evidence_store 的直接 Plain(String) 构造；v2/mod 的 Plain 文本解构后返回 String。不能在 NativeValue 添加裸 into_inner 来消除类型错误。
- B 后续：TextValue/RefValue/RefFields/JsonOwner 的 Plain 内型尚未更换，现有 native_string/native_format/native_plain_ref/native_json_clone/native_vec_room/native_std_map_room/native_tree_room 仍有 retain；本轮不全局改型留下调用断层。
- v2/content_store：read_material_admitted、checked_read/current_checked→ScopedFrame::into_frame 尚保留兼容路径；需要 typed NativeValue<CheckedFrame> 真实消费迁移，不能仅新增无消费者的方法宣称完成。

冻结后续接口：A 的 `NativeValue<String/ContentRef>` 只读 as_ref/Deref + admitted clone；CodecVec/BusinessMap::try_reserve_native；NativeRefFields::get 借用，iter 借用；path::*_native 系列、ScopedFrame::into_native 直接带原 token。NativeTree 没有 DerefMut/裸消费出口。所有更广路径须转接这些原始持有者，不回到 retain 再包装。

源码 SHA256：
- audit_store.rs：844dd7163a4a832647aaafc9d4526cc6027afd23da791a4b69a92202c2c77d7d
- business_json.rs：53a8715adc2d576a342e9d16503e2bb93e4bf681ccab21fd10e842d2dca40f9b
- audit_retention.rs：f5b373e87cffc0763a94648b50c19df4f447844e062679ec6b1bd3c70d1e84d0

补丁仅含 B 三文件；audit_owner 测试追加原文另封存 `1488-raw-owner-test-appendix.rs`，避免把 R 的既有修改误归 B。
