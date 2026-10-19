# #1488 C2：来源顺序树真实 owner 迁移

2026-10-17，绑定 #1488 的工作草稿。只修改恢复树 `resume-1488/rust/src/bin/s_session_v3/source_order.rs` 与本报告。状态：实现候选，待独审；新增和既有 Rust 测试全部 **NOT_RUN**，不是整树可编译、原六成功或全图验收。

## 修改

`Order.nodes` 从裸 BTreeMap 改为既有 NativeTree；本文件 Rc/Weak 别名改指向 NativeRc/NativeWeak，故 child、parent、root、索引和旋转临时句柄全共享原 allocation 许可，不暴露裸强弱指针。Node 的 payload 只有固定标量及这些共享句柄，`new_node` 因而是本消费者具名 shallow 适配器；不拿 shallow 接口认证未持费 String/Vec 等动态字段。

原 `native_tree_room`、`native_admit` 两个分离收费点移交给 NativeTree::insert_admitted 与 NativeRc::try_new_shallow，从当前 CodecRead 的原 WorkScope 先准入。无 scope 保持原普通使用方式，不新增生产逃逸分支。插入先算下一个 handle、构造持费 Node、核统计溢出并准入树，再提交 next、node_bytes 和 AVL 变更。两处资源失败均不消耗 handle，不动已有索引/root/parent；树许可缓冲若因失败阶段已实际扩容，则依其真实 owner 保留到 drop，不能伪装退款。

AVL 的 size/height、旋转、balance、join、merge、split、rank/value 判据及原两项普通顺序测试保持原算法。remove 仍将 live 撤销并脱树，reclaim 仍拒绝活句柄/重复回收。移出索引后 NativeRc/Weak 的实际存活决定许可生命周期，不能按 reclaim 立即退共享块费。索引空时沿原逻辑 clear，树的实际 permit 缓冲仍按其 owner 保留。

结构统计调用主控新增的 `NativeRc::allocation_bytes` 与 `NativeTree::allocation_bytes`，计入共享 lease 和树许可缓冲。Order 新增 `node_bytes`，在插入/回收时更新，保持原 retained_bytes 的 O(1) 复杂度，避免每次保留费查询扫描全 AVL。该字段只为结构统计，绝不调用父账 release，也不替外部临时强/弱 owner 的真实持费。树高水位统计保留到 clear/drop；已回收索引外仍活的句柄由原 RAII 许可维持实际父账。计数核使用 checked 加减，测试 check 重新枚举索引核对统计。

## 必要源码锁

新增三项（`s_capture_native_testonly`），均 **NOT_RUN**：

1. `native_order_reclaimed_node_and_weak_tail_keep_original_parent_charge`：浅 clone/upgrade 不重计，remove/reclaim/drop 树后外部强 owner 持费，最后强消失 Weak 仍保整块，跨工作域后最后 Weak 销毁归父账基线。闭包内使用 scope_baseline，域外使用原 baseline，未漏工作域 metadata。
2. `native_order_admission_failure_preserves_avl_and_next_handle`：分别制造共享 Node 准入失败及仅够 Node 而树槽准入失败，核 root 指针、原索引、顺序、next 不变。首次失败后立即返回失败工作域；重试在新的工作域中进行，未在 sticky-failure scope 中吞错继续。
3. `native_order_rotations_remove_reinsert_and_drop_release_all_nodes`：同一真实父账下 64 次多位置插入、64 次移除/回收/头部重插，逐步核 AVL 平衡、父指针、rank/value/handle 与结构计数；清空/drop 后回到 scope 和域外基线。

这些是待运行源码，不能声称生命周期或资源帽已实测通过。普通 fixture 的 Vec 仅属测试参照序列，不作为生产持费 owner。

## 检查与依赖

已读 CLAUDE.md、Serena manual、来源消费者方案及共享 owner/树 API 独审。代码图对恢复符号无命中后精读本文件和相关接口。单文件 `rustfmt --edition 2021`、本文件 `git diff --check` 均 exit 0；未运行 cargo/rustc、服务、HTTP、浏览器、外盘、远端任务或回放，未 commit/push/合入/关票。未改其他工位文件或资源帽。

依赖主控对两项新增 allocation_bytes 只读统计 API 的独审；共享 owner 与 NativeTree get_mut/remove_entry 原有窄审已通过。最终仍需真实编译、运行本片测试和原六成功合同。新共享 lease/更大句柄对 16/8 MiB 帽的影响未验证。

源码 SHA-256：

```text
3b57603cded0ad4313b4e0567bd73507c810ed5be737a6ff83c7ad28085c324f  rust/src/bin/s_session_v3/source_order.rs
```
