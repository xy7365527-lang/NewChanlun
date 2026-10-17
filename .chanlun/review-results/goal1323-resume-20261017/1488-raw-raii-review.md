# 代码审查报告：#1488 A/B raw RAII 首条切片

日期：2026-10-17。工作树：`.sandcastle/worktrees/resume-1488`；基线 `c2c9c5640a99b98299456fd8b837c58035a2f4a6`。

**结论：PASS（仅下述冻结源码切片的静态审查）；NOT_COMPILED / NOT_RUN。不能据此批准合入、关闭 #1488 或宣称 #1323 全图验收。**

## 概念层质询结果

通过，限定于本次资源所有权变更。raw 的 identity/revision/ordinal 优先级、先按 identity 验证最终坐标、坐标碰撞拒绝和 ordered 输出顺序未改变；没有引入宽松判据或失败后的另一套业务判断。此切片未修改缠论领域定义、Classification 或 Chong 判据，不据实现反推教义。

本次核对的资源概念是：Native 结构上界与 Owned requested Layout 分账；原分配许可随真实载荷转移；同父预算内扩容先计完整新块与仍存活旧块；载荷析构先于对应许可；统计 retained 不能代替真实 RAII 退账。源码未发现与这些要求冲突的确定问题。

构造并逐路径检查的反例包括：Vec/map 满容量扩容被拒绝、空 buffer、非空未持许可的 Plain buffer、IntoIter 提前析构、元素移出迭代器后继续持有、树 remove/reinsert、clear 后保留许可向量容量、scope 结束但 facts/token 仍存活、坏 transport 字段复制与后续验证的错误顺序。现有三条 raw 新测试每次从空事实开始，不能当成跨事务恢复或全生产测试。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

本表只列置信度超过 80% 的缺陷，不把尚未执行的测试写成通过。

### 已核实的结构

1. `CodecVec` 的 storage 和 `CodecVecIntoIter` 的 iterator 在 token 字段之前析构。`into_iter` 同时转移 storage 与原 token；移出的 `NativeValue` 自持其 payload 许可。`clear`/`truncate` 保留 backing buffer 的许可；禁止带 token 的裸 `into_plain` 和容器 clone。显式 Native reserve 拒绝已有非零容量但无来源许可的非 ZST buffer。
2. `CodecVec::reserve_native_work` 和 `BusinessMap::try_reserve_native` 都先 reserve 新完整块、fallible 分配独立容器，随后搬迁，再销毁旧容器，最后替换旧许可。资源拒绝发生在搬迁前。map 插入已有键不要求增长；新键在可用容量耗尽时不能经旧 insert 暗扩容。
3. `NativeValue` 的原 permit 随 payload 返回，无公开裸移出、DerefMut 或 Clone。String 与 ContentRef 先 reserve 后复制；ContentRef 不提前 digest。typed path/ref 输出在原 reserve 点附 token，未先 retain 再补签。`ScopedFrame::into_native` 同时转移真实 frame 和原 token。
4. `NativeTree` 用节点上界许可和许可向量共同持账。许可向量由 Native CodecVec 分配，元素大小包含 token 元数据；remove 保守留节点许可，高水位内重插不累加历史许可，clear 真清树并释放节点许可，但保留许可向量的 backing capacity 账。原树字段在许可向量之前析构。节点预算上界与 retained 统计不是同一数值，不能据两者差额手动 release。
5. `RawPrefixFacts` 的 winner key、两张 map、ordered 树以及 changes 的三个向量都持有相应 owner。`RetainedDelta` 使用新的 key 类型大小，并计入 ordered 许可向量容量变化；统计修改本身不调用预算 release。
6. `audit_owner.rs` 末尾新测试确实调用 `PendingRawState::advance`，再走 `BoundRawAdvance::into_issued` 与 `bind`，scope 返回后分别 drop pending/token。同父重复三轮、最后回基线的断言存在，但未执行。
7. 当前正式 publication 从 `candidate.resources.clone()` 建工作域，并把该 scope 传给 raw advance；此路径没有另起 raw Budget。非 publication 的 `with_bound_raw_advance` 仍另建 Budget 并预收 previous retained，这是既有独立 helper 边界，本轮没有把它当成跨父账等价证明。
8. 冷候选 `CompleteCandidate::capture` 的有 parent 分支复用 parent，未再 `Budget::new(limit, charge.total)`。raw 本次替换的 reserve 点未找到配套旧手动 release。`validate_effects` 的 row IDs 仍是 `budget.vec_grow` 创建的标准 Vec，其原手动 release 不属于本次 RAII payload；`detach_slot` 修改的是统计字段，没有再次释放这些 token。

