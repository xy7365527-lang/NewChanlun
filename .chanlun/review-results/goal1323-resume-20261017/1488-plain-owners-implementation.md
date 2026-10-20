# #1488 B2：Plain 文本、引用、字段与 JSON 拥有者候选

日期：2026-10-17。恢复树：`.sandcastle/worktrees/resume-1488`，基线 `c2c9c5640a99b98299456fd8b837c58035a2f4a6`。

状态：**源码候选、单文件 rustfmt/diff 检查通过；编译、全部 Rust 测试及原六成功合同均 NOT_RUN。存在已明确的跨文件类型断层，不能称为可编译或验收。** 本工位只修改 audit_store.rs、business_json.rs、audit_retention.rs；未碰三个禁区，不覆盖已有 raw/R/S 变更。未运行 cargo/rustc、服务/HTTP/浏览器/外盘/重型回放，未 commit/push。

## 已落的真实持有链

- `TextValue::Plain(NativeValue<String>)`：native_string/native_format/native_text、SQL text 同一原 WorkScope 在构造前准入，原 token 随真实 String。无作用域的旧 From/String/str 入口只移交无费拥有者，不伪称 Native 准入。`try_clone(None)` 对已持 Native token 的文本拒绝免费复制；有当前 scope 时独立收费、原值仍持费。
- `TextValue::from_bytes`：Owned 原分支保留；Plain 分支借用 UTF8 验证保原 String::from_utf8 错误文本，调用 A 新增 `CodecVec<u8>::try_into_native_text` 让原 Vec allocation 与原 token 一同成为 String，未 clone、未重签、未裸 try_into_plain。
- `RefValue::Plain(NativeValue<ContentRef>)`：移除不受准入的深 Clone。native_plain_ref 不做 digest 预验证；native_built_ref 使用 A 的 try_from_built。`native_ref_clone` 须当前 scope；独立复制时新旧同时持费。
- `RefFields::Plain(NativeRefFields)`：get 返回借用 `Option<RefView>`；get_admitted(s,key) 返回独立受准入 RefValue，缺字段仍 None。字段 owner 的键、引用及 buffer 由 A 原 token 真实持有。
- CodecRead 默认 content_ref_view/lookup_ref/lookup_field_ref/optional/array/fields/subset 和 CodecStore::replace_ref 全部改用 A typed 原核出口。没有先走旧 retain 输出再包装。NativeFieldPath 持 NativeValue<String> 并实现 PathView，不拆出裸 String。
- `JsonOwner::Plain(PlainJson)`：本 bin 私有字段拥有者为真实 Value 在先、原 NativeReservation 在后，只读 as_ref/Deref；没有公开 take/DerefMut/裸 into_inner。native_json_clone 在旧结构上界先 reserve，再 clone；独立子 JSON 通过借用或当前域准入复制，父析构不释放仍存活子对象的独立许可。
- `strict_json_owner_admitted` / `legacy_json_owner_admitted`：原 JsonShape::scan(bytes).parse_peak() 准入 token 直接转入最终 PlainJson；语法错误时 token 随失败栈析构。原 strict visitor 的 duplicate-key 检查位置、语法和行列核保持，子节点仅在解析内以无许可中间 owner 移动，整个构造期有外层已签发 token 覆盖。
- `json_from_view_admitted`：构造前计算旧递归构造使用的字符串/数组/树节点结构上界，原 token 覆盖本次完整结果；构造规则及内容不变，最终真实 Value 析构归还。
- CodecRead::parse_rendered_plain 及 StoreView 委派签名改为 `Result<PlainJson,String>`，默认原 strict 语法诊断仍带 `SchemaUnsupported：`。覆盖该方法的其他模块尚待接线。
- `Retained for PlainJson` 仅返回实际 payload 保留统计；没有预算 release/commit，也未用统计差额代替 token。

## 为何 JSON 使用 bin 私有 PlainJson

库 NativeValue<Value> 已提供完整只读 clone owner，却不能承接本 bin `audit_owner::native::JsonShape` 在 strict/legacy parser 前签发的原 token；先调用库 clone 会制造第二次分配和准入并改变解析峰/错误时机。主控已明确接受 bin 的专用解析 owner。其 from_reserved/into_unscoped 都私有：后者只用于严格解析器内部搬移尚未附原许可的中间节点；有 Native token 必拒绝。未公开任意 T+bytes 的“已收费”包装工厂。

库 NativeValue 的 String/ContentRef `from_unscoped` 仅用于旧 `From<String>`、`From<ContentRef>`、普通 from_str：只移动无作用域载荷，不复制，不提供 Native 资格。持 token 的显式 clone 必走当前 scope。

## 对 C/D 冻结接口

