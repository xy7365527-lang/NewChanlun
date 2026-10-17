# #1488 C1a scoped memo 生命周期测试源码补齐

日期：2026-10-17。恢复树：`.sandcastle/worktrees/resume-1488`。范围仅 `rust/src/bin/s_session_v3/source_audit.rs` 的 `tests` 模块；未改生产代码。承接 `1488-source-values-review.md` 的字段命中副本和 witness 父账读数缺口。

## 新增源码锁

1. `native_field_memo_scoped_hit_copies_clear_and_drop_keep_original_permits`（约 3077 行）：真实非空二字段 Merkle 原件，经 Native Budget / WorkScope / captured HashReader / StoreView / ArrayAuditSource 完成一次 miss 和两次 hit。要求计数为 1/2，两个命中 ref 的非空 root 文本地址不同。把实际原缓存 map 及两副本移动出 scope，不复制缓存，也不修改生产准入。使用真实 `NativeFieldMemo::clear`，分别覆盖先清缓存、先销毁副本两种次序；两个副本逐一退等额独立许可，缓存清理不使副本失效。清后容量不变、空桶仍持费、重复 clear 不再退费，最后 memo 销毁才回父账基线。
2. `native_witness_memo_scoped_hits_clear_and_drop_keep_original_permits`（约 3174 行）：真实非空 raw-bars 数组，含 source_coord、OHLC、identity_key。用既有普通 identity 入口计算对照，再经真实 scoped HashReader 调用 memo。首次错误 ID 不留条目/费用；成功 miss 离开 scope 后仍持费；正确、错误、正确三次命中不重算身份且每次 scope 结束父账读数不增。直接清实际 entries 保留桶容量，最后 memo 销毁回基线，重复完整生命周期两次。

两项均以 `s_capture_native_testonly` 为 gate。使用原 Budget，没有第二账、额外退款、差额抵扣或把资源失败吞成成功。测试数据在无 scope 阶段构造，仅作为不可变来源夹具，不宣称其自身存储已计入生产父账。

## 验证与边界

- 单文件 `rustfmt --check --edition 2021 --config skip_children=true`：exit 0。
- 限定文件 `git diff --check`：exit 0。
- **Rust 测试、编译、运行父账读数均 NOT_RUN。** 未运行 cargo/rustc、产品、HTTP、浏览器、远端或外盘。
- 测试读数不按实现公式推算应退字节；仅要求独立副本等额退费、桶保留、最终基线等所有权不变量。
- `NativeReservation::retain` 不会因 codec scope 结束自动退款。这里没有抵扣这种残费；若所经生产读取/验证路径还有 legacy retain，最终回基线或命中稳定读数将失败，届时需修生产路径，不能降低断言。
- field 测试在 scope 内建立缓存，scope 外只移动原 map 到同类型 memo 后 clear/drop；不证明跨请求重新准入、完整 warm restore 或端到端成功。scalar memo 的 PlainJson scoped 命中生命周期仍未新增专门测试，完整审计集成及原六成功也未验。
- witness 的条目清理使用当前真实 `entries.retain`；该 memo 没有生产 `clear` API，未为测试新增生产 API。
- 已读取根 CLAUDE.md / Serena 手册。恢复树图查询结果为 0，故精读实际源码；未把主仓图视作恢复树新增符号证据。

## 文件摘要

`rust/src/bin/s_session_v3/source_audit.rs` SHA256：

`4b6275b8113a1acf4fdeef859894b5794529fc6b59792e9c6f281557cbcacc7f`

此摘要只标识本次落盘时点；其他工位后续修改后应重封，不代表整树冻结。
