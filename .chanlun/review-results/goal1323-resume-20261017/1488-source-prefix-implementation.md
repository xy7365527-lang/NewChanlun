# #1488 C1b：来源前缀 memo 与共享坐标 owner 迁移候选

日期：2026-10-17。恢复树 `.sandcastle/worktrees/resume-1488`。状态：**实施候选，待独审；未编译，测试源码全部 NOT_RUN，不代表 #1488 或六成功验收。**

## 范围与依据

本工位只修改 `rust/src/bin/s_session_v3/source_prefix.rs` 和本报告；没有覆盖其他工位修改。已读根 CLAUDE.md、Serena manual、`1488-source-consumer-plan.md`、共享 owner 实现/独审报告。图工具在真实索引项目 `Users-silencehan-Projects-NewChanlun-rust` 查目标符号零命中，随后精读恢复树文件及必要接口。

共享 owner 基线是已限定静态通过的 NativeRc；未修改其封闭接口、三禁区、原资源帽、资金/隔离/恢复要求或业务准入判据。

## 实际改变

- `SourceCoords::Shared`、Proof.values、coords/intern 共享值均改 `NativeRc<[i64]>`。share 在原 `s.codec_work()` 下 `try_copy_slice`；复制期间原 CodecVec 仍持原许可。intern 命中只浅 clone，共享块不重复签费，临时输入在返回时析构。
- intervals/coords/references/intern 四张持久表改 BusinessMap；interval 每项 runs 改 CodecVec。每次真实新增/扩容在原 scope 调 `native_map_room`/`native_codec_room`，insert/try_push 保持 fallible 错误传播。已有 Proof 替换不为不存在的新增条目扩桶。
- 三张临时树 identity_coords/coord_identities/winners 改 NativeTree。赢家仍按原 identity 有序消费：首键 remove 只移出 Winner，节点高水位许可保持到真实树销毁；不开放裸树出口。
- 移除本文件原 `native_admit/native_std_map_room/native_vec_room/native_tree_room` 保守保留收费点，改由实际容器/共享块 owner 持原 token，没有额外 Budget 或统计差额退款。
- `clear` 对三张 memo 表使用 `retain(|_, _| false)`，删除真实条目但保留实际桶容量及许可；intervals 仍按原合同保留到 memo drop。这里的 retain 是集合筛选，不是许可永久保留。
- 无 scope 的旧 fixture 路径继续无 scope；测试 build helper 从旧 Plain(ContentRef) 改 RefValue::from，以匹配已迁移的 RefValue 封闭接口。

## 保留的语义

未缩短原件、未改变 RawReference 八字段比对或 numeric/public/JsonInner ParseDomain 键。winner effective_precedence、身份坐标固定/跨身份坐标冲突、未来无效输入不取得优化资格、prefix inclusive 区间合并与交集保持；完整来源原判据仍运行，缓存没有取得 closure 资格。没有引入旧核放行或 full-audit 兜底。

HashMap 换 BusinessMap 只改变承载容器；所有业务 map 访问按原完整键，最终赢家结算仍有序，未把 hash 迭代顺序用于业务输出。

## API 与后续消费者

对外只有 `SourceCoords::Shared(Rc<[i64]>) -> Shared(NativeRc<[i64]>)` 的 payload 类型变化。切片只读 Deref、共享 Clone、内容相等继续成立；本文件 ptr_eq 改 NativeRc::ptr_eq。source_audit::copy_source_coords 的 v.clone 调用可沿用，未跨文件修改。

**已告知主控的后续点：** source_contributions::coords_bytes 仍按旧“两计数头 + i64 数组”统计，缺新 lease block；应由 C2/主控统一核定 Retained 统计和共享 owner 布局接口。本片不改该统计，也不允许用旧统计推断父账退款。共享实际父账收费已由 NativeRc 的原 token 承担，但统计完整性不等于已经完成。

## 检查与测试源码

实际执行：单文件 `rustfmt --edition 2021` exit 0；限定文件 `git diff --check` exit 0。已逐段复核差异和新接口静态签名。没有 cargo/rustc、产品服务/HTTP/浏览器、外盘、重型回放、远端任务、commit/push/合入或关票。

以下均 **NOT_RUN**：

1. 改写原 native_prefix_certificates 测试，移除旧永久保留收费预期，加入完整 interval 内容对照、证书及临时树 drop 归 scope 基线、intern 命中相同指针且 live 不增长、clear 保留桶与外部句柄、memo drop 释放桶、共享 owner 跨 WorkScope 存活、最后 clone drop 归父基线。
2. 新增 native_invalid_future_identity_drops_partial_certificate_owners：未来身份冲突返回 None 后，部分临时树许可归 scope 基线，退出 scope 后父账归原基线。
3. 原赢家全前缀对照、相同价格不同 revision/未来来源拒绝、ParseDomain 与坐标词法/升序负控保持；只是共享 ptr_eq 与 fixture 构造适配。

这些是待运行的锁，不记为测试通过。既有前缀缓存业务正负控主要为无 scope fixture；当前 scoped 生命周期测试直接覆盖 share/证书，尚不代替完整 production coords/references、跨代 source 状态或六成功路径。

## 指纹

```text
6aeff2791d5dc72fdaa83a3b13e13ad964ab59e0825a4ae8b5e0585a3c156537  rust/src/bin/s_session_v3/source_prefix.rs
```
