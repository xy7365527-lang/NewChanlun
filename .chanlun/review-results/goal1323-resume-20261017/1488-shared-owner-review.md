# 代码审查报告

日期：2026-10-17。复用 #1488，独审恢复树 `.sandcastle/worktrees/resume-1488` 的 `rust/src/bin/s_session_v3/native_shared.rs` 与 `mod.rs` 注册行；必要依赖只读。本文不代表消费者迁移、整树可编译或原六成功验收。

## 概念层质询结果

**通过（限定共享所有权工程切片）。** 本片不实现缠论判定，不改变 Classification、级别或交易语义。对照主控批准的 `1488-shared-owner-design.md`、实现报告与现有 `ScratchParent/NativeReservation/WorkScope`：共享块从既有父账准入，浅复制保留原父身份，最后真实强/弱句柄结束后释放原许可，没有另建 Budget 或以统计差额退款。对知识库、definitions.yaml、docs/spec、教义目录和谱系的对应关键词检索未发现本片另有领域定义或已知矛盾；该未命中不等于全仓概念验收。

反例核查包括：最后强引用消失但 Weak 仍存活；payload 析构暂停时并发 Weak 退出；并发最后两个 lease；有 Weak 时尝试 get_mut；零长切片、高对齐和布局溢出；payload panic 展开；相同内容不同指针。未发现实现与批准合同冲突，因此进入工程审查。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

结论：**限定静态 PASS**。没有置信度超过 80% 的确定缺陷。允许据此继续独立消费者迁移准备；不等于批准合入或运行。

### 承重核查

- `native_shared.rs:86–93` 强弱 wrapper 均声明 value 在前、lease 在后，且没有自定义 wrapper Drop 抢先释放 lease。所有 clone/downgrade/成功 upgrade 都复制同一 lease，失败 upgrade 不动原 Weak。真实 Weak 尾部因而保留完整 value block 与 lease block；不是只收计数头。
- `:29–51` Rc 最后提取使用 `Rc::try_unwrap`，失败返回值正常析构；Rc 没有并发最后竞争。Arc 的每一份 lease 统一 `Arc::into_inner`，且 lease 的内部 Arc 不暴露、不产生 Weak，避免 `try_unwrap().ok()` 并发失去最后提取者。取得的 LeaseCore 在栈上 drop，原 permit 的退账位于 lease 控制块释放之后。另一个 wrapper 若仍在 value Drop 中，其 lease 尚未退出，不能提早发生最后退款。该结论依赖标准库语义，未以 allocator instrumentation 实测。
- `:61–80、100–104、228–234` 两块布局以 checked extend/padding/add 计算；同一原 scope 在任何两块分配之前一次 reserve。None 分支不分配 lease，不虚构许可。两计数头明确为结构预算模型；不是 Rust 公共 ABI 或 RSS 保证，后续具名构建必须记录实际工具链并核模型。
- `:118–121` get_mut 委托真实 Rc/Arc 的唯一强且无弱条件，不使用 lease 数推断，也没有 make_mut。它仍返回任意 T 的可变引用，不能替消费者证明动态子字段已持费；这是批准的 shallow 边界，需要消费者具名适配器继续审查。当前没有生产消费者，未见违规动态写入实例。
- `:135–175` Deref/AsRef/Borrow 为只读借用，Eq/Ord/Hash 按 payload，ptr_eq 按真实 allocation；没有裸 Rc/Arc 解包出口。固定 i64/u8 slice 复制避免泛型 Clone 隐藏动态分配。
- `NativeReservation` 保存 `Arc<dyn ScratchParent>`，父 trait 为 Send + Sync；Arc 家族并发使用的 lease 没有错误地保存不可 Send 的 WorkScope。调用的 reserve_native、reject、CodecWork API 与模块公开性在依赖源码中存在。未发现确定的局部类型或可见性错误；静态阅读不能证明编译通过。

### 验证与未完成边界

- 实际执行：单文件 `rustfmt --edition 2021 --check .../native_shared.rs` exit 0；`git diff --check -- .../mod.rs` exit 0。已读 staged/unstaged diff：注册文件只增加一行，新增 owner 文件为未跟踪文件，已直接全文审阅，未因 git diff 不显示而漏审。
- 九项新增 Rust 测试源码全部 **NOT_RUN**。计数器替身不是实际 allocator 释放观测。并发、panic、恰帽等测试只是待执行锁，不记作通过。
- 未运行 cargo/rustc、服务、HTTP、浏览器、外盘、回放或远端任务；未修改源码、commit、push、合入、关票。
- 尚未验证新增 lease block 和 wrapper 元素尺寸对原 16/8 MiB 帽的影响；消费者容器计费、动态子分配、跨请求父身份以及原六成功仍由后续整链验收承担。
- 两个宏可读且局部无重复协议分叉；宏长度本身不作为缺陷。只有实际错误或违反现有合同的高置信问题才列为 finding。

## 已审核字节指纹

```text
cdc9ba1e8a000320a67f2bff8083c22f938a8d01e7de8681083c058e3987d864  rust/src/bin/s_session_v3/native_shared.rs
bee83b7b52ef1093a9af29cc027d4a85356d44348593c3d6a063d2137c43bc96  rust/src/bin/s_session_v3/mod.rs
```

以上哈希与实现报告一致。本报告仅覆盖这些字节；后续源码变化需要按差异复核。