### 限定与后续验收

- 共享 `native_codec_room`/`native_map_room` 的变更已影响 raw 之外消费者。TextValue、RefValue、JSON、ScopedFrame 兼容输出及旧 retain 的完整迁移是已知 C/D 待办，未作为本轮新缺陷重复报告；也不能由本报告推导为已闭合。
- 新容器不验证旧 token 与新 WorkScope 的父账身份。正式 publication 当前复用候选父账，但通用 API 不能独立证明任意跨父调用正确。后续生产验收需要真实 warm restore/连续事务证据，不能只复用“新建后销毁三次”的测试。
- 许可内嵌、容器容量按真实元素类型收费；本轮没有证明全进程 allocator 或任意显式 payload clone、slice sort 均受 Native 账覆盖。
- 原 16/8 MiB、六成功锁、完整冷审计、恢复和历史回放均未执行。测试源码存在及静态 PASS 不能替代这些证据。
- 新文件 `native_owned.rs` 当前为 untracked；本审查已读取并纳入哈希。后续交付必须纳入候选，不能只靠 tracked diff 打包。

## 实际验证

已读取未暂存 diff、暂存 diff 状态、相关上下文及 A/B 实装报告。`git diff --check` 退出 0；暂存 diff 为空。三禁区相对工作树基线无 diff：`construction_state.rs`、`audit_owner/native.rs`、`capture_allocation/scratch.rs`。没有改动源码，没有 cargo/rustc、产品执行、服务、外盘或网络操作。

## 冻结指纹

路径均相对于恢复工作树。

```text
844dd7163a4a832647aaafc9d4526cc6027afd23da791a4b69a92202c2c77d7d  rust/src/bin/s_session_v3/audit_store.rs
53a8715adc2d576a342e9d16503e2bb93e4bf681ccab21fd10e842d2dca40f9b  rust/src/bin/s_session_v3/business_json.rs
f5b373e87cffc0763a94648b50c19df4f447844e062679ec6b1bd3c70d1e84d0  rust/src/bin/s_session_v3/audit_retention.rs
7cec900e7c15d0f21c1e6c71a29c2d5b52e4ac1e115e306ddfadb0935afb8c2c  rust/src/s_content/capture_allocation/work.rs
9cce0fd1396d7842a5c7fa9ed7bc7e6209189b7356865be6443899c5c66653c2  rust/src/s_content/capture_allocation/audit_json.rs
d92605a9369007ba053f50e4501329192a90ff71a6d7a555977b7d022acccc52  rust/src/s_content/capture_allocation/native_owned.rs
d556a50a52a372a97744e8286899df7d2298527b30bb38e9aee0b7c76e7078b9  rust/src/s_content/ref_owner.rs
c70015408cd47da08d8947b949d3917c300780134681fcb9f7095d987c30f61b  rust/src/s_content/path.rs
de26366926d42c21747c97dcdb19f06fd5c23b542e780882de270a3dfe729a60  rust/src/s_content/closed.rs
9f6c3b48c3447a0a82de397029658f164d0f8b7ea06c687c024c8b43b8581d34  rust/src/s_content/job.rs
6d15b7b5277d721f2c49e911db6a3556d7fc696d550c190acf46b72e81ac6cab  rust/src/bin/s_session_v3/audit_owner.rs
```

`audit_owner.rs` 指纹含主控及其他工位的前序变更；本报告独审其末尾 raw 生命周期新增测试、raw 父账调用边界及与本切片有关的计费交接，不表示重新通过整个文件的所有改动。
