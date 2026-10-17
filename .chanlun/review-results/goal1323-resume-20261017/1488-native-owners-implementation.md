# #1488 RAII A：Native owner 与 typed codec 输出实施

日期：2026-10-17。工作树：`.sandcastle/worktrees/resume-1488`，基线 `c2c9c5640a99b98299456fd8b837c58035a2f4a6`。

状态：**候选源码已落；rustfmt 与 diff 检查通过；编译、测试、六成功合同全部 NOT_RUN。不是 #1488 或全图完成。** 本工位重派替代旧无反馈工位，未覆盖 R/S 或 B 的改动。

## 实际改动

- `capture_allocation/work.rs`：`CodecVec<T>` 在 storage 后携原 `NativeReservation`，`CodecVecIntoIter` 同序接走许可。新 `try_reserve_native(work, additional)` 先保留旧 buffer 与原许可，再签新完整 buffer 许可、fallible 分配、移动元素，真实释放旧 allocation 后换许可。失败不动旧内容/旧许可。clear/pop 不释放 backing capacity 的费用；IntoIter 提前 drop 释放剩余元素和 buffer，移出的元素仍由自身 owner 持许可。
- Native 容器在旧无 scope reserve/try_push 中只能使用既有容量；需要增长时由显式活动 scope 准入。带 Native token 的 `try_clone`、`try_into_plain` 不允许擦除账本。无 scope 普通 Plain 与 Owned 旧语义保留。`from_plain` 本身不宣称已收费；已有非零 capacity 且没有 token 的对象不能借新 Native 准入重签来掩盖旧 retain，调用方必须回到原构造点。B 的 raw 路径从空 owner 开始。
- `capture_allocation/audit_json.rs`：`BusinessMap` 原 map 后持 Native token；新同名 native reserve 使用现有 SwissTable 桶档结构上界，保留旧表时为完整新表签许可，搬迁键值后释放旧表和旧 token。失败发生在 drain 之前。Native insert 禁止暗扩桶；remove 只析构真实删除键值，表容量许可保持到真实换表或 drop。
- 新 `capture_allocation/native_owned.rs`：`NativeValue<T>` payload 在先、permit 在后，没有公开任意 T 包壳工厂、没有裸 `into_inner`、没有 `Clone`/`DerefMut`。只读视图、Debug/Eq/Ord/Hash/Serialize 委托原 payload；String 提供 Borrow<str>。具体受准入入口为 String `try_from_str`/`try_format`、ContentRef `try_from_view`、JSON Value `try_clone_from`；各有当前 WorkScope 深复制入口。ContentRef 只复制四字段，不提前验证 format/root，不增加资格。JSON 不开放拆子树的可变入口。
- `ref_owner.rs`/`path.rs`：复用原完整路径、数组及字段遍历核心，在原 `reserve_native` 点把 token 附于 typed 引用。新增 `lookup_path_native`、`lookup_path_view_native`、`lookup_optional_field_native`、`lookup_array_items_native`、`lookup_fields_native`、`replace_batch_placeholder_native`。数组持费容器与每个引用各自持费；字段用真实向量、键、引用分别持有原许可，不伪称 BTree 节点分配。`NativeRefFields` 提供借用 get/iter，未引入无条件深复制。
- `closed.rs`：给 typed 输出新增 native_vec_room，沿同次 WorkBudget::Scratch 准入；旧输出路径没有强制换类型或改拒绝合同。
- `job.rs`：`ScopedFrame::into_native` 将真实 frame 与原 permit 同时移动为 `NativeValue<CheckedFrame>`；新增借用 usage。没有重新签费，也不改帧核验资格。
- `capture_allocation.rs` 仅导出新模块。

## 对 B/C/D 的冻结接口

```text
CodecVec<T>::try_reserve_native(&mut self, &WorkScope, additional) -> Result<(), CodecError>
BusinessMap<K,V>::try_reserve_native(&mut self, &WorkScope, additional) -> Result<(), CodecError>
NativeValue<String>::try_from_str(&str, Option<&WorkScope>)
NativeValue<String>::try_format(fmt::Arguments, Option<&WorkScope>)
NativeValue<ContentRef>::try_from_view(RefView, Option<&WorkScope>)
NativeValue<serde_json::Value>::try_clone_from(&Value, Option<&WorkScope>)
各具体 NativeValue 的 try_clone_admitted(&WorkScope)
NativeRefFields::{get,iter,len,is_empty}
From<&NativeValue<ContentRef>> for RefView
ScopedFrame::into_native() -> NativeValue<CheckedFrame>
```

