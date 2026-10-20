# #1488 Native API 后续接口实施

2026-10-17。工作树 `.sandcastle/worktrees/resume-1488`。这是 A 初版之后按主控/B2/D 实际消费需求补的接口；不改 bin 文件，不替 B2/D 声称迁移完成。**编译与全部测试 NOT_RUN。**

## 已落接口

1. `NativeValue<ContentRef>::try_from_built(&Built, Option<&WorkScope>)`。从真实 digest 用 `FixedRoot` 栈上格式化；两段字符串构造前取得一次原 Native token。读取 Built 的 kind/extent，不调用 feature-gated `Built::owned_ref`，默认 feature 配置也有该接口。主控添加的 String/ContentRef `from_unscoped` 与只读 `is_native_admitted` 均保留。
2. `sc::path::content_ref_of_native(&mut dyn NodeSource, &str)`。在 `s_content.rs` 原 `content_ref_ctx` 提取私有 `content_ref_reserved`；旧口和新口共用 **parse_root → reference_header 真实读取/头检查 → reserve_native → 四字段构造**。新口直接移动原 token 到 NativeValue；未调用旧 retain 口、未重签。旧口行为保持。`s_content.rs` 本次只有该处 18 增/4 删。
3. `CodecVec<u8>::try_into_native_text(self)`。Plain 分支由 `String::from_utf8` 接走原 Vec allocation 和原 Native token，不复制字节、不重新收费。UTF-8 错误释放错误内 Vec 和剩余 token；返回原 codec UTF8 错误。Owned 分支拒绝擦除，B2 保留其原 Owned 文本路径。
4. `NativeRefFields::try_from_views<'a>(impl IntoIterator<Item=(&'a str, RefView<'a>)>, Option<&WorkScope>)` 与 `try_clone_admitted(&WorkScope)`。键、引用、buffer 分别当前域准入复制；输入可任意键序，以原地 unstable sort 排成 canonical key 顺序，重复键拒绝。已有 source 仍持原许可，新复制独立持费。与 D 约定：sidecar subset 的重复 names/缺字段 `None` 前置由 D 保留，再构造新结果；这里不替侧车裁定缺字段行为，不提供无条件 Clone。

## 新增源码锁（4 项，NOT_RUN）

- 各 Kind 的 Built → typed reference 与原公开四字段逐项相同；有/无 scope 名分正确，drop 后原 Native 费用归零。
- content_ref_of 共核成功结果相同；坏前缀、坏 hex 长度、非小写 hex 在源读取前拒绝；有效根的来源失败与真实 F 相符但非法 Value tag 的头错误保持原优先级/原错误。成功输出的 live 只剩实际两个字符串，drop 归零。
- Vec → Native String 的指针、capacity 不变，费用不变；UTF8 失败全部释放，普通无 scope 转换仍无 Native 名分。
- 任意键序 fields 构造、同 scope 独立 clone、subset 移出后原结果释放、重复键错误归零。

本模块累计 13 项源码锁。仅运行单文件 rustfmt2021（skip_children）及 `git diff --check`，均通过。没有 cargo/rustc/HTTP/服务/外盘/重型操作。禁区 `construction_state.rs`、`audit_owner/native.rs`、`capture_allocation/scratch.rs` 未修改。

## 后续边界

本补丁不覆盖 JSON Parse/可拆子树消费面，也不替旧兼容 retain 或非本次消费链提供豁免。B2 和 D 正并行接真实调用，其他 source/history/object/aux 的迁移、独审、远端编译与原六成功仍是后续验收。主控原先已在 native_owned.rs 的额外修改已保留。

## 本次冻结 SHA256

```text
652ebcbbec522e178ce5bd3412c7420f48158029eee4371b50fb3235483e9c4c  rust/src/s_content.rs
5b778f381324d1aeee0a8fa152d620e82a2f90a8aa0c8e8d42e336daff2427b9  rust/src/s_content/path.rs
5b8670a42248057a869c8ec01b3406299e4054d0b4a0e4b9374e2750d6e8c450  rust/src/s_content/capture_allocation/native_owned.rs
6868bf3bdc70bf1d45b99a7012752825fc640eb1e608d6aeeb1e92bdb03755d1  rust/src/s_content/capture_allocation/work.rs
```
