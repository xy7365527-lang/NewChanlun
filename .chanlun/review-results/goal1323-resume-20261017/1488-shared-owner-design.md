# #1488 C/D 共享 allocation owner 方案

日期：2026-10-17。票型：复用 #1488 的只读架构准备。源码：`.sandcastle/worktrees/resume-1488`。**暂定，待主控裁定；未实施、未编译、未运行 Rust 测试，不构成六成功或全图验收。** 本轮只写此报告，没有修改生产源码或三个禁区。

## 结论

建议第一版统一采用封闭的 `NativeRc<T>/NativeWeak<T>` 与 `NativeArc<T>/NativeWeakArc<T>`：保留标准库的真实共享载荷 allocation；另设一个极小、无弱引用的共享 lease allocation，所有强/弱 wrapper 各持一份 lease 强句柄。先销毁真实 Rc/Arc/Weak 字段，再销毁 lease 句柄；最后一个 lease 将原 NativeReservation **移出已经释放的 lease allocation**，最后退账。

这同时解决 Node 的 Weak parent、RootCert/lineage 弱证书、无弱引用的坐标/Object/HistoryEvents。代价是每个唯一共享 allocation 增加一个 lease allocation，每个 handle 增加一个指针，必须完整计费并通过原资源帽成功锁。不是按引用数收共享载荷费用；clone/downgrade/upgrade 本身无堆分配，不重新 reserve。

不得只把 token 放进 `Rc<PayloadAndPermit<T>>` 并依靠 payload Drop。最后强引用结束时标准库只析构 T，**整个 RcBox/ArcInner，包括 T 所占的内联存储，都可能因 Weak 继续存活**，并非只剩两个计数器。此时只有 T 的动态子分配应退费；外层 allocation 的完整结构费必须留到最后 Weak。

## 概念与依据状态

依赖的是 #1488 已有资源合同：原父账签发、真实分配先准入、原六成功与 16/8 MiB 帽不变，以及 Rust 强/弱共享生命周期；不改变缠论对象、判据或级别定义。已检索 `.chanlun/genealogy/pending/` 与 `settled/` 的 `#1488/NativeReservation/RAII/内存许可/同父账`，无对应定义或矛盾条目。不能把未命中声称为所有定义已结算；本方案的暂定来自共享所有权工程选择尚待主控确认，而非借用一条未结算的缠论定义。

图工具在恢复树对 SourceCoords/HistoryEvents/NativeReservation 返回零，随后精读实际文件。已读 C/D 消费方案和 RAII 总方案。`scratch.rs:29–46` 的 NativeReservation 已持原 `Arc<dyn ScratchParent>`，clone 父句柄不新增父账；`scratch.rs:73–94` 已有隐藏 Arc、`Arc::into_inner` 后释放自身 metadata 的同类模式，本方案只借鉴，不修改该禁区。

当前 CI 注释说工具链不钉死；不得把 scratch 注释的“Rust 1.98”当成已核运行环境。后续具名远端构建须记录实际工具链，并核 `Arc::into_inner` 与 std 控制块布局假设。

## 能实现的最小结构与析构协议

以下是设计形状，不是已落代码：

```text
NativeRc<T: ?Sized>      { value: Rc<T>,   lease: RcLease }
NativeWeak<T: ?Sized>    { value: Weak<T>, lease: RcLease }
RcLease                 { inner: Option<Rc<LeaseCore>> }
LeaseCore               { permit: NativeReservation }

NativeArc<T: ?Sized>     { value: Arc<T>,        lease: ArcLease }
NativeWeakArc<T: ?Sized> { value: sync::Weak<T>, lease: ArcLease }
ArcLease                { inner: Option<Arc<LeaseCore>> }
```

字段顺序是合同：value 在前，lease 在后。wrapper 不添加先退 lease 的 Drop；依赖 Rust 按声明次序析构字段。`RcLease::drop` 对取出的 Rc 调 `Rc::try_unwrap`：成功取得栈上 LeaseCore 后再销毁 core；失败时正常 drop 返回的 Rc。Rc 单线程，无并发最后提取竞争。LeaseCore 的 Rc 不生成或暴露 Weak，所以成功返回时自身控制块已释放。

