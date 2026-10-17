# 代码审查报告

日期：2026-10-17，绑定 #1488。范围：恢复树 `resume-1488` 的 source_order.rs NativeTree/NativeRc/NativeWeak 迁移；native_shared.rs 和 audit_store.rs 的两个只读 allocation_bytes API；source_contributions.rs 的 coords_bytes 共享分支。其他并行改动不在本次认可范围。

## 概念层质询结果

**通过（限定本片）。** 本片的 Order 只维护 occurrence 顺序，不增加来源资格或缠论判据。节点固定标量加已持费共享句柄，符合此前批准的 shallow adapter 边界。索引结构保留量是统计，原父账退款仍由真实 owner 析构决定；没有用 reclaimed 节点统计或 retained 差额释放许可。

反例覆盖静态推演：Node 准入失败、Node 成功但树槽准入失败、撤销后外部强引用仍存活、最后强引用退出但 Weak 尚存、相同共享坐标内容、跨 scope 持有索引，以及无 testonly feature。后者首版发现编译层依赖缺陷，已在本轮返修并核对，见下文。

## 工程层审查

| 严重级别 | 未修数量 | 状态 |
|---------|---------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass，首版一项已修 |
| MEDIUM | 0 | info |
| LOW | 0 | note |

结论：**限定静态 PASS**。未发现当前字节中超过 80% 置信度的未修缺陷。Rust 编译、所有 Rust 测试及原六成功均 **NOT_RUN**。

### 已发现并当轮修复的 HIGH-1

首版 `NativeTree::allocation_bytes` 无条件调用 `super::audit_retention::tree_slots`，但 mod.rs 仅在 `s_capture_native_testonly` 下注册 audit_retention；因此默认 feature 下无法解析该模块。已向主控报告。主控现改为方法内固定大小槽位公式 `high_water * (11 * size_of::<(K,V)>() + 16 * size_of::<usize>())` 加 permit 缓冲容量，保留饱和运算；已直接核新字节不再引用 testonly 模块，且 mod.rs 的门未扩大。该确定依赖错误已消除；不冒称默认构建已经执行通过。

### 核查依据

- **AVL 与 handle**：diff 未改 size/height、旋转、balance、join、merge、split、rank/value 等主体；只将同族强弱句柄替换为封闭 owner。ptr_eq 与 upgrade 仍按实际 allocation，不把内容相等当身份相等。Node 动态关系只有左右强引用与父弱引用，没有新强引用环。
- **准入和失败**：insert 先 checked 计算 next，但不提交；new_node 在原 WorkScope 下准入 value+lease，再 checked 计算 node_bytes，最后通过 NativeTree 准入索引。只有这些可失败点全部完成后才提交 next/node_bytes 并改 AVL。Node 失败不触碰树；树失败会销毁局部 node 和浅 clone，原 root、next、索引不变。若 permit 缓冲已扩容，则保留其实际持费，不伪装回退到旧 live。
- **撤销与回收**：remove 沿原算法脱树并 live=false；reclaim 仍拒绝活节点/重复回收，移除索引后只 checked 减 node_bytes。局部 node 尚在时即使树已 clear，也仍持自身许可；外部强引用/Weak 延长的费用不由此统计退款。父弱引用和真实 lease 由已审共享族管理。
- **统计复杂度**：Order 按插入/回收 O(1) 更新 node_bytes，retained_bytes 不枚举节点；测试 check 才枚举校对。NativeTree 方法只读 high_water 和容量；NativeRc/Arc 方法只读 Layout::for_value 和 lease 是否存在，切片长度来自元数据，均 O(1)。
- **统计范围**：共享 owner 使用与准入相同的 shared_layout/shared_bytes 核；有 lease 计两块，无 scope 仅 value 块。T 动态子分配不混入浅共享结构量。NativeTree 计已到达的高水位节点上界加 NativeReservation 缓冲容量；remove 不清高水位，clear 后 permit 缓冲容量仍保留。这个物理结构上界不等于累计深度预留 token 总和，也没有据此退款。父账仍由原 token 承担保守持费。
- **共享坐标**：当前 SourceCoords::Shared 的类型已是 NativeRc<[i64]>，新分支直接用 allocation_bytes，包含原两计数头估计遗漏的 lease；不复制坐标、不重新准入。多个归属统计可能保守重复计同一共享块，不能因此重复签费或退款；本片没有这些动作。
- **源码锁可行性**：三项新增 native_order 测试区分 scope_baseline 与域外 baseline；失败用例在 sticky-failure 工作域中仅观察并立即返回，重试另开 scope。前三节点后的高水位要求第四个新槽准入，因此“仅够 Node”用例确实触达树准入失败分支。必要 API 的类型/可见性静态可接，无确定新增类型错误；这不替代编译器。

## 已执行检查与边界

实际执行单文件 rustfmt --check（source_order、native_shared、audit_store）均 exit 0；四文件 git diff --check exit 0。图未索引恢复符号后精读了必要源及依赖。没有运行 cargo/rustc、正式服务、HTTP、浏览器、外盘、回放或远端任务，也没有修改源码、commit/push/合入/关票。

既有两项 Order 测试与新增三项测试全为 **NOT_RUN**。原资源帽、跨请求父身份、完整 source_contributions 迁移及六成功仍待独立整链验证，不能从本片静态 PASS 外推。

## 审核指纹

```text
3b57603cded0ad4313b4e0567bd73507c810ed5be737a6ff83c7ad28085c324f  rust/src/bin/s_session_v3/source_order.rs
2316d62e4ac9c8f36b62f956654b0c45068375ac05b39f31d7fc018581eb207d  rust/src/bin/s_session_v3/native_shared.rs
00ac83c8be22fb85751c549768ce7d0160898b7195002ef90c0c8ba5efe91be1  rust/src/bin/s_session_v3/audit_store.rs
1f0c02a54930a3fb34834249a58e376f747c3bba810c083c7c6b0a641aad3959  rust/src/bin/s_session_v3/source_contributions.rs
```

哈希用于标识当时完整文件字节；对后三文件仅认可本报告定义的新增 API/共享分支，不能把哈希当整文件审查授权。
