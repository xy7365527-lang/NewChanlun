# #1488 D 窄片：NativeRecordFields 真实持费

日期：2026-10-17。恢复树：`.sandcastle/worktrees/resume-1488`。仅修改 `rust/src/bin/s_session_v3/native_record_fields.rs` 及其内测试。状态：**候选实现，Rust 编译/测试全部 NOT_RUN，不是 #1488 验收完成。**

## 已实现

- RecordKey 的 format/root 改 NativeValue<String>，保留 kind、extent、digest 与完整四字段相等/哈希；删除无条件 Clone。没有改用仅 digest 的弱身份键。
- records 改 BusinessMap<RecordKey,NativeRefFields>。从空 map 开始，新增桶用 NativeAdmission.native_map_room，在同一 WorkScope 签原 token。替换前完成所有新字段/key准入，失败保留旧记录。
- seed 复用 A 新增 NativeRefFields::try_from_views，在原同 capture/完整 Public 核验后逐键、引用、向量准入复制。seed 名字重复沿旧 BTreeMap 去重行为，不引入新判据；subset 请求名字重复仍返回 None。
- subset 返回独立持费 NativeRefFields；完整引用身份匹配、缺字段/重复请求/非 Normal 前置不变。借原字段读 RefView，由当前 scope 为新副本签许可，未使用裸 clone/into_plain，也未转移原字段许可。
- clear 使用 BusinessMap.retain 删除条目：实际 key/fields Drop 退项费；真实 hash buffer 仍存在，保留原容量许可，整个 sidecar Drop 后归还。
- 未增加任何 Budget.release、permit.retain、新父账或全 live 差额释放。

## 精确接口接线

```rust
NativeRecordFields::subset(
    &self,
    r: sc::RefView<'_>,
    names: &[&str],
) -> Result<Option<NativeRefFields>, sc::CodecError>
```

A 工位已落：

```rust
NativeRefFields::try_from_views<'a>(
    views: impl IntoIterator<Item = (&'a str, sc::RefView<'a>)>,
    work: Option<&WorkScope>,
) -> Result<NativeRefFields, sc::CodecError>
```

B2 工位已落 `RefFields::Plain(NativeRefFields)`，`RefFields::get` 返回借用 RefView。现 `source_audit.rs:555` 的 `records.subset(r,n)? → RefFields::Plain(fields)` 正好匹配新类型，不需要丢 token 转换。该文件不归本工位，未修改；其余 C 源消费面仍由后续实施负责，不能据这个命中分支断言全链可编译。

## 测试源码

- 保留同 capture/不同 capture、overlay、完整四字段伪造、重复请求名、缺字段、clear 后 miss 负控。与普通输出逐字段或迭代序列比较，token 不参与业务等价。
- 更新 shared_scope 正控：重复 seed/subset 同内容；clear 后项费减少且桶费仍在；sidecar Drop 后回原基线。
- 新增 subset 离开 scope 后仍可读，原 sidecar clear/drop 不提前退结果费；重复三轮结果销毁回父账元数据基线。
- 新增父账余量耗尽导致替换失败：仅借用观察旧四字段及内容仍在，无失败域重试；清理后无临时收费残留。

全部 **NOT_RUN**。只运行单文件 `rustfmt --edition 2021 --config skip_children=true` 与 `git diff --check -- <本文件>`，均通过。未执行 cargo/rustc、服务、HTTP、外盘、重型回放、commit/push。三个禁区与其他代理文件未改。

SHA256：`2dab513163b652eac20ef7feaaad358e4da3fd222aa7dc2ec66c0a0ba63c73c7`。

仍需独立源码复核与获准环境的实际编译/生命周期测试。对象/辅助索引、ExactRows/effects、cache/capture 与公开 String 返回边界不属于本窄片完成项；原六成功、16/8 MiB帽和全图验收均未被替代。