`ArcLease::drop` 必须对每一份取出的 Arc 调 `Arc::into_inner`，成功取出的 core 才退 permit。**禁止 `Arc::try_unwrap(x).ok()`**：并发最后两份都可能失败后各自 drop，从而没人走所需的“先释放 lease allocation，再销毁栈上 token”路径。禁止用 strong_count 预判最后引用后手动退费。

LeaseCore 不持真实 value 的 Rc/Weak，不反持业务对象，因此不增加强引用环。每个完成 value drop 的 wrapper 才能释放自己的 lease；若另一线程仍在执行最后 T 的析构，它仍持自身 lease，不能提前出现最后 lease。payload Drop panic 的展开路径也必须验证：value 的 std 析构清理完成后才走 lease 字段清理；不得用忽略 panic 的旁路提前退款。

最后 strong 时序：T 动态子 owner 析构并各退原许可；若还有 Weak，外层 value allocation 与 lease allocation 均保留。最后 Weak 时序：真实 Weak 先释放，value allocation 真正消失；lease 最后提取并释放自身 allocation；最后 drop 栈上 permit。无 Weak 时相同顺序在最后 strong 的 wrapper 上发生。

### 构造与费用

1. 从调用点的**既有** WorkScope 一次 reserve：`value allocation 结构上界 + lease allocation 结构上界`。没有独立 Budget、新 parent 或全 live 差额退款。
2. 在原 Rc/Arc 构造之前取得该 token，再构造 value 和 lease。并非先分配后按观察值补票。失败资源准入保持原对象有效。
3. `Layout::new::<[usize; 2]>().extend(Layout::new::<T>()).pad_to_align()` 用于 sized value；slice 用 `Layout::array::<i64/u8>(len)`；lease 同样对 `LeaseCore` 计算完整布局。所有加法/布局错误 checked，不能 saturating 后继续分配。
4. **Native 是结构预算而非 allocator/RSS 实测。** 上述两个 std 计数头的结构假设须与项目支持工具链核对，不能宣称 Rust 公共 ABI 保证该内部布局。T 的对齐、尾部 padding 和 wrapper 扩大后的父容器元素尺寸必须计入。
5. LeaseCore 内的 NativeReservation 只含父 Arc 句柄与字节数，直接算进 LeaseCore inline 大小；不会为每个 token 再递归生成另一张 metadata token。父账 Arc 本身是原实例，不因 clone 重收。沿现有 `reserve_native` 记 Native 字节，不修改禁区 ScratchCharge。
6. std `Rc::new/Arc::new/from(slice)` 的分配失败行为维持当前标准库路径，第一版不新写 unsafe allocator。资源 reserve 可失败不等于已实现可恢复系统 OOM。新增一块 lease 的真实开销必须测，不能放宽帽或把成功改拒绝掩盖。

无 scope 兼容入口可以保留 `lease=None`，避免给普通路径添加虚构预算；只能用于本来没有 WorkScope 的路径。Native scope 下不可借 None 把新分配变成普通值。

### 窄接口

- 强句柄仅实现 payload 借用、浅 Clone、`ptr_eq`；不返回裸 Rc/Arc，不提供 `into_raw/from_raw/into_inner/make_mut`。
- 弱句柄提供 `new`（空、无 allocation）、Clone、`upgrade`、`ptr_eq`；downgrade 复制 lease 强句柄。upgrade 失败不改变原 Weak/lease；成功只复制同一 lease。
- `ptr_eq` 比较真实 value allocation，不比较父预算或 lease；Hash/Eq/Ord 则按 payload 值，保留坐标 intern 的值相等语义。不能把逻辑相等改为指针相等。
- `get_mut` 仅在确实必要的 crate 内部入口提供，委托真实 `Rc/Arc::get_mut`；不得按 lease 引用数决定唯一性，也不做 clone-on-write。HistoryEvents 独占更新的旧错误保留。内部 RefCell<Node> 的 borrow_mut 保留，但 Node 字段仍只能使用受准入 owner。
- 泛型构造器明确名为 shallow 并限制在 bin 内：只包 T 所在共享块，不认证 T 的动态内容。各领域用具名 adapter 接已经持费的 Object/Node/HistoryEvents/Cert/RootCert；不接受调用者声称的任意 bytes 作为“全部已付费”的凭证。
- 仅提供 i64/u8 的 slice copy 构造，避免泛型 Clone 在复制过程中偷偷分配。新 slice 许可先签、旧 CodecVec 仍持原费，两块同时活着直到旧 buffer 真正销毁。

