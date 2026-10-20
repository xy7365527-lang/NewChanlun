# #1488 共用强弱共享 owner 小片

日期：2026-10-17。基于主控批准的 `1488-shared-owner-design.md` 实施。状态：**实现候选，待独审；未编译，九项测试源码全部 NOT_RUN；不是消费者迁移完成或六成功验收。**

## 修改范围

- 新增恢复树 `rust/src/bin/s_session_v3/native_shared.rs`。
- 同目录 `mod.rs` 仅加 `pub(crate) mod native_shared;`。修改前已核该文件无现存未提交 diff，修改后 diff 仅此一行。
- 没有改消费者、库 native_owned.rs、三个禁区、资源帽、业务判据或公开协议；没有 cargo/rustc、服务/HTTP、浏览器、外盘、重型回放、commit/push/dispatch/关票。

## 实装契约

`NativeRc<T>`/`NativeWeak<T>` 和 `NativeArc<T>`/`NativeWeakArc<T>` 均是 value 字段在前、lease 字段在后的封闭 wrapper。所有真实强/弱句柄共享同一 lease 的强计数；lease 没有暴露的 Weak、没有指回真实 value。浅 clone/downgrade/upgrade 不产生新 allocation 或父账预留。

RcLease 最后通过 `Rc::try_unwrap` 把 LeaseCore 移到栈上；ArcLease 的每份引用都通过 `Arc::into_inner` 释放，防止并发最后提取丢失。原许可随后随栈上 core drop。字段顺序保证真实 value/Weak 先释放，故 weak 尾部保留整个 value block 的费用；最后 strong 时，T 的持费动态字段先自行销毁。

新共享块从既有 WorkScope 一次预留 `value block + lease block`。两块均用 checked Layout extend/padding，lease 中 NativeReservation 的 inline metadata 算在内；无递归 metadata token、无新 Budget。std 两计数头是当前结构预算模型，不宣称公共 ABI/allocator/RSS 精确保证。NativeReservation 克隆原父 Arc 不重新收费父对象。

`try_new_shallow` **只计共享外壳和 T 的 inline 存储**，不认证任意 T 的动态子分配。调用点要用具名适配器接已持费的 Object/Node/HistoryEvents/Cert 等。None 仅保持原无 scope 兼容，不是 Native 的免收费入口。std Rc/Arc 的系统 OOM 行为沿旧路径，不宣称新增可恢复 allocator 错误。

没有裸 Rc/Arc/Weak 出口、into_inner/into_raw/from_raw/make_mut、DerefMut、任意 bytes 准入接口。`get_mut` 为 bin 内部方法，严格使用真实 std 唯一强且无弱的条件，不根据 lease 强引用数推断；消费工位不得用它往动态字段塞入未准入分配。

## 冻结接口

从 `crate::v3::native_shared` 导入：

```rust
NativeRc<T>::try_new_shallow(value: T, work: Option<&WorkScope>)
    -> Result<NativeRc<T>, CodecError>
NativeArc<T>::try_new_shallow(value: T, work: Option<&WorkScope>)
    -> Result<NativeArc<T>, CodecError>

// 只对 [i64] 和 [u8] 分别实现，非泛型 Clone。
NativeRc::<[i64]>::try_copy_slice(&[i64], Option<&WorkScope>) -> Result<Self, CodecError>
NativeRc::<[u8]>::try_copy_slice(&[u8], Option<&WorkScope>) -> Result<Self, CodecError>
NativeArc::<[i64]>::try_copy_slice(&[i64], Option<&WorkScope>) -> Result<Self, CodecError>
NativeArc::<[u8]>::try_copy_slice(&[u8], Option<&WorkScope>) -> Result<Self, CodecError>

NativeRc::downgrade(&Self) -> NativeWeak<T>
NativeArc::downgrade(&Self) -> NativeWeakArc<T>
NativeRc::ptr_eq(&Self, &Self) -> bool
NativeArc::ptr_eq(&Self, &Self) -> bool
NativeRc::get_mut(&mut Self) -> Option<&mut T>
NativeArc::get_mut(&mut Self) -> Option<&mut T>
strong.is_native_admitted() -> bool

NativeWeak::<T>::new() -> Self       // T: Sized；空弱句柄无 allocation
NativeWeakArc::<T>::new() -> Self
weak.upgrade() -> Option<Strong<T>>
NativeWeak::ptr_eq(&Self, &Self) -> bool
NativeWeakArc::ptr_eq(&Self, &Self) -> bool
```

