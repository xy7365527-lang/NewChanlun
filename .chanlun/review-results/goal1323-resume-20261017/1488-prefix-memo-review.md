# 代码审查报告

日期：2026-10-17。复用 #1488。恢复树：`.sandcastle/worktrees/resume-1488`。独审范围为 `source_prefix.rs` 的 C1b 所有权迁移，以及 `source_audit.rs` 新增的两项 scoped memo 生命周期测试；依赖仅为解释这些变更而读取。没有修改源码。

## 概念层质询结果

**通过（限定来源缓存与所有权切片）。** 这次变更不定义笔、中枢、背驰或交易判据；检索知识库、definitions.yaml、docs/spec、教义和谱系的具名 NativeSourceMemo / SequenceKey / NativeFieldMemo / NativeWitnessIdentityMemo / #1488 未发现另一个对应领域定义。工程依据为原来源核判据、批准的共享 owner 设计及已审的共享 owner 接口，不能把工程缓存称为教义裁定。

逐项比对旧版：SequenceKey 仍完整包含 digest/kind/extent/domain/numeric；Public 与 JsonInner 不串域。winner 的 effective_precedence、完整 identity→coord 与 coord→identity 检查、未来异常使证书不可复用、闭区间交并、原 RawReference 核验及 capture 资格均保持。证书失效时运行原完整来源谓词，不是换宽松谓词放行。新增许可失败继续传播，不转换为成功。

反例静态核查覆盖：相同坐标不同身份、同身份未来换坐标、较低 revision 晚到、同字节不同 parse domain、共享块仍有外部强引用时 clear、两份字段命中副本按不同次序销毁，以及错误 witness ID 首次计算和缓存命中。未发现与上述原判据矛盾，进入工程层。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

结论：**限定静态 PASS**。未发现置信度超过 80% 的新增确定缺陷。不是整树编译、运行通过或六成功验收。

### 来源前缀承重检查

- `source_prefix.rs:464–470、493–497`：四张表由 BusinessMap 承载；clear 删除 coords/references/intern 的实际元素，保留实际桶及其 token。intervals 保持原来直到 memo drop 的有效期。没有人为退统计差额。
- `:512–596`：三张临时有序树采用 NativeTree；insert_admitted 在节点增长前签费，顺序消费用 first_key_value/remove，未换成无序 HashMap 赢家消费。每个 Interval 向量由 CodecVec 独立持费，扩容在 try_push 前准入。失败早退会正常析构已建树、向量和输出表。
- `:598–615`：intern 未命中先经当前 codec_work 构造 NativeRc，再签表桶费；复制时原 CodecVec 仍存在。命中只浅 clone，传入临时 CodecVec 正常退出。NativeRc 的 Borrow<[i64]>/内容 Hash、Eq 与 BusinessMap 的查询接口相容。
- `:632–640、697–711`：coords 只在 miss 新增；references 替换已有 Proof 不误签新增桶扩容。key 全为内联值，Proof 和表 key/value 持同一共享 lease，没有裸 Rc 出口或动态未持费字符串藏在键内。
- `:71–166` 新锁在 scope 内取 scope_base，包含 CodecWork 元数据；scope 外才比原父 baseline。clear 后共享块由外部句柄持有，memo drop 才释放桶，最后 clone drop 才释放共享块。没有把 scope 元数据与缓存费混算，也没有按估算布局硬写应退金额。

### 新增 field / witness 测试

- `source_audit.rs:3078–3170` 经真实 NativeFieldMemo、ArrayAuditSource、StoreView 与 scoped HashReader，执行一次 miss、两次 hit，确认复制出的非空 root 文本地址不同。移出的是原 memo.fields，不是重新构造缓存内容；在 scope 结束后观察父账。先 clear / 后 clear 两种次序都覆盖独立副本退费、空桶仍计费和最终回基线。
- `:3175–3247` 的 witness 错误首次计算、成功 miss、正确/错误/正确命中都调用真实 check_identity；错误 stored ID 不取得缓存准入。命中 render_calls 不增长，父账要求稳定。清理使用实际 entries.retain，保留真实容量，最后 drop 回基线；没有仅清统计器。
- 原 NativeReservation::retain 不会随 scope 结束退款。两项测试没有退款补偿，也没有减去 legacy 残费；若依赖仍走这种路径，最终回基线或命中稳定断言应当失败。必要依赖里 native_verify_original 的正常 checked_read_scoped 分支将原 permit 放入 ScopedFrame；HashReader 的 captured scoped 读取同样返回持费帧。旧 read/into_frame 入口仍存在，**未执行测试，不能由正常路径静态阅读保证每条内部调用均无残费**。
- 测试源码的返回 owner 不保存 WorkScope 借用；field key 的静态名称与独立 NativeRefFields 允许移出闭包。所用 NativeRc、NativeTree、BusinessMap、RefView 和字段 API 在依赖中有对应签名；未发现局部必然的类型/生命周期错误。

### 明确的后续边界

1. `source_contributions::coords_bytes` 缺新共享 lease 的统计是已知 C2 后续责任。真实父账已由 NativeRc 收两块原费，本片不根据该旧统计退款；因此不阻这份前缀切片的静态结论，但 **阻止宣称整体 Retained 统计与六成功闭合**。不得在后续交接继续使用旧两计数头公式充当完整账。
2. 当前 `history.rs:2714` 的 parse_rendered_plain 仍返回 serde_json::Value，与已迁移 CodecRead 的 PlainJson 返回类型不同；多处 lookup_* 仍以裸 ContentRef 构造新版 RefValue::Plain。它们已经在 `1488-source-consumer-plan.md` 的 history 迁移清单内，是现存整树集成断层，不是本次前缀或新增测试的独立新缺陷。**当前整树不能据本报告认定可编译。**
3. 新 prefix scoped 锁覆盖 share/证书生命周期，不覆盖生产 coords/references 的完整 scoped 命中闭环；新增 field 锁没有证明跨请求重新准入或 warm restore；scalar PlainJson 缓存仍缺专门 scoped 生命周期锁。原业务前缀对拍多为无 scope，不能外推原资源帽内成功。
4. 所有新增 Rust 测试均 **NOT_RUN**。不放宽 16/8 MiB 帽，不认定原六成功、恢复或跨请求父账交接通过。

## 已执行验证与指纹

- 已读取 staged/unstaged 对应差异，staged 无对应改动；目标恢复树图查询返回 0 后精读源文件。
- 两目标文件 `rustfmt --edition 2021 --check --config skip_children=true`：exit 0。
- 两目标文件 `git diff --check`：exit 0。
- 没有 cargo/rustc、服务、HTTP、浏览器、外盘、重型回放、远端运行、commit/push/main 合入或关票。

```text
6aeff2791d5dc72fdaa83a3b13e13ad964ab59e0825a4ae8b5e0585a3c156537  rust/src/bin/s_session_v3/source_prefix.rs
4b6275b8113a1acf4fdeef859894b5794529fc6b59792e9c6f281557cbcacc7f  rust/src/bin/s_session_v3/source_audit.rs
```

摘要与本次两个实施报告一致。source_audit 指纹标识整文件字节，独审范围仅新增两项测试及其必要依赖，并未重新审定整个生产来源核。