## 与复用持费容器方案的比较

| 方案 | 优点 | 不能覆盖或代价 | 本轮建议 |
|---|---|---|---|
| `NativeValue<Rc<T>>` 外包单 token | 改动小 | 裸 Rc clone 易脱离 token；不能随真实最后强/弱释放 | 拒绝 |
| `Rc<Payload{T,permit}>`，靠 payload Drop | 单共享块 | Weak 尾部整个 block 漏费；自身 metadata 在释放前退款 | 拒绝 |
| 无 Weak 的封闭共享块，最后强引用 try_unwrap/into_inner 后退 permit | 少一块 allocation，适用于确定无 Weak 的类 | 需要第二类共享 API；不能迁 source_order 或 cache 弱证明；泛型动态内容仍须各自 owner | 可后续优化，首版不分叉 |
| `Rc<CodecVec<i64>>` 等持费容器再共享 | 可移动旧 buffer，动态许可自然随 T | 共享块仍需收费；改变坐标单 allocation 结构和 cap 保留，不能自动解决 Weak | 不是本次必要前置 |
| 封闭 value handle + 无 Weak lease handle | 保持 std Rc/Weak 语义，safe Rust，可统一强弱路径，元数据先销毁再退款 | 每个唯一分配多一块 lease，handle 变大；原帽必须实证 | 推荐最小统一实现 |
| 自写 Rc/Arc allocator/control block | 能把 token 和引用计数合成一块 | unsafe、原子内存序、DST、panic/overflow/soundness 审查面大 | 本轮不取 |

## 真实调用面与交接

以下行号是本轮读取快照，实施用符号定位；不覆盖他人修改。

| 文件 / 入口 | 已核事实 | 迁移要求 |
|---|---|---|
| source_prefix.rs:23、409、419、421、552–574 | Shared、Proof.values、coords、intern 都持 `Rc<[i64]>`；share 原来先 native_admit 后 Rc::from | 全部同一个 NativeRc<[i64]>；原准入点改签到 lease，删除同块旧 native_admit；intern 命中 clone 不签新费；ptr_eq 与内容 Hash/Eq 同时保留 |
| source_contributions.rs:358、1055–1061、1136、1511 | Entry.value 的 Rc<Object> 被多处浅克隆；原收点独立 native_admit | 先把 Object 内文字/字段/SourceCoords 持费，再同父共享；多个索引不能按 Retained.heap_bytes 重复退共享费 |
| source_order.rs:8、14、21、45–47、64–139、220、252–266、294、300+ | Node 子强父弱；旋转/排名用 ptr_eq；节点表与局部 Link 可各持强引用 | Link、parent、nodes 全换同族 wrapper；空 Weak/upgrade/ptr_eq 完整接；reclaim 脱表后局部句柄与 Weak 仍按真生命周期持费；tree capacity 另属 NativeTree |
| object_index.rs:782、1748、1873、2255–2295、2377、2568 | HistoryEvents 的两游标共享 Arc，update_from_owner 用 Arc::get_mut | NativeArc<HistoryEvents>；get_mut 仍拒绝旧 facade 借用；birth/death 桶、ranks、文字等须分别持费，外层 shallow token 不覆盖这些子分配 |
| v2/evidence_store.rs:94–110、1174–1197、1262、1831、1930、1963–1971、2117、2179 | bytes/Value/Cert/RootCert 多种 Arc；admit_byte_arc 原来丢掉许可 | bytes→NativeArc<[u8]>；Value→NativeArc<PlainJson>，不能包裸 Value 宣称动态 JSON 已覆盖；Cert payload/children 等子分配持费后才包外层共享块 |
| v2/cache.rs:48–54、64、91、136–154、193–197、402、525–570、820+、1214+、1264+、1424+、1656 | Root.originals、Handle.root_cert、BatchHeaderEntry.root_cert、RootCert.lineage 全是 Weak；cert_lineage 是 Arc<()> | **不能只改 evidence_store**；cache 与其函数签名同包迁移，强弱身份判据保留；lineage 轮换新 allocation 也要原 scope，不能遗留裸 Arc::new(()) |