强句柄：浅 Clone、只读 Deref/AsRef/Borrow、Debug；Eq/Ord/Hash 按 payload。弱句柄：浅 Clone/Default/Debug，不作业务值比较。ptr_eq 按真实 value allocation，区别于内容相等。slice copy 在 reserve 后才复制；原 CodecVec 必须在复制完成前保持原许可。

## 九项必要测试源码

| 测试 | 所锁范围 |
|---|---|
| rc_dynamic_payload_releases_at_last_strong_but_entire_block_waits_for_weak | NativeValue 字符串动态费最后 strong 退，整个共享块最后 weak 退；clone/upgrade 不重复收费 |
| get_mut_preserves_std_strong_and_weak_exclusivity | 强 clone 与 Weak 都阻止独占可变访问；释放后恢复 |
| slices_use_content_hash_and_borrow_but_pointer_identity_is_distinct | 内容等价、HashMap 借 slice 查询、ptr_eq 区分、空 slice、Arc slice 弱尾部 |
| empty_weak_and_unscoped_compatibility_have_no_native_lease | 空 Weak 与原 None 兼容没有虚构许可 |
| resource_failure_keeps_existing_owner_and_exact_limit_succeeds | 新准入失败保留旧 owner，恰等完整父帽成功；最后释放归零 |
| alignment_and_checked_layout_overflow_do_not_use_unpadded_payload_size | 256 对齐、头部与尾部 padding、布局溢出 |
| arc_concurrent_last_strong_and_weak_release_once | 16 个线程并发退强弱句柄，payload 一次销毁、许可一次退还 |
| arc_does_not_refund_while_payload_destructor_is_still_running | T Drop 被同步屏障暂停时，即使其他 Weak 退出也不提前退账 |
| payload_unwind_drops_payload_before_lease_and_keeps_weak_tail | payload panic 展开后 Weak 仍保整块，最后 Weak 才归零 |

全部 **NOT_RUN**。测试 Parent 是计数器替身；屏障、线程和探针是外部测试设施，并非被 shallow 合同认证为动态内容已准入。测试源码核对的是资源账和析构生命周期，**没有 allocator instrumentation，不能宣称已运行证明实际 dealloc 指令先于 refund**；这一顺序目前来自封闭字段、std 语义和静态审阅，仍需独审及允许后的真实编译/运行。

## 已执行检查与指纹

- 单文件 `rustfmt --edition 2021 rust/src/bin/s_session_v3/native_shared.rs` exit 0。
- `git diff --check -- rust/src/bin/s_session_v3/mod.rs` 无输出。
- 新文件 `git diff --no-index --check -- /dev/null rust/src/bin/s_session_v3/native_shared.rs` 无空白错误输出；exit 1 是 no-index 对新增文件报告存在差异，不是编译或测试结果。
- mod.rs diff 只有一行注册；未对 mod.rs 执行递归 rustfmt。

SHA-256：

```text
cdc9ba1e8a000320a67f2bff8083c22f938a8d01e7de8681083c058e3987d864  native_shared.rs
bee83b7b52ef1093a9af29cc027d4a85356d44348593c3d6a063d2137c43bc96  mod.rs
```

## 仍需主控门

先独审核心强弱协议、布局、接口及测试，再派 C1/C2/D1/D2-cache 消费迁移。当前没有生产路径使用这些 owner；因此不能声称 source/cache 持费已经闭合。每个唯一 allocation 新增 lease block、每个 wrapper 增加指针，消费容器的真实元素尺寸也要计入。最终必须保住原六成功和 16/8 MiB 帽；如失败，优化实现，不改成功合同。
