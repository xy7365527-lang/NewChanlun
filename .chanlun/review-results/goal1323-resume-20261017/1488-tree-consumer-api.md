# #1488 NativeTree 消费接口补充

2026-10-17，绑定 #1488 的工作草稿。主控独占修改恢复树 `resume-1488/rust/src/bin/s_session_v3/audit_store.rs`。

为来源贡献索引的已知消费需求增加 `get_mut` 和 `remove_entry`。前者只借既有值，不提供键或树容量变更；动态字段仍须自身准入接口。后者把键值及其持有许可移出，保留原树节点高水位费直到 clear/drop。不提供 DerefMut 或裸树消费出口，不改原收费、失败或业务判据。

新增源码测试 `native_tree_removed_entry_keeps_payload_charge_after_tree_and_scope_drop`：真实 Budget/WorkScope 下插入持费文本键和值，借用修改固定大小标量，检查缺键无变化、移出不退款、树销毁仍保留键值费用、跨 scope 后最终键值销毁回基线。**NOT_RUN**，未运行 cargo/rustc。单文件 rustfmt 通过。该变更尚待独审，不覆盖其他消费文件或六成功合同。

## 独审返修与统计接口续接

独审指出闭包内的 baseline 漏计仍存活的 WorkScope metadata；已改为闭包入口的 scope_baseline，闭包外仍用外层 baseline。窄复审限定静态通过，见 `1488-tree-api-review.md`；未执行测试。

为 source_order 与共享坐标的保留结构统计补充只读 `allocation_bytes()`：NativeRc/Arc 共用原 Layout 核，有 lease 时包含 value+lease，无 scope 时只计 value；不包含 T 的动态子字段。NativeTree 返回 tree_slots(high_water) 加许可缓冲容量的保守结构上界。均不读父账差额、不退款、不再签费。`source_contributions::coords_bytes` 的共享分支接入同一方法，修复旧两计数头估计遗漏 lease 的统计缺口。新统计接口与该单行消费者改动仍待独审，不扩大前一版 PASS 指纹。
