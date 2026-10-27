# 代码审查报告

日期：2026-10-17。复用 #1488。恢复树 `.sandcastle/worktrees/resume-1488`。范围是 `history.rs` 当前差异：`impl CodecRead for HashReader` 的 Plain 返回值接线及末尾 `native_reader_owner_tests`。承接 `1488-history-reader-compat.md`；其余 history 消费面不在本轮重审范围内。

## 概念层质询结果

**通过（限定读取接口和返回值所有权）。** 沿用前一份 `1488-prefix-memo-review.md` 的领域边界：本片不改变缠论定义或判据。实际所用概念为原件路径定位、strict JSON、Native/Owned 资格和原父账持费，不新增来源核豁免或缓存资格。

对旧接口与新接口逐项回溯：path/field/optional-field/array/fields 均进入相同泛型语义核，变化为返回 owner 类型；fields/subset 仍分别传 false/true，底层仍转换成 `!subset` 的完整字段要求。content_ref 仍为 parse_root、真实节点头核验和原许可签发，不冒充完整闭包验证。Owned 分支、sync_owned、verify_ref 和 render_ref 未改。

反例检查包括重复键与转义后相等键、尾随 JSON、缺字段与可选字段、字段全集/子集差别、错误路径、无 scope 普通读取、Native 分配失败，以及解析结果跨 reader/scope 存活。未发现新宽松准入或判据分叉，进入工程层。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

结论：**限定静态 PASS**。未发现置信度超过 80% 的新增确定缺陷；不等于测试运行通过或整树可编译。

### 承重依据

- `history.rs:2713–2718`：parse_rendered_plain 返回既定 PlainJson，直接把当前 self.work 传给 strict_plain_json_admitted。原 native.reserve/commit 与 scope.reserve/retain 已删除，没有先旧收费再包新 owner。既有 helper 在解析前取原 NativeReservation；失败正常析构 permit，成功将同一 permit 放进 PlainJson，value 字段在 permit 字段之前析构。
- `business_json.rs:2190–2218、2239–2266、2323–2392` 与原 `session_protocol.rs:174–232` 对照：同 serde_json slice decoder、同完整输入结束检查；重复键在 key 解析后、value 解析前拒绝，文本仍为 duplicate key，语法错误仍加 SchemaUnsupported 前缀。资源错误不被映射为语法错误，WorkScope 的资源失败标记继续由原 reserve/reject 保存。原 helper 的独立测试矩阵不能算本次已执行证据。
- `history.rs:2734–2740`：typed 数组枚举得到 CodecVec<NativeValue<ContentRef>>，结果 RefValue 向量通过 native_codec_room 准入。源向量 IntoIter 自带原 buffer permit，搬移每个元素时不复制其字符串或另签原件费用；新旧向量同时存在时都在原父账，旧 buffer 消失后才退对应费。未用裸 Vec 或 retain 代替真实生命周期。
- `:2747–2829`：content_ref_view、lookup_ref、lookup_field_ref、lookup_optional_field_ref、lookup_fields_ref、lookup_subset_ref 的新返回类型逐个匹配 RefValue::Plain / RefFields::Plain。`path.rs:2037–2094` 对应函数存在且使用原 CodecContext::Source；optional 仍保留 None，路径仍保 BorrowedFieldPath，未额外构造路径字符串。NativeRefFields 的 append_reserved 与原泛型 object_fields_output 共核。
- content_ref_of_native 直接接 `content_ref_reserved` 原许可；旧 content_ref_ctx 的 retain 擦除层不再经过。它只核根节点的语义保持，未增加闭包资格。
- `history.rs:6723–6757` 新测试：MemoryNodes::new、HashReader::new、feature 门下 with_codec_work、PlainJson::as_ref 和 CodecRead 方法均有对应定义。闭包返回的 PlainJson 仅保存 parent/token，不借 reader/scope。before 在 scope 建立后取值，包含工作域元数据；scope 外才与最初 baseline 比较。重复键失败不留费、非空成功结果跨 scope 持费、最后 drop 回基线的断言没有减去残费或手工退款。

### 验证与限制

- 已读当前 staged/unstaged diff；staged 无该文件变化。
- 单文件 `rustfmt --edition 2021 --check --config skip_children=true`：exit 0。
- 限定文件 `git diff --check`：exit 0。
- 新增 Rust 测试 **NOT_RUN**。未执行 cargo/rustc、产品服务、HTTP、浏览器、外盘、回放或远端任务；未改源码。
- 新测试直接覆盖 parser 生命周期；**没有经过实际 path/fields/array 返回值测试**，这些接线仅完成静态同核及 owner 检查。必要后续需在具名构建中执行旧/新接口的业务结果、错误/读取门序和释放锁。
- PlainJson 当前沿既定方案持有解析峰许可到 value drop，不是旧 `commit(0, heap_bytes)` 的账面行为。不能据本片认定更长存活期仍满足原 16/8 MiB 帽；须保持原帽和六成功验证。
- 这次解决了上一报告点名的 HashReader trait 返回值/裸 RefValue 构造断层；未检查并修复全部调用者。history 的 binding、stack、overlay、旧 NodeRead/CheckedFrame 返回、capture/cache 交接，以及其它模块的剩余迁移仍待续。**不能宣称整树可编译、全域无 retain 或 #1488 完成。**

## 已审字节

```text
08aa9de9b121e83ca1776a321aa061e28b74f52cbc8e2a694f5e72227e5f1f51  rust/src/bin/s_session_v3/history.rs
```

摘要标识整个文件；审查结论只覆盖当前差异及必要依赖，不涵盖该文件全部历史逻辑。