cache 的 `source bytes` 单预算只决定可选保留资格，和真实 Native 内存父账不是一个统计。不得用 memo.certs_bytes/handle.cost 当许可退款，也不因浅 clone 又向当前请求父账签一份共享费用。跨请求对象必须保留其原 token/parent：若全链合同要求新请求与历史对象同一父账，应在 owner/session 层复用既有父身份；shared wrapper 无法把两个既存父账自动统一，不得偷偷转账或新建父账。lineage 默认构造无 scope 与 admitted 轮换的边界须在接线时明确，不能为它随意找一个临时 budget。

## 建议文件归属

1. **共用 owner 实现工位**：新增 `rust/src/bin/s_session_v3/native_shared.rs`；仅主控协调修改同目录 `mod.rs` 加 `pub(crate) mod native_shared`。选择 bin 级共用模块可让 v2/cache 与 v3 同用，又不撞 A/B 正在改的库 native_owned.rs。引用现有公开 NativeReservation/WorkScope，不修改 scratch.rs。含核心强弱单元锁；第一版不要顺手改业务消费者。
2. **C1**：source_prefix.rs 的坐标共享；**C2**：source_contributions.rs、source_order.rs 及必要 source_pending 签名；共同使用已冻结 owner API，不各造 lease。
3. **D1**：object_index.rs 的 HistoryEvents；保留真实 facade/唯一可变借用合同。
4. **D2-cache 专包**：v2/cache.rs + v2/evidence_store.rs，连同类型签名、Weak lineage、子内容 owner 和旧收点一起迁；须从现 D2 所有者协调接管这两文件，不能双工位同时写。其他 D2 文件可以独立推进。

不触碰 `construction_state.rs`、`audit_owner/native.rs`、`s_content/capture_allocation/scratch.rs`。Retained 统计调整由消费工位逐分配解释，不能因为全局换 owner 就删除所有旧 reserve/release。

## 必要验证矩阵（本轮全部 NOT_RUN）

| 层 | 正负控与预期 |
|---|---|
| 独占强句柄 | 准入失败不造共享对象；一次成功计 value+lease 完整块；最后 drop 回到起始父 live；clone N 次不变，父容器变大另计 |
| 弱尾部 | 建 strong+weak，最后 strong drop 后 T 的动态缓冲已退，但完整 value block+lease 均仍计；upgrade=None；最后 weak drop 后归基线 |
| 多弱/复活 | downgrade/weak.clone/成功 upgrade 不新签费；drop 顺序全排列；不能把只有 Weak 的对象复活，空 Weak 无堆费 |
| Arc 并发 | 多线程并行最后 strong/weak 退出，恰一次最终退款，无 try_unwrap 竞争漏退；payload 还在 Drop 时另一线程退出不能先退总块 |
| metadata 与析构次序 | 测试-only 事件记录 value allocator 释放、lease allocator 释放、parent.release 的顺序；无法观测实际释放时只可声称 drop 顺序锁，不能冒充 allocator 证明；panic unwind 保留相同顺序 |
| 对齐/超限 | aligned T、空 slice、checked layout overflow、恰等完整帽、少一个 byte 的失败；include token/core padding；不按 size_of<T> 推断动态子分配 |
| Node | AVL 旋转、remove/reclaim、局部 Link 保留、Weak parent 尾部，输出 rank/handle/order 与旧核一致；多代删除重插不累计死费 |
| 坐标/Object | intern 命中 ptr_eq 和内容键不变；memo.clear 后外持 SourceCoords 不提前退款；Entry/StepContribution 相继 drop；失败保持旧缓存可用 |
| HistoryEvents | 双游标 clone 共用单块；旧 facade 存活则 get_mut 仍拒；最后游标与 cache drop 后释放子容器及外层块 |
| cache | evict 强证书而旧弱根仍在：证书 payload 释放，控制块持费；lineage rotation 使旧 Weak 匹配失败；两个 request 复用不重收，旧父依然可查 |
| 集成 | 原六成功、16/8 MiB、同父峰恰帽、跨代短命释放、尾检失败不得 COMMIT。额外 lease 成本如果使原成功失败，返回实现优化，不更改合同 |

只有 API 和以上核心强弱锁经独审后才能分派消费者批量替换；本设计不授权本机构建、服务、外盘访问、远端运行、合入或关票。