```text
TextValue::Plain(NativeValue<String>)
RefValue::Plain(NativeValue<ContentRef>)                    // 不再 Clone
RefFields::Plain(NativeRefFields)
RefFields::get(key) -> Option<RefView<'_>>                 // 借用
RefFields::get_admitted(source,key) -> Result<Option<RefValue>,CodecError>
CodecRead::native_string/native_format -> NativeValue<String>
CodecRead::native_plain_ref -> NativeValue<ContentRef>
CodecRead::native_ref_clone -> RefValue
CodecRead::native_json_clone -> PlainJson
CodecRead::parse_rendered_plain -> Result<PlainJson,String>
JsonOwner::Plain(PlainJson)
PlainJson::{as_ref,is_native_admitted,try_clone_from}
TextValue::from_bytes(CodecVec<u8>) -> Result<TextValue,String>
```

PlainJson `From<Value>` 仅用于无 scope 旧拥有值；所有 Native 构造须经原准入点。不要通过 Deref 后裸 `Value::clone`、`String::clone` 再套 From 规避预算。无 scope `JsonOwner::push/insert` 保留旧语义；Native 实例不开放无 scope 变更，这只是阻止不带账增长，**不是以拒绝替代原成功合同**。

## 明确未闭合和下步

1. **HashReader::parse_rendered_plain（history.rs 约2713）仍返回 Value，存在旧 native::Budget.reserve→commit 和 scope.reserve_native→retain。** 须改为调用上述 same-scope strict_plain_json_admitted，删除对应旧收费，而非保留旧费后二次包装。source_audit 的代理 override/cache Value 同步改 PlainJson；缓存借用与受准入 clone 保留来源判据。
2. **object_index.rs change_array/push_change/changes_at（约2126/2134/2197）直接匹配 `Plain(Value::Array)`、把独立子值推入裸 Vec、拆出 Vec<Value>，目前是显式编译断层。** 需真实 array builder 在 buffer 扩容前签 token、让子 owner 的原 token 随聚合 owner 移动；可设计私有 Value+buffer token+子 token 集合，token 集合自身也要 CodecVec 持费。不可简单暴露 PlainJson.as_mut/take，亦不可把子 token drop 后只留下裸 Value。旧无 scope changes_at 可以另保兼容出口，但 Native 主链须返回/借用持费 owner。本包未实现这条聚合接口。
3. `JsonOwner::push/insert` 当前实际内部调用来自 `project_wire_integers_with_owner`（business_json.rs）；其数组/对象创建入口只接受 None 或 OwnedBudget，没有 WorkScope，原无 scope/Owned 路径保持。生产 `projection.rs` 若要把这条投影接 Native scope，须提供显式 admitted 版本并核结果 owner，不得调用新的防错拒绝当成功。
4. RefFields 更改后，history/source_audit/native_record_fields 原 CodecFields 或 get(...).view() 消费须同步。D 已独占 native_record_fields 窄片在改；其他 C/D 全面迁移仍未派发完成。
5. Text/Ref Plain 直接构造、深 clone、裸 String/ContentRef 返回的其他模块都需适配；主控只在 q_runtime/q_http_input 处理无 scope 兼容，不代表全部生产面已接通。
6. audit_store 中 native_admit、native_vec_room/std_map_room/tree_room、native_ref_fields_bytes 的旧兼容 retain，以及 native_verify_original::Current::read/checked_read 兼容输出仍存在；它们不属于已完成的 typed text/ref/JSON/path 输出。真实消费者要迁 CodecVec/BusinessMap/NativeTree/ScopedFrame，不能将这些残留当作已闭合。
7. 三禁区、16/8 MiB、六成功输入不动；新 inline owner 元数据诚实反映真实大小，不降低统计。原六成功和完整 publication/回放生命周期须在类型闭合、独审后再做获准的远端执行。

## 源码锁与检查

新增 3 个测试（NOT_RUN）：

- text/ref/JSON 真实结果跨 scope 后继续持费；重复三轮；独立子 JSON 在父析构后仍有效并持费；逐项 drop 父账回零；离域免费 clone 拒绝。
- duplicate key 与坏语法对旧 session_protocol::strict_json 完整错误字符串对拍；失败析构归还原 parse token。
- Native JSON 无准入可变/拆出保护，不改变原数组及持费；drop 后回零。

已调整既有两组 Native text/JSON 及 ref 测试的“永久保费”断言为实际 payload drop 退费，并保持同输入的值、缺字段、类型与引用序列化检查。所有 Rust 测试和编译均 NOT_RUN。实际仅 `rustfmt --edition 2021 --config skip_children=true` 三文件及 `git diff --check` 通过。

`1488-plain-owners-working.patch` 是这三文件相对恢复基线的当前完整差异，**包含前一 raw B 切片**，不是纯 B2 增量；不得把其中已有 raw 改动重复认领。

SHA256：

- audit_store.rs：`8161577d099f4deddb0d2cfda228e71c2b6c76ebca702e75e0f95f7d163789a8`
- business_json.rs：`d93afd648fdf4664c1a61bb8392ad361cca2aa26fb4fd8c397249b3fa879244e`
- audit_retention.rs：`bdda0deea41ae536a7433242d23a0bb2b2683c9787d91494ce33112f3b67ed48`