新的 Vec/map 许可承接发生在原 `reserve` 当场，未执行 retain 后重签。跨代新 scope 的父账身份由现有 publication/store 绑定链负责；没有保存旧 WorkScope 来暗续约，也没有按 live/charge.total 差猜释放。新字段 wrapper 的 token 是内嵌字段，没有为 token 另建 Arc 元数据；NativeReservation 克隆的是原父 Arc，而不是新账。容器 buffer 以实际元素类型大小计费，包含内嵌 token 的大小。

## 源码验证锁（9 项，NOT_RUN）

1. String 离开 scope 后只读/销毁，同父重复申请/删除回到零，独立 clone 同时收费。
2. IntoIter 提前销毁释放剩余元素与 buffer；已移出元素独立持费。
3. Vec 扩容峰覆盖新旧 buffer；clear 保留实际 capacity 费用。
4. Vec 扩容失败保旧值/旧许可，无 scope 不可暗扩容。
5. BusinessMap 多档增长、String key Borrow 查找、删除真实退键费、最终归零。
6. BusinessMap 增长被拒绝时不 drain 旧表。
7. 坏 format/root 的 ContentRef 复制不提前验证，随后 digest 错误与原对象一致。
8. payload 析构观察到自己的许可仍存活，随后才退费。
9. typed path/array/fields 与旧核同输入结果逐项一致，跨 scope 结果销毁归零；坏 format、坏 root、缺字段错误保持原分类。

验证只运行了单文件 `rustfmt --edition 2021 --config skip_children=true` 与 `git diff --check`。**没有 cargo/rustc、正式服务/HTTP/浏览器、外盘、commit/push。** 三禁区 `construction_state.rs`、`audit_owner/native.rs`、`capture_allocation/scratch.rs` 未修改。

## 仍未闭合的生产面

B 本轮只接 raw owner 与两个容器 reserve，TextValue/RefValue/RefFields/JsonOwner 的 Plain payload 尚待 C/D 统一迁移。旧 path refview/ContentRef 输出、CodecContext::plain_vec_room、FieldOutput 默认兼容输出、ScopedBody::into_plain、ScopedFrame::into_frame 仍有 retain；typed API 存在不等于这些生产调用已替换。冷审计、source/history、object/aux 临时/持久载荷仍需全链迁移与独审。普通标准 Vec/HashMap/BTreeMap 的既有费用，不因新增通用 owner 自动转移。原六 success、16/8 MiB 及同父 publication 生命周期均须最终真实验证。

**不宣称任意调用者的深复制或临时算法分配均受此 owner 自动计量。** 比如只读 payload 的显式 Clone、slice sort 等仍是调用方新的分配；生产必须使用当前域的受准入入口并由 C/D 核具体调用面。新 API 的 provenance 拒绝不算旧成功合同通过。

## 冻结 SHA256

```text
4c684b4b00cabc68d6ce5f6d97c44b3d1ee7db0b605e82572af91fb9a88ddfa0  rust/src/s_content/capture_allocation.rs
7cec900e7c15d0f21c1e6c71a29c2d5b52e4ac1e115e306ddfadb0935afb8c2c  rust/src/s_content/capture_allocation/work.rs
9cce0fd1396d7842a5c7fa9ed7bc7e6209189b7356865be6443899c5c66653c2  rust/src/s_content/capture_allocation/audit_json.rs
d92605a9369007ba053f50e4501329192a90ff71a6d7a555977b7d022acccc52  rust/src/s_content/capture_allocation/native_owned.rs
c70015408cd47da08d8947b949d3917c300780134681fcb9f7095d987c30f61b  rust/src/s_content/path.rs
de26366926d42c21747c97dcdb19f06fd5c23b542e780882de270a3dfe729a60  rust/src/s_content/closed.rs
9f6c3b48c3447a0a82de397029658f164d0f8b7ea06c687c024c8b43b8581d34  rust/src/s_content/job.rs
d556a50a52a372a97744e8286899df7d2298527b30bb38e9aee0b7c76e7078b9  rust/src/s_content/ref_owner.rs
```
