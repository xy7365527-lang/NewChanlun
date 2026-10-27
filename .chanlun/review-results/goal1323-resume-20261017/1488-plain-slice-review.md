# 代码审查报告：#1488 第二片 Plain 载荷与字段拥有者

日期：2026-10-17。工作树：`.sandcastle/worktrees/resume-1488`。核实 HEAD 为 `c2c9c5640a99b98299456fd8b837c58035a2f4a6`。

**结论：PASS，仅限本文指纹对应的第二片源码静态审查。NOT_COMPILED / NOT_RUN；整树尚有明确的跨文件类型断层，不可据此批准运行、合入或关闭 #1488 / #1323。**

本次先读 `1488-native-api-followup.md`、`1488-native-owners-implementation.md`、`1488-root-compatibility.md`，再核已冻结的 `1488-plain-owners-implementation.md` 与 `1488-native-record-fields-implementation.md`。已检查暂存区和工作区差异；源码图对本批新符号返回 0 条，转为精读源码及基线差异。只写本报告，未修改其他代理源码。

## 概念层质询结果

**通过，限定资源所有权和同核适配语义。** 本片没有改动缠论教义、Classification、级别或 Chong 的判据；没有以另一套宽松业务判断接管原核失败。对 `.chanlun/definitions/`、`definitions.yaml`、`缠论知识库.md`、`docs/spec/` 与谱系的本任务关键词检查未发现本片引入新的领域裁定；实际约束来自主控批准的 `1488-raii-migration-plan.md` 与原成功合同。

核对的概念为：Native 上界与 Owned Layout 分账、原准入许可随实际 allocation 移动、深复制时新旧同时收费、先析构 payload 再归还许可、借用不制造第二份拥有者、统计 Retained 不代替真实退费。本次未发现这些概念在限定切片内的确定矛盾。

反例检查覆盖：非 UTF-8 字节、有效根伴来源失败、坏根与来源失败并存、合法 digest 对非法 Value 头、坏 transport 声明的复制时机、空数组/空字段集、任意字段请求序、重复和缺字段、scope 结束而返回值仍存活、解析失败、父 JSON 销毁而独立子副本仍存活、sidecar 替换准入失败、clear 后 backing hash buffer 仍存在。这里只按控制流与拥有关系推演，不声称执行过反例。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

未发现置信度超过 80% 的新增局部缺陷；未运行编译器，不能把这个表理解成类型检查通过。

### 已核实的实现关系

1. **原 bytes → String 许可没有补签。** `work.rs::CodecVec<u8>::try_into_native_text` 直接把 Plain Vec 交给 `String::from_utf8`，同时移动原 `native_permit`。失败时错误持有的 Vec 先销毁，剩余 token 随栈释放。`TextValue::from_bytes` 先借用验证 UTF-8，以保留原诊断文字；Owned 分支仍独立进入原 TextOwner。没有使用 `try_into_plain` 后重新收费。
2. **root/path 仍共核。** `s_content.rs::content_ref_reserved` 保留 parse_root → reference_header 的真实源读取及头核验 → reserve_native → 四字段构造的顺序。旧口 retain，新的 `content_ref_of_native` 移动同一 token；新 path/array/fields adapter 复用原 RefOutput/FieldOutput 遍历与错误门。`try_from_view` 仍只复制四字段，不提前 digest 核验；`try_from_built` 用 Built 本身的 digest/kind/extent，不依赖 feature-gated owned_ref。
3. **PlainJson 承接原 parse token。** `business_json.rs:2167` 的 `admit_json_parse` 返回原许可；`:2195` 在 strict parser 运行前取许可，`:2198` 交给最终 PlainJson；legacy 同样在解析前取许可。严格 visitor 内的临时节点尚未附许可，因此私有 `unscoped_mut`/`into_unscoped` 搬移不把合法解析变成新拒绝。重复键检查仍位于 next_key 后、next_value 前；语法错误返回路径会析构许可。
4. **JSON 只读和独立子项。** PlainJson 的 Value 字段先于 token；无 DerefMut、公开 take 或裸 into_inner。持费实例的无 scope push/insert 被显式阻止。本文件实际通用投影调用从 None/Owned object/array 开始，保留原路径；尚未迁移的 Native 聚合消费不能用这个防错拒绝当作成功。`native_json_clone` 在复制前准入，独立子副本持自己的原 token，父析构不影响其费。`Retained for PlainJson` 只借实际 Value 统计，不调用 release。
5. **字段 clone/drop 与 sidecar。** NativeRefFields 为键、引用、结果 buffer 分别持费，按 canonical 序输出；get/iter 借用，clone_admitted 独立准入。`native_record_fields.rs` 保留 capture 身份、Public 原件核验及完整 format/root/kind/extent/digest 键，subset 重复 names/缺字段仍 None。seed 在全部新载荷/key 准入后才 insert，失败不替换旧项。clear 删除实际项并保留尚存 hash 容量许可；独立 subset 不借原 sidecar 的拥有权。seed 的既有重复 names 去重语义保留。
6. **Q 兼容片范围正确。** q_runtime 无 allocation 的实例 ID，以及 q_http_input 的无 Owned 预算文本/错误入口改用 TextValue::from；只是旧已拥有文本的移动，不给 Native 资格。plain_wire 测试 oracle 通过 PlainJson.as_ref 借 Value；未改变业务判据。

### 明确不属于通过范围的整合缺口

- 审查时 `history.rs::HashReader::parse_rendered_plain` 与 `source_audit.rs` override 仍为旧 Value 返回签名，与新 trait 不一致；这是已登记且正在后续迁移的跨文件断层。报告不把当前共享工作树称为可编译。
- object_index 的 Plain(Value::Array) 构造/匹配、push_change 与 changes_at 裸 Vec 子项移出尚需拥有者聚合接口；不能直接开放 PlainJson 可变借用或丢掉子 token 来完成类型接线。
- 其他 Text/Ref/JSON 调用者、cache、长期 source/history、索引与控制面，以及 audit_store 的旧 retain/ScopedFrame 兼容面仍未全域闭合。本报告只覆盖本批 typed 输出，不能推断全部临时分配均可正确退费。
- 新的无 scope clone/mutation 拒绝是 API 防错约束；原六成功必须仍成功，不能修改输入、放宽 16/8 MiB 或将成功变拒绝后宣布验收。此次没有运行六成功、跨事务恢复、全历史双跑或服务。
- `NativeValue`/PlainJson 的只读 Deref 仍允许调用者显式克隆底层 payload；它不是阻断任意 Rust 代码绕过记账的安全边界。实际 Native 调用者须逐条审查受准入构造和复制，不能以包装器存在替代调用链验收。

### 检查及冻结指纹

独立执行了限定文件的 `git diff --check`，通过。未调用 cargo/rustc、服务、HTTP、浏览器、外盘或重型验证。三个禁区 `construction_state.rs`、`audit_owner/native.rs`、`capture_allocation/scratch.rs` 对本树 HEAD 的 diff 均为空。

作者报告中的 A 后续、B2、D 指纹与审查时实际字节一致。Q 两文件另行取当前指纹。下列路径相对恢复工作树：

```text
652ebcbbec522e178ce5bd3412c7420f48158029eee4371b50fb3235483e9c4c  rust/src/s_content.rs
5b778f381324d1aeee0a8fa152d620e82a2f90a8aa0c8e8d42e336daff2427b9  rust/src/s_content/path.rs
d556a50a52a372a97744e8286899df7d2298527b30bb38e9aee0b7c76e7078b9  rust/src/s_content/ref_owner.rs
5b8670a42248057a869c8ec01b3406299e4054d0b4a0e4b9374e2750d6e8c450  rust/src/s_content/capture_allocation/native_owned.rs
6868bf3bdc70bf1d45b99a7012752825fc640eb1e608d6aeeb1e92bdb03755d1  rust/src/s_content/capture_allocation/work.rs
8161577d099f4deddb0d2cfda228e71c2b6c76ebca702e75e0f95f7d163789a8  rust/src/bin/s_session_v3/audit_store.rs
d93afd648fdf4664c1a61bb8392ad361cca2aa26fb4fd8c397249b3fa879244e  rust/src/bin/s_session_v3/business_json.rs
bdda0deea41ae536a7433242d23a0bb2b2683c9787d91494ce33112f3b67ed48  rust/src/bin/s_session_v3/audit_retention.rs
2dab513163b652eac20ef7feaaad358e4da3fd222aa7dc2ec66c0a0ba63c73c7  rust/src/bin/s_session_v3/native_record_fields.rs
e98c67ca4872dc25c71aa4bbd633d7cc57d337319f6e0ec36adbcfbbbc306156  rust/src/bin/s_session_v3/q_http_input.rs
eff1786b051a60d0ee8a6055f4796b144cc343d6f8776990db3228b1c12fc5e1  rust/src/bin/s_session_v3/q_runtime.rs
```

**结论：PASS（限定冻结片静态），全票未验收。**
